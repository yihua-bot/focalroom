//! 出版物图层与分组：只保存对象引用及顺序，不复制图像或几何。
use super::{PublicationAction, invalid, versions};
use crate::{
    command_audit::{encode, serialization},
    domain::{AppError, Result},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Sqlite, Transaction};
use std::collections::{BTreeMap, BTreeSet};
use versions::PublicationLayout;
pub const KEY: &str = "focalroomLayersV1";
pub const MAX_SPEC: usize = 1024 * 1024;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Node {
    pub id: String,
    pub kind: String,
    pub parent_id: Option<String>,
    pub name: Option<String>,
    pub visible: bool,
    pub locked: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layers {
    pub nodes: Vec<Node>,
}
type Objects = BTreeMap<String, (String, String)>;
pub fn spec(raw: &str) -> Result<Value> {
    if raw.len() > MAX_SPEC {
        return Err(invalid("出版设置过大，请拆分为多个分册"));
    }
    let value: Value =
        serde_json::from_str(raw).map_err(|_| invalid("出版设置不完整，请恢复之前的版本"))?;
    if !value.is_object() || value.get(KEY).is_none() && raw.len() > 32768 {
        return Err(invalid("出版设置不完整，请恢复之前的版本"));
    }
    Ok(value)
}
pub fn has(raw: &str) -> Result<bool> {
    Ok(spec(raw)?.get(KEY).is_some())
}
fn stored(raw: &str) -> Result<Option<Layers>> {
    spec(raw)?
        .get(KEY)
        .map(|v| {
            serde_json::from_value(v.clone())
                .map_err(|_| invalid("图层设置不完整，请恢复之前的版本"))
        })
        .transpose()
}
fn objects(layout: &PublicationLayout) -> Objects {
    layout
        .pages
        .iter()
        .flat_map(|p| {
            p.frames
                .iter()
                .map(|f| (f.id.clone(), ("frame".into(), p.id.clone())))
                .chain(
                    p.texts
                        .iter()
                        .map(|t| (t.id.clone(), ("text".into(), p.id.clone()))),
                )
        })
        .collect()
}
fn defaults(layout: &PublicationLayout) -> Result<Layers> {
    let mut nodes = Vec::new();
    let continuous = super::spreads::validate(layout)?;
    let node = |id: &str, kind: &str| Node {
        id: id.into(),
        kind: kind.into(),
        parent_id: None,
        name: None,
        visible: true,
        locked: false,
    };
    for p in &layout.pages {
        for f in &p.frames {
            if continuous.contains(&f.id) {
                nodes.push(node(&f.id, "frame"));
            }
        }
    }
    for p in &layout.pages {
        for f in &p.frames {
            if !continuous.contains(&f.id) {
                nodes.push(node(&f.id, "frame"));
            }
        }
    }
    for p in &layout.pages {
        for t in &p.texts {
            nodes.push(node(&t.id, "text"));
        }
    }
    Ok(Layers { nodes })
}
pub fn read(layout: &PublicationLayout) -> Result<Layers> {
    if let Some(layers) = stored(&layout.publication.spec_json)? {
        if layout.schema_version != 3 {
            return Err(invalid("请更新FocalRoom后再打开这份图层版面"));
        }
        if layers
            .nodes
            .iter()
            .any(|n| n.kind == "group" && layout.pages.iter().any(|p| p.id == n.id))
        {
            return Err(invalid("分组编号与页面重复"));
        }
        validate(
            &layers,
            &objects(layout),
            &super::spreads::validate(layout)?,
        )?;
        Ok(layers)
    } else {
        if layout.schema_version == 3 {
            return Err(invalid("图层设置缺失，请恢复之前的版面"));
        }
        defaults(layout)
    }
}
fn validate(layers: &Layers, objects: &Objects, continuous: &BTreeSet<String>) -> Result<()> {
    let mut by_id = BTreeMap::new();
    let mut leaves = BTreeSet::new();
    let mut groups = 0;
    if layers.nodes.len() > 4608 {
        return Err(invalid("图层过多，请拆分为多个分册"));
    }
    for n in &layers.nodes {
        if uuid::Uuid::parse_str(&n.id).is_err()
            || by_id.insert(n.id.clone(), n).is_some()
            || n.name.as_ref().is_some_and(|v| {
                v.trim().is_empty() || v.chars().count() > 80 || v.chars().any(char::is_control)
            })
        {
            return Err(invalid("图层编号或名称无效"));
        }
        if n.kind == "group" {
            groups += 1;
            if objects.contains_key(&n.id) {
                return Err(invalid("分组编号不能覆盖照片或文字"));
            }
        } else if objects.get(&n.id).is_none_or(|o| o.0 != n.kind) || !leaves.insert(n.id.clone()) {
            return Err(invalid("图层引用不属于当前出版物"));
        }
    }
    if groups > 512 || leaves.len() != objects.len() {
        return Err(invalid("图层需要完整保留这本出版物的照片和文字"));
    }
    let mut descendants: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for n in &layers.nodes {
        let mut parent = n.parent_id.as_ref();
        let mut seen = BTreeSet::from([n.id.clone()]);
        while let Some(id) = parent {
            if !seen.insert(id.clone()) || seen.len() > 9 {
                return Err(invalid("分组不能循环或超过8层"));
            }
            let group = by_id
                .get(id)
                .filter(|p| p.kind == "group")
                .ok_or_else(|| invalid("找不到所属分组"))?;
            if n.kind != "group" {
                if continuous.contains(&n.id) {
                    return Err(invalid("请先将跨页照片转为单页，再加入分组"));
                }
                descendants
                    .entry(id.clone())
                    .or_default()
                    .insert(objects[&n.id].1.clone());
            }
            parent = group.parent_id.as_ref();
        }
    }
    for n in layers.nodes.iter().filter(|n| n.kind == "group") {
        if descendants.get(&n.id).is_none_or(|pages| pages.len() != 1) {
            return Err(invalid("一个分组需要包含同一页的照片或文字"));
        }
    }
    Ok(())
}
pub fn effective(layers: &Layers, id: &str) -> Result<(bool, bool)> {
    let map: BTreeMap<_, _> = layers.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let Some(mut node) = map.get(id).copied() else {
        return Ok((true, false));
    };
    let mut visible = node.visible;
    let mut locked = node.locked;
    let mut depth = 0;
    while let Some(parent) = &node.parent_id {
        depth += 1;
        if depth > 8 {
            return Err(invalid("分组关联无效"));
        }
        node = map
            .get(parent.as_str())
            .copied()
            .ok_or_else(|| invalid("分组关联无效"))?;
        visible &= node.visible;
        locked |= node.locked;
    }
    Ok((visible, locked))
}
pub fn guard(layout: &PublicationLayout, action: &PublicationAction) -> Result<()> {
    if !has(&layout.publication.spec_json)? {
        return Ok(());
    }
    let layers = read(layout)?;
    let by_id: BTreeMap<_, _> = layers.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let locked = |id: &str| -> Result<()> {
        if effective(&layers, id)?.1 {
            return Err(AppError::new("CONFLICT", "此内容已锁定，请先在图层中解锁"));
        }
        Ok(())
    };
    let frame = match action {
        PublicationAction::UpdateFrame(d) => Some(d.frame_id.as_str()),
        PublicationAction::RemoveFrame(d) => Some(d.frame_id.as_str()),
        PublicationAction::SetFramePhoto(d) => Some(d.frame_id.as_str()),
        PublicationAction::SetSpreadFrame(d) => Some(d.frame_id.as_str()),
        PublicationAction::UnsetSpreadFrame(d) => Some(d.frame_id.as_str()),
        _ => None,
    };
    if let Some(id) = frame {
        locked(id)?;
        if matches!(action, PublicationAction::SetSpreadFrame(_))
            && by_id.get(id).is_some_and(|n| n.parent_id.is_some())
        {
            return Err(AppError::new("CONFLICT", "请先解除分组，再展开为跨页照片"));
        }
    }
    match action {
        PublicationAction::UpdateText(d) => locked(&d.text_id)?,
        PublicationAction::UpdateTextGeometry(d) => locked(&d.text_id)?,
        PublicationAction::RemoveText(d) => locked(&d.text_id)?,
        PublicationAction::Generate(_) => {
            for n in &layers.nodes {
                locked(&n.id)?;
            }
        }
        PublicationAction::RemovePage(d) => {
            guard_pages(layout, &layers, std::slice::from_ref(&d.page_id))?
        }
        PublicationAction::ApplyMaster(d) => guard_pages(layout, &layers, &d.page_ids)?,
        _ => {}
    }
    Ok(())
}
pub(super) fn guard_pages(
    layout: &PublicationLayout,
    layers: &Layers,
    pages: &[String],
) -> Result<()> {
    for p in layout.pages.iter().filter(|p| pages.contains(&p.id)) {
        for id in p
            .frames
            .iter()
            .map(|f| &f.id)
            .chain(p.texts.iter().map(|t| &t.id))
        {
            if effective(layers, id)?.1 {
                return Err(AppError::new(
                    "CONFLICT",
                    "页面包含锁定内容，请先在图层中解锁",
                ));
            }
        }
    }
    Ok(())
}
pub(super) async fn set(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    publication: &str,
    data: &Layers,
) -> Result<()> {
    let mut layout = versions::layout_tx(tx, project, publication).await?;
    validate(data, &objects(&layout), &super::spreads::validate(&layout)?)?;
    put(&mut layout, data)?;
    write(tx, project, publication, &layout.publication.spec_json).await
}
pub(super) fn put(layout: &mut PublicationLayout, data: &Layers) -> Result<()> {
    let mut value = spec(&layout.publication.spec_json)?;
    value[KEY] = serde_json::to_value(data).map_err(serialization)?;
    let raw = encode(&value)?;
    if raw.len() > MAX_SPEC {
        return Err(invalid("图层设置过大，请拆分为多个分册"));
    }
    layout.publication.spec_json = raw;
    layout.schema_version = 3;
    Ok(())
}
async fn write(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    publication: &str,
    raw: &str,
) -> Result<()> {
    sqlx::query("UPDATE publications SET spec_json=? WHERE id=? AND project_id=?")
        .bind(raw)
        .bind(publication)
        .bind(project)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
fn reconciled(mut layers: Layers, current: &Objects) -> Layers {
    layers
        .nodes
        .retain(|n| n.kind == "group" || current.get(&n.id).is_some_and(|o| o.0 == n.kind));
    loop {
        let parents: BTreeSet<_> = layers
            .nodes
            .iter()
            .filter_map(|n| n.parent_id.clone())
            .collect();
        let before = layers.nodes.len();
        layers
            .nodes
            .retain(|n| n.kind != "group" || parents.contains(&n.id));
        if layers.nodes.len() == before {
            break;
        }
    }
    let present: BTreeSet<_> = layers.nodes.iter().map(|n| n.id.clone()).collect();
    for (id, (kind, _)) in current {
        if !present.contains(id) {
            layers.nodes.push(Node {
                id: id.clone(),
                kind: kind.clone(),
                parent_id: None,
                name: None,
                visible: true,
                locked: false,
            });
        }
    }
    layers
}
pub fn reconcile_layout(layout: &mut PublicationLayout) -> Result<()> {
    if let Some(old) = stored(&layout.publication.spec_json)? {
        let next = reconciled(old, &objects(layout));
        put(layout, &next)?;
    }
    Ok(())
}
pub(super) async fn reconcile(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    publication: &str,
) -> Result<()> {
    let raw: String =
        sqlx::query_scalar("SELECT spec_json FROM publications WHERE id=? AND project_id=?")
            .bind(publication)
            .bind(project)
            .fetch_one(&mut **tx)
            .await?;
    let Some(old) = stored(&raw)? else {
        return Ok(());
    };
    let rows:Vec<(String,String,String)>=sqlx::query_as("SELECT f.id,'frame',f.page_id FROM publication_frames f JOIN publication_pages p ON p.id=f.page_id WHERE p.publication_id=? UNION ALL SELECT t.id,'text',t.page_id FROM publication_texts t JOIN publication_pages p ON p.id=t.page_id WHERE p.publication_id=?").bind(publication).bind(publication).fetch_all(&mut **tx).await?;
    let current = rows
        .into_iter()
        .map(|(id, kind, page)| (id, (kind, page)))
        .collect();
    let next = reconciled(old, &current);
    let mut value = spec(&raw)?;
    value[KEY] = serde_json::to_value(next).map_err(serialization)?;
    let next = encode(&value)?;
    if next != raw {
        if next.len() > MAX_SPEC {
            return Err(invalid("图层设置过大"));
        }
        write(tx, project, publication, &next).await?;
    }
    Ok(())
}
