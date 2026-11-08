use super::types::*;
use crate::{
    command_audit::{encode, serialization},
    db::Store,
    domain::{self, AppError, Result},
};
use serde_json::{Value, json};
use sqlx::{Row, Sqlite, Transaction};

pub(crate) async fn load(tx: &mut Transaction<'_, Sqlite>, id: &str) -> Result<Task> {
    let r = sqlx::query("SELECT * FROM agent_tasks WHERE id=?")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(|| AppError::new("NOT_FOUND", "任务不存在"))?;
    Ok(Task {
        id: r.get("id"),
        project_id: r.get("project_id"),
        goal: r.get("goal"),
        base_revision: r.get("base_revision"),
        memory_revision: r.get("memory_revision"),
        allowed_photo_ids: parse(r.get("allowed_ids_json"))?,
        selected_photo_ids: parse(r.get("selected_ids_json"))?,
        target_count: r
            .get::<Option<i64>, _>("canvas_target_count")
            .unwrap_or_else(|| r.get("target_count")) as u32,
        state: r.get("state"),
        reason: r.get("reason"),
        budget: parse(r.get("budget_json"))?,
        usage: parse(r.get("usage_json"))?,
        elapsed_ms: r.get::<i64, _>("elapsed_ms") as u64,
        question: r.get("question"),
        answer: r.get("answer"),
        final_draft_ids: parse(r.get("final_ids_json"))?,
        pending: r
            .get::<Option<String>, _>("pending_json")
            .map(parse)
            .transpose()?,
        provider_id: r.get("provider_id"),
        consent_id: r.get("consent_id"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    })
}
fn parse<T: serde::de::DeserializeOwned>(raw: String) -> Result<T> {
    serde_json::from_str(&raw).map_err(serialization)
}
pub(crate) async fn save(tx: &mut Transaction<'_, Sqlite>, t: &Task) -> Result<()> {
    sqlx::query("UPDATE agent_tasks SET state=?,reason=?,budget_json=?,usage_json=?,elapsed_ms=?,question=?,answer=?,final_ids_json=?,pending_json=?,updated_at=? WHERE id=?")
 .bind(&t.state).bind(&t.reason).bind(encode(&t.budget)?).bind(encode(&t.usage)?).bind(t.elapsed_ms as i64).bind(&t.question).bind(&t.answer).bind(encode(&t.final_draft_ids)?).bind(t.pending.as_ref().map(encode).transpose()?).bind(domain::timestamp()).bind(&t.id).execute(&mut **tx).await?;
    Ok(())
}
pub(crate) async fn step(
    tx: &mut Transaction<'_, Sqlite>,
    id: &str,
    kind: &str,
    body: Value,
    summary: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO agent_steps(task_id,kind,body_json,summary,created_at) VALUES(?,?,?,?,?)",
    )
    .bind(id)
    .bind(kind)
    .bind(body.to_string())
    .bind(summary)
    .bind(domain::timestamp())
    .execute(&mut **tx)
    .await?;
    Ok(())
}
pub(crate) async fn transition(
    tx: &mut Transaction<'_, Sqlite>,
    t: &mut Task,
    next: &str,
    reason: &str,
) -> Result<()> {
    let legal = match t.state.as_str() {
        "queued" => matches!(
            next,
            "planning" | "paused" | "cancelled" | "interrupted" | "failed"
        ),
        "planning" => matches!(
            next,
            "running" | "paused" | "cancelled" | "interrupted" | "failed"
        ),
        "running" => matches!(
            next,
            "needs_input"
                | "review_ready"
                | "completed"
                | "paused"
                | "cancelled"
                | "interrupted"
                | "failed"
        ),
        "needs_input" => matches!(next, "queued" | "cancelled"),
        "paused" | "interrupted" => matches!(next, "queued" | "cancelled"),
        "review_ready" => matches!(next, "completed" | "stale" | "cancelled"),
        "stale" => matches!(next, "completed" | "cancelled"),
        _ => false,
    };
    if !legal {
        return Err(AppError::new(
            "INVALID_TRANSITION",
            "任务当前状态不允许此操作",
        ));
    }
    let old = t.state.clone();
    t.state = next.into();
    t.reason = reason.into();
    save(tx, t).await?;
    step(tx, &t.id, "state", json!({"from":old,"to":next}), reason).await
}
pub(crate) async fn protocol(
    tx: &mut Transaction<'_, Sqlite>,
    id: &str,
) -> Result<Vec<ProtocolMessage>> {
    let rows=sqlx::query("SELECT kind,body_json FROM agent_steps WHERE task_id=? AND kind IN('model','tool') ORDER BY id").bind(id).fetch_all(&mut **tx).await?;
    rows.iter()
        .map(|r| {
            Ok(ProtocolMessage {
                kind: r.get("kind"),
                body: parse(r.get("body_json"))?,
            })
        })
        .collect()
}
pub(crate) async fn checkpoint(tx: &mut Transaction<'_, Sqlite>, task: &Task) -> Result<()> {
    let history = protocol(tx, &task.id).await?;
    let conversation_binding = crate::focal_chat::binding(tx, &task.id).await?;
    let creative_binding = super::creative::binding(tx, task).await?;
    let grant_binding = super::task_scope::binding(tx, task).await?;
    let author_binding =
        super::memory_scope::binding_value(super::memory_scope::load(tx, &task.id).await?);
    let identity = super::model_binding::load(tx, &task.id).await?;
    let model = identity
        .as_ref()
        .map(|value| value.model_id.as_str())
        .unwrap_or(if task.provider_id == super::PROVIDER_OPENAI {
            "unbound"
        } else {
            super::MODEL_ID
        });
    let ids: Vec<String> =
        sqlx::query_scalar("SELECT call_id FROM tool_calls WHERE task_id=? ORDER BY rowid")
            .bind(&task.id)
            .fetch_all(&mut **tx)
            .await?;
    sqlx::query("INSERT INTO agent_checkpoints(task_id,state,context_json,completed_calls_json,budget_json,created_at) VALUES(?,?,?,?,?,?)").bind(&task.id).bind(&task.state).bind(json!({"visionReceiptBinding":super::vision_recovery::binding(tx,&task.id).await?,"creativeBinding":creative_binding,"conversationBinding":conversation_binding,"taskGrantBinding":grant_binding,"authorMemoryBinding":author_binding,"memoryRevision":task.memory_revision,"providerId":task.provider_id,"modelId":model,"modelIdentity":identity,"promptVersion":super::prompt_version(&task.provider_id),"goal":task.goal,"projectId":task.project_id,"baseRevision":task.base_revision,"allowedPhotoIds":task.allowed_photo_ids,"selectedPhotoIds":task.selected_photo_ids,"pending":task.pending,"protocol":history,"question":task.question,"answer":task.answer,"finalDraftIds":task.final_draft_ids}).to_string()).bind(encode(&ids)?).bind(json!({"limits":task.budget,"usage":task.usage,"elapsedMs":task.elapsed_ms}).to_string()).bind(domain::timestamp()).execute(&mut **tx).await?;
    Ok(())
}
/// 逐张口径一次最多投多少条，与界面契约里 `perPhoto` 的上限是同一个数：超过就少说，
/// 界面上那句"逐张口径只点到上面列出的那几张"负责把差额说出来。
const PER_PHOTO_LIMIT: usize = 100;
/// 回执只把前 12 张的逐张口径带在身上（那一份是模型看得见的，为了界面把它撑大会挤掉对话预算），
/// 剩下的照片靠库里的同一套优先级补齐。补之前先把回执自己点到的每一张跟现在库里挑出来的对一遍：
/// 名字对不上，或者"库里有画面记录的张数"不等于"用上的＋排除的"，就说明库和回执看的不是同一晚的
/// 状态，这时一条都不补。界面宁可少说几句，也不能把一张没核对过的照片说成"按某档问过"。
async fn marks_from_library(
    tx: &mut Transaction<'_, Sqlite>,
    task: &Task,
    draft: &str,
    receipt: &Value,
    observed: i64,
    limit: usize,
) -> Result<Vec<Value>> {
    if limit == 0 {
        return Ok(vec![]);
    }
    let items: Vec<String> =
        sqlx::query_scalar("SELECT photo_id FROM draft_items WHERE draft_id=? ORDER BY position")
            .bind(draft)
            .fetch_all(&mut **tx)
            .await?;
    // 与评估同一份范围：只在许可内的照片里挑，许可外的本来就没被核对过。
    let scoped: Vec<String> = task
        .allowed_photo_ids
        .iter()
        .filter(|id| items.contains(id))
        .cloned()
        .collect();
    if scoped.is_empty() {
        return Ok(vec![]);
    }
    let rows = super::tools::pick_observations(tx, &scoped, &task.project_id, false).await?;
    let live: std::collections::BTreeMap<String, &'static str> = rows
        .iter()
        .map(|row| {
            (
                row.get("photo_id"),
                super::tools::observation_contract(row.get::<i64, _>("contract")),
            )
        })
        .collect();
    // 回执点到的名单按原样读：陌生档位对不上号，正好让补齐停在这里，而不是被悄悄当成没这回事。
    let omitted_rows = receipt["omitted_visual_evidence"]
        .as_array()
        .map_or(&[][..], Vec::as_slice);
    let mut claimed: Vec<(&str, &str)> = vec![];
    for (row, column) in receipt["item_facts"]
        .as_array()
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .map(|row| (row, "evidence_contract"))
        .chain(omitted_rows.iter().map(|row| (row, "contract")))
    {
        let (Some(photo), Some(name)) = (
            row["photo_id"].as_str().filter(|id| !id.is_empty()),
            row[column].as_str(),
        ) else {
            return Ok(vec![]);
        };
        claimed.push((photo, name));
    }
    let listed: std::collections::BTreeSet<&str> =
        claimed.iter().map(|(photo, _)| *photo).collect();
    // 一张照片在名单上出现两次，就说明这份回执不是自己写的，逐张口径不补。
    if listed.len() != claimed.len()
        // 库里有画面记录的张数，要正好等于回执说的"用上的"加"排除的"：多出来的那条要么后来才有，
        // 要么当时因为读不懂被悄悄丢掉——两种都不能再替它作保。
        || live.len() as i64 != observed + omitted_rows.len() as i64
        || claimed
            .iter()
            .any(|(photo, name)| live.get(*photo).copied() != Some(*name))
    {
        return Ok(vec![]);
    }
    Ok(items
        .iter()
        .filter(|photo| !listed.contains(photo.as_str()))
        .filter_map(|photo| live.get(photo).map(|name| (photo, name)))
        .take(limit)
        .map(|(photo, name)| json!({"photoId": photo, "contract": name, "used": true}))
        .collect())
}
/// 界面要能说清"这份方案的画面事实是按哪一档问法挑出来的"，所以把本地结构化评估回执里的计数投出去。
/// 只投计数：观察正文、事实字符串都不进投影。哪一条该说、读不读得回来由 `super::receipts` 定，
/// 这里只负责把读回来的那一份拆成界面上的数字——宁可对某份方案什么也不说，
/// 也不能把一份猜出来的摘要摆给人看。
async fn draft_evidence(tx: &mut Transaction<'_, Sqlite>, task: &Task) -> Result<Vec<Value>> {
    // 已经是"每份方案最新的那一条"，按时间先后给出，所以过一遍留下的就是它现在该说的口径。
    let results = super::receipts::for_task(tx, &task.id).await?;
    let mut latest: std::collections::BTreeMap<String, Value> = Default::default();
    let count = |value: &Value| value.as_i64().filter(|n| *n >= 0);
    for result in results {
        let Some(draft) = result["draft_id"].as_str().filter(|id| !id.is_empty()) else {
            continue;
        };
        let (metrics, tiers) = (&result["metrics"], &result["metrics"]["evidence_contracts"]);
        let (
            Some(items),
            Some(observed),
            Some(unobserved),
            Some(page),
            Some(detail),
            Some(other_version),
            Some(other_tool),
            Some(pairs),
        ) = (
            count(&result["count"]),
            count(&metrics["observed"]),
            count(&metrics["unobserved"]),
            count(&tiers["current_page_tier"]),
            count(&tiers["current_detail_tier"]),
            count(&tiers["other_prompt_version"]),
            count(&tiers["other_tool"]),
            count(&metrics["cross_contract_adjacent_pairs"]),
        )
        else {
            continue;
        };
        let omitted = result["omitted_visual_evidence"]
            .as_array()
            .map_or(0, Vec::len) as i64;
        // 口径是对着方案的哪一版算的：老回执没记过就当没记过，界面上不猜版本号。
        let revision = result["draft_revision"].as_i64().filter(|n| *n > 0);
        // 逐张只投"这张按哪一档问法、或者为什么没用上"：问法与图像版本都是有限几种，
        // 名字之外的任何写法一律丢掉，观察正文本来就不在这条投影里。
        fn tier(value: &Value) -> Option<&str> {
            let name = value.as_str()?;
            let known = matches!(
                name,
                "current_page_tier" | "current_detail_tier" | "other_prompt_version" | "other_tool"
            );
            known.then_some(name)
        }
        fn pixel(value: &Value) -> Option<&str> {
            let name = value.as_str()?;
            let known = matches!(name, "mismatch" | "unreadable" | "unrecorded");
            known.then_some(name)
        }
        // 老回执没查过原图，这一格就空着：界面照旧只说预览那一层，不替它猜原图在不在。
        fn original(value: &Value) -> Option<&str> {
            let name = value.as_str()?;
            let known = matches!(name, "missing" | "changed" | "matches" | "unchecked");
            known.then_some(name)
        }
        let mut per_photo: Vec<Value> = Vec::new();
        for row in result["item_facts"]
            .as_array()
            .map_or(&[][..], Vec::as_slice)
        {
            let (Some(photo), Some(name)) = (
                row["photo_id"].as_str().filter(|id| !id.is_empty()),
                tier(&row["evidence_contract"]),
            ) else {
                continue;
            };
            per_photo.push(json!({"photoId": photo, "contract": name, "used": true}));
        }
        let mut scanned = per_photo.len() as i64;
        // 回执带不全的那些张，先跟库对一遍账再补；补上的仍旧只有"这张按哪一档"，没有别的。
        if scanned < observed && per_photo.len() < PER_PHOTO_LIMIT {
            let extra = marks_from_library(
                tx,
                task,
                draft,
                &result,
                observed,
                PER_PHOTO_LIMIT - per_photo.len(),
            )
            .await?;
            scanned += extra.len() as i64;
            per_photo.extend(extra);
        }
        for row in result["omitted_visual_evidence"]
            .as_array()
            .map_or(&[][..], Vec::as_slice)
        {
            if per_photo.len() >= PER_PHOTO_LIMIT {
                break;
            }
            let (Some(photo), Some(version)) = (
                row["photo_id"].as_str().filter(|id| !id.is_empty()),
                pixel(&row["pixel_version"]),
            ) else {
                continue;
            };
            let mut entry = json!({
                "photoId": photo,
                "contract": tier(&row["contract"]),
                "used": false,
                "pixelVersion": version,
            });
            if let Some(status) = original(&row["original_status"]) {
                entry["originalStatus"] = json!(status);
            }
            per_photo.push(entry);
        }
        latest.insert(
            draft.to_string(),
            json!({
                "draftId": draft,
                "count": items,
                "observed": observed,
                "unobserved": unobserved,
                "pageTier": page,
                "detailTier": detail,
                "otherVersion": other_version,
                "otherTool": other_tool,
                "crossContractPairs": pairs,
                "omitted": omitted,
                "draftRevision": revision,
                "scanned": scanned,
                "perPhoto": per_photo,
            }),
        );
    }
    // 整条读不回来的那些不是"没算过"：计数在库里就地取，逐张档位确实没读回来就照实标出来，
    // 免得这一格和从来没算过的方案共用一句含糊话。
    let readable: Vec<String> = latest.keys().cloned().collect();
    let summaries = super::receipts::counts_for_unreadable(tx, &task.id, &readable).await?;
    for row in summaries {
        if latest.len() >= super::receipts::MAX_ENTRIES {
            break;
        }
        let Some(draft) = row["draft"].as_str().filter(|id| !id.is_empty()) else {
            continue;
        };
        let (
            Some(items),
            Some(observed),
            Some(unobserved),
            Some(page),
            Some(detail),
            Some(other_version),
            Some(other_tool),
            Some(pairs),
            Some(omitted),
        ) = (
            count(&row["count"]),
            count(&row["observed"]),
            count(&row["unobserved"]),
            count(&row["page"]),
            count(&row["detail"]),
            count(&row["other_version"]),
            count(&row["other_tool"]),
            count(&row["pairs"]),
            count(&row["omitted"]),
        )
        else {
            continue;
        };
        // 版本号写得不像整数就当没记过，与整条读回来那一路同一个口径。
        let revision = row["revision"]
            .as_i64()
            .filter(|_| row["revision_kind"].as_str() == Some("integer"))
            .filter(|n| *n > 0);
        latest.insert(
            draft.to_string(),
            json!({
                "draftId": draft,
                "count": items,
                "observed": observed,
                "unobserved": unobserved,
                "pageTier": page,
                "detailTier": detail,
                "otherVersion": other_version,
                "otherTool": other_tool,
                "crossContractPairs": pairs,
                "omitted": omitted,
                "draftRevision": revision,
                // 逐张档位没读回来，就不能装成扫过更多张：能说的只有"和观察数一样"。
                "scanned": observed,
                "perPhoto": [],
                "basisSummary": true,
            }),
        );
    }
    Ok(latest.into_values().collect())
}

