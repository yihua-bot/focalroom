// v2.0 M5 提案协议：operations 化的编辑提案与人工审批（ADR 0020）。
// 语义：create（可提案者/Agent）→ submit → apply（can_approve 者 + nonce + 部分采纳）。
// 人工审批经唯一 CommandHandler 在一个事务内执行，审批状态、正式编辑与历史一起提交。
use crate::{
    db::Store,
    domain::{self, AppError, Result},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::Row;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Proposal {
    pub id: String,
    pub project_id: String,
    pub task_id: Option<String>,
    pub author: String,
    pub title: String,
    pub base_revision: i64,
    pub revision: i64,
    pub state: String,
    pub approval_nonce: String,
    pub operations: Vec<Operation>,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub operation_id: String,
    pub photo_id: String,
    pub op: String,
    pub to_index: Option<i64>,
    #[serde(default)]
    pub reason: String,
}

fn validate_operations(operations: &[Operation]) -> Result<()> {
    if operations.is_empty() || operations.len() > 200 {
        return Err(AppError::new("VALIDATION_ERROR", "提案须含 1–200 个操作"));
    }
    for op in operations {
        if !matches!(op.op.as_str(), "add" | "move" | "remove") {
            return Err(AppError::new("VALIDATION_ERROR", "操作类型不合法"));
        }
        if (op.op == "add" || op.op == "move") && op.to_index.is_none() {
            return Err(AppError::new(
                "VALIDATION_ERROR",
                "add/move 操作必须带 to_index",
            ));
        }
    }
    Ok(())
}

pub(crate) async fn create_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    project_id: &str,
    author: &str,
    task_id: Option<&str>,
    base_revision: i64,
    title: &str,
    operations: Value,
) -> Result<Proposal> {
    let parsed: Vec<Operation> = serde_json::from_value(operations)
        .map_err(|_| AppError::new("VALIDATION_ERROR", "操作列表结构不合法"))?;
    validate_operations(&parsed)?;
    if title.trim().is_empty() || title.chars().count() > 200 {
        return Err(AppError::new("VALIDATION_ERROR", "提案标题不合法"));
    }
    // 照片归属校验（防跨项目引用）。
    for op in &parsed {
        let owned: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM photos WHERE id=? AND project_id=?")
                .bind(&op.photo_id)
                .bind(project_id)
                .fetch_one(&mut **tx)
                .await
                .unwrap_or(0);
        if owned != 1 {
            return Err(AppError::new(
                "PERMISSION_DENIED",
                "操作引用了不属于此项目的照片",
            ));
        }
    }
    let project = crate::db::snapshot_tx(tx, project_id, None).await?;
    if base_revision != project.revision {
        return Err(AppError::new(
            "REVISION_CONFLICT",
            "提案必须基于当前项目版本",
        ));
    }
    let now = domain::timestamp();
    let id = domain::id();
    let nonce = domain::id();
    sqlx::query("INSERT INTO proposals(id,project_id,task_id,author,title,base_revision,state,approval_nonce,created_at,updated_at) VALUES(?,?,?,?,?,?,'draft',?,?,?)")
        .bind(&id).bind(project_id).bind(task_id).bind(author.trim()).bind(title.trim()).bind(base_revision).bind(&nonce).bind(&now).bind(&now)
        .execute(&mut **tx).await?;
    for op in &parsed {
        sqlx::query("INSERT INTO proposal_operations(proposal_id,operation_id,photo_id,op,to_index,reason) VALUES(?,?,?,?,?,?)")
            .bind(&id).bind(&op.operation_id).bind(&op.photo_id).bind(&op.op).bind(op.to_index).bind(&op.reason)
            .execute(&mut **tx).await?;
    }
    view_tx(tx, &id).await
}

pub(crate) async fn submit_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    project_id: &str,
    proposal_id: &str,
    expected_revision: i64,
) -> Result<Proposal> {
    transition(
        tx,
        project_id,
        proposal_id,
        "draft",
        "submitted",
        expected_revision,
    )
    .await
}

