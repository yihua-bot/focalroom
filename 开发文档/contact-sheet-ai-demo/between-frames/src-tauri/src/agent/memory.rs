// M4 长期审美记忆：L2 人工证据（append-only、按命令因果去重）与 L3 可纠正假设。
// Agent 草案工具不写本表；只有经唯一 CommandHandler 的人工命令产生证据。
// 撤销/重做/重复提交走同一命令回执，不新增支持次数（dedup_key=command_id）。
use crate::domain::{AppError, Result};
use serde_json::{Value, json};
use sqlx::{Row, Sqlite, Transaction};
use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceView {
    pub id: i64,
    pub project_id: String,
    pub photo_id: Option<String>,
    pub kind: String,
    pub detail: Value,
    pub created_at: String,
    pub excluded: bool,
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
    pub cross_project_enabled: bool,
    pub author_hypotheses: Vec<HypothesisView>,
    pub author_memory_notice: String,
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
                let mut notes = vec![];
                for photo in photo_ids {
                    let note: String = sqlx::query_scalar(
                        "SELECT user_note FROM photo_decisions WHERE project_id=? AND photo_id=?",
                    )
                    .bind(&request.project_id)
                    .bind(photo)
                    .fetch_one(&mut **tx)
                    .await?;
                    if !note.trim().is_empty() {
                        notes.push(json!({"photo_id":photo,"note":note}));
                    }
                }
                vec![(
                    if photo_ids.len() == 1 {
                        Some(photo_ids[0].clone())
                    } else {
                        None
                    },
                    kind,
                    json!({"status":status,"photo_ids":photo_ids,"notes":notes}),
                )]
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
                let status: String = sqlx::query_scalar(
                    "SELECT status FROM photo_decisions WHERE project_id=? AND photo_id=?",
                )
                .bind(&request.project_id)
                .bind(photo_id)
                .fetch_one(&mut **tx)
                .await?;
                vec![(
                    Some(photo_id.clone()),
                    "note",
                    json!({"note": note,"status":status}),
                )]
            }
        }
        GuiCommand::ApplyProposal {
            proposal_id,
            accepted_operation_ids,
            ..
        } if !accepted_operation_ids.is_empty() => vec![(
            None,
            "draft_apply",
            json!({"proposal_id":proposal_id,"accepted_operation_ids":accepted_operation_ids}),
        )],
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