pub async fn projection(store: &Store, project: &str) -> Result<AgentProjection> {
    projection_filtered(store, project, None).await
}
pub async fn projection_for_account(
    store: &Store,
    project: &str,
    account: &str,
) -> Result<AgentProjection> {
    projection_filtered(store, project, Some(account)).await
}
async fn projection_filtered(
    store: &Store,
    project: &str,
    account: Option<&str>,
) -> Result<AgentProjection> {
    let mut tx = store.pool.begin().await?;
    let revision: i64 =
        sqlx::query_scalar("SELECT revision FROM projects WHERE id=? AND deleted_at IS NULL")
            .bind(project)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| AppError::new("NOT_FOUND", "项目不存在"))?;
    let ids:Vec<String>=sqlx::query_scalar("SELECT id FROM agent_tasks WHERE project_id=? ORDER BY created_at DESC,rowid DESC LIMIT 50").bind(project).fetch_all(&mut *tx).await?;
    let mut tasks = vec![];
    for id in ids {
        let t = load(&mut tx, &id).await?;
        if let Some(account) = account {
            let owner = super::model_binding::load(&mut tx, &id)
                .await?
                .and_then(|i| i.account_id);
            if owner.as_deref() != Some(account) {
                continue;
            }
        }
        let grant = super::task_scope::load_for_projection(&mut tx, &t).await?;
        let boards=sqlx::query_as("SELECT b.* FROM boards b JOIN board_draft_tasks d ON d.board_id=b.id WHERE b.project_id=? AND d.task_id=? ORDER BY b.created_at,b.rowid")
            .bind(&t.project_id).bind(&t.id).fetch_all(&mut *tx).await?;
        let steps = sqlx::query(
            "SELECT id,kind,summary,created_at FROM agent_steps WHERE task_id=? ORDER BY id",
        )
        .bind(&id)
        .fetch_all(&mut *tx)
        .await?
        .iter()
        .map(|r| Step {
            id: r.get("id"),
            kind: r.get("kind"),
            summary: r.get("summary"),
            created_at: r.get("created_at"),
        })
        .collect();
        let budget_issue = if t.state == "paused" {
            let raw: Option<String> = sqlx::query_scalar("SELECT body_json FROM agent_steps WHERE task_id=? AND kind='budget_limit' ORDER BY id DESC LIMIT 1")
                .bind(&id).fetch_optional(&mut *tx).await?;
            raw.and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
                .filter(|body| body["reason"].as_str() == Some(t.reason.as_str()))
                .and_then(|body| serde_json::from_value(body["issue"].clone()).ok())
        } else {
            None
        };
        tasks.push(TaskView {
            memory_trace: super::memory_trace::projection(&mut tx, &t).await?,
            photo_judgments: super::photo_judgment::projection(&mut tx, &t).await?,
            queue_cancellation: super::vision_recovery::cancellation(&mut tx, &t.id).await?,
            vision_recovery: super::vision_recovery::projection(&mut tx, &t).await?,
            creative_references: super::creative::references(&mut tx, &t.id).await?,
            knowledge_searches: super::creative::searches(&mut tx, &t.id).await?,
            draft_lineage: super::creative::lineage(&mut tx, &t.id).await?,
            preference_proposals: super::preference_proposal::projection(&mut tx, &t.id).await?,
            continuation: super::creative::continuation(&mut tx, &t.id).await?,
            inspected_photos: crate::focal_chat::inspected(&mut tx, &t).await?,
            draft_evidence: draft_evidence(&mut tx, &t).await?,
            conversation: crate::focal_chat::load(&mut tx, &t.id).await?.is_some(),
            messages: crate::focal_chat::messages(&mut tx, &t.id).await?,
            budget_issue,
            canvas_task: grant.canvas,
            canvas_source: grant.source,
            board_drafts: boards,
            id: t.id,
            goal: t.goal,
            base_revision: t.base_revision,
            state: t.state,
            reason: t.reason,
            budget: t.budget,
            usage: t.usage,
            elapsed_ms: t.elapsed_ms,
            question: t.question,
            final_draft_ids: t.final_draft_ids,
            provider_id: t.provider_id,
            steps,
            created_at: t.created_at,
        });
    }
    let mut drafts = vec![];
    for r in sqlx::query(
        "SELECT * FROM drafts WHERE project_id=? ORDER BY created_at DESC,rowid DESC LIMIT 100",
    )
    .bind(project)
    .fetch_all(&mut *tx)
    .await?
    {
        if account.is_some() && !tasks.iter().any(|t| t.id == r.get::<String, _>("task_id")) {
            continue;
        }
        let id: String = r.get("id");
        let items = items(&mut tx, &id).await?;
        let base_revision = r.get("base_revision");
        drafts.push(Draft {
            id,
            project_id: r.get("project_id"),
            task_id: r.get("task_id"),
            title: r.get("title"),
            base_revision,
            revision: r.get("revision"),
            state: r.get("state"),
            stale: base_revision != revision,
            summary: r.get("summary"),
            items,
            created_at: r.get("created_at"),
        });
    }
    tx.commit().await?;
    Ok(AgentProjection { tasks, drafts })
}
pub(crate) async fn items(tx: &mut Transaction<'_, Sqlite>, id: &str) -> Result<Vec<DraftItem>> {
    Ok(
        sqlx::query("SELECT * FROM draft_items WHERE draft_id=? ORDER BY position")
            .bind(id)
            .fetch_all(&mut **tx)
            .await?
            .iter()
            .map(|r| DraftItem {
                photo_id: r.get("photo_id"),
                proposed_action: r.get("proposed_action"),
                position: r.get("position"),
                rationale: r.get("rationale"),
            })
            .collect(),
    )
}
pub(crate) async fn mark_stale(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    revision: i64,
) -> Result<()> {
    let ids:Vec<String>=sqlx::query_scalar("SELECT id FROM agent_tasks WHERE project_id=? AND state='review_ready' AND base_revision<>?").bind(project).bind(revision).fetch_all(&mut **tx).await?;
    for id in ids {
        let mut t = load(tx, &id).await?;
        transition(tx, &mut t, "stale", "正式项目已更新，草案需重新比较基线").await?;
    }
    Ok(())
}

