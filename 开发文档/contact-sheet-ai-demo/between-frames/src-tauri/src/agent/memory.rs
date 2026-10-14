// M4 长期审美记忆：L2 人工证据（append-only、按命令因果去重）与 L3 可纠正假设。
// Agent 草案工具不写本表；只有经唯一 CommandHandler 的人工命令产生证据。
// 撤销/重做/重复提交走同一命令回执，不新增支持次数（dedup_key=command_id）。
use crate::domain::{AppError, Result};
use serde_json::{Value, json};
use sqlx::{Row, Sqlite, Transaction};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceView {
    pub id: i64,
    pub project_id: String,
    pub photo_id: Option<String>,
    pub kind: String,
    pub detail: Value,
    pub created_at: String,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HypothesisView {
    pub id: i64,
    pub project_id: String,
    pub scope: String,
    pub statement: String,
    pub evidence_ids: Vec<i64>,
    pub counterexample_ids: Vec<i64>,
    pub status: String,
    pub confidence: String,
    pub user_correction: Option<String>,
    pub disabled: bool,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryProjection {
    pub evidence: Vec<EvidenceView>,
    pub hypotheses: Vec<HypothesisView>,
    pub notice: String,
}

/// 从人工 GUI 命令记录 L2 证据。Undo/Redo、版本、项目设置不是偏好行为，不记录。
pub(crate) async fn record_from_command(
    tx: &mut Transaction<'_, Sqlite>,
    request: &crate::command_handler::GuiCommandRequest,
    now: &str,
) -> Result<()> {
    use crate::commands_generated::GuiCommand;
    let rows: Vec<(Option<String>, &str, Value)> = match &request.command {
        GuiCommand::SetPhotoDecision { photo_ids, status } => {
            if status == "undecided" {
                vec![]
            } else {
                let kind = if status == "keep" { "keep" } else { "reject" };
                photo_ids
                    .iter()
                    .map(|photo| {
                        (
                            Some(photo.clone()),
                            kind,
                            json!({"status": status, "photo_ids": photo_ids}),
                        )
                    })
                    .collect()
            }
        }
        GuiCommand::RecordComparison {
            left_id,
            right_id,
            preference,
            reason,
        } => vec![(
            None,
            "comparison",
            json!({"left": left_id, "right": right_id, "preference": preference, "reason": reason}),
        )],
        GuiCommand::SetPhotoNote { photo_id, note } => {
            if note.trim().is_empty() {
                vec![]
            } else {
                vec![(Some(photo_id.clone()), "note", json!({"note": note}))]
            }
        }
        GuiCommand::ApplyDraft {
            draft_id,
            approved_photo_ids,
            ..
        } => vec![(
            None,
            "draft_apply",
            json!({"draft_id": draft_id, "approved_photo_ids": approved_photo_ids}),
        )],
        GuiCommand::DiscardDraft { draft_id, .. } => {
            vec![(None, "draft_reject", json!({"draft_id": draft_id}))]
        }
        _ => vec![],
    };
    for (photo_id, kind, detail) in rows {
        sqlx::query("INSERT OR IGNORE INTO preference_evidence(project_id,photo_id,kind,detail_json,dedup_key,created_at) VALUES(?,?,?,?,?,?)")
            .bind(&request.project_id)
            .bind(photo_id)
            .bind(kind)
            .bind(detail.to_string())
            .bind(&request.command_id)
            .bind(now)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

/// 惰性物化项目内假设：从 keep/note/draft_apply 证据的文本中提取重复出现的 2-gram
/// 关键词（≥2 条支持证据），reject 证据同关键词计入反例。纠正过的同表述假设
/// 不复活（AG-11）。生成是确定性规则，不调用模型。
pub(crate) async fn materialize(tx: &mut Transaction<'_, Sqlite>, project: &str) -> Result<()> {
    let now = crate::domain::timestamp();
    let evidence = load_evidence(tx, project).await?;
    let supporting: Vec<&EvidenceView> = evidence
        .iter()
        .filter(|e| ["keep", "note", "draft_apply", "comparison"].contains(&e.kind.as_str()))
        .collect();
    // 收集证据文本（keep/draft_apply 关联照片的当前备注 + note/comparison 理由）。
    let mut texts: Vec<(i64, String)> = vec![];
    for item in &supporting {
        let text = match item.kind.as_str() {
            "note" => item.detail["note"].as_str().unwrap_or("").to_owned(),
            "comparison" => item.detail["reason"].as_str().unwrap_or("").to_owned(),
            _ => String::new(),
        };
        if !text.trim().is_empty() {
            texts.push((item.id, text));
        } else if let Some(photo) = &item.photo_id {
            let note: Option<String> = sqlx::query_scalar(
                "SELECT user_note FROM photo_decisions WHERE project_id=? AND photo_id=?",
            )
            .bind(project)
            .bind(photo)
            .fetch_optional(&mut **tx)
            .await?;
            if let Some(note) = note.filter(|n| !n.trim().is_empty()) {
                texts.push((item.id, note));
            }
        }
    }
    // 2-gram 频次统计（仅中文/字母数字连续段），确定性。
    let mut counts: std::collections::BTreeMap<String, Vec<i64>> = Default::default();
    for (id, text) in &texts {
        let normalized: Vec<char> = text
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { ' ' })
            .collect();
        let joined: String = normalized.iter().collect();
        for token in joined.split_whitespace() {
            let chars: Vec<char> = token.chars().collect();
            if chars.len() < 2 {
                continue;
            }
            for window in chars.windows(2) {
                let gram: String = window.iter().collect();
                let bucket = counts.entry(gram).or_default();
                if !bucket.contains(id) {
                    bucket.push(*id);
                }
            }
        }
    }
    let existing: Vec<(i64, String, String)> =
        sqlx::query("SELECT id,statement,status FROM preference_hypotheses WHERE project_id=?")
            .bind(project)
            .fetch_all(&mut **tx)
            .await?
            .iter()
            .map(|r| (r.get("id"), r.get("statement"), r.get("status")))
            .collect();
    let mut candidates: Vec<(String, Vec<i64>)> = counts
        .into_iter()
        .filter(|(_, ids)| ids.len() >= 2)
        .collect();
    candidates.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));
    for (gram, ids) in candidates.into_iter().take(5) {
        let statement = format!(
            "本项目多次人工判断的说明中出现「{gram}」（{} 条证据）",
            ids.len()
        );
        // 纠正/停用过的同表述不复活；已存在则合并新证据。
        if existing
            .iter()
            .any(|(_, s, status)| *s == statement && status != "confirmed")
        {
            continue;
        }
        if let Some((id, _, _)) = existing.iter().find(|(_, s, _)| *s == statement) {
            let mut merged: Vec<i64> = sqlx::query_scalar(
                "SELECT value FROM json_each(SELECT evidence_ids_json FROM preference_hypotheses WHERE id=?)",
            )
            .bind(id)
            .fetch_all(&mut **tx)
            .await
            .unwrap_or_default();
            for evidence in &ids {
                if !merged.contains(evidence) {
                    merged.push(*evidence);
                }
            }
            sqlx::query(
                "UPDATE preference_hypotheses SET evidence_ids_json=?,updated_at=? WHERE id=?",
            )
            .bind(serde_json::to_string(&merged).unwrap())
            .bind(&now)
            .bind(id)
            .execute(&mut **tx)
            .await?;
            continue;
        }
        sqlx::query("INSERT INTO preference_hypotheses(project_id,scope,statement,evidence_ids_json,created_at,updated_at) VALUES(?,'project_local',?,?,?,?)")
            .bind(project)
            .bind(&statement)
            .bind(serde_json::to_string(&ids).unwrap())
            .bind(&now)
            .bind(&now)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

async fn load_evidence(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
) -> Result<Vec<EvidenceView>> {
    Ok(sqlx::query(
        "SELECT id,project_id,photo_id,kind,detail_json,created_at FROM preference_evidence WHERE project_id=? ORDER BY id DESC LIMIT 500",
    )
    .bind(project)
    .fetch_all(&mut **tx)
    .await?
    .iter()
    .map(|r| EvidenceView {
        id: r.get("id"),
        project_id: r.get("project_id"),
        photo_id: r.get("photo_id"),
        kind: r.get("kind"),
        detail: serde_json::from_str(&r.get::<String, _>("detail_json")).unwrap_or(Value::Null),
        created_at: r.get("created_at"),
    })
    .collect())
}
async fn load_hypotheses(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
) -> Result<Vec<HypothesisView>> {
    Ok(sqlx::query(
        "SELECT * FROM preference_hypotheses WHERE project_id=? ORDER BY id DESC LIMIT 100",
    )
    .bind(project)
    .fetch_all(&mut **tx)
    .await?
    .iter()
    .map(|r| HypothesisView {
        id: r.get("id"),
        project_id: r.get("project_id"),
        scope: r.get("scope"),
        statement: r.get("statement"),
        evidence_ids: parse_ids(r.get("evidence_ids_json")),
        counterexample_ids: parse_ids(r.get("counterexample_ids_json")),
        status: r.get("status"),
        confidence: r.get("confidence"),
        user_correction: r.get("user_correction"),
        disabled: r.get::<i64, _>("disabled_for_retrieval") != 0,
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    })
    .collect())
}
fn parse_ids(raw: String) -> Vec<i64> {
    serde_json::from_str(&raw).unwrap_or_default()
}

pub async fn projection(db: &crate::db::Store, project: &str) -> Result<MemoryProjection> {
    let mut tx = db.pool.begin().await?;
    let exists: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM projects WHERE id=? AND deleted_at IS NULL")
            .bind(project)
            .fetch_one(&mut *tx)
            .await?;
    if exists == 0 {
        return Err(AppError::new("NOT_FOUND", "项目不存在"));
    }
    materialize(&mut tx, project).await?;
    let result = MemoryProjection {
        evidence: load_evidence(&mut tx, project).await?,
        hypotheses: load_hypotheses(&mut tx, project).await?,
        notice: "证据仅来自人工命令；Agent 草案不构成偏好。假设为规则推断，可纠正或停用。".into(),
    };
    tx.commit().await?;
    Ok(result)
}

/// Agent 检索视图：仅项目内、未停用、非纠正/退役；排序 确认>未确认、证据数、时间。
pub async fn retrievable(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    limit: usize,
) -> Result<Value> {
    materialize(tx, project).await?;
    let all = load_hypotheses(tx, project).await?;
    let mut usable: Vec<HypothesisView> = all
        .into_iter()
        .filter(|h| !h.disabled && !["corrected", "retired"].contains(&h.status.as_str()))
        .collect();
    usable.sort_by(|a, b| {
        let rank = |h: &HypothesisView| if h.status == "confirmed" { 0 } else { 1 };
        rank(a)
            .cmp(&rank(b))
            .then(b.evidence_ids.len().cmp(&a.evidence_ids.len()))
            .then(b.id.cmp(&a.id))
    });
    usable.truncate(limit);
    let summaries: Vec<Value> = usable
        .iter()
        .map(|h| {
            json!({
                "statement": h.statement,
                "scope": h.scope,
                "status": h.status,
                "confidence": h.confidence,
                "evidence_ids": h.evidence_ids,
                "counterexample_ids": h.counterexample_ids,
                "user_correction": h.user_correction,
            })
        })
        .collect();
    Ok(json!({
        "hypotheses": summaries,
        "mode": "evidence_backed",
        "notice": "假设附带人工证据 ID，可被用户纠正；纠正后不再进入检索",
    }))
}

pub struct HypothesisCorrection {
    pub hypothesis_id: i64,
    pub project_id: String,
    pub correction: String,
}
pub async fn correct(db: &crate::db::Store, input: HypothesisCorrection) -> Result<HypothesisView> {
    if input.correction.trim().is_empty() || input.correction.chars().count() > 2000 {
        return Err(AppError::new("VALIDATION_ERROR", "纠正文本不合法"));
    }
    // 纠正即停止后续检索引用（AG-11：纠正后的假设不再进入 Agent 上下文）。
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let changed = sqlx::query(
        "UPDATE preference_hypotheses SET status='corrected', user_correction=?, disabled_for_retrieval=1, updated_at=? WHERE id=? AND project_id=?",
    )
    .bind(input.correction.trim())
    .bind(crate::domain::timestamp())
    .bind(input.hypothesis_id)
    .bind(&input.project_id)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if changed == 0 {
        return Err(AppError::new("NOT_FOUND", "假设不属于该项目"));
    }
    let view = row_view(&mut tx, input.hypothesis_id).await?;
    tx.commit().await?;
    Ok(view)
}
pub async fn set_enabled(
    db: &crate::db::Store,
    project: &str,
    hypothesis_id: i64,
    enabled: bool,
) -> Result<HypothesisView> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let changed = sqlx::query(
        "UPDATE preference_hypotheses SET disabled_for_retrieval=?, updated_at=? WHERE id=? AND project_id=?",
    )
    .bind(!enabled as i64)
    .bind(crate::domain::timestamp())
    .bind(hypothesis_id)
    .bind(project)
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if changed == 0 {
        return Err(AppError::new("NOT_FOUND", "假设不属于该项目"));
    }
    let view = row_view(&mut tx, hypothesis_id).await?;
    tx.commit().await?;
    Ok(view)
}
async fn row_view(tx: &mut Transaction<'_, Sqlite>, id: i64) -> Result<HypothesisView> {
    let r = sqlx::query("SELECT * FROM preference_hypotheses WHERE id=?")
        .bind(id)
        .fetch_one(&mut **tx)
        .await?;
    Ok(HypothesisView {
        id: r.get("id"),
        project_id: r.get("project_id"),
        scope: r.get("scope"),
        statement: r.get("statement"),
        evidence_ids: parse_ids(r.get("evidence_ids_json")),
        counterexample_ids: parse_ids(r.get("counterexample_ids_json")),
        status: r.get("status"),
        confidence: r.get("confidence"),
        user_correction: r.get("user_correction"),
        disabled: r.get::<i64, _>("disabled_for_retrieval") != 0,
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    })
}