/// 仅使用当时保存的人工说明，排除状态立即传播到支持、反例及检索。
pub(crate) async fn materialize(tx: &mut Transaction<'_, Sqlite>, project: &str) -> Result<()> {
    let evidence = load_evidence(tx, project).await?;
    let mut supports: std::collections::BTreeMap<String, Vec<i64>> = Default::default();
    let mut counters: std::collections::BTreeMap<String, Vec<i64>> = Default::default();
    for item in evidence
        .iter()
        .filter(|item| !item.excluded && item.detail["explicitPreference"] != true)
    {
        let text = if item.kind == "note" {
            item.detail["note"].as_str().unwrap_or("").to_owned()
        } else if item.kind == "comparison" {
            item.detail["reason"].as_str().unwrap_or("").to_owned()
        } else {
            item.detail["notes"]
                .as_array()
                .map(|notes| {
                    notes
                        .iter()
                        .filter_map(|note| note["note"].as_str())
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .unwrap_or_default()
        };
        let negative = item.kind == "reject" || item.detail["status"] == "reject";
        if item.kind == "comparison" && item.detail["preference"] == "uncertain" {
            continue;
        }
        for word in super::memory_search::terms(&text)
            .into_iter()
            .filter(|word| word.chars().count() == 2)
        {
            let ids = if negative {
                counters.entry(word).or_default()
            } else {
                supports.entry(word).or_default()
            };
            ids.push(item.id);
        }
    }
    let existing = load_hypotheses(tx, project).await?;
    let mut candidates: Vec<_> = supports.iter().filter(|(_, ids)| ids.len() >= 2).collect();
    candidates.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));
    // 物理删除的假设写有墓碑（按关键词）：同词候选跳过，删除不因物化复活
    // （ADR 0014 修订：表述含证据数会漂移，按词匹配才可靠）。
    let tombstoned: Vec<String> = sqlx::query_scalar(
        "SELECT ref_key FROM memory_tombstones WHERE project_id=? AND kind='hypothesis'",
    )
    .bind(project)
    .fetch_all(&mut **tx)
    .await
    .unwrap_or_default();
    for (word, _) in candidates.into_iter().take(5) {
        let statement = format!("本项目多次人工判断的说明中出现「{word}」");
        if tombstoned
            .iter()
            .any(|key| super::memory_search::keyword(key) == word)
        {
            continue;
        }
        if existing
            .iter()
            .any(|h| statement_key(&h.statement) == statement)
        {
            continue;
        }
        let now = crate::domain::timestamp();
        sqlx::query("INSERT INTO preference_hypotheses(project_id,scope,statement,created_at,updated_at) VALUES(?,'project_local',?,?,?)")
            .bind(project).bind(statement).bind(&now).bind(&now).execute(&mut **tx).await?;
    }
    for h in load_hypotheses(tx, project).await? {
        let explicit: Vec<_> = evidence
            .iter()
            .filter(|e| {
                e.detail["explicitPreference"] == true
                    && e.detail["hypothesisId"].as_i64() == Some(h.id)
            })
            .collect();
        if !explicit.is_empty() {
            let ids: Vec<_> = explicit
                .iter()
                .filter(|e| !e.excluded)
                .map(|e| e.id)
                .collect();
            if ids != h.evidence_ids {
                sqlx::query("UPDATE preference_hypotheses SET evidence_ids_json=?,updated_at=? WHERE id=? AND project_id=?").bind(serde_json::to_string(&ids).unwrap()).bind(crate::domain::timestamp()).bind(h.id).bind(project).execute(&mut **tx).await?;
            }
            super::memory_search::sync(tx, project, h.id, &h.statement).await?;
            continue;
        }
        let word = super::memory_search::keyword(&h.statement);
        let ids = supports.get(word).cloned().unwrap_or_default();
        let against = counters.get(word).cloned().unwrap_or_default();
        if ids != h.evidence_ids || against != h.counterexample_ids {
            sqlx::query("UPDATE preference_hypotheses SET evidence_ids_json=?,counterexample_ids_json=?,updated_at=? WHERE id=? AND project_id=?")
                .bind(serde_json::to_string(&ids).unwrap()).bind(serde_json::to_string(&against).unwrap()).bind(crate::domain::timestamp()).bind(h.id).bind(project).execute(&mut **tx).await?;
        }
        super::memory_search::sync(tx, project, h.id, &h.statement).await?;
    }
    Ok(())
}

async fn load_evidence(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
) -> Result<Vec<EvidenceView>> {
    Ok(sqlx::query(
        "SELECT e.*,COALESCE(s.excluded,0) AS excluded FROM preference_evidence e LEFT JOIN memory_evidence_state s ON s.evidence_id=e.id WHERE e.project_id=? ORDER BY e.id DESC",
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
        excluded: r.get::<i64,_>("excluded") != 0,
    })
    .collect())
}
async fn load_hypotheses(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
) -> Result<Vec<HypothesisView>> {
    Ok(
        sqlx::query("SELECT * FROM preference_hypotheses WHERE project_id=? AND scope='project_local' ORDER BY id DESC")
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
            .collect(),
    )
}
fn parse_ids(raw: String) -> Vec<i64> {
    serde_json::from_str(&raw).unwrap_or_default()
}

/// 兼容旧版将证据数拼入 statement 的记录；支持次数不构成假设身份。
fn statement_key(statement: &str) -> String {
    statement
        .split_once("」（")
        .map(|(prefix, _)| format!("{prefix}」"))
        .unwrap_or_else(|| statement.to_owned())
}

