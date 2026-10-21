use super::{drafts::replace_items, store, types::*, vision::VisionPrefetch};
use crate::{
    command_audit::serialization,
    domain::{self, AppError, Result},
};
use serde_json::{Value, json};
use sqlx::{Row, Sqlite, Transaction};
use std::collections::{HashMap, HashSet};

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
        "create_board_draft" | "place_photos_on_board" | "get_board" => {
            return execute_canvas_tool(tx, name, &call.args, task).await;
        }
        "group_nodes" | "create_sequence_draft" => {
            return execute_canvas_tool_extended(tx, name, &call.args, task).await;
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
        "inspect_photos" | "compare_photos" => {
            let ids: Vec<String> =
                serde_json::from_value(a["photo_ids"].clone()).map_err(serialization)?;
            let mut data = photos(tx, task).await?;
            data.retain(|p| ids.iter().any(|id| p["photo_id"] == *id));
            match vision_data.filter(|v| v.kind == call.name) {
                Some(prefetch) => {
                    // 观察落缓存（photo_hash 维度），配额/用量/费用同事务扣减。
                    for (photo_id, observation) in &prefetch.observations {
                        let hash: Option<String> = sqlx::query_scalar(
                            "SELECT content_sha256 FROM photos WHERE id=? AND project_id=?",
                        )
                        .bind(photo_id)
                        .bind(&task.project_id)
                        .fetch_optional(&mut **tx)
                        .await?;
                        if let Some(hash) = hash {
                            sqlx::query("INSERT INTO photo_observations(photo_id,photo_hash,model_id,prompt_version,kind,observation_json,created_at) VALUES(?,?,?,?,?,?,?)")
                                .bind(photo_id)
                                .bind(&hash)
                                .bind(&prefetch.model_id)
                                .bind(&prefetch.cache_version)
                                .bind(call.name.as_str())
                                .bind(observation.to_string())
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
                    let cached = (prefetch.observations.len() as u32).saturating_sub(uploaded);
                    Ok(
                        json!({"photos":data,"mode":"vision_observed","uploaded_images":uploaded,"cache_hits":cached,"observations":observations,"uncertainty":"主观解读与不确定性由模型标注；服务端已校验 photo_id 均属请求集合"}),
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
            let memory =
                super::memory::retrievable_for_goal(tx, &task.project_id, limit, &task.goal)
                    .await?;
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
            let items = store::items(tx, a["draft_id"].as_str().unwrap()).await?;
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
            let observations: HashMap<String, (Vec<String>, String)> = if scoped.is_empty() {
                HashMap::new()
            } else {
                let scope_json = crate::command_audit::encode(&scoped)?;
                let rows = sqlx::query(
                    r#"
                    SELECT o.photo_id, o.observation_json FROM photo_observations o
                    JOIN (SELECT MAX(e.id) AS id FROM photo_observations e
                          JOIN photos p ON p.id=e.photo_id AND p.content_sha256=e.photo_hash
                          JOIN json_each(?) scope ON scope.value=p.id
                          WHERE p.project_id=? GROUP BY e.photo_id) latest ON latest.id=o.id
                    WHERE length(CAST(o.observation_json AS BLOB))<=65536
                    "#,
                )
                .bind(&scope_json)
                .bind(&task.project_id)
                .fetch_all(&mut **tx)
                .await?;
                rows.iter()
                    .filter_map(|row| {
                        let raw: String = row.get("observation_json");
                        let value = serde_json::from_str::<Value>(&raw).ok()?;
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
                        Some((row.get::<String, _>("photo_id"), (visible, interpretation)))
                    })
                    .collect()
            };
            // 3) 相邻相似：按草案顺序的相邻两张，visible_facts Jaccard >= 0.6。
            let mut similar_adjacent: Vec<Value> = vec![];
            for pair in items.windows(2) {
                let (Some((left, _)), Some((right, _))) = (
                    observations.get(&pair[0].photo_id),
                    observations.get(&pair[1].photo_id),
                ) else {
                    continue;
                };
                let set_a: HashSet<&String> = left.iter().collect();
                let set_b: HashSet<&String> = right.iter().collect();
                let shared: Vec<&String> = set_a.intersection(&set_b).cloned().collect();
                let union = set_a.union(&set_b).count();
                if union > 0 && shared.len() as f64 / union as f64 >= 0.6 {
                    similar_adjacent.push(json!({
                        "positions": [pair[0].position, pair[1].position],
                        "photo_ids": [pair[0].photo_id, pair[1].photo_id],
                        "shared_facts": shared.iter().take(3).map(|s| s.to_string()).collect::<Vec<_>>(),
                    }));
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
                    let (visible, interpretation) = observations.get(&i.photo_id)?;
                    Some(json!({
                        "photo_id": i.photo_id,
                        "position": i.position,
                        "top_facts": visible.iter().take(3).collect::<Vec<_>>(),
                        "interpretation": interpretation,
                    }))
                })
                .collect();
            Ok(json!({
                "draft_id": a["draft_id"],
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
                },
                "item_facts": item_facts,
                "mode": "structured_local_evaluation",
                "uncertainty": "重复性与结构指标为本地确定性计算（内容哈希与观察事实比对）；观察缓存可能来自历史模型版本且未观察照片不参与相似性比对；叙事判断由策展模型结合 item_facts 进行，人工评审保留最终决定。",
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
            sqlx::query("INSERT INTO boards(id,project_id,title,state,created_at,updated_at) VALUES(?,?,?,'draft',?,?)")
                .bind(&id).bind(&task.project_id).bind(title).bind(&now).bind(&now)
                .execute(&mut **tx).await?;
            Ok(json!({"board_id": id, "title": title, "state": "draft"}))
        }
        "place_photos_on_board" => {
            let board_id = args["board_id"].as_str().unwrap_or("");
            // H0 复核：画布必须属于当前项目且为 draft 状态（Agent 不能写 official/frozen）
            let board_state: Option<String> =
                sqlx::query_scalar("SELECT state FROM boards WHERE id=? AND project_id=?")
                    .bind(board_id)
                    .bind(&task.project_id)
                    .fetch_optional(&mut **tx)
                    .await?;
            match board_state.as_deref() {
                Some("draft") => {}
                Some(other) => {
                    return Err(crate::domain::AppError::new(
                        "PERMISSION_DENIED",
                        format!("画布为 {other} 状态，Agent 只能写入 draft 草稿"),
                    ));
                }
                None => return Err(crate::domain::AppError::new("NOT_FOUND", "画布不存在")),
            }
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
                let Some(asset) = asset_id else { continue };
                let x = p["x"].as_f64().unwrap_or(0.0);
                let y = p["y"].as_f64().unwrap_or(0.0);
                let w = p["width"].as_f64().unwrap_or(220.0);
                let h = p["height"].as_f64().unwrap_or(160.0);
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
                "SELECT id, node_type, x, y, width, height, z_order, asset_id FROM canvas_nodes WHERE board_id=? AND visible=1 ORDER BY z_order",
            )
            .bind(board_id)
            .fetch_all(&mut **tx).await?;
            use sqlx::Row;
            let result: Vec<Value> = nodes.iter().map(|r| {
                json!({"id": r.get::<String,_>("id"), "type": r.get::<String,_>("node_type"), "x": r.get::<f64,_>("x"), "y": r.get::<f64,_>("y"), "width": r.get::<f64,_>("width"), "height": r.get::<f64,_>("height"), "z": r.get::<i64,_>("z_order"), "asset_id": r.get::<Option<String>,_>("asset_id")})
            }).collect();
            Ok(json!({"board_id": board_id, "node_count": result.len(), "nodes": result}))
        }
        _ => Err(crate::domain::AppError::new(
            "PERMISSION_DENIED",
            format!("未知画布工具 {name}"),
        )),
    }
}

// G4 补充：group_nodes / create_sequence_draft / evaluate_sequence
pub async fn execute_canvas_tool_extended(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    name: &str,
    args: &Value,
    task: &super::types::Task,
) -> Result<Value> {
    match name {
        "group_nodes" => {
            let board_id = args["board_id"].as_str().unwrap_or("");
            let node_ids: Vec<String> = args["node_ids"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let label = args["label"].as_str().unwrap_or("分组");
            if node_ids.is_empty() {
                return Err(crate::domain::AppError::new(
                    "VALIDATION_ERROR",
                    "至少需要一个节点",
                ));
            }
            let now = crate::domain::timestamp();
            let group_id = crate::domain::id();
            sqlx::query("INSERT INTO canvas_nodes(id,board_id,node_type,x,y,width,height,rotation,z_order,content_json,children_json,created_at,updated_at) VALUES(?,?, 'group',0,0,0,0,0,0,?,?,?,?)")
                .bind(&group_id).bind(board_id)
                .bind(serde_json::json!({"label": label}).to_string())
                .bind(serde_json::json!(node_ids).to_string())
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
            Ok(json!({"draft_id": draft_id, "title": title, "photo_count": photo_ids.len()}))
        }
        "evaluate_sequence" => {
            let ids: Vec<String> = sqlx::query_scalar(
                "SELECT photo_id FROM sequence_items WHERE project_id=? ORDER BY position",
            )
            .bind(&task.project_id)
            .fetch_all(&mut **tx)
            .await?;
            if ids.is_empty() {
                return Ok(json!({"empty": true, "message": "作品序列为空"}));
            }
            // 读取照片元数据做基本评估
            let photos = super::tools::photos(tx, task).await?;
            let total = ids.len();
            let rated: Vec<&Value> = photos
                .iter()
                .filter(|p| p["rating"].as_i64().unwrap_or(0) > 0)
                .collect();
            let avg_rating = if !rated.is_empty() {
                rated
                    .iter()
                    .map(|p| p["rating"].as_f64().unwrap_or(0.0))
                    .sum::<f64>()
                    / rated.len() as f64
            } else {
                0.0
            };
            // 相邻同名检测（简化重复性）
            let names: Vec<String> = ids
                .iter()
                .filter_map(|id| {
                    photos
                        .iter()
                        .find(|p| p["photo_id"] == *id)
                        .and_then(|p| p["name"].as_str().map(String::from))
                })
                .collect();
            let duplicates = names.windows(2).filter(|w| w[0] == w[1]).count();
            Ok(json!({
                "total_photos": total,
                "avg_rating": (avg_rating * 10.0).round() / 10.0,
                "adjacent_duplicates": duplicates,
                "rated_count": rated.len(),
                "diversity_score": if total > 1 { 1.0 - (duplicates as f64 / total as f64) } else { 1.0 },
                "note": "基于元数据的结构化评估；视觉节奏需人工判断"
            }))
        }
        _ => Err(crate::domain::AppError::new(
            "PERMISSION_DENIED",
            format!("未知扩展工具 {name}"),
        )),
    }
}
