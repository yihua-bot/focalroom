//! 审稿模拟与正式采纳共用同一投影；不信任前端回传的节点和预览结果。
use super::*;
use crate::{canvas::validate_rectangle, command_handler::CommandHandler, domain};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[path = "canvas_merge_fields.rs"]
mod fields;
pub(crate) use fields::changed_fields;
pub use fields::{AttributeConflict, EditorialResolution, ResolutionChoice};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditorialSelection {
    pub draft_id: String,
    pub draft_revision: i64,
    pub target_board_id: String,
    pub expected_target_revision: i64,
    pub selected_change_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resolutions: Vec<EditorialResolution>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields_by_change: BTreeMap<String, Vec<String>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyEditorialChanges {
    pub selection: EditorialSelection,
    pub simulation_id: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeSimulation {
    pub simulation_id: String,
    pub draft_revision: i64,
    pub target_revision: i64,
    pub effective_change_ids: Vec<String>,
    pub auto_included_change_ids: Vec<String>,
    pub conflicts: Vec<String>,
    pub attribute_conflicts: Vec<AttributeConflict>,
    pub merged_nodes: Vec<CanvasNode>,
    pub can_apply: bool,
    pub node_id_map: HashMap<String, String>,
}
fn conflict(message: &str) -> AppError {
    AppError::new("CONFLICT", message)
}

impl Store {
    pub async fn simulate_editorial_merge(
        &self,
        project: &str,
        selection: &EditorialSelection,
    ) -> Result<MergeSimulation> {
        let mut tx = self.pool.begin().await?;
        let result = simulate_tx(&mut tx, project, selection).await?;
        tx.commit().await?;
        Ok(result)
    }
}

async fn simulate_tx(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    selection: &EditorialSelection,
) -> Result<MergeSimulation> {
    let selected: BTreeSet<_> = selection.selected_change_ids.iter().cloned().collect();
    if selected.len() > 300 || selected.len() != selection.selected_change_ids.len() {
        return Err(AppError::new(
            "VALIDATION_ERROR",
            "请选择不重复的改动，每次最多 300 项",
        ));
    }
    let comparison = comparison_tx(tx, project, &selection.draft_id).await?;
    if comparison.draft.revision != selection.draft_revision {
        return Err(conflict("方案已有修订，请重新查看并选择改动"));
    }
    if comparison.source_board_id.as_deref() != Some(selection.target_board_id.as_str()) {
        return Err(conflict("这份方案没有可合并的原画布，可另存为独立方案"));
    }
    let target: Board = sqlx::query_as("SELECT * FROM boards WHERE id=? AND project_id=?")
        .bind(&selection.target_board_id)
        .bind(project)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(|| conflict("原画布已不存在"))?;
    let mut conflicts = Vec::new();
    if target.revision != selection.expected_target_revision {
        conflicts.push("原画布已有修改，请重新比较或生成新方案，避免覆盖已有编辑".into());
    }
    if target.owning_task_id.is_some()
        || target.state == "frozen"
        || comparison.draft.owning_task_id.is_none()
    {
        conflicts.push("这张画布暂时无法采纳改动".into());
    }
    let ready: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM agent_tasks t JOIN projects p ON p.id=t.project_id WHERE t.id=? AND t.project_id=? AND p.deleted_at IS NULL AND t.base_revision=p.revision AND t.state IN ('review_ready','completed'))")
        .bind(&comparison.draft.owning_task_id).bind(project).fetch_one(&mut **tx).await?;
    if !ready {
        conflicts.push("请等待方案生成完成，或重新生成已过期的方案".into());
    }
    let changes: BTreeMap<_, _> = comparison
        .changes
        .iter()
        .map(|c| (c.id.clone(), c))
        .collect();
    if selected.iter().any(|id| !changes.contains_key(id)) {
        return Err(conflict("所选建议已变化，请重新查看并选择"));
    }
    if selection.fields_by_change.len() > 300
        || selection.fields_by_change.iter().any(|(id, fields)| {
            !selected.contains(id)
                || changes.get(id).is_none_or(|c| {
                    c.kind != "change" || fields.iter().any(|f| !c.fields.contains(f))
                })
                || fields.is_empty()
                || fields.len() > fields::FIELDS.len()
                || fields.iter().collect::<BTreeSet<_>>().len() != fields.len()
        })
    {
        return Err(conflict("所选属性已变化，请重新查看改动"));
    }
    let mut resolution_keys = BTreeSet::new();
    if selection.resolutions.len() > 2400
        || selection.resolutions.iter().any(|r| {
            !selected.contains(&r.change_id)
                || !fields::FIELDS.contains(&r.field.as_str())
                || !resolution_keys.insert((&r.change_id, &r.field))
        })
    {
        return Err(conflict("冲突选择已变化，请重新比较后选择"));
    }
    let mut effective = selected.clone();
    loop {
        let old = effective.len();
        for id in effective.clone() {
            effective.extend(changes[&id].required_dependencies.iter().cloned());
        }
        if effective.len() == old {
            break;
        }
        if effective.len() > 300 {
            return Err(conflict("关联改动过多，请拆分方案"));
        }
    }
    if effective
        .difference(&selected)
        .any(|id| matches!(changes[id].decision.as_str(), "rejected" | "deferred"))
    {
        conflicts.push("关联建议已被暂缓或拒绝，请先明确选择这些改动".into());
    }
    let rows: Vec<CanvasNode> = sqlx::query_as(
        "SELECT * FROM canvas_nodes WHERE board_id=? AND visible=1 ORDER BY z_order,id LIMIT 5001",
    )
    .bind(&target.id)
    .fetch_all(&mut **tx)
    .await?;
    if rows.len() > 5000 {
        return Err(conflict("画布内容过多，请分成较小的方案"));
    }
    let mut merged: BTreeMap<_, _> = rows.into_iter().map(|n| (n.id.clone(), n)).collect();
    let origins: Vec<(String, String)> = sqlx::query_as(
        "SELECT node_id,source_node_id FROM board_draft_node_origins WHERE board_id=?",
    )
    .bind(&selection.draft_id)
    .fetch_all(&mut **tx)
    .await?;
    let mut mapping: HashMap<String, String> = origins.into_iter().collect();
    let mut attribute_conflicts = Vec::new();
    for node in &comparison.draft_nodes {
        mapping
            .entry(node.id.clone())
            .or_insert_with(|| mapped_id(&target.id, &node.id));
    }
    for id in &effective {
        let change = changes[id];
        let target_id = mapping
            .get(&change.node_id)
            .ok_or_else(|| conflict("节点关联已变化"))?;
        if let Some(before) = &change.before {
            let current = merged.get(target_id);
            if before.locked || current.is_some_and(|n| n.locked) {
                conflicts.push("这项建议涉及已锁定的内容，请排除此项改动".into());
                continue;
            }
            if current.is_none() && change.after.is_some() {
                conflicts.push("这项建议涉及已移除的内容，请重新生成方案".into());
                continue;
            }
            if let Some(current) = current
                && (current.asset_id != before.asset_id || current.node_type != before.node_type)
            {
                conflicts.push("原照片关联已变化，请重新生成方案".into());
                continue;
            }
        }
        if let Some(after) = &change.after {
            if let Some(before) = &change.before
                && (after.asset_id != before.asset_id
                    || after.node_type != before.node_type
                    || after.locked != before.locked)
            {
                conflicts.push("建议不能替换照片身份、节点类型或更改锁定状态".into());
                continue;
            }
            let mut node = after.clone();
            node.id = target_id.clone();
            node.board_id = target.id.clone();
            node.children_json = after
                .children_json
                .as_ref()
                .map(|_| -> Result<String> {
                    encode(
                        &child_ids(after)?
                            .iter()
                            .map(|id| mapping.get(id).cloned().unwrap_or_else(|| id.clone()))
                            .collect::<Vec<_>>(),
                    )
                })
                .transpose()?;
            // 时间戳不参与模拟；正式写入统一赋值。
            if let Some(current) = merged.get(target_id) {
                node.created_at = current.created_at.clone();
                node.updated_at = current.updated_at.clone();
                if let Some(before) = &change.before {
                    let (result, found) = fields::merge_attributes(
                        id,
                        before,
                        current,
                        &node,
                        &selection.resolutions,
                        selection.fields_by_change.get(id),
                    );
                    node = result;
                    attribute_conflicts.extend(found);
                } else if geometry(current, &HashMap::new())? != geometry(&node, &HashMap::new())? {
                    conflicts.push("新增内容与已有内容不一致，请重新比较".into());
                    continue;
                }
            }
            merged.insert(target_id.clone(), node);
        } else {
            if let (Some(before), Some(current)) = (&change.before, merged.get(target_id))
                && geometry(before, &HashMap::new())? != geometry(current, &HashMap::new())?
            {
                conflicts.push("准备移除的内容已有修改，请先处理这项建议".into());
                continue;
            }
            merged.remove(target_id);
            let anchored: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM board_annotations WHERE board_id=? AND node_id=? AND visible=1)")
                .bind(&target.id).bind(target_id).fetch_one(&mut **tx).await?;
            if anchored {
                conflicts.push("移除的节点有作品评论，请先处理评论再采纳".into());
            }
        }
    }
    if merged.len() > 5000 {
        conflicts.push("采纳后画布内容过多，请拆分为较小方案".into());
    }
    for node in merged.values() {
        if ![node.x, node.y, node.width, node.height, node.rotation]
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 1e9)
            || node.width < 0.
            || node.height < 0.
            || node.rotation.abs() > 360.
        {
            conflicts.push("建议中的位置或尺寸无效".into());
        }
        if (node.node_type == "photo") != node.asset_id.is_some() {
            conflicts.push("照片关联不完整，请重新生成方案".into());
        }
        if !comparison.baseline_nodes.iter().any(|n| n.id == node.id) && node.locked {
            conflicts.push("新建议不能锁定作品内容，请重新生成方案".into());
        }
        if !matches!(node.node_type.as_str(), "arrow" | "ink") {
            validate_rectangle(node.x, node.y, node.width, node.height)?;
        }
        let children = child_ids(node)?;
        if children
            .iter()
            .any(|id| id == &node.id || !merged.contains_key(id))
            || children.iter().collect::<BTreeSet<_>>().len() != children.len()
            || (node.node_type == "arrow" && (children.len() != 2 || children[0] == children[1]))
        {
            conflicts.push("连接或分组缺少关联节点，请同时选择关联改动".into());
        }
        if let Some(asset) = &node.asset_id {
            let valid: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM source_assets WHERE id=? AND project_id=?)",
            )
            .bind(asset)
            .bind(project)
            .fetch_one(&mut **tx)
            .await?;
            if !valid {
                conflicts.push("照片已不在当前项目中，请重新选择".into());
            }
        }
        if let Some(rendition) = &node.rendition_ref {
            let valid: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM renditions r JOIN edit_variants v ON v.id=r.variant_id WHERE r.id=? AND v.asset_id=?)")
                .bind(rendition).bind(&node.asset_id).fetch_one(&mut **tx).await?;
            if !valid {
                conflicts.push("调色版本与照片不匹配，无法采纳".into());
            }
            // 现有缩略图渲染尚不支持逐 Rendition 色彩对比；不能让用户批准错误预览。
            if effective.iter().any(|id| {
                changes[id]
                    .after
                    .as_ref()
                    .is_some_and(|n| mapping.get(&n.id) == Some(&node.id))
                    && changes[id]
                        .before
                        .as_ref()
                        .is_none_or(|b| b.rendition_ref != node.rendition_ref)
            }) {
                conflicts.push("此建议涉及调色版本，请在支持版本对比后采纳".into());
            }
        }
    }
    if !acyclic(&merged)? {
        conflicts.push("分组或连接形成了循环，请调整关联内容".into());
    }
    conflicts.sort();
    conflicts.dedup();
    let merged_nodes: Vec<_> = merged.into_values().collect();
    let effective_change_ids: Vec<_> = effective.iter().cloned().collect();
    let auto_included_change_ids = effective.difference(&selected).cloned().collect();
    let simulation_id = fingerprint(
        &json!({"project":project,"selection":selection,"effective":effective_change_ids,"nodes":merged_nodes,"conflicts":conflicts,"attributeConflicts":attribute_conflicts}),
    );
    Ok(MergeSimulation {
        simulation_id,
        draft_revision: comparison.draft.revision,
        target_revision: target.revision,
        can_apply: !effective.is_empty() && conflicts.is_empty() && attribute_conflicts.is_empty(),
        effective_change_ids,
        auto_included_change_ids,
        conflicts,
        attribute_conflicts,
        merged_nodes,
        node_id_map: mapping,
    })
}
fn mapped_id(board: &str, node: &str) -> String {
    let hash = fingerprint(&json!([board, node]));
    format!(
        "{}-{}-4{}-a{}-{}",
        &hash[..8],
        &hash[8..12],
        &hash[13..16],
        &hash[17..20],
        &hash[20..32]
    )
}