pub async fn projection(db: &crate::db::Store, project: &str) -> Result<MemoryProjection> {
    projection_for_query(db, project, "").await
}
pub async fn projection_for_query(
    db: &crate::db::Store,
    project: &str,
    query: &str,
) -> Result<MemoryProjection> {
    if query.chars().count() > 400 {
        return Err(AppError::new("VALIDATION_ERROR", "检索词过长"));
    }
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
    let matches = super::memory_search::ranks(&mut tx, project, query).await?;
    let mut hypotheses = load_hypotheses(&mut tx, project).await?;
    let cross = super::memory_scope::settings(&mut tx)
        .await?
        .cross_project_enabled;
    let mut author_notice = "明确开启后，离线 Agent 可引用其他本地项目已确认的相关假设；共享/迁移中项目与云模型不参与。当前桌面按单一本地作者处理。".to_owned();
    let author_hypotheses = if cross {
        match author_hypotheses(&mut tx, project, query, 10).await {
            Ok(items) => items,
            Err(error) if error.code == "MEMORY_SCOPE_LIMIT" => {
                author_notice = error.message;
                vec![]
            }
            Err(error) => return Err(error),
        }
    } else {
        vec![]
    };
    if !query.trim().is_empty() {
        hypotheses.retain(|h| matches.contains_key(&h.id));
    }
    let result = MemoryProjection {
        evidence: load_evidence(&mut tx, project).await?,
        hypotheses,
        cross_project_enabled: cross,
        author_hypotheses,
        author_memory_notice: author_notice,
        notice: "假设为项目内关键词规则推断，可确认、纠正或移除。排除证据只停止参与学习；原始人工记录、任务历史与备份仍保留。".into(),
    };
    tx.commit().await?;
    Ok(result)
}

/// Agent 检索视图：仅项目内、未停用、非纠正/退役；排序 相关性、确认、证据数、时间。
pub async fn retrievable(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    limit: usize,
) -> Result<Value> {
    retrievable_for_goal(tx, project, limit, "").await
}
pub async fn retrievable_for_goal(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    limit: usize,
    goal: &str,
) -> Result<Value> {
    materialize(tx, project).await?;
    let all = load_hypotheses(tx, project).await?;
    let corrections: Vec<String> = sqlx::query_scalar("SELECT statement FROM preference_hypotheses WHERE project_id=? AND scope='project_local' AND status IN('corrected','retired')")
        .bind(project).fetch_all(&mut **tx).await?;
    let corrected: std::collections::HashSet<_> =
        corrections.iter().map(|s| statement_key(s)).collect();
    let keyword_ranks = super::memory_search::ranks(tx, project, goal).await?;
    let mut usable: Vec<HypothesisView> = all
        .into_iter()
        .filter(|h| {
            !h.disabled
                && !corrected.contains(&statement_key(&h.statement))
                && h.evidence_ids.len() >= if h.status == "confirmed" { 1 } else { 2 }
        })
        .collect();
    // 混合检索：关键词 BM25 与本地句向量余弦等权融合；语义不可用（非 macOS/超时/无模型）自动退化为关键词。
    let semantic_ranks = if goal.trim().is_empty() {
        Default::default()
    } else {
        let statements: Vec<String> = usable.iter().map(|h| h.statement.clone()).collect();
        let keys: Vec<String> = usable.iter().map(|h| h.id.to_string()).collect();
        let ranking =
            super::semantic::rank_cached(goal.to_string(), statements, 300, None, keys).await;
        ranking
            .scores
            .iter()
            .enumerate()
            .filter_map(|(index, score)| {
                let id = usable.get(index).map(|h| h.id)?;
                score.map(|value| (id, Some(value)))
            })
            .collect::<HashMap<i64, Option<f64>>>()
    };
    let relevance = super::memory_search::hybrid_scores(&keyword_ranks, &semantic_ranks);
    usable.sort_by(|a, b| {
        let rank = |h: &HypothesisView| if h.status == "confirmed" { 0 } else { 1 };
        relevance
            .get(&a.id)
            .map_or(1, |_| 0)
            .cmp(&relevance.get(&b.id).map_or(1, |_| 0))
            .then_with(|| {
                // 混合分越高越相关（旧 BM25 负值升序语义在 hybrid_scores 内已归一为正序分）。
                relevance
                    .get(&b.id)
                    .unwrap_or(&0.0)
                    .total_cmp(relevance.get(&a.id).unwrap_or(&0.0))
            })
            .then(
                rank(a)
                    .cmp(&rank(b))
                    .then(b.evidence_ids.len().cmp(&a.evidence_ids.len()))
                    .then(b.id.cmp(&a.id)),
            )
    });
    let mut seen = std::collections::HashSet::new();
    usable.retain(|h| seen.insert(statement_key(&h.statement)));
    usable.truncate(limit);
    let summaries: Vec<Value> = usable.iter().map(summary).collect();
    Ok(json!({
        "hypotheses": summaries,
        "mode": "evidence_backed",
        "notice": "假设附带人工证据 ID，可被用户纠正；纠正后不再进入检索",
    }))
}