// 中断/暂停中的未完成模型等待不能免费重用时长预算；按已持久化尝试时间保守结算。
pub(crate) async fn settle_waiting(
    tx: &mut Transaction<'_, Sqlite>,
    task: &mut Task,
    reason: &str,
) -> Result<()> {
    if task.state != "running" || task.pending.is_some() || task.usage.model_turns == 0 {
        return Ok(());
    }
    let round = task.usage.model_turns;
    let unsettled: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM model_usage WHERE task_id=? AND request_id=? AND output_estimate=0",
    )
    .bind(&task.id)
    .bind(format!("turn-{round}"))
    .fetch_one(&mut **tx)
    .await?;
    let settled:i64=sqlx::query_scalar("SELECT COUNT(*) FROM agent_steps WHERE task_id=? AND kind='model_wait_settled' AND json_extract(body_json,'$.round')=?").bind(&task.id).bind(round as i64).fetch_one(&mut **tx).await?;
    if unsettled == 0 || settled > 0 {
        return Ok(());
    }
    let raw:Option<String>=sqlx::query_scalar("SELECT json_extract(body_json,'$.startedAt') FROM agent_steps WHERE task_id=? AND kind='model_attempt' AND json_extract(body_json,'$.round')=? ORDER BY id DESC LIMIT 1").bind(&task.id).bind(round as i64).fetch_optional(&mut **tx).await?;
    let started = chrono::DateTime::parse_from_rfc3339(raw.as_deref().unwrap_or(&task.updated_at))
        .map_err(|_| AppError::new("RECOVERY_ERROR", "尝试时间无效，不能重用预算"))?;
    let waited = chrono::Utc::now()
        .signed_duration_since(started)
        .num_milliseconds()
        .max(0) as u64;
    let remaining =
        (u64::from(task.budget.max_duration_seconds) * 1000).saturating_sub(task.elapsed_ms);
    let charge = waited.min(remaining);
    task.elapsed_ms += charge;
    step(
        tx,
        &task.id,
        "model_wait_settled",
        json!({"round":round,"chargedMs":charge}),
        reason,
    )
    .await?;
    save(tx, task).await
}

