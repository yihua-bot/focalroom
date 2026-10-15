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
        target_count: r.get::<i64, _>("target_count") as u32,
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
        "queued" => matches!(next, "planning" | "paused" | "cancelled" | "interrupted"),
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
    let ids: Vec<String> =
        sqlx::query_scalar("SELECT call_id FROM tool_calls WHERE task_id=? ORDER BY rowid")
            .bind(&task.id)
            .fetch_all(&mut **tx)
            .await?;
    sqlx::query("INSERT INTO agent_checkpoints(task_id,state,context_json,completed_calls_json,budget_json,created_at) VALUES(?,?,?,?,?,?)").bind(&task.id).bind(&task.state).bind(json!({"memoryRevision":task.memory_revision,"providerId":task.provider_id,"modelId":model_id_of(&task.provider_id),"promptVersion":super::prompt_version(&task.provider_id),"goal":task.goal,"projectId":task.project_id,"baseRevision":task.base_revision,"allowedPhotoIds":task.allowed_photo_ids,"selectedPhotoIds":task.selected_photo_ids,"pending":task.pending,"protocol":history,"question":task.question,"answer":task.answer,"finalDraftIds":task.final_draft_ids}).to_string()).bind(encode(&ids)?).bind(json!({"limits":task.budget,"usage":task.usage,"elapsedMs":task.elapsed_ms}).to_string()).bind(domain::timestamp()).execute(&mut **tx).await?;
    Ok(())
}
fn model_id_of(provider: &str) -> String {
    if provider == super::PROVIDER_OPENAI {
        std::env::var(super::openai::MODEL_ENV)
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| super::openai::DEFAULT_MODEL.into())
    } else {
        super::MODEL_ID.into()
    }
}
pub async fn projection(store: &Store, project: &str) -> Result<AgentProjection> {
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
        tasks.push(TaskView {
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