fn summary(h: &HypothesisView) -> Value {
    json!({
        "memory_id":h.id, "source_project_id":h.project_id,
        "statement":h.statement, "scope":h.scope, "status":h.status,
        "confidence":h.confidence, "evidence_ids":h.evidence_ids,
        "counterexample_ids":h.counterexample_ids, "user_correction":h.user_correction,
        "evidence_refs":h.evidence_ids.iter().map(|id| json!({"project_id":h.project_id,"evidence_id":id})).collect::<Vec<_>>(),
        "counterexample_refs":h.counterexample_ids.iter().map(|id| json!({"project_id":h.project_id,"evidence_id":id})).collect::<Vec<_>>(),
    })
}
/// 只返回已确认、有有效证据且与目标相关的作者线索；不暴露来源照片或备注正文。
async fn author_hypotheses(
    tx: &mut Transaction<'_, Sqlite>,
    target: &str,
    goal: &str,
    limit: usize,
) -> Result<Vec<HypothesisView>> {
    if !super::memory_scope::settings(tx)
        .await?
        .cross_project_enabled
    {
        return Ok(vec![]);
    }
    let sources = super::memory_scope::sources(tx, target).await?;
    let mut blocked = std::collections::HashSet::new();
    let mut candidates = vec![];
    for project in std::iter::once(target).chain(sources.iter().map(String::as_str)) {
        materialize(tx, project).await?;
        let relevance = super::memory_search::ranks(tx, project, goal).await?;
        for h in load_hypotheses(tx, project).await? {
            let key = super::memory_search::keyword(&h.statement).to_owned();
            if h.disabled || ["corrected", "retired"].contains(&h.status.as_str()) {
                blocked.insert(key);
            } else if project != target
                && h.status == "confirmed"
                && !h.evidence_ids.is_empty()
                && (goal.trim().is_empty() || relevance.contains_key(&h.id))
            {
                let rank = *relevance.get(&h.id).unwrap_or(&0.0);
                candidates.push((h, rank));
            }
        }
        let tombstones: Vec<String> = sqlx::query_scalar(
            "SELECT ref_key FROM memory_tombstones WHERE project_id=? AND kind='hypothesis'",
        )
        .bind(project)
        .fetch_all(&mut **tx)
        .await?;
        blocked.extend(
            tombstones
                .iter()
                .map(|s| super::memory_search::keyword(s).to_owned()),
        );
    }
    candidates.sort_by(|(a, ra), (b, rb)| {
        ra.total_cmp(rb)
            .then(b.evidence_ids.len().cmp(&a.evidence_ids.len()))
            .then(b.id.cmp(&a.id))
    });
    let mut seen = std::collections::HashSet::new();
    let result: Vec<_> = candidates
        .into_iter()
        .map(|(mut h, _)| {
            h.scope = "cross_project".into();
            h
        })
        .filter(|h| {
            let key = super::memory_search::keyword(&h.statement).to_owned();
            !blocked.contains(&key) && seen.insert(key)
        })
        .take(limit.min(10))
        .collect();
    // 来源内陈述保持原样，来源项目 ID 决定其解释范围。
    Ok(result)
}
pub async fn retrievable_author(
    tx: &mut Transaction<'_, Sqlite>,
    task: &super::types::Task,
    limit: usize,
) -> Result<Value> {
    if !super::memory_scope::may_use_author(tx, task).await? {
        return Err(AppError::new(
            "PERMISSION_DENIED",
            "跨项目记忆仅供明确授权的本地离线任务使用",
        ));
    }
    let items = author_hypotheses(tx, &task.project_id, &task.goal, limit).await?;
    let mut result = json!({"hypotheses":items.iter().map(summary).collect::<Vec<_>>(),"mode":"confirmed_local_author","notice":"其他本地项目的已确认假设，不代表永久偏好；引用必须带来源项目和证据 ID，不能作为照片操作权限"});
    super::preference_proposal::filter_for_task(tx, task, &mut result).await?;
    Ok(result)
}
pub async fn retrievable_for_task(
    tx: &mut Transaction<'_, Sqlite>,
    task: &super::types::Task,
    limit: usize,
) -> Result<Value> {
    let limit = limit.min(10);
    let mut memory = retrievable_for_goal(tx, &task.project_id, limit, &task.goal).await?;
    if super::memory_scope::may_use_author(tx, task).await? {
        let author = retrievable_author(tx, task, limit).await?;
        let local = memory["hypotheses"].as_array_mut().unwrap();
        let mut seen: std::collections::HashSet<String> = local
            .iter()
            .filter_map(|h| h["statement"].as_str())
            .map(|s| super::memory_search::keyword(s).to_owned())
            .collect();
        for h in author["hypotheses"].as_array().unwrap() {
            if local.len() == limit {
                break;
            }
            if seen
                .insert(super::memory_search::keyword(h["statement"].as_str().unwrap()).to_owned())
            {
                local.push(h.clone());
            }
        }
    }
    super::preference_proposal::filter_for_task(tx, task, &mut memory).await?;
    Ok(memory)
}