#[cfg(test)]
mod draft_evidence_tests {
    use super::*;
    use crate::command_handler::CommandHandler;

    async fn store_with_project() -> (tempfile::TempDir, Store, String) {
        let temp = tempfile::tempdir().unwrap();
        let db = Store::open(&temp.path().join("evidence.sqlite"))
            .await
            .unwrap();
        let project = db.create("口径出处", "投影摘要").await.unwrap().id;
        // 建任务要求项目里有照片（额度 1–500），所以夹具先放五张，和真实使用同一条路径。
        for n in 0..5 {
            let file = temp.path().join(format!("{n}.png"));
            image::RgbImage::from_pixel(8, 8, image::Rgb([(n * 40) as u8, 60, 100]))
                .save(&file)
                .unwrap();
            let image = crate::image_pipeline::prepare(&file, &temp.path().join("cache")).unwrap();
            db.insert_photo(&project, &image).await.unwrap();
        }
        (temp, db, project)
    }

    async fn new_task(db: &Store, project: &str) -> String {
        CommandHandler::new(db)
            .agent_request(AgentRequest {
                request_id: domain::id(),
                project_id: project.into(),
                command: TaskCommand::StartTask {
                    goal: "核验草案".into(),
                    selected_photo_ids: vec![],
                    target_count: 5,
                    budget: Budget::default(),
                    consent_id: None,
                },
            })
            .await
            .unwrap()
            .id
    }

