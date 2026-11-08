use super::{drafts::replace_items, store, types::*, vision::VisionPrefetch};
use crate::{
    command_audit::serialization,
    domain::{self, AppError, Result},
};
use serde_json::{Value, json};
use sqlx::{FromRow, Row, Sqlite, Transaction};
use std::collections::{HashMap, HashSet};

#[cfg(test)]
#[path = "canvas_tools_tests.rs"]
mod canvas_tools_tests;

/// 一张照片的画面事实是用哪一种口径问出来的。
/// 名字与下面 SQL 里的优先级分组一一对应：分组决定选哪条，名字负责把选中的这条说清楚。
struct DraftObservation {
    visible: Vec<String>,
    interpretation: String,
    contract: &'static str,
    prompt_version: String,
    kind: String,
}

pub(crate) fn observation_contract(priority: i64) -> &'static str {
    match priority {
        0 => "current_page_tier",
        1 => "current_detail_tier",
        2 => "other_prompt_version",
        _ => "other_tool",
    }
}

/// "同一张照片只留一条观察"的优先级写在这一处：当前页面档 > 当前细节档 > 同工具的其他版本 >
/// 其他工具，同级取最新一条。评估画面依据和界面补逐张口径都从这里挑，两处各写一遍的话，
/// 界面上说的"这张是按哪一档问的"就可能和真正拿去比对的口径不是同一把尺子。
/// 绑定顺序：当前页面档前缀 ×2、当前细节档前缀 ×2、照片范围、项目。
const OBSERVATION_PRIORITY_SQL: &str = r#"
      SELECT o.id, o.photo_id, o.observation_json, o.preview_sha256, o.prompt_version, o.kind, p.preview_path,
        p.original_path, p.content_sha256,
        CASE WHEN o.kind='inspect_photos' AND substr(o.prompt_version,1,length(?))=? THEN 0
             WHEN o.kind='inspect_photos' AND substr(o.prompt_version,1,length(?))=? THEN 1
             WHEN o.kind='inspect_photos' THEN 2 ELSE 3 END AS contract
      FROM photo_observations o JOIN photos p ON p.id=o.photo_id AND p.content_sha256=o.photo_hash
      JOIN json_each(?) scope ON scope.value=p.id
      WHERE p.project_id=? AND length(CAST(o.observation_json AS BLOB))<=65536
    "#;

/// 给这批照片每张挑一条观察。`with_bodies` 决定要不要连观察正文一起取回来：评估要比事实，
/// 界面只说"哪一档"，没必要把几百份正文搬进每次加载都跑的投影。
pub(crate) async fn pick_observations(
    tx: &mut Transaction<'_, Sqlite>,
    scoped: &[String],
    project_id: &str,
    with_bodies: bool,
) -> Result<Vec<sqlx::sqlite::SqliteRow>> {
    let columns = if with_bodies {
        "photo_id, observation_json, preview_sha256, prompt_version, kind, preview_path, contract, original_path, content_sha256"
    } else {
        "photo_id, contract"
    };
    let sql = format!(
        "SELECT {columns} FROM (SELECT *, ROW_NUMBER() OVER (PARTITION BY photo_id ORDER BY contract, id DESC) AS pick FROM ({OBSERVATION_PRIORITY_SQL})) WHERE pick=1"
    );
    let scope_json = crate::command_audit::encode(&scoped)?;
    Ok(sqlx::query(&sql)
        .bind(format!("{}@", super::vision::VISION_PROMPT_VERSION))
        .bind(format!("{}@", super::vision::VISION_PROMPT_VERSION))
        .bind(format!("{}@", super::vision::DETAIL_PROMPT_VERSION))
        .bind(format!("{}@", super::vision::DETAIL_PROMPT_VERSION))
        .bind(&scope_json)
        .bind(project_id)
        .fetch_all(&mut **tx)
        .await?)
}

