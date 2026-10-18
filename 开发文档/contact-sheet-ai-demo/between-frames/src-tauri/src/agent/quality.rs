// M3 质量驱动选模：盲评记录的写入与读取。
// 记录来自摄影师盲评（AGENT-QUALITY-EVAL 协议）；运行时只读消费，写入方须保证理由可核验。
use super::router::{QualityRecord, TaskType};
use crate::domain::{AppError, Result};
use sqlx::{Pool, Row, Sqlite};

fn task_type_key(task_type: TaskType) -> &'static str {
    match task_type {
        TaskType::Index => "index",
        TaskType::VisionObserve => "vision_observe",
        TaskType::AgentOrchestrate => "agent_orchestrate",
        TaskType::Curation => "curation",
        TaskType::MemoryExtract => "memory_extract",
        TaskType::ReviewSummary => "review_summary",
    }
}

pub(crate) fn parse_task_type(raw: &str) -> Result<TaskType> {
    Ok(match raw {
        "index" => TaskType::Index,
        "vision_observe" => TaskType::VisionObserve,
        "agent_orchestrate" => TaskType::AgentOrchestrate,
        "curation" => TaskType::Curation,
        "memory_extract" => TaskType::MemoryExtract,
        "review_summary" => TaskType::ReviewSummary,
        other => {
            return Err(AppError::new(
                "VALIDATION_ERROR",
                format!("未知任务类型：{other}"),
            ));
        }
    })
}

/// 读取某 provider（可选限定任务类型）的全部有效质量记录，按评估时间升序；
/// 路由取每候选最新一条（effective_mean 用 rev().find()）。
pub async fn load_quality_records(
    pool: &Pool<Sqlite>,
    provider_id: &str,
) -> Result<Vec<QualityRecord>> {
    let rows = sqlx::query(
        "SELECT provider_id, model_id, task_type, score_version, sample_count, hard_fail, \
         (intent_score + narrative_score + rationale_score + fact_score + review_score) AS total \
         FROM model_quality_scores WHERE provider_id=? ORDER BY evaluated_at, id",
    )
    .bind(provider_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .iter()
        .map(|row| QualityRecord {
            provider_id: row.get("provider_id"),
            model_id: row.get("model_id"),
            task_type: parse_task_type(row.get::<String, _>("task_type").as_str())
                .unwrap_or(TaskType::Curation),
            mean_score: row.get::<i64, _>("total") as f64 / 25.0 * 5.0,
            sample_count: row.get::<i64, _>("sample_count") as u32,
            score_version: row.get("score_version"),
            hard_fail: row.get::<i64, _>("hard_fail") != 0,
        })
        .collect())
}

/// 评测工具写入一条盲评记录；五维分、样本量与评审理由为硬性要求。
#[allow(clippy::too_many_arguments)]
pub async fn record_quality_score(
    pool: &Pool<Sqlite>,
    provider_id: &str,
    model_id: &str,
    task_type: TaskType,
    score_version: &str,
    scores: [i64; 5],
    sample_count: u32,
    hard_fail: bool,
    rationale: &str,
) -> Result<()> {
    if scores.iter().any(|score| !(1..=5).contains(score)) {
        return Err(AppError::new("VALIDATION_ERROR", "五维评分须在 1–5 之间"));
    }
    if sample_count == 0 {
        return Err(AppError::new("VALIDATION_ERROR", "样本量至少为 1"));
    }
    if rationale.trim().is_empty() {
        return Err(AppError::new(
            "VALIDATION_ERROR",
            "评审理由不能为空（可核验性硬要求）",
        ));
    }
    sqlx::query(
        "INSERT INTO model_quality_scores(provider_id, model_id, task_type, score_version, \
         intent_score, narrative_score, rationale_score, fact_score, review_score, rationale, sample_count, hard_fail, evaluated_at) \
         VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?)",
    )
    .bind(provider_id)
    .bind(model_id)
    .bind(task_type_key(task_type))
    .bind(score_version)
    .bind(scores[0])
    .bind(scores[1])
    .bind(scores[2])
    .bind(scores[3])
    .bind(scores[4])
    .bind(rationale)
    .bind(sample_count as i64)
    .bind(hard_fail as i64)
    .bind(crate::domain::timestamp())
    .execute(pool)
    .await?;
    Ok(())
}