    async fn receipt(db: &Store, task: &str, result: Value, ok: bool) {
        // 夹具序列化真实的 ToolResult：回执字段怎么拼由它自己定，测试不能另编一套键名。
        let payload = ToolResult {
            call_id: domain::id(),
            ok,
            result: Some(result),
            error: None,
            observation_refs: vec![],
            duration_ms: 0,
            billing_estimate: 0,
        };
        sqlx::query("INSERT INTO tool_calls(task_id,call_id,tool_name,fingerprint,result_json) VALUES(?,?,'evaluate_draft','fixture',?)")
            .bind(task)
            .bind(&payload.call_id)
            .bind(serde_json::to_string(&payload).unwrap())
            .execute(&db.pool)
            .await
            .unwrap();
    }

    fn evaluation(draft: &str) -> Value {
        json!({
            "draft_id": draft, "mode": "structured_local_evaluation", "count": 5, "draft_revision": 2,
            "item_facts": [
                {"photo_id":"p","top_facts":["室内暖光"],"interpretation":"像是停顿","evidence_contract":"current_page_tier"},
                {"photo_id":"r","top_facts":[],"evidence_contract":"current_detail_tier"},
                {"photo_id":"z","top_facts":[],"evidence_contract":"随便写的档位"},
            ],
            "omitted_visual_evidence": [
                {"photo_id":"q","pixel_version":"unreadable","contract":"other_tool"},
                {"photo_id":"w","pixel_version":"verified","contract":"current_page_tier"},
            ],
            "metrics": {"observed":2,"unobserved":3,"evidence_contracts":{"current_page_tier":2,"current_detail_tier":0,"other_prompt_version":0,"other_tool":0},"cross_contract_adjacent_pairs":1},
        })
    }

