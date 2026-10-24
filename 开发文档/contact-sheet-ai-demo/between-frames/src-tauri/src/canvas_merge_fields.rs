//! 三方属性合并：仅比较基线、当前与建议，不以整个建议节点覆盖人工编辑。
use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditorialResolution {
    pub change_id: String,
    pub field: String,
    pub choice: ResolutionChoice,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionChoice {
    KeepCurrent,
    UseDraft,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttributeConflict {
    pub change_id: String,
    pub node_id: String,
    pub field: String,
    pub baseline: Value,
    pub current: Value,
    pub proposed: Value,
}

pub(super) const FIELDS: &[&str] = &[
    "position",
    "size",
    "rotation",
    "order",
    "content",
    "style",
    "children",
    "rendition",
];
pub(crate) fn changed_fields(
    before: &CanvasNode,
    after: &CanvasNode,
    origins: &HashMap<String, String>,
) -> Result<Vec<String>> {
    let mut normalized = after.clone();
    if after.children_json.is_some() {
        normalized.children_json = Some(encode(
            &child_ids(after)?
                .iter()
                .map(|id| origins.get(id).unwrap_or(id))
                .collect::<Vec<_>>(),
        )?);
    }
    let before = values(before);
    let after = values(&normalized);
    Ok(FIELDS
        .iter()
        .filter(|field| before[*field] != after[*field])
        .map(|field| (*field).into())
        .collect())
}
fn values(n: &CanvasNode) -> BTreeMap<&'static str, Value> {
    BTreeMap::from([
        ("position", json!([n.x, n.y])),
        ("size", json!([n.width, n.height])),
        ("rotation", json!(n.rotation)),
        ("order", json!(n.z_order)),
        ("content", json!(n.content_json)),
        ("style", json!(n.style_json)),
        ("children", json!(n.children_json)),
        ("rendition", json!(n.rendition_ref)),
    ])
}
fn copy(field: &str, from: &CanvasNode, to: &mut CanvasNode) {
    match field {
        "position" => {
            to.x = from.x;
            to.y = from.y;
        }
        "size" => {
            to.width = from.width;
            to.height = from.height;
        }
        "rotation" => to.rotation = from.rotation,
        "order" => to.z_order = from.z_order,
        "content" => to.content_json = from.content_json.clone(),
        "style" => to.style_json = from.style_json.clone(),
        "children" => to.children_json = from.children_json.clone(),
        "rendition" => to.rendition_ref = from.rendition_ref.clone(),
        _ => unreachable!("属性来自固定白名单"),
    }
}
pub(super) fn merge_attributes(
    change: &str,
    baseline: &CanvasNode,
    current: &CanvasNode,
    proposed: &CanvasNode,
    resolutions: &[EditorialResolution],
    selected_fields: Option<&Vec<String>>,
) -> (CanvasNode, Vec<AttributeConflict>) {
    let base = values(baseline);
    let now = values(current);
    let next = values(proposed);
    let mut merged = current.clone();
    let mut conflicts = Vec::new();
    for field in FIELDS {
        if selected_fields.is_some_and(|selected| !selected.iter().any(|s| s == field)) {
            continue;
        }
        if next[field] == base[field] || next[field] == now[field] {
            continue;
        }
        if now[field] == base[field] {
            copy(field, proposed, &mut merged);
            continue;
        }
        match resolutions
            .iter()
            .find(|r| r.change_id == change && r.field == *field)
            .map(|r| &r.choice)
        {
            Some(ResolutionChoice::UseDraft) => copy(field, proposed, &mut merged),
            Some(ResolutionChoice::KeepCurrent) => {}
            None => conflicts.push(AttributeConflict {
                change_id: change.into(),
                node_id: current.id.clone(),
                field: (*field).into(),
                baseline: base[field].clone(),
                current: now[field].clone(),
                proposed: next[field].clone(),
            }),
        }
    }
    (merged, conflicts)
}