pub(crate) async fn execute(
    tx: &mut Transaction<'_, Sqlite>,
    task: &mut Task,
    call: &ToolCall,
    vision_data: Option<&VisionPrefetch>,
    semantic_data: Option<&super::photo_search::SemanticPrefetch>,
) -> Result<Value> {
    let a = &call.args;
    let canonical = super::policy::ToolRegistry::canonical(&call.name);
    let name = canonical.as_deref().unwrap_or(call.name.as_str());
    match name {
        "explain_photo_judgment" => super::photo_judgment::explain(tx, task, a).await,
        "read_creative_context" | "fork_creative_draft" => {
            return super::creative::execute(tx, task, call).await;
        }
        "search_photography_knowledge"
        | "search_photographers"
        | "get_photobook_structure"
        | "compare_photography_series" => super::knowledge::execute(None, name, a).await,
        "propose_preference" => return super::preference_proposal::propose(tx, task, a).await,
        "create_board_draft" | "place_photos_on_board" | "get_board" => {
            return execute_canvas_tool(tx, name, &call.args, task).await;
        }
        "group_nodes" | "create_sequence_draft" => {
            return execute_canvas_tool_extended(tx, name, &call.args, task).await;
        }
        "move_nodes" | "remove_nodes" | "create_connector" | "add_editorial_note" => {
            return super::canvas_edit::execute(tx, name, a, task).await;
        }
        "get_project_context" => {
            let row = sqlx::query("SELECT brief,revision FROM projects WHERE id=?")
                .bind(&task.project_id)
                .fetch_one(&mut **tx)
                .await?;
            let mut facts = photos(tx, task).await?;
            let high = facts
                .iter()
                .filter(|p| p["rating"].as_i64().unwrap_or(0) >= 4)
                .count();
            facts.truncate(8);
            Ok(
                json!({"project_id":task.project_id,"revision":row.get::<i64,_>("revision"),"brief":row.get::<String,_>("brief"),"scope_count":task.allowed_photo_ids.len(),"selected_ids":task.selected_photo_ids,"high_rated_count":high,"human_facts":facts,"mock":true}),
            )
        }
        "get_sequence" => {
            let ids: Vec<String> = sqlx::query_scalar(
                "SELECT photo_id FROM sequence_items WHERE project_id=? ORDER BY position",
            )
            .bind(&task.project_id)
            .fetch_all(&mut **tx)
            .await?;
            let mut scoped: Vec<_> = ids
                .into_iter()
                .filter(|id| task.allowed_photo_ids.contains(id))
                .collect();
            for id in &task.selected_photo_ids {
                if !scoped.contains(id) {
                    scoped.push(id.clone());
                }
            }
            Ok(json!({"photo_ids":scoped,"base_revision":task.base_revision}))
        }
        "list_photos" | "search_photos" | "search_photos_semantic" => {
            let mut data = photos(tx, task).await?;
            if call.name == "list_photos" {
                if let Some(status) = a["status"].as_str() {
                    data.retain(|p| p["status"] == status);
                }
                if let Some(cursor) = a["cursor"].as_str() {
                    let n = cursor
                        .parse::<usize>()
                        .map_err(|_| AppError::new("VALIDATION_ERROR", "分页 cursor 不合法：首个分页请传 JSON null（不加引号）或字符串 \"0\"；续页为十进制偏移字符串，如 \"100\"；空串和字符串 \"null\" 均不合法。"))?;
                    data = data.into_iter().skip(n).collect();
                }
            } else {
                return super::photo_search::search(
                    tx,
                    task,
                    a["query"].as_str().unwrap(),
                    a["limit"].as_u64().unwrap() as usize,
                    data,
                    call,
                    semantic_data,
                )
                .await;
            }
            let limit = a["limit"].as_u64().unwrap() as usize;
            let total = data.len();
            data.truncate(limit);
            Ok(json!({"photos":data,"remaining":total.saturating_sub(limit),"mock":true}))
        }
        "inspect_photos" | "compare_photos" | "inspect_photo_detail" => {
            let ids: Vec<String> =
                serde_json::from_value(a["photo_ids"].clone()).map_err(serialization)?;
            let mut data = photos(tx, task).await?;
            data.retain(|p| ids.iter().any(|id| p["photo_id"] == *id));
            match vision_data.filter(|v| v.kind == call.name) {
                Some(prefetch) => {
                    let mut provenance = Vec::new();
                    // 观察落缓存（photo_hash 维度），配额/用量/费用同事务扣减。
                    for (photo_id, observation) in &prefetch.observations {
                        let hash: Option<String> = sqlx::query_scalar(
                            "SELECT content_sha256 FROM photos WHERE id=? AND project_id=?",
                        )
                        .bind(photo_id)
                        .bind(&task.project_id)
                        .fetch_optional(&mut **tx)
                        .await?;
                        // 图像版本跟着观察走：核对过或原先记录过的指纹原样落库；
                        // 无从得知时留空，写上当前的指纹会把"没核对"洗成"已核对"。
                        let version = prefetch
                            .image_versions
                            .iter()
                            .find(|v| v.photo_id == *photo_id);
                        let preview_sha256 = version.and_then(|v| v.preview_sha256.clone());
                        if let Some(hash) = hash {
                            provenance.push(json!({"photo_id":photo_id,"photo_hash":hash,"model_id":prefetch.model_id,"prompt_version":prefetch.cache_version,"preview_sha256":preview_sha256,"pixelVersion":version.map(|v| v.status.as_str()).unwrap_or("unrecorded")}));
                            sqlx::query("INSERT INTO photo_observations(photo_id,photo_hash,model_id,prompt_version,kind,observation_json,preview_sha256,created_at) VALUES(?,?,?,?,?,?,?,?)")
                                .bind(photo_id)
                                .bind(&hash)
                                .bind(&prefetch.model_id)
                                .bind(&prefetch.cache_version)
                                .bind(if call.name=="inspect_photo_detail" {"inspect_photos"}else{call.name.as_str()})
                                .bind(observation.to_string())
                                .bind(preview_sha256)
                                .bind(domain::timestamp())
                                .execute(&mut **tx)
                                .await?;
                        }
                    }
                    // 外发在网络请求前预留、返回后结算；此处只落观察，不能重复扣费。
                    let observations: Vec<Value> = prefetch
                        .observations
                        .iter()
                        .map(|(id, o)| {
                            json!({"photo_id":id,"visible_facts":o["visible_facts"],"interpretation":o["interpretation"],"uncertainty":o["uncertainty"]})
                        })
                        .collect();
                    let uploaded = prefetch.uploaded;
                    // 看过画面的只有拿到观察这些张：上传数里得扣掉模型漏答的那几张，
                    // 否则丢一张就被算成一次缓存命中，看着像"没花钱也已经看过了"。
                    let observed: Vec<&String> =
                        prefetch.observations.iter().map(|(id, _)| id).collect();
                    // 两份名单互不相交：一份是"出去了没人看"，一份是"根本没出去"，
                    // 请求里的照片正好等于"看过 + 这两份"，不重复也不遗漏。
                    let unobserved = prefetch.dropped.clone();
                    // 没送出去的每张各带一个原因：是原图不在这台机器上，还是原图内容
                    // 已经和库里的记录对不上，用户能做的事完全不同。
                    let unreadable_photos: Vec<Value> = prefetch
                        .unreadable_reasons
                        .iter()
                        .map(|(id, reason)| json!({"photo_id": id, "reason": reason}))
                        .collect();
                    let cached = (prefetch.observations.len() as u32)
                        .saturating_sub(uploaded.saturating_sub(prefetch.dropped.len() as u32));
                    Ok(
                        json!({"photos":data,"mode":"vision_observed","uploaded_images":uploaded,"cache_hits":cached,"observations":observations,"provenance":provenance,"requested_photo_ids":ids,"observed_photo_ids":observed,"unobserved_photo_ids":unobserved,"unreadable_photo_ids":prefetch.unreadable,"unreadable_photos":unreadable_photos,"preview_rebuilt_photo_ids":prefetch.rebuilt,"coverageNotice":"只有 observed_photo_ids 里的照片实际读过画面；unobserved_photo_ids 已经上传但没有得到观察，不能当成看过，也不要为它写单张判断依据；unreadable_photo_ids 是本地预览读不出来、一个字节都没送出去的，既没占额度也没被看过，每张的原因见 unreadable_photos（original_missing=原图不在这台机器上，original_unreadable=原图也解不开，original_changed=原图内容与库里记录不一致因此不用它重算，rebuild_failed=重算失败，rebuild_over_limit=这张没排上重算）。preview_rebuilt_photo_ids 是本地预览坏了但已按原图只读重算出像素并真的看过的那些张，可以照常引用；图库里那份缓存文件本轮没有改写。","resolutionPolicy":if call.name=="inspect_photo_detail" {"preview1400-high-v1"}else{"preview512"},"uncertainty":"主观解读与不确定性由模型标注；服务端已校验 photo_id 均属请求集合","ladderHint":"顺序判断只认这份 requested_photo_ids 的先后；画面细节按概览→抽样检查→按顺序比较逐级收窄，别一次传整批。"}),
                    )
                }
                None => Ok(
                    json!({"photos":data,"mode":"mock_metadata_only","uncertainty":"缺少图像授权或提供方不支持视觉；仅对照人工评分/备注，未读取像素"}),
                ),
            }
        }
        "get_comments" => {
            let photo = a["photo_id"].as_str().filter(|id| !id.is_empty());
            let limit = a["limit"].as_u64().unwrap_or(10).min(50) as usize;
            let listing = super::comments::list_tx(tx, &task.project_id, photo, limit).await?;
            Ok(json!({"threads": listing["threads"], "notice": listing["notice"]}))
        }
        "retrieve_preferences" => {
            let limit = a["limit"].as_u64().unwrap_or(5).min(10) as usize;
            if a["scope"] == "global_confirmed" {
                let memory = super::memory::retrievable_author(tx, task, limit).await?;
                return Ok(
                    json!({"hypotheses":memory["hypotheses"],"human_facts":[],"mode":memory["mode"],"notice":memory["notice"]}),
                );
            }
            let mut memory =
                super::memory::retrievable_for_goal(tx, &task.project_id, limit, &task.goal)
                    .await?;
            super::preference_proposal::filter_for_task(tx, task, &mut memory).await?;
            let mut facts = photos(tx, task).await?;
            facts.retain(|p| {
                p["rating"].as_i64().unwrap_or(0) > 0
                    || p["note"].as_str().is_some_and(|s| !s.is_empty())
            });
            facts.truncate(limit);
            Ok(
                json!({"hypotheses":memory["hypotheses"],"human_facts":facts,"mode":"evidence_backed_project_local","notice":"假设来自人工证据的确定性推断，附证据 ID；纠正后不再检索。Agent 草案不计为证据"}),
            )
        }
        "create_selection_draft" => {
            if a["base_project_revision"].as_i64() != Some(task.base_revision) {
                return Err(AppError::new(
                    "REVISION_CONFLICT",
                    "草案必须使用任务原始基线",
                ));
            }
            let items: Vec<DraftItem> =
                serde_json::from_value(a["items"].clone()).map_err(serialization)?;
            validate_items(&items)?;
            let id = domain::id();
            let now = domain::timestamp();
            sqlx::query("INSERT INTO drafts(id,project_id,task_id,title,base_revision,state,summary,created_at,updated_at) VALUES(?,?,?,?,?,'pending',?,?,?)").bind(&id).bind(&task.project_id).bind(&task.id).bind(a["title"].as_str()).bind(task.base_revision).bind(a["summary"].as_str()).bind(&now).bind(&now).execute(&mut **tx).await?;
            replace_items(tx, &id, &items).await?;
            Ok(json!({"draft_id":id,"draft_revision":1,"count":items.len(),"title":a["title"]}))
        }
        "update_draft_sequence" => {
            let id = a["draft_id"].as_str().unwrap();
            let revision: i64 = sqlx::query_scalar("SELECT revision FROM drafts WHERE id=?")
                .bind(id)
                .fetch_one(&mut **tx)
                .await?;
            if Some(revision) != a["expected_draft_revision"].as_i64() {
                return Err(AppError::new("REVISION_CONFLICT", "草案已更新"));
            }
            let old = store::items(tx, id).await?;
            let ids: Vec<String> =
                serde_json::from_value(a["ordered_photo_ids"].clone()).map_err(serialization)?;
            if ids.is_empty() {
                return Err(AppError::new("VALIDATION_ERROR", "草案不能空"));
            }
            let items: Vec<_> = ids
                .iter()
                .enumerate()
                .map(|(position, id)| {
                    old.iter()
                        .find(|i| &i.photo_id == id)
                        .cloned()
                        .map(|mut i| {
                            i.position = position as i64;
                            i
                        })
                        .unwrap_or(DraftItem {
                            photo_id: id.clone(),
                            proposed_action: "keep".into(),
                            position: position as i64,
                            rationale: "Mock：按已授权元数据修订候选".into(),
                        })
                })
                .collect();
            validate_items(&items)?;
            replace_items(tx, a["draft_id"].as_str().unwrap(), &items).await?;
            sqlx::query("UPDATE drafts SET revision=revision+1,updated_at=? WHERE id=?")
                .bind(domain::timestamp())
                .bind(id)
                .execute(&mut **tx)
                .await?;
            Ok(json!({"draft_id":id,"draft_revision":revision+1,"count":items.len()}))
        }
        "evaluate_draft" => {
            // 结构化本地评估：内容哈希完全重复、观察事实相邻相似、评分/状态分布与观察覆盖。
            // 叙事判断仍由策展模型结合本证据进行；不触发上传、不修改正式状态。
            let draft_id = a["draft_id"].as_str().unwrap();
            let items = store::items(tx, draft_id).await?;
            // 口径要说"对着哪一版算的"，所以把方案当时的版本号一并记进回执；取不到就当没记过。
            let draft_revision: Option<i64> =
                sqlx::query_scalar("SELECT revision FROM drafts WHERE id=?")
                    .bind(draft_id)
                    .fetch_optional(&mut **tx)
                    .await?;
            let facts = photos(tx, task).await?;
            let draft_ids: Vec<String> = items.iter().map(|i| i.photo_id.clone()).collect();
            // 1) 完全重复：草案内 content_sha256 相同的照片组。
            let hash_rows = sqlx::query("SELECT id, content_sha256 FROM photos WHERE project_id=?")
                .bind(&task.project_id)
                .fetch_all(&mut **tx)
                .await?;
            let mut by_hash: HashMap<String, Vec<String>> = HashMap::new();
            for row in &hash_rows {
                let id: String = row.get("id");
                if draft_ids.contains(&id) {
                    by_hash
                        .entry(row.get::<String, _>("content_sha256"))
                        .or_default()
                        .push(id);
                }
            }
            let mut identical: Vec<Vec<String>> =
                by_hash.into_values().filter(|g| g.len() > 1).collect();
            for group in &mut identical {
                group.sort();
            }
            identical.sort();
            // 2) 观察缓存：草案照片内每张最新观察（photo_hash 对齐，正文有界 64KiB）。
            let scoped: Vec<String> = task
                .allowed_photo_ids
                .iter()
                .filter(|id| draft_ids.contains(id))
                .cloned()
                .collect();
            let mut omitted_observations = Vec::new();
            let mut original_checks = super::vision::MAX_ORIGINAL_CHECKS;
            // 每张只留一条，留哪一条的规矩写在 `OBSERVATION_PRIORITY_SQL`：同一张照片可能同时存着
            // 当前页面档、更大尺寸的细节档、以及历史提示词版本留下的记录。这几种"画面事实"是在
            // 不同问法下答出来的，谁的编号大就听谁的，等于悄悄拿两种口径的字符串比相同程度。
            let observations: HashMap<String, DraftObservation> = if scoped.is_empty() {
                HashMap::new()
            } else {
                let rows = pick_observations(tx, &scoped, &task.project_id, true).await?;
                let mut observations = HashMap::new();
                for row in &rows {
                    let photo: String = row.get("photo_id");
                    let stored: Option<String> = row.get("preview_sha256");
                    let path: String = row.get("preview_path");
                    let version: String = row.get("prompt_version");
                    let contract = observation_contract(row.get::<i64, _>("contract"));
                    let kind: String = row.get("kind");
                    let status = super::vision::current_preview_status_for_version(
                        stored.as_deref(),
                        &path,
                        &version,
                    )
                    .await;
                    if status != "verified" {
                        // 没记指纹那一类本地永远翻不过来，不必再花名额去哈希原图。
                        let original = if status == "unrecorded" {
                            None
                        } else {
                            Some(
                                super::vision::original_status(
                                    &row.get::<String, _>("original_path"),
                                    &row.get::<String, _>("content_sha256"),
                                    &mut original_checks,
                                )
                                .await,
                            )
                        };
                        let mut entry =
                            json!({"photo_id":photo,"pixel_version":status,"contract":contract});
                        if let Some(original) = original {
                            entry["original_status"] = json!(original);
                        }
                        omitted_observations.push(entry);
                        continue;
                    }
                    let parsed = (|| {
                        let raw: String = row.get("observation_json");
                        let value = serde_json::from_str::<Value>(&raw).ok()?;
                        if !super::vision::valid_cached_observation(&value, &photo) {
                            return None;
                        }
                        let visible: Vec<String> = value["visible_facts"]
                            .as_array()?
                            .iter()
                            .filter_map(|fact| fact.as_str().map(String::from))
                            .collect();
                        if visible.is_empty() {
                            return None;
                        }
                        let interpretation = value["interpretation"]
                            .as_str()
                            .unwrap_or("")
                            .chars()
                            .take(160)
                            .collect::<String>();
                        Some(DraftObservation {
                            visible,
                            interpretation,
                            contract,
                            prompt_version: row.get("prompt_version"),
                            kind,
                        })
                    })();
                    if let Some(evidence) = parsed {
                        observations.insert(photo, evidence);
                    }
                }
                observations
            };
            // 3) 相邻相似：按草案顺序的相邻两张，visible_facts Jaccard >= 0.6。
            //    两种口径答出来的事实照样算，但要把各自口径带上，别让读者以为比的是同一把尺子。
            let mut cross_contract_pairs = 0usize;
            let mut similar_adjacent: Vec<Value> = vec![];
            for pair in items.windows(2) {
                let (Some(left), Some(right)) = (
                    observations.get(&pair[0].photo_id),
                    observations.get(&pair[1].photo_id),
                ) else {
                    continue;
                };
                let set_a: HashSet<&String> = left.visible.iter().collect();
                let set_b: HashSet<&String> = right.visible.iter().collect();
                let shared: Vec<&String> = set_a.intersection(&set_b).cloned().collect();
                let union = set_a.union(&set_b).count();
                if union > 0 && shared.len() as f64 / union as f64 >= 0.6 {
                    let same_contract = left.contract == right.contract;
                    if !same_contract {
                        cross_contract_pairs += 1;
                    }
                    // 每一对都得留在这份名单里：本地兜底去重读的就是这里的 photo_ids，
                    // 少列一对就少排除一张，选片结果会跟着变。所以省体积只能省在赘述上——
                    // 共用的是哪几条事实改成"共用几条"（正文本来就在 item_facts 的逐张里），
                    // 两边同一口径时不再把同一个口径名写两遍。
                    let mut row = json!({
                        "positions": [pair[0].position, pair[1].position],
                        "photo_ids": [pair[0].photo_id, pair[1].photo_id],
                        "shared_fact_count": shared.len(),
                        "same_contract": same_contract,
                    });
                    if !same_contract {
                        row["contracts"] = json!([left.contract, right.contract]);
                    }
                    similar_adjacent.push(row);
                }
            }
            // 4) 人工备注中的重复组提示（保留旧通道，作为 human_hint）。
            let mut groups: HashMap<String, Vec<String>> = HashMap::new();
            for i in &items {
                if let Some(note) = facts
                    .iter()
                    .find(|p| p["photo_id"] == i.photo_id)
                    .and_then(|p| p["note"].as_str())
                    && let Some(group) = note
                        .split("重复组:")
                        .nth(1)
                        .and_then(|s| s.split_whitespace().next())
                {
                    groups
                        .entry(group.into())
                        .or_default()
                        .push(i.photo_id.clone());
                }
            }
            let mut human_hints: Vec<Vec<String>> =
                groups.into_values().filter(|ids| ids.len() > 1).collect();
            for group in &mut human_hints {
                group.sort();
            }
            human_hints.sort();
            // 5) 结构指标与逐张事实（有界）。
            let observed = items
                .iter()
                .filter(|i| observations.contains_key(&i.photo_id))
                .count();
            let rated: Vec<i64> = items
                .iter()
                .filter_map(|i| {
                    facts
                        .iter()
                        .find(|p| p["photo_id"] == i.photo_id)
                        .and_then(|p| p["rating"].as_i64())
                })
                .collect();
            let rated_count = rated.len() as f64;
            let avg_rating = if rated.is_empty() {
                0.0
            } else {
                (rated.iter().sum::<i64>() as f64 / rated_count * 10.0).round() / 10.0
            };
            let status_count = |want: &str| {
                items
                    .iter()
                    .filter(|i| {
                        facts
                            .iter()
                            .find(|p| p["photo_id"] == i.photo_id)
                            .is_some_and(|p| p["status"] == want)
                    })
                    .count()
            };
            let item_facts: Vec<Value> = items
                .iter()
                .take(12)
                .filter_map(|i| {
                    let evidence = observations.get(&i.photo_id)?;
                    Some(json!({
                        "photo_id": i.photo_id,
                        "position": i.position,
                        "top_facts": evidence.visible.iter().take(3).collect::<Vec<_>>(),
                        "interpretation": evidence.interpretation,
                        "evidence_contract": evidence.contract,
                        "evidence_prompt_version": evidence.prompt_version,
                        "evidence_tool": evidence.kind,
                    }))
                })
                .collect();
            let contract_count = |want: &'static str| {
                items
                    .iter()
                    .filter(|i| {
                        observations
                            .get(&i.photo_id)
                            .is_some_and(|e| e.contract == want)
                    })
                    .count()
            };
            Ok(json!({
                "draft_id": draft_id,
                "draft_revision": draft_revision,
                "valid": true,
                "count": items.len(),
                "identical_groups": identical,
                "similar_adjacent": similar_adjacent,
                "human_hint_groups": human_hints,
                "metrics": {
                    "observed": observed,
                    "unobserved": items.len() - observed,
                    "rated": rated.len(),
                    "avg_rating": avg_rating,
                    "status_mix": {"keep": status_count("keep"), "undecided": status_count("undecided"), "reject": status_count("reject")},
                    "evidence_contracts": {"current_page_tier": contract_count("current_page_tier"), "current_detail_tier": contract_count("current_detail_tier"), "other_prompt_version": contract_count("other_prompt_version"), "other_tool": contract_count("other_tool")},
                    "cross_contract_adjacent_pairs": cross_contract_pairs,
                },
                "item_facts": item_facts,
                "omitted_visual_evidence": omitted_observations,
                "visual_evidence_policy": "Only observations matching the current preview pixels are used; no images were uploaded. Per photo one row is chosen by a fixed priority (current page tier, then current detail tier, then the same tool under another prompt version, then other tools), and each row states which one was used.",
                "mode": "structured_local_evaluation",
                "uncertainty": "重复性与结构指标为本地确定性计算（内容哈希与观察事实比对）；每张观察取的口径写在 evidence_contracts 里，同一张照片只留一条，优先当前页面档，跨口径的相邻相似对在 same_contract=false 里点明并带上两边口径，不能当成同一把尺子的比较；相邻相似对按方案顺序完整列出、不截断，但每对只报共用几条事实（shared_fact_count）而不抄文字，文字只在 item_facts 里，那里只带前 12 张、每张最多 3 条，第 12 张之后要看正文得回逐张口径或重新问；未观察照片不参与相似性比对；叙事判断由策展模型结合 item_facts 进行，人工评审保留最终决定。",
            }))
        }
        "request_feedback" => {
            task.question = Some(a["question"].as_str().unwrap().into());
            store::transition(tx, task, "needs_input", "等待人工答复").await?;
            sqlx::query("INSERT INTO task_messages(task_id,kind,body,created_at) VALUES(?,'clarification_question',?,?)").bind(&task.id).bind(&task.question).bind(domain::timestamp()).execute(&mut **tx).await?;
            Ok(json!({"question":task.question,"options":a["options"]}))
        }
        "finish_task" => {
            if a["task_id"] != task.id {
                return Err(AppError::new("PERMISSION_DENIED", "不能结束其他任务"));
            }
            let ids: Vec<String> =
                serde_json::from_value(a["draft_ids"].clone()).map_err(serialization)?;
            validate_final(tx, task, &ids).await?;
            crate::focal_chat::assistant(tx, &task.id, a["summary"].as_str().unwrap()).await?;
            task.final_draft_ids = ids;
            store::transition(tx, task, "review_ready", a["summary"].as_str().unwrap()).await?;
            let revision: i64 = sqlx::query_scalar("SELECT revision FROM projects WHERE id=?")
                .bind(&task.project_id)
                .fetch_one(&mut **tx)
                .await?;
            if revision != task.base_revision {
                store::transition(tx, task, "stale", "正式编辑已更新，请重新比较草案基线").await?;
            }
            Ok(json!({"draft_ids":task.final_draft_ids,"formal_state_unchanged":true}))
        }
        "finish_canvas_task" => {
            if a["task_id"] != task.id {
                return Err(AppError::new("PERMISSION_DENIED", "不能结束其他任务"));
            }
            let ids: Vec<String> =
                serde_json::from_value(a["board_ids"].clone()).map_err(serialization)?;
            if ids.is_empty()
                || ids.len() > 10
                || ids.iter().collect::<HashSet<_>>().len() != ids.len()
            {
                return Err(AppError::new(
                    "VALIDATION_ERROR",
                    "请选择当前任务的画布草案",
                ));
            }
            for id in &ids {
                require_task_board(tx, id, task).await?;
            }
            crate::focal_chat::assistant(
                tx,
                &task.id,
                a["summary"].as_str().unwrap_or("画布方案已准备好"),
            )
            .await?;
            store::transition(
                tx,
                task,
                "review_ready",
                a["summary"].as_str().unwrap_or("画布方案已准备好"),
            )
            .await?;
            Ok(json!({"board_ids":ids,"formal_state_unchanged":true}))
        }
        _ => Err(AppError::new("PERMISSION_DENIED", "未知工具")),
    }
}
pub(crate) async fn validate_final(
    tx: &mut Transaction<'_, Sqlite>,
    task: &Task,
    ids: &[String],
) -> Result<()> {
    if ids.is_empty() || ids.len() > 10 || ids.iter().collect::<HashSet<_>>().len() != ids.len() {
        return Err(AppError::new("VALIDATION_ERROR", "完成必须引用真实草案"));
    }
    for id in ids {
        let owned:i64=sqlx::query_scalar("SELECT COUNT(*) FROM drafts WHERE id=? AND task_id=? AND project_id=? AND state='pending'").bind(id).bind(&task.id).bind(&task.project_id).fetch_one(&mut **tx).await?;
        if owned != 1 {
            return Err(AppError::new(
                "NOT_FOUND",
                "最终结果引用了不存在或其他任务的草案",
            ));
        }
    }
    Ok(())
}
fn validate_items(items: &[DraftItem]) -> Result<()> {
    if items.is_empty()
        || items.len() > 500
        || items
            .iter()
            .map(|i| &i.photo_id)
            .collect::<HashSet<_>>()
            .len()
            != items.len()
        || items
            .iter()
            .enumerate()
            .any(|(p, i)| i.position != p as i64)
    {
        return Err(AppError::new(
            "VALIDATION_ERROR",
            "草案位置须连续且照片不得重复",
        ));
    }
    Ok(())
}
pub(crate) async fn photos(tx: &mut Transaction<'_, Sqlite>, task: &Task) -> Result<Vec<Value>> {
    let rows=sqlx::query("SELECT p.id,p.filename,d.status,d.rating,d.user_note FROM photos p JOIN photo_decisions d ON d.photo_id=p.id AND d.project_id=p.project_id WHERE p.project_id=? ORDER BY d.rating DESC,p.import_index").bind(&task.project_id).fetch_all(&mut **tx).await?;
    Ok(rows.iter().filter(|r|task.allowed_photo_ids.contains(&r.get::<String,_>("id"))).map(|r|json!({"photo_id":r.get::<String,_>("id"),"name":r.get::<String,_>("filename").chars().take(80).collect::<String>(),"status":r.get::<String,_>("status"),"rating":r.get::<i64,_>("rating"),"note":r.get::<String,_>("user_note").chars().take(160).collect::<String>()})).collect())
}