    #[tokio::test]
    async fn omitted_rows_carry_the_original_status_only_when_it_is_a_known_code() {
        let (_temp, db, project) = store_with_project().await;
        let task = new_task(&db, &project).await;
        let mut result = evaluation("draft-original");
        result["omitted_visual_evidence"] = json!([
            {"photo_id":"q","pixel_version":"unreadable","contract":"other_tool","original_status":"missing"},
            {"photo_id":"w","pixel_version":"mismatch","contract":null,"original_status":"changed"},
            {"photo_id":"e","pixel_version":"unreadable","contract":null,"original_status":"matches"},
            {"photo_id":"t","pixel_version":"unreadable","contract":null,"original_status":"unchecked"},
            {"photo_id":"u","pixel_version":"unreadable","contract":null,"original_status":"随便写的"},
            {"photo_id":"v","pixel_version":"unreadable","contract":null},
        ]);
        receipt(&db, &task, result, true).await;
        let evidence = projection(&db, &project).await.unwrap().tasks[0]
            .draft_evidence
            .clone();
        let row = evidence
            .iter()
            .find(|entry| entry["draftId"] == "draft-original")
            .unwrap();
        let per = row["perPhoto"].as_array().unwrap();
        let get = |id: &str| per.iter().find(|entry| entry["photoId"] == id).unwrap();
        assert_eq!(get("q")["originalStatus"], json!("missing"));
        assert_eq!(get("w")["originalStatus"], json!("changed"));
        assert_eq!(get("e")["originalStatus"], json!("matches"));
        assert_eq!(get("t")["originalStatus"], json!("unchecked"));
        assert!(
            get("u")["originalStatus"].is_null(),
            "陌生取值一律丢掉，不替它猜：{:?}",
            get("u")
        );
        assert!(
            get("v")["originalStatus"].is_null(),
            "老回执没查过原图，这一格就空着"
        );
        assert_eq!(
            get("q")["pixelVersion"],
            json!("unreadable"),
            "预览那一层照旧说，不被原图状态顶掉"
        );
    }

    #[tokio::test]
    async fn counts_are_still_shown_when_the_receipt_is_too_big_to_read_back() {
        let (_temp, db, project) = store_with_project().await;
        let task = new_task(&db, &project).await;
        let mut big = evaluation("draft-big");
        // 顶破单条那道 512KB：垫在界面不读的那份清单上，跟真实顶破那道门时同一个形状。
        big["similar_adjacent"] = json!([{"pad": "x".repeat(600 * 1024)}]);
        receipt(&db, &task, big, true).await;
        receipt(&db, &task, evaluation("draft-small"), true).await;
        let evidence = projection(&db, &project).await.unwrap().tasks[0]
            .draft_evidence
            .clone();
        let summary = evidence
            .iter()
            .find(|row| row["draftId"] == "draft-big")
            .expect("整条读不回来不等于没算过：那几个计数得从库里就地取出来");
        assert_eq!(summary["observed"], 2);
        assert_eq!(summary["omitted"], 2, "排除几张也是就地数出来的");
        assert_eq!(summary["scanned"], 2, "逐张没读回来就不许装成扫过更多张");
        assert_eq!(summary["draftRevision"], 2);
        assert!(summary["perPhoto"].as_array().unwrap().is_empty());
        assert_eq!(summary["basisSummary"], true);
        let plain = evidence
            .iter()
            .find(|row| row["draftId"] == "draft-small")
            .unwrap();
        assert!(
            plain["basisSummary"].is_null(),
            "读得回来的那份不许被贴上降级标记：{plain:?}"
        );
        assert_eq!(plain["perPhoto"].as_array().unwrap().len(), 3);
        let digest = serde_json::to_string(&evidence).unwrap();
        for leaked in ["室内暖光", "像是停顿", "pad"] {
            assert!(
                !digest.contains(leaked),
                "就地取的计数也不许把原文顺路带回来：{leaked}"
            );
        }
    }

    #[tokio::test]
    async fn projects_counts_only_and_keeps_the_latest_receipt_per_draft() {
        let (_temp, db, project) = store_with_project().await;
        let task = new_task(&db, &project).await;
        receipt(&db, &task, evaluation("draft-1"), true).await;
        let mut newer = evaluation("draft-1");
        newer["metrics"]["evidence_contracts"]["current_page_tier"] = json!(3);
        newer["metrics"]["observed"] = json!(3);
        newer["metrics"]["unobserved"] = json!(2);
        receipt(&db, &task, newer, true).await;
        receipt(&db, &task, evaluation("draft-2"), false).await;
        // 老回执没记版本号：删掉字段，模拟升级前留在库里的那批核验记录。
        let mut no_revision = evaluation("draft-3");
        no_revision
            .as_object_mut()
            .unwrap()
            .remove("draft_revision");
        receipt(&db, &task, no_revision, true).await;
        let view = projection(&db, &project).await.unwrap();
        let evidence = &view.tasks[0].draft_evidence;
        assert_eq!(evidence.len(), 2, "失败的回执不能投成依据：{evidence:?}");
        assert_eq!(evidence[0]["draftId"], "draft-1");
        assert_eq!(
            evidence[0]["draftRevision"], 2,
            "口径要能说对着方案的哪一版算的"
        );
        assert!(
            evidence[1]["draftRevision"].is_null(),
            "没记过版本的老回执不能猜一个版本号"
        );
        assert_eq!(evidence[0]["pageTier"], 3, "同一份方案留最近一次核验");
        assert_eq!(
            evidence[0]["omitted"], 2,
            "总数按回执列出的排除条数算，逐张只点名认得出的那种"
        );
        assert_eq!(evidence[0]["crossContractPairs"], 1);
        // 逐张只认写死的几种档位与图像版本：陌生名字整条丢掉，不猜。
        assert_eq!(evidence[0]["scanned"], 2, "陌生档位的逐张记录不该投出去");
        assert_eq!(evidence[0]["perPhoto"].as_array().unwrap().len(), 3);
        assert_eq!(
            evidence[0]["perPhoto"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| row["photoId"] == "z" || row["photoId"] == "w")
                .count(),
            0
        );
        assert_eq!(evidence[0]["perPhoto"][0]["contract"], "current_page_tier");
        assert_eq!(evidence[0]["perPhoto"][0]["used"], true);
        assert_eq!(evidence[0]["perPhoto"][2]["used"], false);
        assert_eq!(evidence[0]["perPhoto"][2]["pixelVersion"], "unreadable");
        assert_eq!(evidence[0]["perPhoto"][2]["contract"], "other_tool");
        // 只投计数与档位：观察正文一个字都不能进投影。
        let digest = serde_json::to_string(evidence).unwrap();
        for leaked in ["室内暖光", "像是停顿", "随便写的", "verified"] {
            assert!(
                !digest.contains(leaked),
                "投影里混进了不该出现的原文：{leaked}"
            );
        }
    }