fn acyclic(nodes: &BTreeMap<String, CanvasNode>) -> Result<bool> {
    let mut degrees: BTreeMap<String, usize> = nodes.keys().map(|id| (id.clone(), 0)).collect();
    for node in nodes.values() {
        for child in child_ids(node)? {
            if let Some(degree) = degrees.get_mut(&child) {
                *degree += 1;
            }
        }
    }
    let mut queue: Vec<_> = degrees
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(id, _)| id.clone())
        .collect();
    let mut visited = 0;
    while let Some(id) = queue.pop() {
        visited += 1;
        for child in child_ids(&nodes[&id])? {
            if let Some(degree) = degrees.get_mut(&child) {
                *degree -= 1;
                if *degree == 0 {
                    queue.push(child);
                }
            }
        }
    }
    Ok(visited == nodes.len())
}

impl CommandHandler<'_> {
    pub(crate) async fn apply_editorial_tx(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        project: &str,
        board: &str,
        cmd: &ApplyEditorialChanges,
    ) -> Result<()> {
        if cmd.selection.target_board_id != board {
            return Err(conflict("改动不属于当前画布"));
        }
        let simulation = simulate_tx(tx, project, &cmd.selection).await?;
        if !simulation.can_apply {
            let message = if simulation.conflicts.is_empty() {
                "请先选择要采纳的改动".to_owned()
            } else {
                simulation.conflicts.join("；")
            };
            return Err(conflict(&message));
        }
        if simulation.simulation_id != cmd.simulation_id {
            return Err(conflict("预览已变化，请重新查看后再采纳"));
        }
        let current: Vec<CanvasNode> =
            sqlx::query_as("SELECT * FROM canvas_nodes WHERE board_id=? AND visible=1")
                .bind(board)
                .fetch_all(&mut **tx)
                .await?;
        let before: BTreeMap<_, _> = current.into_iter().map(|n| (n.id.clone(), n)).collect();
        let after: BTreeMap<_, _> = simulation
            .merged_nodes
            .into_iter()
            .map(|n| (n.id.clone(), n))
            .collect();
        let ids: BTreeSet<_> = before.keys().chain(after.keys()).cloned().collect();
        let mut patches = Vec::new();
        for id in ids {
            if encode(&before.get(&id))? == encode(&after.get(&id))? {
                continue;
            }
            let old = before.get(&id).cloned();
            let new = after.get(&id).cloned();
            write_node(tx, board, &id, new.as_ref()).await?;
            patches.push(json!({"id":id,"before":old,"after":new}));
        }
        if !patches.is_empty() {
            let history = json!({"draftId":cmd.selection.draft_id,"draftRevision":cmd.selection.draft_revision,"changes":simulation.effective_change_ids,"patches":patches});
            Self::log_canvas_op(tx, project, board, "editorial_merge", &history, &history).await?;
        }
        Ok(())
    }
}