async fn transition(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    project_id: &str,
    proposal_id: &str,
    from: &str,
    to: &str,
    expected_revision: i64,
) -> Result<Proposal> {
    let row = sqlx::query("SELECT revision,state FROM proposals WHERE id=? AND project_id=?")
        .bind(proposal_id)
        .bind(project_id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(|| AppError::new("NOT_FOUND", "提案不属于此项目"))?;
    let (revision, state): (i64, String) = (row.get("revision"), row.get("state"));
    if state != from {
        return Err(AppError::new(
            "INVALID_TRANSITION",
            format!("提案状态为 {state}，不能执行此操作"),
        ));
    }
    if revision != expected_revision {
        return Err(AppError::new("REVISION_CONFLICT", "提案版本不匹配"));
    }
    sqlx::query("UPDATE proposals SET state=?,revision=revision+1,updated_at=? WHERE id=?")
        .bind(to)
        .bind(domain::timestamp())
        .bind(proposal_id)
        .execute(&mut **tx)
        .await?;
    view_tx(tx, proposal_id).await
}

/// 同一事务中按提案顺序解释操作，全部成功后才发布正式序列与审批状态。
pub(crate) async fn apply_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    project_id: &str,
    proposal_id: &str,
    expected_revision: i64,
    accepted: &[String],
    nonce: &str,
) -> Result<()> {
    let proposal = view_tx(tx, proposal_id).await?;
    if proposal.project_id != project_id {
        return Err(AppError::new("NOT_FOUND", "提案不属于此项目"));
    }
    if proposal.state != "submitted" {
        return Err(AppError::new(
            "INVALID_TRANSITION",
            "仅已提交提案可审批执行",
        ));
    }
    if proposal.revision != expected_revision {
        return Err(AppError::new("REVISION_CONFLICT", "提案版本不匹配"));
    }
    if nonce != proposal.approval_nonce {
        return Err(AppError::new("PERMISSION_DENIED", "审批 nonce 不匹配"));
    }
    let project = crate::db::snapshot_tx(tx, project_id, None).await?;
    if project.revision != proposal.base_revision {
        return Err(AppError::new(
            "REVISION_CONFLICT",
            "项目已更新；请丢弃该提案并基于新版本重建",
        ));
    }
    let known: std::collections::HashSet<_> = proposal
        .operations
        .iter()
        .map(|op| &op.operation_id)
        .collect();
    if accepted.len() > 200
        || accepted
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len()
            != accepted.len()
        || accepted.iter().any(|id| !known.contains(id))
    {
        return Err(AppError::new(
            "VALIDATION_ERROR",
            "接受清单包含重复或不存在的操作",
        ));
    }
    let mut sequence = project.sequence;
    for op in proposal
        .operations
        .iter()
        .filter(|op| accepted.contains(&op.operation_id))
    {
        crate::edit_state::validate_selection(tx, project_id, std::slice::from_ref(&op.photo_id))
            .await?;
        match op.op.as_str() {
            "add" | "move" => {
                let from = sequence.iter().position(|id| id == &op.photo_id);
                if op.op == "move" && from.is_none() {
                    return Err(AppError::new("VALIDATION_ERROR", "move 的照片不在当前序列"));
                }
                if op.op == "add" && from.is_some() {
                    return Err(AppError::new(
                        "VALIDATION_ERROR",
                        "add 的照片已在序列中，请使用 move",
                    ));
                }
                if let Some(from) = from {
                    sequence.remove(from);
                }
                let to = op
                    .to_index
                    .and_then(|v| usize::try_from(v).ok())
                    .filter(|to| *to <= sequence.len())
                    .ok_or_else(|| AppError::new("VALIDATION_ERROR", "to_index 超出序列范围"))?;
                sequence.insert(to, op.photo_id.clone());
            }
            "remove" => sequence.retain(|id| id != &op.photo_id),
            _ => return Err(AppError::new("VALIDATION_ERROR", "操作类型不合法")),
        }
        if op.op != "move" {
            sqlx::query("UPDATE photo_decisions SET status=?,updated_at=? WHERE project_id=? AND photo_id=?")
                .bind(if op.op == "add" { "keep" } else { "undecided" })
                .bind(domain::timestamp()).bind(project_id).bind(&op.photo_id).execute(&mut **tx).await?;
        }
    }
    if !accepted.is_empty() {
        crate::db::write_sequence(tx, project_id, &sequence).await?;
    }
    sqlx::query("UPDATE proposals SET state=?,revision=revision+1,updated_at=? WHERE id=?")
        .bind(if accepted.is_empty() {
            "discarded"
        } else {
            "applied"
        })
        .bind(domain::timestamp())
        .bind(proposal_id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn view(store: &Store, proposal_id: &str) -> Result<Proposal> {
    let mut tx = store.pool.begin().await?;
    let result = view_tx(&mut tx, proposal_id).await?;
    tx.commit().await?;
    Ok(result)
}
async fn view_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    proposal_id: &str,
) -> Result<Proposal> {
    let row = sqlx::query("SELECT * FROM proposals WHERE id=?")
        .bind(proposal_id)
        .fetch_one(&mut **tx)
        .await?;
    let operations =
        sqlx::query("SELECT * FROM proposal_operations WHERE proposal_id=? ORDER BY rowid")
            .bind(proposal_id)
            .fetch_all(&mut **tx)
            .await?
            .iter()
            .map(|r| Operation {
                operation_id: r.get("operation_id"),
                photo_id: r.get("photo_id"),
                op: r.get("op"),
                to_index: r.get("to_index"),
                reason: r.get("reason"),
            })
            .collect();
    Ok(Proposal {
        id: row.get("id"),
        project_id: row.get("project_id"),
        task_id: row.get("task_id"),
        author: row.get("author"),
        title: row.get("title"),
        base_revision: row.get("base_revision"),
        revision: row.get("revision"),
        state: row.get("state"),
        approval_nonce: row.get("approval_nonce"),
        operations,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

/// 项目提案列表（新在前；含操作与状态，供 GUI 审批视图）。
pub async fn list(store: &Store, project_id: &str, limit: usize) -> Result<Value> {
    let rows: Vec<(String, String)> = sqlx::query(
        "SELECT id,state FROM proposals WHERE project_id=? ORDER BY updated_at DESC, rowid DESC LIMIT ?",
    )
    .bind(project_id)
    .bind(limit as i64)
    .fetch_all(&store.pool)
    .await?
    .iter()
    .map(|r| (r.get("id"), r.get("state")))
    .collect();
    let mut items = vec![];
    for (id, _) in rows {
        let view = view(store, &id).await?;
        items.push(json!({
            "id": view.id, "taskId": view.task_id, "author": view.author,
            "title": view.title, "state": view.state,
            "baseRevision": view.base_revision, "revision": view.revision,
            "approvalNonce": view.approval_nonce,
            "operations": view.operations.iter().map(|op| json!({
                "operationId": op.operation_id, "photoId": op.photo_id,
                "op": op.op, "toIndex": op.to_index, "reason": op.reason,
            })).collect::<Vec<_>>(),
            "createdAt": view.created_at, "updatedAt": view.updated_at,
        }));
    }
    Ok(json!({"proposals": items}))
}