    #[tokio::test]
    async fn skips_receipts_that_do_not_match_the_documented_shape() {
        let (_temp, db, project) = store_with_project().await;
        let task = new_task(&db, &project).await;
        let mut half = evaluation("draft-1");
        half["metrics"]
            .as_object_mut()
            .unwrap()
            .remove("cross_contract_adjacent_pairs");
        receipt(&db, &task, half, true).await;
        receipt(
            &db,
            &task,
            json!({"draft_id":"draft-1","mode":"vision_observed"}),
            true,
        )
        .await;
        sqlx::query("INSERT INTO tool_calls(task_id,call_id,tool_name,fingerprint,result_json) VALUES(?,?,'evaluate_draft','fixture','这不是 JSON')")
            .bind(&task)
            .bind(domain::id())
            .execute(&db.pool)
            .await
            .unwrap();
        let view = projection(&db, &project).await.unwrap();
        assert!(
            view.tasks[0].draft_evidence.is_empty(),
            "形状对不上就不投，不能猜：{:?}",
            view.tasks[0].draft_evidence
        );
    }

    // ---- 逐张口径补齐：回执带不全的那几张，靠库里的同一套优先级补 ----

    async fn photos(db: &Store, project: &str) -> Vec<(String, String)> {
        sqlx::query_as("SELECT id, content_sha256 FROM photos WHERE project_id=? ORDER BY filename")
            .bind(project)
            .fetch_all(&db.pool)
            .await
            .unwrap()
    }

    /// 只写一条画面记录进库：像素核对是评估那一步的事，投影只看这张按哪一档挑。
    async fn observe(db: &Store, photo: &(String, String), version: &str) {
        sqlx::query("INSERT INTO photo_observations(photo_id,photo_hash,model_id,prompt_version,kind,observation_json,created_at) VALUES(?,?,?,?,?,?,?)")
            .bind(&photo.0)
            .bind(&photo.1)
            .bind("fixture-model")
            .bind(format!("{version}@endpoint"))
            .bind("inspect_photos")
            .bind(json!({"photo_id":photo.0,"visible_facts":["室内暖光"],"interpretation":"像是停顿"}).to_string())
            .bind(domain::timestamp())
            .execute(&db.pool)
            .await
            .unwrap();
    }

    async fn draft(db: &Store, project: &str, task: &str, ids: &[&str]) -> String {
        let id = domain::id();
        sqlx::query("INSERT INTO drafts(id,project_id,task_id,title,base_revision,revision,state,summary,created_at,updated_at) VALUES(?,?,?,?,1,1,'pending',?,?,?)")
            .bind(&id)
            .bind(project)
            .bind(task)
            .bind("补齐夹具")
            .bind("按本地事实挑出的一组")
            .bind(domain::timestamp())
            .bind(domain::timestamp())
            .execute(&db.pool)
            .await
            .unwrap();
        for (position, photo) in ids.iter().enumerate() {
            sqlx::query("INSERT INTO draft_items(draft_id,photo_id,proposed_action,position,rationale) VALUES(?,?, 'keep',?, '序列顺序')")
                .bind(&id)
                .bind(photo)
                .bind(position as i64)
                .execute(&db.pool)
                .await
                .unwrap();
        }
        id
    }

    /// 一份"只点了前几张"的回执：用上的名单、排除的名单与回执自报的用上张数。
    fn partial(
        draft: &str,
        used: &[(&str, &str)],
        omitted: &[(&str, &str)],
        observed: i64,
    ) -> Value {
        json!({
            "draft_id":draft,"mode":"structured_local_evaluation","count":5,"draft_revision":1,
            "item_facts": used.iter().map(|(p,c)| json!({"photo_id":p,"evidence_contract":c})).collect::<Vec<_>>(),
            "omitted_visual_evidence": omitted.iter().map(|(p,c)| json!({"photo_id":p,"pixel_version":"unrecorded","contract":c})).collect::<Vec<_>>(),
            "metrics":{"observed":observed,"unobserved":5-observed,
              "evidence_contracts":{"current_page_tier":observed,"current_detail_tier":0,"other_prompt_version":0,"other_tool":0},
              "cross_contract_adjacent_pairs":0},
        })
    }

    async fn marks(db: &Store, project: &str, draft: &str) -> Value {
        let view = projection(db, project).await.unwrap();
        view.tasks[0]
            .draft_evidence
            .iter()
            .find(|row| row["draftId"] == draft)
            .cloned()
            .unwrap_or_else(|| panic!("方案 {draft} 的口径没投出来：{view:?}"))
            .get("perPhoto")
            .cloned()
            .unwrap_or(json!([]))
    }

    #[tokio::test]
    async fn fills_the_photos_the_receipt_could_not_list_and_uses_the_shared_priority() {
        let (_temp, db, project) = store_with_project().await;
        let task = new_task(&db, &project).await;
        let all = photos(&db, &project).await;
        let ids: Vec<&str> = all.iter().map(|photo| photo.0.as_str()).collect();
        for photo in &all[0..4] {
            observe(&db, photo, crate::agent::vision::VISION_PROMPT_VERSION).await;
        }
        // 第三张同时存着两种问法：补口径要按评估那套优先级说它是页面档问出来的，不能挑最新的编号。
        observe(&db, &all[2], crate::agent::vision::DETAIL_PROMPT_VERSION).await;
        let draft = draft(&db, &project, &task, &ids).await;
        receipt(
            &db,
            &task,
            partial(
                &draft,
                &[(ids[0], "current_page_tier"), (ids[1], "current_page_tier")],
                &[(ids[3], "current_page_tier")],
                3,
            ),
            true,
        )
        .await;
        let rows = marks(&db, &project, &draft).await;
        let listed = rows.as_array().unwrap();
        let mark = |id: &str| {
            listed
                .iter()
                .find(|row| row["photoId"] == id)
                .cloned()
                .unwrap_or_default()
        };
        assert_eq!(mark(ids[2])["used"], true, "回执没点到的那张要补上");
        assert_eq!(
            mark(ids[2])["contract"],
            "current_page_tier",
            "补口径要跟着评估那套优先级走"
        );
        assert_eq!(
            mark(ids[4]),
            json!(null),
            "库里没有画面记录的照片不能凭空补一条"
        );
        assert_eq!(
            mark(ids[3])["used"],
            false,
            "回执说排除的那张不能被补成用上了"
        );
        assert_eq!(
            listed
                .iter()
                .filter(|row| row["used"] == json!(true))
                .count(),
            3,
            "逐张点到的张数要和回执说的用上一致：{listed:?}"
        );
        assert_eq!(
            projection(&db, &project).await.unwrap().tasks[0].draft_evidence[0]["scanned"],
            3,
            "补齐后界面就不该再说\u{201c}只点到列出的那几张\u{201d}"
        );
    }