pub struct HypothesisCorrection {
    pub hypothesis_id: i64,
    pub project_id: String,
    pub correction: String,
}
pub async fn correct(db: &crate::db::Store, input: HypothesisCorrection) -> Result<HypothesisView> {
    crate::command_handler::CommandHandler::new(db)
        .execute_memory(super::memory_control::MemoryRequest {
            request_id: crate::domain::id(),
            project_id: input.project_id,
            command: super::memory_control::MemoryCommand::CorrectHypothesis {
                hypothesis_id: input.hypothesis_id,
                correction: input.correction,
            },
        })
        .await?;
    let mut tx = db.pool.begin().await?;
    row_view(&mut tx, input.hypothesis_id).await
}
pub async fn set_enabled(
    db: &crate::db::Store,
    project: &str,
    hypothesis_id: i64,
    enabled: bool,
) -> Result<HypothesisView> {
    crate::command_handler::CommandHandler::new(db)
        .execute_memory(super::memory_control::MemoryRequest {
            request_id: crate::domain::id(),
            project_id: project.into(),
            command: super::memory_control::MemoryCommand::SetHypothesisEnabled {
                hypothesis_id,
                enabled,
            },
        })
        .await?;
    let mut tx = db.pool.begin().await?;
    row_view(&mut tx, hypothesis_id).await
}
pub(crate) async fn row_view(tx: &mut Transaction<'_, Sqlite>, id: i64) -> Result<HypothesisView> {
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
