//! 草案对照只读投影；差异不构成人工审批，也不自动写回源画布。
use crate::{
    canvas::{Board, CanvasNode, CanvasPhotoSource},
    command_audit::{encode, fingerprint, serialization},
    db::Store,
    domain::{AppError, Result},
};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::HashMap;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasChange {
    pub id: String,
    pub node_id: String,
    pub source_node_id: Option<String>,
    pub kind: String,
    pub node_type: String,
    pub before: Option<CanvasNode>,
    pub after: Option<CanvasNode>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasComparison {
    pub draft: Board,
    pub source_board_id: Option<String>,
    pub source_revision: Option<i64>,
    pub current_source_revision: Option<i64>,
    pub source_changed: bool,
    pub baseline_nodes: Vec<CanvasNode>,
    pub changes: Vec<CanvasChange>,
    pub photo_sources: Vec<CanvasPhotoSource>,
}
fn geometry(node: &CanvasNode, origins: &HashMap<String, String>) -> Result<Value> {
    let children = node
        .children_json
        .as_ref()
        .map(|raw| -> Result<Value> {
            let ids: Vec<String> = serde_json::from_str(raw).map_err(serialization)?;
            Ok(json!(
                ids.iter()
                    .map(|id| origins.get(id).unwrap_or(id))
                    .collect::<Vec<_>>()
            ))
        })
        .transpose()?;
    Ok(
        json!({"type":node.node_type,"x":node.x,"y":node.y,"width":node.width,"height":node.height,"rotation":node.rotation,"z":node.z_order,"asset":node.asset_id,"rendition":node.rendition_ref,"content":node.content_json,"style":node.style_json,"locked":node.locked,"children":children}),
    )
}
impl Store {
    pub async fn canvas_comparison(&self, project: &str, board: &str) -> Result<CanvasComparison> {
        let mut tx = self.pool.begin().await?;
        let draft:Board=sqlx::query_as("SELECT b.* FROM boards b JOIN board_draft_tasks t ON t.board_id=b.id WHERE b.id=? AND b.project_id=?")
            .bind(board).bind(project).fetch_optional(&mut *tx).await?.ok_or_else(||AppError::new("NOT_FOUND","画布方案不存在"))?;
        let source: Option<(String, i64)> = sqlx::query_as(
            "SELECT source_board_id,source_revision FROM board_draft_origins WHERE board_id=?",
        )
        .bind(board)
        .fetch_optional(&mut *tx)
        .await?;
        let current_source_revision = if let Some((id, _)) = &source {
            sqlx::query_scalar("SELECT revision FROM boards WHERE id=? AND project_id=?")
                .bind(id)
                .bind(project)
                .fetch_optional(&mut *tx)
                .await?
        } else {
            None
        };
        let origins:Vec<(String,String,String)>=sqlx::query_as("SELECT node_id,source_node_id,base_json FROM board_draft_node_origins WHERE board_id=? ORDER BY rowid")
            .bind(board).fetch_all(&mut *tx).await?;
        let source_map: HashMap<String, String> = origins
            .iter()
            .map(|(node, source, _)| (node.clone(), source.clone()))
            .collect();
        let mut before_map: HashMap<String, CanvasNode> = HashMap::new();
        for (node, _, raw) in &origins {
            before_map.insert(
                node.clone(),
                serde_json::from_str(raw).map_err(serialization)?,
            );
        }
        let baseline_nodes = origins
            .iter()
            .map(|(id, _, _)| before_map[id].clone())
            .collect::<Vec<_>>();
        let nodes: Vec<CanvasNode> = sqlx::query_as(
            "SELECT * FROM canvas_nodes WHERE board_id=? ORDER BY z_order,rowid LIMIT 5001",
        )
        .bind(board)
        .fetch_all(&mut *tx)
        .await?;
        if nodes.len() > 5000 {
            return Err(AppError::new(
                "VALIDATION_ERROR",
                "画布内容过多，请分成较小的方案再比较",
            ));
        }
        let mut changes = Vec::new();
        for node in &nodes {
            let before = before_map.get(&node.id);
            let kind = match before {
                None if node.visible => "add",
                Some(_) if !node.visible => "remove",
                Some(before)
                    if geometry(before, &HashMap::new())? != geometry(node, &source_map)? =>
                {
                    "change"
                }
                _ => continue,
            };
            let id = fingerprint(
                &json!({"board":board,"revision":draft.revision,"node":node.id,"before":before,"after":node}),
            );
            changes.push(CanvasChange {
                id,
                node_id: node.id.clone(),
                source_node_id: source_map.get(&node.id).cloned(),
                kind: kind.into(),
                node_type: node.node_type.clone(),
                before: before.cloned(),
                after: node.visible.then(|| node.clone()),
            });
        }
        let assets: Vec<_> = baseline_nodes
            .iter()
            .chain(nodes.iter())
            .filter_map(|n| n.asset_id.as_ref())
            .collect();
        let photo_sources=sqlx::query_as("SELECT a.id AS asset_id,MIN(p.thumbnail_path) AS thumbnail_path FROM source_assets a JOIN asset_aliases alias ON alias.asset_id=a.id JOIN photos p ON p.id=alias.photo_id AND p.project_id=a.project_id WHERE a.project_id=? AND a.id IN(SELECT value FROM json_each(?)) GROUP BY a.id")
            .bind(project).bind(encode(&assets)?).fetch_all(&mut *tx).await?;
        tx.commit().await?;
        let source_changed = source
            .as_ref()
            .is_some_and(|(_, revision)| Some(*revision) != current_source_revision);
        Ok(CanvasComparison {
            draft,
            source_board_id: source.as_ref().map(|(id, _)| id.clone()),
            source_revision: source.map(|(_, revision)| revision),
            current_source_revision,
            source_changed,
            baseline_nodes,
            changes,
            photo_sources,
        })
    }
}
