//! 工具授权来自用户委托，不从目标文字、模型输出或检查点升级。
use super::{policy::ToolRegistry, types::Task};
use crate::{
    canvas::{Board, CanvasNode},
    command_audit::{encode, fingerprint, serialization},
    domain::{self, AppError, Result},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Sqlite, Transaction};
use std::collections::{HashMap, HashSet};

#[cfg(test)]
#[path = "task_scope_tests.rs"]
mod tests;
pub const CANVAS_SYSTEM_PROMPT: &str = include_str!("../../../prompts/agent-canvas-v1.md");

const SELECTION_TOOLS: &[&str] = &[
    "get_project_context",
    "list_photos",
    "search_photos",
    "search_photos_semantic",
    "inspect_photos",
    "compare_photos",
    "retrieve_preferences",
    "get_sequence",
    "create_selection_draft",
    "update_draft_sequence",
    "evaluate_draft",
    "request_feedback",
    "finish_task",
    "get_comments",
    "create_sequence_draft",
];
pub const CANVAS_TOOLS: &[&str] = &[
    "create_board_draft",
    "place_photos_on_board",
    "get_board",
    "group_nodes",
    "move_nodes",
    "remove_nodes",
    "create_connector",
    "add_editorial_note",
    "finish_canvas_task",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CanvasTaskSource {
    pub board_id: String,
    pub revision: i64,
    pub selected_node_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskGrant {
    pub version: u8,
    pub project_id: String,
    pub photo_ids: Vec<String>,
    pub target_count: u32,
    pub tools: Vec<String>,
    pub tool_contract_sha256: String,
    pub canvas_prompt_sha256: Option<String>,
    pub canvas: bool,
    pub source: Option<CanvasTaskSource>,
    pub source_sha256: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BoardBaseline {
    pub board: Board,
    pub nodes: Vec<CanvasNode>,
}
fn denied(message: &str) -> AppError {
    AppError::new("PERMISSION_DENIED", message)
}
fn defaults(task: &Task) -> TaskGrant {
    let mut grant = TaskGrant {
        version: 1,
        project_id: task.project_id.clone(),
        photo_ids: task.allowed_photo_ids.clone(),
        target_count: task.target_count,
        tools: SELECTION_TOOLS.iter().map(|s| s.to_string()).collect(),
        tool_contract_sha256: String::new(),
        canvas_prompt_sha256: None,
        canvas: false,
        source: None,
        source_sha256: None,
    };
    grant.tool_contract_sha256 = fingerprint(&ToolRegistry::model_specs_for_grant(&grant));
    grant
}
pub async fn load(tx: &mut Transaction<'_, Sqlite>, task: &Task) -> Result<TaskGrant> {
    let row: Option<(String, String)> =
        sqlx::query_as("SELECT project_id,grant_json FROM agent_task_grants WHERE task_id=?")
            .bind(&task.id)
            .fetch_optional(&mut **tx)
            .await?;
    let Some((project, raw)) = row else {
        return Ok(defaults(task));
    };
    let grant: TaskGrant = serde_json::from_str(&raw).map_err(serialization)?;
    if project != task.project_id
        || grant.project_id != task.project_id
        || grant.photo_ids != task.allowed_photo_ids
        || grant.target_count != task.target_count
        || grant.version != 1
        || grant.tool_contract_sha256 != fingerprint(&ToolRegistry::model_specs_for_grant(&grant))
        || grant.canvas_prompt_sha256
            != grant
                .canvas
                .then(|| fingerprint(&json!(CANVAS_SYSTEM_PROMPT)))
    {
        return Err(AppError::new(
            "RECOVERY_ERROR",
            "任务授权与当前范围不一致，请重新委托",
        ));
    }
    Ok(grant)
}
pub async fn binding(tx: &mut Transaction<'_, Sqlite>, task: &Task) -> Result<Value> {
    let raw: Option<String> =
        sqlx::query_scalar("SELECT grant_json FROM agent_task_grants WHERE task_id=?")
            .bind(&task.id)
            .fetch_optional(&mut **tx)
            .await?;
    Ok(match raw {
        Some(_) => json!(fingerprint(
            &serde_json::to_value(load(tx, task).await?).map_err(serialization)?
        )),
        None => Value::Null,
    })
}
pub async fn bind(
    tx: &mut Transaction<'_, Sqlite>,
    task: &Task,
    canvas: bool,
    source: Option<&CanvasTaskSource>,
) -> Result<()> {
    let mut grant = defaults(task);
    grant.canvas = canvas;
    grant.canvas_prompt_sha256 = canvas.then(|| fingerprint(&json!(CANVAS_SYSTEM_PROMPT)));
    let mut snapshot = None;
    if canvas {
        crate::agent::promotion::ensure_local_write(tx, &task.project_id).await?;
        grant
            .tools
            .extend(CANVAS_TOOLS.iter().map(|s| s.to_string()));
        if let Some(source) = source {
            let board:Board=sqlx::query_as("SELECT * FROM boards WHERE id=? AND project_id=? AND owning_task_id IS NULL AND state<>'frozen'")
                .bind(&source.board_id).bind(&task.project_id).fetch_optional(&mut **tx).await?
                .ok_or_else(||denied("请选择本项目可编辑的画布"))?;
            if board.revision != source.revision {
                return Err(AppError::new(
                    "REVISION_CONFLICT",
                    "画布已更新，请重新打开后再发送",
                ));
            }
            let nodes:Vec<CanvasNode>=sqlx::query_as("WITH RECURSIVE chosen(id) AS (SELECT id FROM canvas_nodes WHERE board_id=? AND visible=1 AND (? OR id IN(SELECT value FROM json_each(?))) UNION SELECT child.value FROM canvas_nodes n JOIN chosen c ON c.id=n.id JOIN json_each(COALESCE(n.children_json,'[]')) child WHERE n.board_id=? AND n.visible=1) SELECT n.* FROM canvas_nodes n JOIN chosen c ON c.id=n.id WHERE n.board_id=? AND n.visible=1 ORDER BY n.z_order,n.rowid LIMIT 1001")
                .bind(&source.board_id).bind(source.selected_node_ids.is_empty()).bind(encode(&source.selected_node_ids)?).bind(&source.board_id).bind(&source.board_id).fetch_all(&mut **tx).await?;
            if nodes.len() > 1000 {
                return Err(AppError::new(
                    "VALIDATION_ERROR",
                    "请缩小画布选区，每次最多处理 1000 个节点",
                ));
            }
            let by_id: HashMap<_, _> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();
            let mut selected: HashSet<String> = source.selected_node_ids.iter().cloned().collect();
            if selected.len() != source.selected_node_ids.len()
                || selected.iter().any(|id| !by_id.contains_key(id.as_str()))
            {
                return Err(denied("所选节点不属于当前画布"));
            }
            if selected.is_empty() {
                selected.extend(nodes.iter().map(|n| n.id.clone()));
            }
            // 选择组/连接时包含其依赖对象，维持稳定关联而非悬空引用。
            loop {
                let mut added = false;
                for node in &nodes {
                    if selected.contains(&node.id)
                        && let Some(raw) = &node.children_json
                    {
                        let children: Vec<String> =
                            serde_json::from_str(raw).map_err(serialization)?;
                        for child in children {
                            if !by_id.contains_key(child.as_str()) {
                                return Err(denied("画布包含失效的关联，请先修正后再发送"));
                            }
                            added |= selected.insert(child);
                        }
                    }
                }
                if !added {
                    break;
                }
            }
            let nodes: Vec<_> = nodes
                .into_iter()
                .filter(|n| selected.contains(&n.id))
                .collect();
            for node in &nodes {
                if node.width < 0. || node.height < 0. {
                    return Err(AppError::new(
                        "VALIDATION_ERROR",
                        "画布包含无效尺寸，请先修正后再发送",
                    ));
                }
                crate::canvas::validate_rectangle(
                    node.x,
                    node.y,
                    node.width.max(1.),
                    node.height.max(1.),
                )?;
                if let Some(asset) = &node.asset_id {
                    let valid: bool = sqlx::query_scalar(
                        "SELECT EXISTS(SELECT 1 FROM source_assets WHERE id=? AND project_id=?)",
                    )
                    .bind(asset)
                    .bind(&task.project_id)
                    .fetch_one(&mut **tx)
                    .await?;
                    if !valid {
                        return Err(denied("画布中的照片不属于当前项目"));
                    }
                }
            }
            let mut ids: Vec<_> = selected.into_iter().collect();
            ids.sort();
            grant.source = Some(CanvasTaskSource {
                board_id: source.board_id.clone(),
                revision: source.revision,
                selected_node_ids: ids,
            });
            let baseline = BoardBaseline { board, nodes };
            let value = serde_json::to_value(&baseline).map_err(serialization)?;
            let raw = encode(&baseline)?;
            if raw.len() > 2 * 1024 * 1024 {
                return Err(AppError::new(
                    "VALIDATION_ERROR",
                    "画布内容过多，请缩小选区后再发送",
                ));
            }
            grant.source_sha256 = Some(fingerprint(&value));
            snapshot = Some(raw);
        }
    }
    grant.tool_contract_sha256 = fingerprint(&ToolRegistry::model_specs_for_grant(&grant));
    sqlx::query("INSERT INTO agent_task_grants(task_id,project_id,grant_json,source_snapshot_json,created_at) VALUES(?,?,?,?,?)")
        .bind(&task.id).bind(&task.project_id).bind(encode(&grant)?).bind(snapshot).bind(domain::timestamp()).execute(&mut **tx).await?;
    Ok(())
}
pub async fn baseline(
    tx: &mut Transaction<'_, Sqlite>,
    task: &Task,
) -> Result<Option<BoardBaseline>> {
    let grant = load(tx, task).await?;
    let raw: Option<String> =
        sqlx::query_scalar("SELECT source_snapshot_json FROM agent_task_grants WHERE task_id=?")
            .bind(&task.id)
            .fetch_optional(&mut **tx)
            .await?
            .flatten();
    let value = raw
        .map(|raw| serde_json::from_str::<BoardBaseline>(&raw).map_err(serialization))
        .transpose()?;
    if value
        .as_ref()
        .map(|v| fingerprint(&serde_json::to_value(v).unwrap()))
        != grant.source_sha256
    {
        return Err(AppError::new(
            "RECOVERY_ERROR",
            "画布来源记录不一致，请重新委托",
        ));
    }
    Ok(value)
}
pub async fn require_tool(
    tx: &mut Transaction<'_, Sqlite>,
    task: &Task,
    name: &str,
) -> Result<TaskGrant> {
    let grant = load(tx, task).await?;
    let name = ToolRegistry::canonical(name)?;
    if !grant.tools.contains(&name) {
        return Err(denied("此任务没有执行这项操作的授权，请重新选择任务目标"));
    }
    Ok(grant)
}
pub async fn clone_source(
    tx: &mut Transaction<'_, Sqlite>,
    task: &Task,
    board: &str,
) -> Result<()> {
    let Some(source) = baseline(tx, task).await? else {
        return Ok(());
    };
    let grant = load(tx, task).await?;
    sqlx::query("INSERT INTO board_draft_origins(board_id,task_id,source_board_id,source_revision,snapshot_sha256,created_at) VALUES(?,?,?,?,?,?)")
        .bind(board).bind(&task.id).bind(&source.board.id).bind(source.board.revision).bind(&grant.source_sha256).bind(domain::timestamp()).execute(&mut **tx).await?;
    let ids: HashMap<_, _> = source
        .nodes
        .iter()
        .map(|n| (n.id.clone(), domain::id()))
        .collect();
    let now = domain::timestamp();
    for node in source.nodes {
        let children = node
            .children_json
            .as_ref()
            .map(|raw| -> Result<String> {
                let old: Vec<String> = serde_json::from_str(raw).map_err(serialization)?;
                encode(
                    &old.iter()
                        .map(|id| ids.get(id).ok_or_else(|| denied("节点关联不完整")))
                        .collect::<Result<Vec<_>>>()?,
                )
            })
            .transpose()?;
        let id = &ids[&node.id];
        sqlx::query("INSERT INTO canvas_nodes(id,board_id,node_type,x,y,width,height,rotation,z_order,asset_id,rendition_ref,content_json,style_json,children_json,locked,visible,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,1,?,?)")
            .bind(id).bind(board).bind(&node.node_type).bind(node.x).bind(node.y).bind(node.width).bind(node.height).bind(node.rotation).bind(node.z_order).bind(&node.asset_id).bind(&node.rendition_ref).bind(&node.content_json).bind(&node.style_json).bind(children).bind(node.locked).bind(&now).bind(&now).execute(&mut **tx).await?;
        sqlx::query("INSERT INTO board_draft_node_origins(node_id,board_id,source_node_id,base_json) VALUES(?,?,?,?)")
            .bind(id).bind(board).bind(&node.id).bind(encode(&node)?).execute(&mut **tx).await?;
    }
    Ok(())
}