// ── G4 Agent Canvas Tools ──

pub(super) async fn require_task_board(
    tx: &mut Transaction<'_, Sqlite>,
    board_id: &str,
    task: &Task,
) -> Result<()> {
    let owned: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM boards WHERE id=? AND project_id=? AND state='draft' AND owning_task_id=?",
    )
    .bind(board_id).bind(&task.project_id).bind(&task.id)
    .fetch_one(&mut **tx).await?;
    if owned != 1 {
        return Err(AppError::new(
            "PERMISSION_DENIED",
            "只能编辑当前任务拥有的画布草案",
        ));
    }
    Ok(())
}

fn canvas_node_view(
    node: &crate::canvas::CanvasNode,
    photo: Option<&String>,
    source: Option<&String>,
) -> Value {
    json!({"id":node.id,"source_node_id":source,"photo_id":photo,"can_inspect":photo.is_some(),"type":node.node_type,"x":node.x,"y":node.y,"width":node.width,"height":node.height,"z":node.z_order,"asset_id":node.asset_id,"content":node.content_json,"children":node.children_json,"locked":node.locked})
}

pub async fn execute_canvas_tool(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    name: &str,
    args: &Value,
    task: &super::types::Task,
) -> Result<Value> {
    match name {
        "create_board_draft" => {
            let title = args["title"].as_str().unwrap_or("Agent 草案").trim();
            if title.is_empty() || title.chars().count() > 200 {
                return Err(crate::domain::AppError::new(
                    "VALIDATION_ERROR",
                    "标题须为 1–200 字符",
                ));
            }
            let id = crate::domain::id();
            let now = crate::domain::timestamp();
            // H0-03：画布绑定任务身份——Agent 只能写自己创建的沙盒
            sqlx::query("INSERT INTO boards(id,project_id,title,state,owning_task_id,created_at,updated_at) VALUES(?,?,?,'draft',?,?,?)")
                .bind(&id).bind(&task.project_id).bind(title).bind(&task.id).bind(&now).bind(&now)
                .execute(&mut **tx).await?;
            sqlx::query("INSERT INTO board_draft_tasks(board_id,task_id,created_at) VALUES(?,?,?)")
                .bind(&id)
                .bind(&task.id)
                .bind(&now)
                .execute(&mut **tx)
                .await?;
            super::task_scope::clone_source(tx, task, &id).await?;
            Ok(json!({"board_id": id, "title": title, "state": "draft", "owned_by_task": true}))
        }
        "place_photos_on_board" => {
            let board_id = args["board_id"].as_str().unwrap_or("");
            require_task_board(tx, board_id, task).await?;
            let placements = args["placements"].as_array().cloned().unwrap_or_default();
            // H0 复核：placements 内嵌套 photo_id 必须在任务授权范围内
            for p in &placements {
                let pid = p["photo_id"].as_str().unwrap_or("");
                if !pid.is_empty() && !task.allowed_photo_ids.contains(&pid.to_owned()) {
                    return Err(crate::domain::AppError::new(
                        "PERMISSION_DENIED",
                        format!("照片 {pid} 不在任务授权范围内"),
                    ));
                }
            }
            let mut placed = Vec::new();
            let now = crate::domain::timestamp();
            for p in &placements {
                let photo_id = p["photo_id"].as_str().unwrap_or("");
                // 通过 alias 解析 asset_id（限定项目内）
                let asset_id: Option<String> = sqlx::query_scalar(
                    "SELECT id FROM source_assets WHERE id=? AND project_id=? OR id=(SELECT asset_id FROM asset_aliases WHERE photo_id=?) AND project_id=?",
                )
                .bind(photo_id).bind(&task.project_id).bind(photo_id).bind(&task.project_id)
                .fetch_optional(&mut **tx).await?;
                let asset =
                    asset_id.ok_or_else(|| AppError::new("NOT_FOUND", "照片不属于当前项目"))?;
                let x = p["x"].as_f64().unwrap_or(0.0);
                let y = p["y"].as_f64().unwrap_or(0.0);
                let w = p["width"].as_f64().unwrap_or(220.0);
                let h = p["height"].as_f64().unwrap_or(160.0);
                crate::canvas::validate_rectangle(x, y, w, h)?;
                let node_id = crate::domain::id();
                let z: i64 = sqlx::query_scalar(
                    "SELECT COALESCE(MAX(z_order),0)+1 FROM canvas_nodes WHERE board_id=?",
                )
                .bind(board_id)
                .fetch_one(&mut **tx)
                .await?;
                sqlx::query("INSERT INTO canvas_nodes(id,board_id,node_type,x,y,width,height,rotation,z_order,asset_id,created_at,updated_at) VALUES(?,?, 'photo',?,?,?,?,0,?,?,?,?)")
                    .bind(&node_id).bind(board_id).bind(x).bind(y).bind(w).bind(h)
                    .bind(z).bind(&asset).bind(&now).bind(&now)
                    .execute(&mut **tx).await?;
                placed.push(json!({"node_id": node_id, "photo_id": photo_id, "x": x, "y": y}));
            }
            Ok(json!({"board_id": board_id, "placed_count": placed.len(), "nodes": placed}))
        }
        "get_board" => {
            let board_id = args["board_id"].as_str().unwrap_or("");
            let aliases:Vec<(String,String)>=sqlx::query_as("SELECT a.id,MIN(alias.photo_id) FROM source_assets a JOIN asset_aliases alias ON alias.asset_id=a.id JOIN photos p ON p.id=alias.photo_id AND p.project_id=a.project_id WHERE a.project_id=? AND alias.photo_id IN(SELECT value FROM json_each(?)) GROUP BY a.id")
                .bind(&task.project_id).bind(crate::command_audit::encode(&task.allowed_photo_ids)?).fetch_all(&mut **tx).await?;
            let aliases: HashMap<String, String> = aliases.into_iter().collect();
            let grant = super::task_scope::load(tx, task).await?;
            if grant
                .source
                .as_ref()
                .is_some_and(|source| source.board_id == board_id)
            {
                let baseline = super::task_scope::baseline(tx, task)
                    .await?
                    .ok_or_else(|| AppError::new("RECOVERY_ERROR", "缺少画布来源"))?;
                return Ok(
                    json!({"board_id":board_id,"revision":baseline.board.revision,"source":"task_baseline","nodes":baseline.nodes.iter().map(|n|canvas_node_view(n,aliases.get(n.asset_id.as_deref().unwrap_or("")),Some(&n.id))).collect::<Vec<_>>(),"node_count":baseline.nodes.len()}),
                );
            }
            require_task_board(tx, board_id, task).await?;
            // H0 复核：画布必须属于当前任务的项目
            let board_owner: Option<String> =
                sqlx::query_scalar("SELECT project_id FROM boards WHERE id=?")
                    .bind(board_id)
                    .fetch_optional(&mut **tx)
                    .await?;
            match board_owner.as_deref() {
                Some(pid) if pid == task.project_id => {}
                _ => {
                    return Err(crate::domain::AppError::new(
                        "PERMISSION_DENIED",
                        "画布不存在或不属于当前项目",
                    ));
                }
            }
            let nodes = sqlx::query(
                "SELECT n.*,o.source_node_id FROM canvas_nodes n LEFT JOIN board_draft_node_origins o ON o.node_id=n.id WHERE n.board_id=? AND n.visible=1 ORDER BY n.z_order,n.rowid LIMIT 1001",
            )
            .bind(board_id)
            .fetch_all(&mut **tx).await?;
            use sqlx::Row;
            if nodes.len() > 1000 {
                return Err(AppError::new(
                    "VALIDATION_ERROR",
                    "请把方案拆分成较小的创作范围",
                ));
            }
            let result: Vec<Value> = nodes
                .iter()
                .map(|row| -> Result<Value> {
                    let node = crate::canvas::CanvasNode::from_row(row)?;
                    Ok(canvas_node_view(
                        &node,
                        aliases.get(node.asset_id.as_deref().unwrap_or("")),
                        row.get::<Option<String>, _>("source_node_id").as_ref(),
                    ))
                })
                .collect::<Result<_>>()?;
            Ok(json!({"board_id": board_id, "node_count": result.len(), "nodes": result}))
        }
        _ => Err(crate::domain::AppError::new(
            "PERMISSION_DENIED",
            format!("未知画布工具 {name}"),
        )),
    }
}