    #[tokio::test]
    async fn stops_extending_when_the_library_and_the_receipt_disagree() {
        let (_temp, db, project) = store_with_project().await;
        let task = new_task(&db, &project).await;
        let all = photos(&db, &project).await;
        let ids: Vec<&str> = all.iter().map(|photo| photo.0.as_str()).collect();
        for photo in &all[0..4] {
            observe(&db, photo, crate::agent::vision::DETAIL_PROMPT_VERSION).await;
        }
        let draft = draft(&db, &project, &task, &ids).await;
        // 回执说这些是页面档问的，库里现在挑出来的却是细节档：对不上号，一条都不补。
        receipt(
            &db,
            &task,
            partial(
                &draft,
                &[(ids[0], "current_page_tier"), (ids[1], "current_page_tier")],
                &[(ids[3], "current_page_tier")],
                3,
            ),
            true,
        )
        .await;
        let rows = marks(&db, &project, &draft).await;
        assert_eq!(
            rows.as_array().unwrap().len(),
            3,
            "只留回执自己点到的：{rows:?}"
        );
        assert_eq!(
            projection(&db, &project).await.unwrap().tasks[0].draft_evidence[0]["scanned"],
            2,
            "对不上账时逐张口径停在回执列出的那几张"
        );
        // 回执之后库里又多出一条画面记录：多出来的那张没人核对过，同样不能借补齐的名义说成看过。
        observe(&db, &all[4], crate::agent::vision::DETAIL_PROMPT_VERSION).await;
        receipt(
            &db,
            &task,
            partial(
                &draft,
                &[
                    (ids[0], "current_detail_tier"),
                    (ids[1], "current_detail_tier"),
                ],
                &[(ids[3], "current_detail_tier")],
                3,
            ),
            true,
        )
        .await;
        let rows = marks(&db, &project, &draft).await;
        assert_eq!(
            rows.as_array()
                .unwrap()
                .iter()
                .filter(|row| row["photoId"] == ids[2] || row["photoId"] == ids[4])
                .count(),
            0,
            "库里比回执多出一条记录时不能补齐：{rows:?}"
        );
    }

    #[tokio::test]
    async fn reconciles_against_a_receipt_the_evaluator_wrote_itself() {
        let (_temp, db, project) = store_with_project().await;
        let task = new_task(&db, &project).await;
        let all = photos(&db, &project).await;
        let ids: Vec<&str> = all.iter().map(|photo| photo.0.as_str()).collect();
        for photo in &all {
            observe(&db, photo, crate::agent::vision::VISION_PROMPT_VERSION).await;
        }
        let draft = draft(&db, &project, &task, &ids).await;
        // 走真正的本地核验写回执：键名与档位名字对不上，这份测试就会失败，而不是让补齐悄悄失灵。
        // 这些观察都没记像素版本，全都要被排除，逐张口径得把五张都点到。
        crate::agent::recheck::run(&db, &project, &draft)
            .await
            .unwrap();
        let rows = marks(&db, &project, &draft).await;
        let listed = rows.as_array().unwrap();
        assert_eq!(listed.len(), 5, "真实回执点到的每一张都要上屏：{listed:?}");
        assert!(
            listed
                .iter()
                .all(|row| row["used"] == json!(false) && row["pixelVersion"] == "unrecorded"),
            "没核对过像素的一张都不能被说成用上了：{listed:?}"
        );
    }

    #[tokio::test]
    async fn projects_a_receipt_larger_than_the_length_gate_it_used_to_have() {
        let (_temp, db, project) = store_with_project().await;
        let task = new_task(&db, &project).await;
        let all = photos(&db, &project).await;
        let ids: Vec<&str> = all.iter().map(|photo| photo.0.as_str()).collect();
        for photo in &all {
            observe(&db, photo, crate::agent::vision::VISION_PROMPT_VERSION).await;
        }
        let draft = draft(&db, &project, &task, &ids).await;
        // 顶破原先那道 64KB 的正是"相邻相似"这类不计条数的清单：界面一个字的它都不读，
        // 可它一大，整份方案的口径就连同那几张逐张说法一起被挡在门外。
        let mut big = partial(&draft, &[(ids[0], "current_page_tier")], &[], 5);
        big["similar_adjacent"] = json!(
            (0..1500)
                .map(|n| json!({
                    "positions": [n, n + 1],
                    "photo_ids": [format!("photo-{n}"), format!("photo-{}", n + 1)],
                    "shared_fact_count": 3,
                    "same_contract": true,
                }))
                .collect::<Vec<Value>>()
        );
        let raw = serde_json::to_string(&big).unwrap();
        assert!(
            raw.len() > 65536 && raw.len() < 512 * 1024,
            "夹具没造出真实超限的体积：{} 字节",
            raw.len()
        );
        receipt(&db, &task, big, true).await;
        let evidence = projection(&db, &project).await.unwrap().tasks[0]
            .draft_evidence
            .clone();
        assert_eq!(
            evidence.len(),
            1,
            "大到原先那道门的回执也要投出口径：{evidence:?}"
        );
        let row = &evidence[0];
        assert_eq!(row["observed"], 5, "头上的计数照旧说得出");
        assert_eq!(row["pageTier"], 5);
        assert_eq!(
            row["perPhoto"].as_array().unwrap().len(),
            5,
            "回执只点了第一张，其余四张靠库里的同一套优先级补齐"
        );
        assert_eq!(row["scanned"], 5, "补齐不能因为整条变大就停住");
    }
}
