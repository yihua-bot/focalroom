// 人工记忆控制独立于正式编辑栈，仍经唯一 CommandHandler 与持久化回执。
use super::{memory, store};
use crate::{
    command_audit::{fingerprint, serialization},
    db::Store,
    domain::{self, AppError, Result},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{Row, Sqlite, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemoryRequest {
    pub request_id: String,
    pub project_id: String,
    pub command: MemoryCommand,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all_fields = "camelCase", deny_unknown_fields)]
pub enum MemoryCommand {
    SetEvidenceExcluded {
        evidence_id: i64,
        excluded: bool,
    },
    ConfirmHypothesis {
        hypothesis_id: i64,
    },
    RetireHypothesis {
        hypothesis_id: i64,
    },
    RestoreHypothesis {
        hypothesis_id: i64,
    },
    CorrectHypothesis {
        hypothesis_id: i64,
        correction: String,
    },
    SetHypothesisEnabled {
        hypothesis_id: i64,
        enabled: bool,
    },
    /// 物理删除：不可撤销；写墓碑防止确定性物化复活（ADR 0014）。
    DeleteHypothesis {
        hypothesis_id: i64,
    },
    DeleteEvidence {
        evidence_id: i64,
    },
    /// 清空本项目的视觉观察缓存（photo_observations）；不影响证据/假设/草案。
    PurgeObservationCache {},
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryReceipt {
    pub request_id: String,
    pub changed: bool,
    pub memory_revision: i64,
}
pub fn reply(request_id: &str, result: Result<MemoryReceipt>) -> serde_json::Value {
    match result {
        Ok(receipt) => json!({"ok":true,"data":receipt,"error":null,"requestId":request_id}),
        Err(error) => {
            json!({"ok":false,"data":null,"error":{"code":error.code,"message":error.message,"retryable":error.retriable},"requestId":request_id})
        }
    }
}

pub(crate) async fn revision(tx: &mut Transaction<'_, Sqlite>, project: &str) -> Result<i64> {
    Ok(
        sqlx::query_scalar("SELECT COUNT(*) FROM memory_commands WHERE project_id=? AND changed=1")
            .bind(project)
            .fetch_one(&mut **tx)
            .await?,
    )
}
pub(crate) async fn execute(db: &Store, request: MemoryRequest) -> Result<MemoryReceipt> {
    super::policy::validate_schema(
        &serde_json::from_str(include_str!(
            "../../../contracts/memory-command-request.schema.json"
        ))
        .unwrap(),
        &serde_json::to_value(&request).unwrap(),
    )?;
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let active: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM projects WHERE id=? AND deleted_at IS NULL")
            .bind(&request.project_id)
            .fetch_one(&mut *tx)
            .await?;
    if active != 1 {
        return Err(AppError::new("NOT_FOUND", "项目不存在或已归档"));
    }
    let fp = fingerprint(&serde_json::to_value(&request).unwrap());
    if let Some(row) = sqlx::query(
        "SELECT project_id,fingerprint,result_json FROM memory_commands WHERE request_id=?",
    )
    .bind(&request.request_id)
    .fetch_optional(&mut *tx)
    .await?
    {
        if row.get::<String, _>("project_id") != request.project_id
            || row.get::<String, _>("fingerprint") != fp
        {
            return Err(AppError::new(
                "IDEMPOTENCY_CONFLICT",
                "记忆请求 ID 已用于不同参数",
            ));
        }
        return serde_json::from_str(&row.get::<String, _>("result_json")).map_err(serialization);
    }
    memory::materialize(&mut tx, &request.project_id).await?;
    let changed = match &request.command {
        MemoryCommand::DeleteHypothesis { hypothesis_id } => {
            let old = memory::row_view(&mut tx, *hypothesis_id).await?;
            if old.project_id != request.project_id {
                return Err(AppError::new("NOT_FOUND", "假设不属于本项目"));
            }
            // 墓碑按关键词登记（表述含证据数，会随新证据漂移导致复活；ADR 0014 修订）。
            sqlx::query("INSERT OR IGNORE INTO memory_tombstones(project_id,kind,ref_key,created_at) VALUES(?,'hypothesis',?,?)")
                .bind(&request.project_id)
                .bind(super::memory_search::keyword(&old.statement))
                .bind(domain::timestamp())
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM preference_hypotheses WHERE id=? AND project_id=?")
                .bind(hypothesis_id)
                .bind(&request.project_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM memory_search WHERE project_id=?")
                .bind(&request.project_id)
                .execute(&mut *tx)
                .await?;
            true
        }
        MemoryCommand::DeleteEvidence { evidence_id } => {
            let owner: Option<i64> = sqlx::query_scalar(
                "SELECT id FROM preference_evidence WHERE id=? AND project_id=?",
            )
            .bind(evidence_id)
            .bind(&request.project_id)
            .fetch_optional(&mut *tx)
            .await?;
            if owner.is_none() {
                return Err(AppError::new("NOT_FOUND", "证据不属于本项目"));
            }
            sqlx::query("DELETE FROM memory_evidence_state WHERE evidence_id=?")
                .bind(evidence_id)
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM preference_evidence WHERE id=?")
                .bind(evidence_id)
                .execute(&mut *tx)
                .await?;
            // 引用该证据的假设移除引用；引用清空的假设退役（不可再确认）。
            let hypotheses: Vec<i64> = sqlx::query_scalar(
                "SELECT id FROM preference_hypotheses WHERE project_id=? AND evidence_ids_json LIKE ?",
            )
            .bind(&request.project_id)
            .bind(format!("%{evidence_id}%"))
            .fetch_all(&mut *tx)
            .await?;
            for hypothesis in hypotheses {
                let view = memory::row_view(&mut tx, hypothesis).await?;
                let remaining: Vec<i64> = view
                    .evidence_ids
                    .into_iter()
                    .filter(|id| id != evidence_id)
                    .collect();
                if remaining.is_empty() {
                    sqlx::query("UPDATE preference_hypotheses SET evidence_ids_json='[]',status='retired',disabled_for_retrieval=1,updated_at=? WHERE id=?")
                        .bind(domain::timestamp())
                        .bind(hypothesis)
                        .execute(&mut *tx)
                        .await?;
                } else {
                    sqlx::query("UPDATE preference_hypotheses SET evidence_ids_json=?,updated_at=? WHERE id=?")
                        .bind(serde_json::to_string(&remaining).unwrap())
                        .bind(domain::timestamp())
                        .bind(hypothesis)
                        .execute(&mut *tx)
                        .await?;
                }
            }
            sqlx::query("DELETE FROM memory_search WHERE project_id=?")
                .bind(&request.project_id)
                .execute(&mut *tx)
                .await?;
            true
        }
        MemoryCommand::PurgeObservationCache {} => {
            let removed = sqlx::query("DELETE FROM photo_observations WHERE photo_id IN (SELECT id FROM photos WHERE project_id=?)")
                .bind(&request.project_id)
                .execute(&mut *tx)
                .await?
                .rows_affected();
            removed > 0
        }
        MemoryCommand::SetEvidenceExcluded {
            evidence_id,
            excluded,
        } => {
            let old:Option<i64>=sqlx::query_scalar("SELECT COALESCE(s.excluded,0) FROM preference_evidence e LEFT JOIN memory_evidence_state s ON s.evidence_id=e.id WHERE e.id=? AND e.project_id=?").bind(evidence_id).bind(&request.project_id).fetch_optional(&mut *tx).await?;
            let old = old.ok_or_else(|| AppError::new("NOT_FOUND", "证据不属于本项目"))?;
            let changed = (old != 0) != *excluded;
            if changed {
                sqlx::query("INSERT INTO memory_evidence_state VALUES(?,?,?) ON CONFLICT(evidence_id) DO UPDATE SET excluded=excluded.excluded,updated_at=excluded.updated_at").bind(evidence_id).bind(*excluded).bind(domain::timestamp()).execute(&mut *tx).await?;
            }
            changed
        }
        command => {
            let id = match command {
                MemoryCommand::ConfirmHypothesis { hypothesis_id }
                | MemoryCommand::RetireHypothesis { hypothesis_id }
                | MemoryCommand::RestoreHypothesis { hypothesis_id }
                | MemoryCommand::CorrectHypothesis { hypothesis_id, .. }
                | MemoryCommand::SetHypothesisEnabled { hypothesis_id, .. } => *hypothesis_id,
                _ => unreachable!(),
            };
            let old = memory::row_view(&mut tx, id).await?;
            if old.project_id != request.project_id {
                return Err(AppError::new("NOT_FOUND", "假设不属于本项目"));
            }
            if old.scope != "project_local" {
                return Err(AppError::new("PERMISSION_DENIED", "跨项目记忆尚未启用"));
            }
            let (mut status, mut disabled, mut correction) = (
                old.status.clone(),
                old.disabled,
                old.user_correction.clone(),
            );
            match command {
                MemoryCommand::ConfirmHypothesis { .. } => {
                    if ["corrected", "retired"].contains(&status.as_str())
                        || old.evidence_ids.is_empty()
                    {
                        return Err(AppError::new(
                            "INVALID_TRANSITION",
                            "该假设不能确认，请先恢复有效证据",
                        ));
                    }
                    status = "confirmed".into();
                }
                MemoryCommand::RetireHypothesis { .. } => {
                    status = "retired".into();
                    disabled = true;
                }
                MemoryCommand::RestoreHypothesis { .. } => {
                    if status != "retired" {
                        return Err(AppError::new("INVALID_TRANSITION", "假设未移除"));
                    }
                    status = if correction.is_some() {
                        "corrected"
                    } else {
                        "unconfirmed"
                    }
                    .into();
                    disabled = correction.is_some();
                }
                MemoryCommand::CorrectHypothesis {
                    correction: text, ..
                } => {
                    if text.trim().is_empty() || text.chars().count() > 2000 || status == "retired"
                    {
                        return Err(AppError::new(
                            "VALIDATION_ERROR",
                            "纠正文本不合法或假设已移除",
                        ));
                    }
                    status = "corrected".into();
                    disabled = true;
                    correction = Some(text.trim().into());
                }
                MemoryCommand::SetHypothesisEnabled { enabled, .. } => {
                    if *enabled && ["corrected", "retired"].contains(&status.as_str()) {
                        return Err(AppError::new(
                            "INVALID_TRANSITION",
                            "已纠正或移除的假设不能恢复检索",
                        ));
                    }
                    disabled = !*enabled;
                }
                _ => unreachable!(),
            }
            let changed = status != old.status
                || disabled != old.disabled
                || correction != old.user_correction;
            if changed {
                sqlx::query("UPDATE preference_hypotheses SET status=?,disabled_for_retrieval=?,user_correction=?,updated_at=? WHERE id=? AND project_id=?").bind(status).bind(disabled).bind(correction).bind(domain::timestamp()).bind(id).bind(&request.project_id).execute(&mut *tx).await?;
            }
            changed
        }
    };
    let receipt = MemoryReceipt {
        request_id: request.request_id.clone(),
        changed,
        memory_revision: revision(&mut tx, &request.project_id).await? + i64::from(changed),
    };
    sqlx::query("INSERT INTO memory_commands VALUES(?,?,?,?,?,?,?)")
        .bind(&request.request_id)
        .bind(&request.project_id)
        .bind(fp)
        .bind(serde_json::to_string(&request.command).unwrap())
        .bind(changed)
        .bind(serde_json::to_string(&receipt).unwrap())
        .bind(domain::timestamp())
        .execute(&mut *tx)
        .await?;
    if changed {
        memory::materialize(&mut tx, &request.project_id).await?;
        let ids:Vec<String>=sqlx::query_scalar("SELECT id FROM agent_tasks WHERE project_id=? AND state IN('queued','planning','running')").bind(&request.project_id).fetch_all(&mut *tx).await?;
        for id in ids {
            let mut task = store::load(&mut tx, &id).await?;
            store::settle_waiting(&mut tx, &mut task, "记忆变更前结算模型等待").await?;
            store::transition(
                &mut tx,
                &mut task,
                "paused",
                "记忆已调整，请基于当前记忆重新委托；旧草案保留",
            )
            .await?;
            store::checkpoint(&mut tx, &task).await?;
        }
    }
    tx.commit().await?;
    Ok(receipt)
}

impl crate::command_handler::CommandHandler<'_> {
    pub async fn execute_memory(&self, request: MemoryRequest) -> Result<MemoryReceipt> {
        execute(self.store, request).await
    }
}
