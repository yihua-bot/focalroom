//! 可复用页面母版：纯预览计划与人工事务写入分开；关联页不被后台更新。
use super::{guides, invalid, versions};
use crate::{
    command_audit::{encode, fingerprint, serialization},
    db::Store,
    domain::{AppError, Result},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Sqlite, Transaction};
use std::collections::BTreeSet;
use versions::{LayoutFrame, LayoutPage, LayoutText, PublicationLayout};
const KEY: &str = "focalroomPageMastersV1";
const LINK: &str = "focalroomPageMasterV1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveMaster {
    pub master_id: String,
    pub name: String,
    pub source_page_id: String,
    pub expected_master_revision: Option<i64>,
    pub include_text: bool,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub include_guides: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyMaster {
    pub master_id: String,
    pub master_revision: i64,
    pub page_ids: Vec<String>,
    pub instance_id: String,
    pub overwrite_local: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MasterFrame {
    pub slot_id: String,
    pub x_mm: f64,
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
    pub fit_mode: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PageMaster {
    pub id: String,
    pub revision: i64,
    pub name: String,
    pub source_page_id: String,
    pub width_mm: f64,
    pub height_mm: f64,
    pub margin_mm: f64,
    pub bleed_mm: f64,
    pub frames: Vec<MasterFrame>,
    pub texts: Vec<LayoutText>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub guides: Option<guides::PageGuides>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Binding {
    pub slot_id: String,
    pub object_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MasterLink {
    pub id: String,
    pub revision: i64,
    pub signature: String,
    pub frames: Vec<Binding>,
    pub texts: Vec<Binding>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present"
    )]
    pub guide_signature: Option<String>,
}
// 旧数据缺字段时保持原字节形状；显式null不能伪装成旧版未声明能力。
fn present<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> std::result::Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterPreview {
    pub revision: i64,
    pub layout: PublicationLayout,
    pub previews: Vec<crate::publication_previews::FramePreview>,
    pub overridden_page_ids: Vec<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MasterState {
    pub page_id: String,
    pub master_id: String,
    pub applied_revision: i64,
    pub current_revision: Option<i64>,
    pub locally_changed: bool,
}
pub fn states(layout: &PublicationLayout) -> Result<Vec<MasterState>> {
    let definitions = masters(layout)?;
    let mut result = Vec::new();
    for page in &layout.pages {
        if let Some(link) = link(page)? {
            result.push(MasterState {
                page_id: page.id.clone(),
                master_id: link.id.clone(),
                applied_revision: link.revision,
                current_revision: definitions
                    .iter()
                    .find(|m| m.id == link.id)
                    .map(|m| m.revision),
                locally_changed: locally_changed(page, &link)?,
            });
        }
    }
    Ok(result)
}
fn identifier(id: &str) -> Result<()> {
    uuid::Uuid::parse_str(id).map_err(|_| invalid("这项母版内容无效，请重新打开"))?;
    Ok(())
}
fn object(raw: &str) -> Result<Value> {
    let value: Value = serde_json::from_str(raw).map_err(serialization)?;
    if !value.is_object() {
        return Err(invalid("这页的旧版设置无法直接关联母版，请先另建页面"));
    }
    Ok(value)
}
pub fn masters(layout: &PublicationLayout) -> Result<Vec<PageMaster>> {
    let spec: Value = serde_json::from_str(&layout.publication.spec_json).map_err(serialization)?;
    let result: Vec<PageMaster> = spec
        .get(KEY)
        .map(|v| {
            serde_json::from_value(v.clone())
                .map_err(|_| invalid("母版记录不完整，请恢复之前的版面版本"))
        })
        .transpose()?
        .unwrap_or_default();
    if result.len() > 16 || encode(&result)?.len() > 32768 {
        return Err(invalid("每本出版物最多保存16份母版"));
    }
    let mut seen = BTreeSet::new();
    for master in &result {
        identifier(&master.id)?;
        identifier(&master.source_page_id)?;
        if !seen.insert(&master.id)
            || master.revision < 1
            || master.revision > 9_007_199_254_739_000
            || master.name.trim().is_empty()
            || master.name.chars().count() > 80
            || master.frames.len() > 64
            || master.texts.len() > 16
        {
            return Err(invalid("母版记录不完整，请恢复之前的版面版本"));
        }
        let page = LayoutPage {
            id: master.source_page_id.clone(),
            page_number: 1,
            side: "right".into(),
            width_mm: master.width_mm,
            height_mm: master.height_mm,
            margin_mm: master.margin_mm,
            bleed_mm: master.bleed_mm,
            layout_json: None,
            frames: master
                .frames
                .iter()
                .map(|f| LayoutFrame {
                    id: f.slot_id.clone(),
                    x_mm: f.x_mm,
                    y_mm: f.y_mm,
                    width_mm: f.width_mm,
                    height_mm: f.height_mm,
                    fit_mode: f.fit_mode.clone(),
                    asset_id: None,
                    rendition_id: None,
                    crop_x: 0.5,
                    crop_y: 0.5,
                    crop_zoom: 1.,
                    source: None,
                })
                .collect(),
            texts: master.texts.clone(),
        };
        if let Some(settings) = &master.guides {
            guides::validate(&page, settings)?;
        }
        for frame in &master.frames {
            identifier(&frame.slot_id)?;
        }
        for text in &master.texts {
            identifier(&text.id)?;
        }
        let sample = PublicationLayout {
            schema_version: 1,
            publication: versions::LayoutPublication {
                spec_json: "{}".into(),
                ..layout.publication.clone()
            },
            pages: vec![page],
        };
        versions::validate_layout(&sample)?;
    }
    Ok(result)
}
pub fn link(page: &LayoutPage) -> Result<Option<MasterLink>> {
    let Some(raw) = &page.layout_json else {
        return Ok(None);
    };
    let value: Value = serde_json::from_str(raw).map_err(serialization)?;
    let Some(value) = value.get(LINK) else {
        return Ok(None);
    };
    if raw.len() > 32768 {
        return Err(invalid("页面的母版关联过大"));
    }
    let link: MasterLink = serde_json::from_value(value.clone())
        .map_err(|_| invalid("页面的母版关联不完整，请恢复之前的版面版本"))?;
    identifier(&link.id)?;
    if link.revision < 1
        || link.frames.len() > 100
        || link.texts.len() > 16
        || link.signature.len() != 64
        || !link.signature.bytes().all(|c| c.is_ascii_hexdigit())
        || link.guide_signature.as_ref().is_some_and(|signature| {
            signature.len() != 64
                || !signature
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        })
    {
        return Err(invalid("页面的母版关联不完整"));
    }
    let mut slots = BTreeSet::new();
    let mut objects = BTreeSet::new();
    for binding in link.frames.iter().chain(&link.texts) {
        identifier(&binding.slot_id)?;
        identifier(&binding.object_id)?;
        if !slots.insert(&binding.slot_id) || !objects.insert(&binding.object_id) {
            return Err(invalid("母版关联包含重复内容"));
        }
    }
    Ok(Some(link))
}
fn signature(page: &LayoutPage, texts: &[Binding]) -> Result<String> {
    let continuous: BTreeSet<_> = super::spreads::links(page)?
        .into_iter()
        .map(|link| link.frame_id)
        .collect();
    let mut frames: Vec<_> = page
        .frames
        .iter()
        .filter(|frame| !continuous.contains(&frame.id))
        .map(|f| json!([f.id, f.x_mm, f.y_mm, f.width_mm, f.height_mm, f.fit_mode]))
        .collect();
    frames.sort_by_key(Value::to_string);
    let mut text: Vec<_> = texts
        .iter()
        .map(|b| json!([b.object_id, page.texts.iter().find(|t| t.id == b.object_id)]))
        .collect();
    text.sort_by_key(Value::to_string);
    Ok(fingerprint(&json!([
        page.width_mm,
        page.height_mm,
        page.margin_mm,
        page.bleed_mm,
        frames,
        text
    ])))
}
pub fn locally_changed(page: &LayoutPage, link: &MasterLink) -> Result<bool> {
    let guide_changed = match &link.guide_signature {
        Some(saved) => guide_signature(guides::read(page)?.as_ref()) != *saved,
        None => false,
    };
    Ok(signature(page, &link.texts)? != link.signature || guide_changed)
}
fn guide_signature(settings: Option<&guides::PageGuides>) -> String {
    fingerprint(&json!(settings))
}
fn attach(page: &mut LayoutPage, mut link: MasterLink) -> Result<()> {
    link.signature = signature(page, &link.texts)?;
    let mut meta = object(page.layout_json.as_deref().unwrap_or("{}"))?;
    meta[LINK] = serde_json::to_value(link).map_err(serialization)?;
    let raw = encode(&meta)?;
    if raw.len() > 32768 {
        return Err(invalid("页面的母版关联过大"));
    }
    page.layout_json = Some(raw);
    Ok(())
}
/// 随机实例命名空间派生稳定伪随机UUID，预览和提交共享身份；不承担认证用途。
fn instance_id(instance: &str, page: &str, kind: &str, slot: &str) -> String {
    use sha2::{Digest, Sha256};
    let bytes = Sha256::digest(
        encode(&json!([instance, page, kind, slot]))
            .unwrap()
            .as_bytes(),
    );
    uuid::Builder::from_random_bytes(bytes[..16].try_into().unwrap())
        .into_uuid()
        .to_string()
}
pub fn plan(
    layout: &PublicationLayout,
    data: &ApplyMaster,
) -> Result<(PublicationLayout, Vec<String>)> {
    identifier(&data.master_id)?;
    identifier(&data.instance_id)?;
    if data.page_ids.is_empty()
        || data.page_ids.len() > 100
        || data.page_ids.iter().collect::<BTreeSet<_>>().len() != data.page_ids.len()
    {
        return Err(invalid("请重新选择需要应用母版的页面"));
    }
    let master = masters(layout)?
        .into_iter()
        .find(|m| m.id == data.master_id && m.revision == data.master_revision)
        .ok_or_else(|| AppError::new("CONFLICT", "母版已有更新，请重新查看后再应用"))?;
    let mut result = layout.clone();
    let mut overrides = Vec::new();
    for id in &data.page_ids {
        let page = result
            .pages
            .iter_mut()
            .find(|p| &p.id == id)
            .ok_or_else(|| AppError::new("NOT_FOUND", "页面不属于这本出版物"))?;
        let previous = link(page)?;
        let current_guides = guides::read(page)?;
        let geometry_changed = match &previous {
            Some(link) => signature(page, &link.texts)? != link.signature,
            None => false,
        };
        let replaces_guides = master.guides.is_some() && master.guides != current_guides;
        let overrides_guides = replaces_guides
            && match previous
                .as_ref()
                .and_then(|link| link.guide_signature.as_ref())
            {
                Some(saved) => guide_signature(current_guides.as_ref()) != *saved,
                None => current_guides.is_some(),
            };
        if geometry_changed || overrides_guides {
            overrides.push(id.clone());
        }
        let continuous: BTreeSet<_> = super::spreads::links(page)?
            .into_iter()
            .map(|link| link.frame_id)
            .collect();
        let fixed: Vec<_> = page
            .frames
            .iter()
            .filter(|frame| continuous.contains(&frame.id))
            .cloned()
            .collect();
        let old_frames: Vec<_> = page
            .frames
            .iter()
            .filter(|frame| !continuous.contains(&frame.id))
            .cloned()
            .collect();
        let old_texts = page.texts.clone();
        let mut photos: Vec<_> = old_frames
            .iter()
            .filter(|f| f.asset_id.is_some())
            .cloned()
            .collect();
        photos.sort_by(|a, b| {
            a.y_mm
                .total_cmp(&b.y_mm)
                .then(a.x_mm.total_cmp(&b.x_mm))
                .then(a.id.cmp(&b.id))
        });
        if photos.len() > master.frames.len() {
            return Err(AppError::new(
                "CONFLICT",
                "母版的照片框不够容纳本页照片，请使用更多照片框的母版",
            ));
        }
        let mut bindings = Vec::new();
        let mut frames = Vec::new();
        let mut used = BTreeSet::new();
        for (index, slot) in master.frames.iter().enumerate() {
            let bound = previous
                .as_ref()
                .filter(|l| l.id == master.id)
                .and_then(|l| l.frames.iter().find(|b| b.slot_id == slot.slot_id))
                .and_then(|b| {
                    old_frames
                        .iter()
                        .find(|f| f.id == b.object_id && f.asset_id.is_none())
                });
            let old = photos.get(index).or(bound);
            let id = old.map(|f| f.id.clone()).unwrap_or_else(|| {
                instance_id(&data.instance_id, &page.id, "frame", &slot.slot_id)
            });
            if !used.insert(id.clone()) {
                return Err(invalid("母版的照片关联冲突，请重新查看"));
            }
            let mut frame = old.cloned().unwrap_or(LayoutFrame {
                id: id.clone(),
                x_mm: 0.,
                y_mm: 0.,
                width_mm: 1.,
                height_mm: 1.,
                fit_mode: "fit".into(),
                asset_id: None,
                rendition_id: None,
                crop_x: 0.5,
                crop_y: 0.5,
                crop_zoom: 1.,
                source: None,
            });
            frame.x_mm = slot.x_mm;
            frame.y_mm = slot.y_mm;
            frame.width_mm = slot.width_mm;
            frame.height_mm = slot.height_mm;
            frame.fit_mode = slot.fit_mode.clone();
            bindings.push(Binding {
                slot_id: slot.slot_id.clone(),
                object_id: frame.id.clone(),
            });
            frames.push(frame);
        }
        let previous_texts: BTreeSet<_> = previous
            .as_ref()
            .map(|l| l.texts.iter().map(|b| b.object_id.clone()).collect())
            .unwrap_or_default();
        let mut texts: Vec<_> = old_texts
            .iter()
            .filter(|t| !previous_texts.contains(&t.id))
            .cloned()
            .collect();
        let mut text_bindings = Vec::new();
        for slot in &master.texts {
            let old = previous
                .as_ref()
                .filter(|l| l.id == master.id)
                .and_then(|l| l.texts.iter().find(|b| b.slot_id == slot.id))
                .map(|b| b.object_id.clone());
            let id =
                old.unwrap_or_else(|| instance_id(&data.instance_id, &page.id, "text", &slot.id));
            let mut text = slot.clone();
            text.id = id.clone();
            texts.retain(|t| t.id != id);
            texts.push(text);
            text_bindings.push(Binding {
                slot_id: slot.id.clone(),
                object_id: id,
            });
        }
        page.width_mm = master.width_mm;
        page.height_mm = master.height_mm;
        page.margin_mm = master.margin_mm;
        page.bleed_mm = master.bleed_mm;
        frames.extend(fixed);
        page.frames = frames;
        page.texts = texts;
        page.frames.sort_by(|a, b| a.id.cmp(&b.id));
        page.texts.sort_by(|a, b| a.id.cmp(&b.id));
        if let Some(settings) = &master.guides {
            guides::apply_to_page(page, settings)?;
        }
        attach(
            page,
            MasterLink {
                id: master.id.clone(),
                revision: master.revision,
                signature: String::new(),
                frames: bindings,
                texts: text_bindings,
                guide_signature: master
                    .guides
                    .as_ref()
                    .map(|settings| guide_signature(Some(settings))),
            },
        )?;
    }
    super::layers::reconcile_layout(&mut result)?;
    versions::validate_layout(&result)?;
    if encode(&result)?.len() > versions::MAX_SNAPSHOT {
        return Err(invalid("版面记录过大，请拆分为多个分册"));
    }
    Ok((result, overrides))
}
pub(super) async fn save(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    publication: &str,
    data: &SaveMaster,
) -> Result<()> {
    identifier(&data.master_id)?;
    let mut layout = versions::layout_tx(tx, project, publication).await?;
    let mut definitions = masters(&layout)?;
    let old = definitions.iter().find(|m| m.id == data.master_id);
    if old.map(|m| m.revision) != data.expected_master_revision {
        return Err(AppError::new(
            "CONFLICT",
            "母版已有更新，请重新查看后再保存",
        ));
    }
    if data.name.trim().is_empty() || data.name.chars().count() > 80 {
        return Err(invalid("母版名称须为1–80个字符"));
    }
    let source = layout
        .pages
        .iter_mut()
        .find(|p| p.id == data.source_page_id)
        .ok_or_else(|| AppError::new("NOT_FOUND", "找不到作为母版的页面"))?;
    let prior = link(source)?.filter(|l| l.id == data.master_id);
    let continuous: BTreeSet<_> = super::spreads::links(source)?
        .into_iter()
        .map(|link| link.frame_id)
        .collect();
    let mut photo_order: Vec<_> = source
        .frames
        .iter()
        .filter(|frame| !continuous.contains(&frame.id))
        .collect();
    photo_order.sort_by(|a, b| {
        a.y_mm
            .total_cmp(&b.y_mm)
            .then(a.x_mm.total_cmp(&b.x_mm))
            .then(a.id.cmp(&b.id))
    });
    let slot_for = |kind: bool, id: &str| {
        prior
            .as_ref()
            .and_then(|l| {
                if kind {
                    l.frames.iter().find(|b| b.object_id == id)
                } else {
                    l.texts.iter().find(|b| b.object_id == id)
                }
            })
            .map(|b| b.slot_id.clone())
            .unwrap_or_else(|| id.into())
    };
    let frames: Vec<_> = photo_order
        .iter()
        .map(|f| MasterFrame {
            slot_id: slot_for(true, &f.id),
            x_mm: f.x_mm,
            y_mm: f.y_mm,
            width_mm: f.width_mm,
            height_mm: f.height_mm,
            fit_mode: f.fit_mode.clone(),
        })
        .collect();
    let selected: Vec<_> = source
        .texts
        .iter()
        .filter(|t| {
            data.include_text
                || serde_json::from_str::<Value>(&t.content_json)
                    .ok()
                    .is_some_and(|v| v["kind"] == "page_number")
        })
        .collect();
    let texts: Vec<_> = selected
        .iter()
        .map(|t| {
            let mut text = (*t).clone();
            text.id = slot_for(false, &t.id);
            text
        })
        .collect();
    let revision = old.map_or(1, |m| m.revision + 1);
    let master_guides = match data.include_guides {
        Some(true) => Some(
            guides::read(source)?
                .ok_or_else(|| invalid("请先保存本页网格与参考线，再将其加入母版"))?,
        ),
        Some(false) => None,
        None => old.and_then(|master| master.guides.clone()),
    };
    let link = MasterLink {
        id: data.master_id.clone(),
        revision,
        signature: String::new(),
        frames: photo_order
            .iter()
            .zip(&frames)
            .map(|(f, s)| Binding {
                slot_id: s.slot_id.clone(),
                object_id: f.id.clone(),
            })
            .collect(),
        texts: selected
            .iter()
            .zip(&texts)
            .map(|(t, s)| Binding {
                slot_id: s.id.clone(),
                object_id: t.id.clone(),
            })
            .collect(),
        guide_signature: master_guides
            .as_ref()
            .map(|settings| guide_signature(Some(settings))),
    };
    let master = PageMaster {
        id: data.master_id.clone(),
        revision,
        name: data.name.trim().into(),
        source_page_id: source.id.clone(),
        width_mm: source.width_mm,
        height_mm: source.height_mm,
        margin_mm: source.margin_mm,
        bleed_mm: source.bleed_mm,
        frames,
        texts,
        guides: master_guides,
    };
    attach(source, link)?;
    let source = source.clone();
    definitions.retain(|m| m.id != data.master_id);
    definitions.push(master);
    let mut spec = object(&layout.publication.spec_json)?;
    spec[KEY] = serde_json::to_value(definitions).map_err(serialization)?;
    layout.publication.spec_json = encode(&spec)?;
    if layout.publication.spec_json.len() > super::layers::MAX_SPEC {
        return Err(invalid("母版内容过多，请减少文字或母版数量"));
    }
    masters(&layout)?;
    sqlx::query("UPDATE publications SET spec_json=? WHERE id=? AND project_id=?")
        .bind(&layout.publication.spec_json)
        .bind(publication)
        .bind(project)
        .execute(&mut **tx)
        .await?;
    sqlx::query("UPDATE publication_pages SET layout_json=? WHERE id=? AND publication_id=?")
        .bind(source.layout_json)
        .bind(source.id)
        .bind(publication)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
pub(super) async fn apply(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    publication: &str,
    data: &ApplyMaster,
) -> Result<()> {
    let before = versions::layout_tx(tx, project, publication).await?;
    super::layers::guard(
        &before,
        &super::PublicationAction::ApplyMaster(data.clone()),
    )?;
    let (after, overrides) = plan(&before, data)?;
    if !data.overwrite_local && !overrides.is_empty() {
        return Err(AppError::new(
            "CONFLICT",
            "页面包含手工调整，请先预览并确认是否覆盖",
        ));
    }
    for id in &data.page_ids {
        write_page(
            tx,
            publication,
            after.pages.iter().find(|p| &p.id == id).unwrap(),
        )
        .await?;
    }
    Ok(())
}
async fn write_page(
    tx: &mut Transaction<'_, Sqlite>,
    publication: &str,
    page: &LayoutPage,
) -> Result<()> {
    let now = crate::domain::timestamp();
    sqlx::query("UPDATE publication_pages SET width_mm=?,height_mm=?,margin_mm=?,bleed_mm=?,layout_json=?,updated_at=? WHERE id=? AND publication_id=?")
        .bind(page.width_mm).bind(page.height_mm).bind(page.margin_mm).bind(page.bleed_mm).bind(&page.layout_json).bind(&now).bind(&page.id).bind(publication).execute(&mut **tx).await?;
    let frames: Vec<_> = page.frames.iter().map(|f| &f.id).collect();
    let texts: Vec<_> = page.texts.iter().map(|t| &t.id).collect();
    sqlx::query("DELETE FROM publication_frames WHERE page_id=? AND id NOT IN(SELECT value FROM json_each(?))").bind(&page.id).bind(encode(&frames)?).execute(&mut **tx).await?;
    sqlx::query("DELETE FROM publication_texts WHERE page_id=? AND id NOT IN(SELECT value FROM json_each(?))").bind(&page.id).bind(encode(&texts)?).execute(&mut **tx).await?;
    for f in &page.frames {
        let count=sqlx::query("UPDATE publication_frames SET x_mm=?,y_mm=?,width_mm=?,height_mm=?,fit_mode=?,updated_at=? WHERE id=? AND page_id=?")
            .bind(f.x_mm).bind(f.y_mm).bind(f.width_mm).bind(f.height_mm).bind(&f.fit_mode).bind(&now).bind(&f.id).bind(&page.id).execute(&mut **tx).await?.rows_affected();
        if count == 0 {
            if f.asset_id.is_some() {
                return Err(invalid("照片关联已变化，请重新打开页面"));
            }
            sqlx::query("INSERT INTO publication_frames(id,page_id,x_mm,y_mm,width_mm,height_mm,fit_mode,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?)")
            .bind(&f.id).bind(&page.id).bind(f.x_mm).bind(f.y_mm).bind(f.width_mm).bind(f.height_mm).bind(&f.fit_mode).bind(&now).bind(&now).execute(&mut **tx).await?;
        }
    }
    for t in &page.texts {
        let count=sqlx::query("UPDATE publication_texts SET x_mm=?,y_mm=?,width_mm=?,height_mm=?,content_json=?,style_json=?,updated_at=? WHERE id=? AND page_id=?")
            .bind(t.x_mm).bind(t.y_mm).bind(t.width_mm).bind(t.height_mm).bind(&t.content_json).bind(&t.style_json).bind(&now).bind(&t.id).bind(&page.id).execute(&mut **tx).await?.rows_affected();
        if count == 0 {
            sqlx::query("INSERT INTO publication_texts(id,page_id,x_mm,y_mm,width_mm,height_mm,content_json,style_json,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?)")
            .bind(&t.id).bind(&page.id).bind(t.x_mm).bind(t.y_mm).bind(t.width_mm).bind(t.height_mm).bind(&t.content_json).bind(&t.style_json).bind(&now).bind(&now).execute(&mut **tx).await?;
        }
    }
    Ok(())
}
impl Store {
    pub async fn publication_master_preview(
        &self,
        project: &str,
        publication: &str,
        base: i64,
        data: &ApplyMaster,
    ) -> Result<MasterPreview> {
        let mut tx = self.pool.begin().await?;
        let layout = versions::layout_tx(&mut tx, project, publication).await?;
        let revision: i64 =
            sqlx::query_scalar("SELECT revision FROM publications WHERE id=? AND project_id=?")
                .bind(publication)
                .bind(project)
                .fetch_one(&mut *tx)
                .await?;
        if revision != base {
            return Err(AppError::new(
                "REVISION_CONFLICT",
                "版面已有修改，请关闭后重新打开",
            ));
        }
        super::layers::guard(
            &layout,
            &super::PublicationAction::ApplyMaster(data.clone()),
        )?;
        let (layout, overridden_page_ids) = plan(&layout, data)?;
        let previews = crate::publication_previews::project(&mut tx, &layout).await?;
        tx.commit().await?;
        Ok(MasterPreview {
            revision,
            layout,
            previews,
            overridden_page_ids,
        })
    }
}
