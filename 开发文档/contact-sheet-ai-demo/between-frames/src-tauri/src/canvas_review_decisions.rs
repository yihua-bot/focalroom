//! 审稿决定复用人工命令的不可变日志；不把浏览器选中状态当作持久决定。
use super::*;
use crate::canvas_commands::{CanvasAction, CanvasCommandRequest};
use serde::Deserialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecordEditorialDecision {
    pub draft_id: String,
    pub draft_revision: i64,
    pub change_ids: Vec<String>,
    pub decision: ReviewDecision,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub fields_by_change: std::collections::BTreeMap<String, Vec<String>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewDecision {
    Pending,
    Rejected,
    Deferred,
}

pub(crate) async fn validate_decision(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    board: &str,
    cmd: &RecordEditorialDecision,
) -> Result<()> {
    let comparison = comparison_tx(tx, project, &cmd.draft_id).await?;
    if comparison.draft.revision != cmd.draft_revision
        || comparison.source_board_id.as_deref() != Some(board)
        || cmd.change_ids.is_empty()
        || cmd.change_ids.len() > 300
        || cmd
            .change_ids
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len()
            != cmd.change_ids.len()
        || cmd
            .change_ids
            .iter()
            .any(|id| !comparison.changes.iter().any(|c| &c.id == id))
    {
        return Err(AppError::new(
            "CONFLICT",
            "审稿内容已变化，请重新查看后决定",
        ));
    }
    if cmd.change_ids.iter().any(|id| {
        comparison
            .changes
            .iter()
            .any(|c| &c.id == id && c.decision == "accepted")
    }) {
        return Err(AppError::new(
            "CONFLICT",
            "已采纳的改动请通过历史撤销，不可改写审稿决定",
        ));
    }
    for id in &cmd.change_ids {
        let change = comparison.changes.iter().find(|c| &c.id == id).unwrap();
        if let Some(fields) = cmd.fields_by_change.get(id) {
            if fields.is_empty()
                || fields.len() > 8
                || fields
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    != fields.len()
                || fields.iter().any(|field| {
                    !change.fields.contains(field) || change.accepted_fields.contains(field)
                })
            {
                return Err(AppError::new(
                    "CONFLICT",
                    "所选属性已变化，请重新查看后决定",
                ));
            }
        } else if change.decision == "partial" {
            return Err(AppError::new("CONFLICT", "请选择尚未决定的属性"));
        }
    }
    if cmd
        .fields_by_change
        .keys()
        .any(|id| !cmd.change_ids.contains(id))
    {
        return Err(AppError::new("CONFLICT", "所选属性不属于这些改动"));
    }
    Ok(())
}

#[derive(Default)]
pub(crate) struct DecisionProjection {
    pub states: HashMap<String, String>,
    pub accepted_fields: HashMap<String, std::collections::BTreeSet<String>>,
    pub field_decisions: HashMap<String, std::collections::BTreeMap<String, String>>,
}
pub(crate) async fn decisions_tx(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    source: &str,
    draft: &str,
    revision: i64,
) -> Result<DecisionProjection> {
    let rows: Vec<(String, Option<String>, Option<bool>)> = sqlx::query_as(
        "SELECT e.request_json,l.op_data,(l.undone=1 OR EXISTS(SELECT 1 FROM canvas_op_log r WHERE r.board_id=l.board_id AND r.op_type='editorial_merge' AND json_extract(r.op_data,'$.revertsCommandId')=e.command_id AND r.undone=0)) FROM canvas_command_events e LEFT JOIN canvas_op_log l ON l.command_group=e.command_id AND l.op_type='editorial_merge' WHERE e.project_id=? AND e.board_id=? AND (json_extract(e.request_json,'$.operations[0].action.data.selection.draftId')=? OR json_extract(e.request_json,'$.operations[0].action.data.draftId')=?) ORDER BY e.rowid LIMIT 10001"
    ).bind(project).bind(source).bind(draft).bind(draft).fetch_all(&mut **tx).await?;
    if rows.len() > 10000 {
        return Err(AppError::new(
            "VALIDATION_ERROR",
            "审稿记录过多，请创建新的修订方案",
        ));
    }
    let mut result = DecisionProjection::default();
    for (request, history, undone) in rows {
        let request: CanvasCommandRequest =
            serde_json::from_str(&request).map_err(serialization)?;
        match &request.operations[0].action {
            CanvasAction::RecordEditorialDecision(cmd) if cmd.draft_revision == revision => {
                let state = match cmd.decision {
                    ReviewDecision::Pending => "pending",
                    ReviewDecision::Rejected => "rejected",
                    ReviewDecision::Deferred => "deferred",
                };
                for id in &cmd.change_ids {
                    if let Some(fields) = cmd.fields_by_change.get(id) {
                        for field in fields {
                            result
                                .field_decisions
                                .entry(id.clone())
                                .or_default()
                                .insert(field.clone(), state.into());
                        }
                    } else {
                        result.states.insert(id.clone(), state.into());
                    }
                }
            }
            CanvasAction::ApplyEditorialChanges(cmd)
                if cmd.selection.draft_revision == revision =>
            {
                let mut ids = cmd.selection.selected_change_ids.clone();
                if let Some(history) = history {
                    let history: Value = serde_json::from_str(&history).map_err(serialization)?;
                    if let Some(effective) = history["changes"].as_array() {
                        ids.extend(
                            effective
                                .iter()
                                .filter_map(|v| v.as_str().map(String::from)),
                        );
                    }
                }
                if undone != Some(true) {
                    for id in ids {
                        if let Some(fields) = cmd.selection.fields_by_change.get(&id) {
                            result
                                .accepted_fields
                                .entry(id)
                                .or_default()
                                .extend(fields.iter().cloned());
                        } else {
                            result.states.insert(id, "accepted".into());
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Ok(result)
}
