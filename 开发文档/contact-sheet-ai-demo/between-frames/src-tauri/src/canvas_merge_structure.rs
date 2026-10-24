//! 节点存在性与删除关联的三方处理；只在用户确认的预览中整理引用，评论正文不变。
use super::*;
use sqlx::Row;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuralChoice {
    RestoreProposed,
    RemoveProposed,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StructuralResolution {
    pub change_id: String,
    pub choice: StructuralChoice,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedEffect {
    pub node_id: String,
    pub node_type: String,
    pub effect: &'static str,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuralConflict {
    pub change_id: String,
    pub node_id: String,
    pub kind: &'static str,
    pub current_modified: bool,
    pub related_nodes: Vec<RelatedEffect>,
    pub preserved_comment_count: usize,
    pub resolved: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Anchor {
    pub anchor_type: String,
    pub node_id: Option<String>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub nx: Option<f64>,
    pub ny: Option<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnchorPatch {
    pub id: String,
    pub before: Anchor,
    pub after: Anchor,
}
fn anchor(row: &sqlx::sqlite::SqliteRow) -> Anchor {
    Anchor {
        anchor_type: row.get("anchor_type"),
        node_id: row.get("node_id"),
        x: row.get("x"),
        y: row.get("y"),
        nx: row.get("nx"),
        ny: row.get("ny"),
    }
}
fn valid_anchor(anchor: &Anchor, nodes: &BTreeMap<String, CanvasNode>) -> bool {
    if [anchor.x, anchor.y]
        .iter()
        .flatten()
        .any(|v| !v.is_finite() || v.abs() > 1e9)
    {
        return false;
    }
    match anchor.anchor_type.as_str() {
        "board" => {
            anchor.node_id.is_none()
                && anchor.x.is_some()
                && anchor.y.is_some()
                && anchor.nx.is_none()
                && anchor.ny.is_none()
        }
        "node" => {
            anchor
                .node_id
                .as_ref()
                .is_some_and(|id| nodes.contains_key(id))
                && anchor.nx.is_none()
                && anchor.ny.is_none()
        }
        "normalized" => {
            anchor
                .node_id
                .as_ref()
                .is_some_and(|id| nodes.contains_key(id))
                && [anchor.nx, anchor.ny]
                    .iter()
                    .all(|v| v.is_some_and(|v| v.is_finite() && (0.0..=1.0).contains(&v)))
        }
        _ => false,
    }
}
pub(super) fn choice(selection: &EditorialSelection, id: &str) -> Option<StructuralChoice> {
    selection
        .structural_resolutions
        .iter()
        .find(|r| r.change_id == id)
        .map(|r| r.choice)
}
pub(super) async fn missing_node(
    tx: &mut Transaction<'_, Sqlite>,
    board: &str,
    change: &CanvasChange,
    selection: &EditorialSelection,
    conflicts: &mut Vec<String>,
) -> Result<StructuralConflict> {
    let before = change
        .before
        .as_ref()
        .ok_or_else(|| conflict("原内容记录不完整"))?;
    let hidden: Option<CanvasNode> =
        sqlx::query_as("SELECT * FROM canvas_nodes WHERE id=? AND board_id=?")
            .bind(&before.id)
            .bind(board)
            .fetch_optional(&mut **tx)
            .await?;
    if hidden.as_ref().is_some_and(|n| {
        n.locked || n.asset_id != before.asset_id || n.node_type != before.node_type
    }) {
        conflicts.push("已移除内容的身份或锁定状态已变化，无法恢复".into());
    }
    if selection
        .fields_by_change
        .get(&change.id)
        .is_some_and(|fields| fields.len() != change.fields.len())
    {
        conflicts.push("恢复已移除的内容需要完整建议，请选择这项建议的全部属性".into());
    }
    Ok(StructuralConflict {
        change_id: change.id.clone(),
        node_id: before.id.clone(),
        kind: "current_removed",
        current_modified: false,
        related_nodes: Vec::new(),
        preserved_comment_count: 0,
        resolved: choice(selection, &change.id) == Some(StructuralChoice::RestoreProposed),
    })
}

/// 关联只整理分组成员与连接线；不改变照片、锁定或其他节点属性。
pub(super) async fn removals(
    tx: &mut Transaction<'_, Sqlite>,
    board: &str,
    selection: &EditorialSelection,
    changes: &[&CanvasChange],
    original: &BTreeMap<String, CanvasNode>,
    merged: &mut BTreeMap<String, CanvasNode>,
    conflicts: &mut Vec<String>,
) -> Result<(Vec<StructuralConflict>, Vec<AnchorPatch>)> {
    let mut found = Vec::new();
    let mut patches = Vec::new();
    for change in changes {
        let id = &change.id;
        if change.after.is_some() {
            continue;
        }
        let Some(before) = change.before.as_ref() else {
            continue;
        };
        let Some(current) = original.get(&before.id) else {
            continue;
        };
        if merged.contains_key(&before.id) {
            continue;
        }
        let modified = geometry(before, &HashMap::new())? != geometry(current, &HashMap::new())?;
        let mut removed = BTreeMap::from([(current.id.clone(), current.clone())]);
        let mut related = Vec::new();
        // 移除连接线可能继续影响外层分组，最多每个可见节点处理一次。
        loop {
            let affected: Vec<_> = merged
                .values()
                .filter_map(|node| match child_ids(node) {
                    Ok(ids) if ids.iter().any(|id| removed.contains_key(id)) => {
                        Some(Ok(node.clone()))
                    }
                    Ok(_) => None,
                    Err(error) => Some(Err(error)),
                })
                .collect::<Result<_>>()?;
            if affected.is_empty() {
                break;
            }
            let mut progressed = false;
            for mut node in affected {
                if node.locked || !matches!(node.node_type.as_str(), "group" | "arrow") {
                    conflicts.push("关联内容已锁定或无法整理，请先处理关联内容".into());
                    continue;
                }
                if node.node_type == "arrow" {
                    merged.remove(&node.id);
                    related.push(RelatedEffect {
                        node_id: node.id.clone(),
                        node_type: node.node_type.clone(),
                        effect: "remove",
                    });
                    removed.insert(node.id.clone(), node);
                } else {
                    node.children_json = Some(encode(
                        &child_ids(&node)?
                            .into_iter()
                            .filter(|id| !removed.contains_key(id))
                            .collect::<Vec<_>>(),
                    )?);
                    related.push(RelatedEffect {
                        node_id: node.id.clone(),
                        node_type: node.node_type.clone(),
                        effect: "detach",
                    });
                    merged.insert(node.id.clone(), node);
                }
                progressed = true;
            }
            if !progressed {
                break;
            }
        }
        let ids: Vec<_> = removed.keys().collect();
        // 只读锚点，不读取或修改评论正文、作者、解决状态和时间。
        let rows = sqlx::query("SELECT id,anchor_type,node_id,x,y,nx,ny FROM board_annotations WHERE board_id=? AND visible=1 AND node_id IN(SELECT value FROM json_each(?)) ORDER BY id LIMIT 1001")
            .bind(board).bind(encode(&ids)?).fetch_all(&mut **tx).await?;
        if rows.len() + patches.len() > 1000 {
            return Err(conflict("关联评论过多，请分次处理这些内容"));
        }
        let count = rows.len();
        for row in rows {
            let before = anchor(&row);
            let node = &removed[before
                .node_id
                .as_ref()
                .ok_or_else(|| conflict("评论位置不完整"))?];
            let (nx, ny) = if before.anchor_type == "normalized" {
                (
                    before.nx.ok_or_else(|| conflict("评论位置不完整"))?,
                    before.ny.ok_or_else(|| conflict("评论位置不完整"))?,
                )
            } else if before.anchor_type == "node" {
                (0.5, 0.5)
            } else {
                return Err(conflict("评论位置无法安全保留，请先处理评论"));
            };
            if ![nx, ny]
                .iter()
                .all(|n| n.is_finite() && (0.0..=1.0).contains(n))
            {
                return Err(conflict("评论位置无效，请先处理评论"));
            }
            let angle = node.rotation.to_radians();
            let (dx, dy) = ((nx - 0.5) * node.width, (ny - 0.5) * node.height);
            let (x, y) = (
                node.x + node.width / 2.0 + dx * angle.cos() - dy * angle.sin(),
                node.y + node.height / 2.0 + dx * angle.sin() + dy * angle.cos(),
            );
            if ![x, y].iter().all(|n| n.is_finite() && n.abs() <= 1e9) {
                return Err(conflict("评论位置超出画布范围"));
            }
            patches.push(AnchorPatch {
                id: row.get("id"),
                before,
                after: Anchor {
                    anchor_type: "board".into(),
                    node_id: None,
                    x: Some(x),
                    y: Some(y),
                    nx: None,
                    ny: None,
                },
            });
        }
        let related: Vec<_> = related
            .into_iter()
            .map(|n| (n.node_id.clone(), n))
            .collect::<BTreeMap<_, _>>()
            .into_values()
            .collect();
        if modified || !related.is_empty() || count > 0 {
            found.push(StructuralConflict {
                change_id: id.clone(),
                node_id: current.id.clone(),
                kind: "remove_changed",
                current_modified: modified,
                related_nodes: related,
                preserved_comment_count: count,
                resolved: choice(selection, id) == Some(StructuralChoice::RemoveProposed),
            });
        }
    }
    Ok((found, patches))
}

/// 只投影锚点变更。后来修改正文或解决状态可保留；锚点变化则整组拒绝恢复。
pub(crate) async fn history_patches(
    tx: &mut Transaction<'_, Sqlite>,
    board: &str,
    data: &Value,
    redo: bool,
    nodes: &BTreeMap<String, CanvasNode>,
) -> Result<Vec<AnchorPatch>> {
    let patches: Vec<AnchorPatch> = match data.get("annotationPatches") {
        Some(value) => serde_json::from_value(value.clone()).map_err(serialization)?,
        None => Vec::new(),
    };
    if patches.len() > 1000 {
        return Err(conflict("评论恢复记录过大"));
    }
    let mut ids = BTreeSet::new();
    let mut result = Vec::new();
    for patch in patches {
        if !ids.insert(patch.id.clone()) {
            return Err(conflict("评论恢复记录重复"));
        }
        let row = sqlx::query("SELECT anchor_type,node_id,x,y,nx,ny FROM board_annotations WHERE board_id=? AND id=? AND visible=1")
            .bind(board).bind(&patch.id).fetch_optional(&mut **tx).await?.ok_or_else(|| conflict("相关评论已变化，无法恢复"))?;
        let (before, after) = if redo {
            (patch.before, patch.after)
        } else {
            (patch.after, patch.before)
        };
        if anchor(&row) != before || !valid_anchor(&after, nodes) {
            return Err(conflict("相关评论位置已变化，请先处理评论再恢复"));
        }
        result.push(AnchorPatch {
            id: patch.id,
            before,
            after,
        });
    }
    Ok(result)
}
pub(super) async fn write_patches(
    tx: &mut Transaction<'_, Sqlite>,
    board: &str,
    patches: &[AnchorPatch],
) -> Result<()> {
    for patch in patches {
        let n = &patch.after;
        let result=sqlx::query("UPDATE board_annotations SET anchor_type=?,node_id=?,x=?,y=?,nx=?,ny=? WHERE id=? AND board_id=? AND visible=1")
            .bind(&n.anchor_type).bind(&n.node_id).bind(n.x).bind(n.y).bind(n.nx).bind(n.ny).bind(&patch.id).bind(board).execute(&mut **tx).await?;
        if result.rows_affected() != 1 {
            return Err(conflict("评论位置已变化，无法恢复"));
        }
    }
    Ok(())
}