// G4 补充：group_nodes / create_sequence_draft
pub async fn execute_canvas_tool_extended(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    name: &str,
    args: &Value,
    task: &super::types::Task,
) -> Result<Value> {
    match name {
        "group_nodes" => {
            let board_id = args["board_id"].as_str().unwrap_or("");
            require_task_board(tx, board_id, task).await?;
            let node_ids: Vec<String> = args["node_ids"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let label = args["label"].as_str().unwrap_or("分组").trim();
            let unique: HashSet<_> = node_ids.iter().collect();
            if node_ids.is_empty()
                || node_ids.len() > 100
                || unique.len() != node_ids.len()
                || label.is_empty()
                || label.chars().count() > 50
            {
                return Err(crate::domain::AppError::new(
                    "VALIDATION_ERROR",
                    "分组须包含 1–100 个不重复节点，标签须为 1–50 字符",
                ));
            }
            let children = serde_json::to_string(&node_ids).map_err(serialization)?;
            let valid: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM canvas_nodes WHERE board_id=? AND visible=1 AND locked=0 AND id IN (SELECT value FROM json_each(?))",
            ).bind(board_id).bind(&children).fetch_one(&mut **tx).await?;
            if valid != node_ids.len() as i64 {
                return Err(AppError::new(
                    "PERMISSION_DENIED",
                    "分组节点必须属于当前草案且可编辑",
                ));
            }
            let (x, y, right, bottom): (f64, f64, f64, f64) = sqlx::query_as(
                "SELECT MIN(x),MIN(y),MAX(x+width),MAX(y+height) FROM canvas_nodes WHERE board_id=? AND id IN (SELECT value FROM json_each(?))",
            ).bind(board_id).bind(&children).fetch_one(&mut **tx).await?;
            let now = crate::domain::timestamp();
            let group_id = crate::domain::id();
            sqlx::query("INSERT INTO canvas_nodes(id,board_id,node_type,x,y,width,height,rotation,z_order,content_json,children_json,created_at,updated_at) VALUES(?,?, 'group',?,?,?,?,0,0,?,?,?,?)")
                .bind(&group_id).bind(board_id).bind(x).bind(y).bind(right-x).bind(bottom-y)
                .bind(serde_json::json!({"label": label}).to_string())
                .bind(&children)
                .bind(&now).bind(&now)
                .execute(&mut **tx).await?;
            Ok(json!({"group_id": group_id, "label": label, "node_count": node_ids.len()}))
        }
        "create_sequence_draft" => {
            let photo_ids: Vec<String> = args["photo_ids"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let title = args["title"].as_str().unwrap_or("Agent 序列草案");
            if photo_ids.is_empty() {
                return Err(crate::domain::AppError::new(
                    "VALIDATION_ERROR",
                    "至少需要一张照片",
                ));
            }
            // 验证照片都在 scope 内
            for id in &photo_ids {
                if !task.allowed_photo_ids.contains(id) {
                    return Err(crate::domain::AppError::new(
                        "PERMISSION_DENIED",
                        format!("照片 {id} 不在任务范围内"),
                    ));
                }
            }
            // 创建草案（复用现有 draft 系统）
            let draft_id = crate::domain::id();
            let now = crate::domain::timestamp();
            // H0 复核：匹配 drafts 真实表结构（task_id/base_revision/state/summary 必填）
            sqlx::query(
                "INSERT INTO drafts(id,project_id,task_id,title,base_revision,state,summary,created_at,updated_at) VALUES(?,?,?,?,?,'pending',?,?,?)",
            )
            .bind(&draft_id)
            .bind(&task.project_id)
            .bind(&task.id)
            .bind(title)
            .bind(task.base_revision)
            .bind("Agent 从画布生成的序列草案")
            .bind(&now)
            .bind(&now)
            .execute(&mut **tx)
            .await?;
            // draft_items 列名是 proposed_action 且 rationale 必填
            for (pos, photo_id) in photo_ids.iter().enumerate() {
                sqlx::query(
                    "INSERT INTO draft_items(draft_id,photo_id,proposed_action,position,rationale) VALUES(?,?, 'keep',?, '序列顺序')",
                )
                .bind(&draft_id)
                .bind(photo_id)
                .bind(pos as i64)
                .execute(&mut **tx)
                .await?;
            }
            // 草案与它的画面依据回执必须落在同一条事务里：另开连接写回执会在 WAL 下
            // 等这条事务的写锁，画布工具直接卡死。
            let mut scope = task.clone();
            let recorded = super::recheck::record(&mut *tx, &mut scope, &draft_id)
                .await
                .is_ok();
            let result = json!({
                "draft_id": draft_id,
                "title": title,
                "photo_count": photo_ids.len(),
                "visual_basis": if recorded { "recorded" } else { "not_recorded" }
            });
            Ok(result)
        }
        _ => Err(crate::domain::AppError::new(
            "PERMISSION_DENIED",
            format!("未知扩展工具 {name}"),
        )),
    }
}