// 历史只包含本次变化的节点，绝不以整板快照覆盖后续无关编辑。
pub(crate) async fn inverse_projection(
    tx: &mut Transaction<'_, Sqlite>,
    board: &str,
    data: &Value,
    redo: bool,
) -> Result<BTreeMap<String, CanvasNode>> {
    let patches = data["patches"]
        .as_array()
        .ok_or_else(|| conflict("无法恢复这次采纳"))?;
    let rows: Vec<CanvasNode> =
        sqlx::query_as("SELECT * FROM canvas_nodes WHERE board_id=? AND visible=1 LIMIT 5001")
            .bind(board)
            .fetch_all(&mut **tx)
            .await?;
    if rows.len() > 5000 || patches.len() > 5000 {
        return Err(conflict("画布内容过多，请分成较小方案"));
    }
    let mut merged: BTreeMap<_, _> = rows.into_iter().map(|n| (n.id.clone(), n)).collect();
    for patch in patches {
        let id = patch["id"]
            .as_str()
            .ok_or_else(|| conflict("节点记录不完整"))?;
        let expected: Option<CanvasNode> =
            serde_json::from_value(patch[if redo { "before" } else { "after" }].clone())
                .map_err(serialization)?;
        let next: Option<CanvasNode> =
            serde_json::from_value(patch[if redo { "after" } else { "before" }].clone())
                .map_err(serialization)?;
        let current = merged.get(id);
        if current.is_some_and(|n| n.locked) {
            return Err(conflict("相关内容已锁定，请先解锁再撤回"));
        }
        let result = match (expected.as_ref(), current, next.as_ref()) {
            (Some(expected), Some(current), Some(next))
                if expected.asset_id == current.asset_id
                    && expected.node_type == current.node_type
                    && expected.asset_id == next.asset_id
                    && expected.node_type == next.node_type =>
            {
                let (merged, conflicts) =
                    fields::merge_attributes("history", expected, current, next, &[], None);
                if !conflicts.is_empty() {
                    return Err(conflict("相关属性已有后续修改，请先处理冲突再撤回"));
                }
                Some(merged)
            }
            (Some(expected), Some(current), None)
                if geometry(current, &HashMap::new())? == geometry(expected, &HashMap::new())? =>
            {
                None
            }
            (None, None, Some(next)) => Some(next.clone()),
            _ => return Err(conflict("相关内容已变化，无法直接撤回这次采纳")),
        };
        if let Some(next) = result {
            if next.id != id || next.board_id != board {
                return Err(conflict("历史记录不属于此画布"));
            }
            merged.insert(id.into(), next);
        } else {
            merged.remove(id);
        }
    }
    for node in merged.values() {
        if child_ids(node)?.iter().any(|id| !merged.contains_key(id)) {
            return Err(conflict("后续编辑引用了这些内容，请先处理关联内容"));
        }
        if !matches!(node.node_type.as_str(), "arrow" | "ink") {
            validate_rectangle(node.x, node.y, node.width, node.height)?;
        }
    }
    if !acyclic(&merged)? {
        return Err(conflict("恢复后关联内容形成循环，无法撤回"));
    }
    let ids: Vec<_> = merged.keys().collect();
    let dangling: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM board_annotations WHERE board_id=? AND visible=1 AND node_id IS NOT NULL AND node_id NOT IN (SELECT value FROM json_each(?)))")
        .bind(board).bind(encode(&ids)?).fetch_one(&mut **tx).await?;
    if dangling {
        return Err(conflict("相关内容已有作品评论，请先处理评论"));
    }
    Ok(merged)
}
pub(crate) async fn restore_merge(
    tx: &mut Transaction<'_, Sqlite>,
    board: &str,
    data: &Value,
    redo: bool,
) -> Result<()> {
    let nodes = inverse_projection(tx, board, data, redo).await?;
    for patch in data["patches"]
        .as_array()
        .ok_or_else(|| conflict("无法恢复这次采纳"))?
    {
        let id = patch["id"]
            .as_str()
            .ok_or_else(|| conflict("节点记录不完整"))?;
        write_node(tx, board, id, nodes.get(id)).await?;
    }
    Ok(())
}
async fn write_node(
    tx: &mut Transaction<'_, Sqlite>,
    board: &str,
    id: &str,
    node: Option<&CanvasNode>,
) -> Result<()> {
    let Some(n) = node else {
        sqlx::query("UPDATE canvas_nodes SET visible=0,updated_at=? WHERE id=? AND board_id=?")
            .bind(domain::timestamp())
            .bind(id)
            .bind(board)
            .execute(&mut **tx)
            .await?;
        return Ok(());
    };
    let written = sqlx::query("INSERT INTO canvas_nodes(id,board_id,node_type,x,y,width,height,rotation,z_order,asset_id,rendition_ref,content_json,style_json,children_json,locked,visible,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,1,?,?) ON CONFLICT(id) DO UPDATE SET x=excluded.x,y=excluded.y,width=excluded.width,height=excluded.height,rotation=excluded.rotation,z_order=excluded.z_order,rendition_ref=excluded.rendition_ref,content_json=excluded.content_json,style_json=excluded.style_json,children_json=excluded.children_json,visible=1,updated_at=excluded.updated_at WHERE canvas_nodes.board_id=excluded.board_id AND canvas_nodes.asset_id IS excluded.asset_id AND canvas_nodes.node_type=excluded.node_type")
        .bind(id).bind(board).bind(&n.node_type).bind(n.x).bind(n.y).bind(n.width).bind(n.height).bind(n.rotation).bind(n.z_order).bind(&n.asset_id).bind(&n.rendition_ref).bind(&n.content_json).bind(&n.style_json).bind(&n.children_json).bind(n.locked).bind(&n.created_at).bind(domain::timestamp()).execute(&mut **tx).await?;
    if written.rows_affected() != 1 {
        return Err(conflict("节点身份发生变化，请重新比较方案"));
    }
    Ok(())
}
