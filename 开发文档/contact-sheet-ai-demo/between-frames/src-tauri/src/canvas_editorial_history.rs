//! 采纳历史和补偿撤回：追加新命令，不改写既有审计或整板状态。
use super::*;
use crate::command_handler::CommandHandler;

#[derive(Debug, Clone, serde::Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RevertEditorialChanges {
    pub merge_command_id: String,
    pub simulation_id: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorialHistoryEntry {
    pub cursor: i64,
    pub command_id: String,
    pub draft_id: String,
    pub change_count: usize,
    pub created_at: String,
    pub undone: bool,
    pub compensated: bool,
    pub is_compensation: bool,
    pub node_types: Vec<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevertSimulation {
    pub merge_command_id: String,
    pub target_revision: i64,
    pub simulation_id: String,
    pub current_nodes: Vec<CanvasNode>,
    pub merged_nodes: Vec<CanvasNode>,
    pub conflicts: Vec<String>,
    pub can_apply: bool,
    pub photo_sources: Vec<CanvasPhotoSource>,
}
impl Store {
    pub async fn editorial_history(
        &self,
        project: &str,
        board: &str,
    ) -> Result<Vec<EditorialHistoryEntry>> {
        self.editorial_history_page(project, board, None).await
    }
    pub async fn editorial_history_page(
        &self,
        project: &str,
        board: &str,
        before: Option<i64>,
    ) -> Result<Vec<EditorialHistoryEntry>> {
        let mut tx = self.pool.begin().await?;
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM boards b JOIN projects p ON p.id=b.project_id WHERE b.id=? AND b.project_id=? AND p.deleted_at IS NULL)").bind(board).bind(project).fetch_one(&mut *tx).await?;
        if !exists {
            return Err(AppError::new("NOT_FOUND", "画布不存在"));
        }
        type HistoryRow = (i64, String, String, String, bool, bool);
        let rows: Vec<HistoryRow> = sqlx::query_as("SELECT l.rowid,l.command_group,l.op_data,l.created_at,l.undone,EXISTS(SELECT 1 FROM canvas_op_log r WHERE r.board_id=l.board_id AND r.op_type='editorial_merge' AND json_extract(r.op_data,'$.revertsCommandId')=l.command_group AND r.undone=0) FROM canvas_op_log l WHERE l.project_id=? AND l.board_id=? AND l.op_type='editorial_merge' AND l.command_group IS NOT NULL AND (? IS NULL OR l.rowid<?) ORDER BY l.rowid DESC LIMIT 100")
            .bind(project).bind(board).bind(before).bind(before).fetch_all(&mut *tx).await?;
        let mut entries = Vec::new();
        for (cursor, command_id, data, created_at, undone, compensated) in rows {
            let data: Value = serde_json::from_str(&data).map_err(serialization)?;
            entries.push(EditorialHistoryEntry {
                cursor,
                command_id,
                draft_id: data["draftId"].as_str().unwrap_or("").into(),
                change_count: data["patches"].as_array().map_or(0, Vec::len),
                created_at,
                undone,
                compensated,
                is_compensation: data["revertsCommandId"].is_string(),
                node_types: data["patches"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|patch| {
                        patch["after"]["nodeType"]
                            .as_str()
                            .or_else(|| patch["before"]["nodeType"].as_str())
                            .map(String::from)
                    })
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect(),
            });
        }
        tx.commit().await?;
        Ok(entries)
    }
    pub async fn simulate_editorial_revert(
        &self,
        project: &str,
        board: &str,
        command: &str,
    ) -> Result<RevertSimulation> {
        let mut tx = self.pool.begin().await?;
        let result = simulate_revert_tx(&mut tx, project, board, command).await?;
        tx.commit().await?;
        Ok(result)
    }
}
async fn history_data(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    board: &str,
    command: &str,
) -> Result<Value> {
    let row: Option<(String,bool,bool)> = sqlx::query_as("SELECT l.op_data,l.undone,EXISTS(SELECT 1 FROM canvas_op_log r WHERE r.board_id=l.board_id AND r.op_type='editorial_merge' AND json_extract(r.op_data,'$.revertsCommandId')=? AND r.undone=0) FROM canvas_op_log l JOIN canvas_command_events e ON e.command_id=l.command_group WHERE l.command_group=? AND l.project_id=? AND l.board_id=? AND l.op_type='editorial_merge' AND json_extract(e.request_json,'$.operations[0].action.type')='ApplyEditorialChanges'")
        .bind(command).bind(command).bind(project).bind(board).fetch_optional(&mut **tx).await?;
    let Some((raw, undone, compensated)) = row else {
        return Err(AppError::new("NOT_FOUND", "找不到这次采纳记录"));
    };
    if undone || compensated {
        return Err(AppError::new("CONFLICT", "这次采纳已撤回，无需再次撤回"));
    }
    serde_json::from_str(&raw).map_err(serialization)
}
async fn simulate_revert_tx(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    board: &str,
    command: &str,
) -> Result<RevertSimulation> {
    let target: Board = sqlx::query_as("SELECT * FROM boards WHERE id=? AND project_id=?")
        .bind(board)
        .bind(project)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(|| AppError::new("NOT_FOUND", "画布不存在"))?;
    let data = history_data(tx, project, board, command).await?;
    let current_nodes: Vec<CanvasNode> = sqlx::query_as(
        "SELECT * FROM canvas_nodes WHERE board_id=? AND visible=1 ORDER BY z_order,id LIMIT 5001",
    )
    .bind(board)
    .fetch_all(&mut **tx)
    .await?;
    let mut conflicts = Vec::new();
    if target.owning_task_id.is_some() || target.state == "frozen" {
        conflicts.push("这张画布暂时无法修改".into());
    }
    let merged_nodes = match merge::inverse_projection(tx, board, &data, false).await {
        Ok(nodes) => nodes.into_values().collect(),
        Err(error) if matches!(error.code.as_str(), "CONFLICT" | "VALIDATION_ERROR") => {
            conflicts.push(error.message);
            current_nodes.clone()
        }
        Err(error) => return Err(error),
    };
    let assets: std::collections::BTreeSet<_> = current_nodes
        .iter()
        .chain(merged_nodes.iter())
        .filter_map(|n| n.asset_id.as_ref())
        .collect();
    let photo_sources = sqlx::query_as("SELECT a.id AS asset_id,MIN(p.thumbnail_path) AS thumbnail_path FROM source_assets a JOIN asset_aliases alias ON alias.asset_id=a.id JOIN photos p ON p.id=alias.photo_id AND p.project_id=a.project_id WHERE a.project_id=? AND a.id IN(SELECT value FROM json_each(?)) GROUP BY a.id")
        .bind(project).bind(encode(&assets)?).fetch_all(&mut **tx).await?;
    let simulation_id = fingerprint(
        &json!({"project":project,"board":board,"command":command,"revision":target.revision,"data":data,"current":current_nodes,"merged":merged_nodes,"conflicts":conflicts}),
    );
    Ok(RevertSimulation {
        merge_command_id: command.into(),
        target_revision: target.revision,
        simulation_id,
        current_nodes,
        merged_nodes,
        can_apply: conflicts.is_empty(),
        conflicts,
        photo_sources,
    })
}
impl CommandHandler<'_> {
    pub(crate) async fn revert_editorial_tx(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        project: &str,
        board: &str,
        cmd: &RevertEditorialChanges,
    ) -> Result<()> {
        let simulation = simulate_revert_tx(tx, project, board, &cmd.merge_command_id).await?;
        if !simulation.can_apply || simulation.simulation_id != cmd.simulation_id {
            return Err(AppError::new(
                "CONFLICT",
                "撤回预览已变化，请重新查看后确认",
            ));
        }
        let original = history_data(tx, project, board, &cmd.merge_command_id).await?;
        let before: HashMap<_, _> = simulation
            .current_nodes
            .iter()
            .map(|n| (n.id.clone(), n))
            .collect();
        let after: HashMap<_, _> = simulation
            .merged_nodes
            .iter()
            .map(|n| (n.id.clone(), n))
            .collect();
        let ids: std::collections::BTreeSet<_> = before.keys().chain(after.keys()).collect();
        let mut patches = Vec::new();
        for id in ids {
            if encode(&before.get(id))? != encode(&after.get(id))? {
                patches.push(json!({"id":id,"before":before.get(id),"after":after.get(id)}));
            }
        }
        merge::restore_merge(tx, board, &original, false).await?;
        if !patches.is_empty() {
            let history = json!({"draftId":original["draftId"],"draftRevision":original["draftRevision"],"changes":original["changes"],"revertsCommandId":cmd.merge_command_id,"patches":patches});
            Self::log_canvas_op(tx, project, board, "editorial_merge", &history, &history).await?;
        }
        Ok(())
    }
}
