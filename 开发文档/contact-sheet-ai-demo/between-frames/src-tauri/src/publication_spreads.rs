//! 连续跨页照片：仅左页拥有正式照片框，右页显示同一对象的投影。
use super::{FrameUpdate, invalid, versions};
use crate::{
    command_audit::serialization,
    domain::{self, AppError, Result},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{QueryBuilder, Sqlite, Transaction};
use std::collections::BTreeSet;
use versions::{LayoutFrame, LayoutPage, PublicationLayout};

pub const KEY: &str = "focalroomContinuousFramesV1";
const MAX_METADATA: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Link {
    pub frame_id: String,
    pub right_page_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrameGeometry {
    pub x_mm: f64,
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
    pub fit_mode: String,
    pub crop_x: f64,
    pub crop_y: f64,
    pub crop_zoom: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetSpreadFrame {
    pub frame_id: String,
    pub left_page_id: String,
    pub right_page_id: String,
    pub geometry: FrameGeometry,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnsetSpreadFrame {
    pub frame_id: String,
    pub target_page_id: String,
    pub geometry: FrameGeometry,
}

fn metadata(raw: Option<&str>) -> Result<Value> {
    let Some(raw) = raw else {
        return Ok(serde_json::json!({}));
    };
    if raw.len() > versions::MAX_SNAPSHOT {
        return Err(invalid("页面设置过大，请拆分为多个页面"));
    }
    let value: Value = serde_json::from_str(raw).map_err(serialization)?;
    if !value.is_object() {
        return Err(invalid("这页的设置不完整，请恢复之前的版面版本"));
    }
    Ok(value)
}
pub fn links_raw(raw: Option<&str>) -> Result<Vec<Link>> {
    let value = metadata(raw)?;
    if value.get(KEY).is_some() && raw.is_some_and(|raw| raw.len() > MAX_METADATA) {
        return Err(invalid("跨页照片关联过大，请拆分为多个页面"));
    }
    let links: Vec<Link> = value
        .get(KEY)
        .map(|value| serde_json::from_value(value.clone()).map_err(serialization))
        .transpose()?
        .unwrap_or_default();
    let mut seen = BTreeSet::new();
    if links.len() > 100
        || links.iter().any(|link| {
            uuid::Uuid::parse_str(&link.frame_id).is_err()
                || uuid::Uuid::parse_str(&link.right_page_id).is_err()
                || !seen.insert(&link.frame_id)
        })
    {
        return Err(invalid("跨页照片关联不完整，请恢复之前的版面版本"));
    }
    Ok(links)
}
pub fn links(page: &LayoutPage) -> Result<Vec<Link>> {
    links_raw(page.layout_json.as_deref())
}
pub fn owned(page: &LayoutPage, frame: &str) -> Result<bool> {
    Ok(links(page)?.iter().any(|link| link.frame_id == frame))
}

pub fn pair<'a>(
    layout: &'a PublicationLayout,
    left: &str,
    right: &str,
) -> Result<(&'a LayoutPage, &'a LayoutPage)> {
    let index = layout
        .pages
        .iter()
        .position(|page| page.id == left)
        .ok_or_else(|| invalid("找不到跨页的左页"))?;
    let l = &layout.pages[index];
    let r = layout
        .pages
        .get(index + 1)
        .filter(|page| page.id == right)
        .ok_or_else(|| invalid("跨页照片的左右页需要保持相邻"))?;
    if layout.publication.pub_type != "photobook"
        || l.side != "left"
        || r.side != "right"
        || l.page_number < 2
        || l.page_number % 2 != 0
        || r.page_number != l.page_number + 1
    {
        return Err(invalid("请选择摄影书中相邻的左页和右页"));
    }
    if l.width_mm != r.width_mm || l.height_mm != r.height_mm || l.bleed_mm != r.bleed_mm {
        return Err(invalid("连续跨页照片需要左右页的尺寸和出血一致"));
    }
    Ok((l, r))
}
pub fn bounds(left: &LayoutPage, right: &LayoutPage) -> LayoutPage {
    LayoutPage {
        id: left.id.clone(),
        page_number: left.page_number,
        side: "spread".into(),
        width_mm: left.width_mm + right.width_mm,
        height_mm: left.height_mm,
        margin_mm: left.margin_mm.max(right.margin_mm),
        bleed_mm: left.bleed_mm,
        layout_json: None,
        frames: Vec::new(),
        texts: Vec::new(),
    }
}
pub fn frame_pair<'a>(
    layout: &'a PublicationLayout,
    frame: &str,
) -> Result<Option<(&'a LayoutPage, &'a LayoutPage)>> {
    for page in &layout.pages {
        if let Some(link) = links(page)?.iter().find(|link| link.frame_id == frame) {
            return pair(layout, &page.id, &link.right_page_id).map(Some);
        }
    }
    Ok(None)
}
pub fn visible<'a>(
    layout: &'a PublicationLayout,
    page: &LayoutPage,
) -> Result<Vec<&'a LayoutFrame>> {
    let own = layout
        .pages
        .iter()
        .find(|p| p.id == page.id)
        .ok_or_else(|| invalid("页面已不存在"))?;
    let mut result: Vec<_> = own.frames.iter().collect();
    for owner in &layout.pages {
        for link in links(owner)? {
            if link.right_page_id == page.id {
                let frame = owner
                    .frames
                    .iter()
                    .find(|frame| frame.id == link.frame_id)
                    .ok_or_else(|| invalid("跨页照片已不存在"))?;
                result.push(frame);
            }
        }
    }
    Ok(result)
}
pub fn validate(layout: &PublicationLayout) -> Result<BTreeSet<String>> {
    let mut ids = BTreeSet::new();
    for left in &layout.pages {
        for link in links(left)? {
            if !matches!(layout.schema_version, 2 | 3) {
                return Err(invalid("请更新FocalRoom后再打开这份跨页版面"));
            }
            let (left, right) = pair(layout, &left.id, &link.right_page_id)?;
            let frame = left
                .frames
                .iter()
                .find(|frame| frame.id == link.frame_id)
                .ok_or_else(|| invalid("跨页照片关联的对象已不存在"))?;
            if !ids.insert(frame.id.clone())
                || frame.asset_id.is_none()
                || frame.source.as_ref().is_none_or(|source| {
                    Some(&source.asset_id) != frame.asset_id.as_ref()
                        || source.rendition_id != frame.rendition_id
                })
            {
                return Err(invalid("跨页照片的来源不完整，请恢复之前的版面版本"));
            }
            versions::rectangle(
                &bounds(left, right),
                frame.x_mm,
                frame.y_mm,
                frame.width_mm,
                frame.height_mm,
            )?;
            if frame.x_mm >= left.width_mm || frame.x_mm + frame.width_mm <= left.width_mm {
                return Err(invalid(
                    "跨页照片需要覆盖装订线；移到单页前请先转为单页照片",
                ));
            }
            for page in [left, right] {
                if visible(layout, page)?.iter().any(|other| {
                    other.id != frame.id
                        && other.asset_id == frame.asset_id
                        && other.rendition_id == frame.rendition_id
                }) {
                    return Err(invalid("这组跨页已有相同照片版本，请先整理已有照片框"));
                }
            }
        }
    }
    Ok(ids)
}

async fn write_links(
    tx: &mut Transaction<'_, Sqlite>,
    publication: &str,
    page: &LayoutPage,
    links: &[Link],
) -> Result<()> {
    if self::links(page)? == links {
        return Ok(());
    }
    let mut value = metadata(page.layout_json.as_deref())?;
    if links.is_empty() {
        value.as_object_mut().unwrap().remove(KEY);
    } else {
        value[KEY] = serde_json::to_value(links).map_err(serialization)?;
    }
    let raw = if value.as_object().unwrap().is_empty() {
        None
    } else {
        let raw = value.to_string();
        if raw.len() > MAX_METADATA {
            return Err(invalid("页面设置过大，请拆分为多个页面"));
        }
        Some(raw)
    };
    sqlx::query(
        "UPDATE publication_pages SET layout_json=?,updated_at=? WHERE id=? AND publication_id=?",
    )
    .bind(raw)
    .bind(domain::timestamp())
    .bind(&page.id)
    .bind(publication)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
pub async fn remove_link(
    tx: &mut Transaction<'_, Sqlite>,
    publication: &str,
    page: &LayoutPage,
    frame: &str,
) -> Result<()> {
    let old = links(page)?;
    let next: Vec<_> = old
        .iter()
        .filter(|link| link.frame_id != frame)
        .cloned()
        .collect();
    if old.len() != next.len() {
        write_links(tx, publication, page, &next).await?;
    }
    Ok(())
}
fn validate_geometry(geometry: &FrameGeometry) -> Result<()> {
    if ![
        geometry.x_mm,
        geometry.y_mm,
        geometry.width_mm,
        geometry.height_mm,
        geometry.crop_x,
        geometry.crop_y,
        geometry.crop_zoom,
    ]
    .iter()
    .all(|value| value.is_finite())
        || geometry.width_mm <= 0.
        || geometry.height_mm <= 0.
        || !matches!(geometry.fit_mode.as_str(), "fit" | "fill" | "stretch")
        || !(0.0..=1.0).contains(&geometry.crop_x)
        || !(0.0..=1.0).contains(&geometry.crop_y)
        || !(1.0..=20.0).contains(&geometry.crop_zoom)
    {
        return Err(invalid("请检查跨页照片的位置、大小和裁切设置"));
    }
    Ok(())
}
fn geometry(old: &LayoutFrame, geometry: &FrameGeometry) -> FrameUpdate {
    FrameUpdate {
        frame_id: old.id.clone(),
        x_mm: geometry.x_mm,
        y_mm: geometry.y_mm,
        width_mm: geometry.width_mm,
        height_mm: geometry.height_mm,
        fit_mode: geometry.fit_mode.clone(),
        crop_x: geometry.crop_x,
        crop_y: geometry.crop_y,
        crop_zoom: geometry.crop_zoom,
        asset_id: old.asset_id.clone(),
        rendition_id: old.rendition_id.clone(),
    }
}
pub async fn set(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    publication: &str,
    data: &SetSpreadFrame,
) -> Result<()> {
    validate_geometry(&data.geometry)?;
    let layout = versions::layout_tx(tx, project, publication).await?;
    let (left, right) = pair(&layout, &data.left_page_id, &data.right_page_id)?;
    let (owner, frame) = layout
        .pages
        .iter()
        .flat_map(|page| page.frames.iter().map(move |frame| (page, frame)))
        .find(|(_, frame)| frame.id == data.frame_id)
        .ok_or_else(|| AppError::new("NOT_FOUND", "照片框已不存在"))?;
    if owner.id != left.id && owner.id != right.id || frame.asset_id.is_none() {
        return Err(invalid("请选择这组跨页中的一张照片"));
    }
    for page in [left, right] {
        ensure_unique_photo(
            tx,
            publication,
            &page.id,
            frame.asset_id.as_deref(),
            frame.rendition_id.as_deref(),
            Some(&frame.id),
        )
        .await?;
    }
    for page in &layout.pages {
        let old = links(page)?;
        let retained: Vec<_> = old
            .into_iter()
            .filter(|link| link.frame_id != frame.id)
            .collect();
        if page.id == left.id {
            let mut next = retained;
            next.push(Link {
                frame_id: frame.id.clone(),
                right_page_id: right.id.clone(),
            });
            write_links(tx, publication, page, &next).await?;
        } else {
            write_links(tx, publication, page, &retained).await?;
        }
    }
    sqlx::query("UPDATE publication_frames SET page_id=? WHERE id=?")
        .bind(&left.id)
        .bind(&frame.id)
        .execute(&mut **tx)
        .await?;
    super::update_frame(tx, project, publication, &geometry(frame, &data.geometry)).await?;
    Ok(())
}
pub async fn unset(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    publication: &str,
    data: &UnsetSpreadFrame,
) -> Result<()> {
    validate_geometry(&data.geometry)?;
    let layout = versions::layout_tx(tx, project, publication).await?;
    let (left, right) =
        frame_pair(&layout, &data.frame_id)?.ok_or_else(|| invalid("这张照片不是连续跨页照片"))?;
    if data.target_page_id != left.id && data.target_page_id != right.id {
        return Err(invalid("请选择这组跨页中的左页或右页"));
    }
    let frame = left
        .frames
        .iter()
        .find(|frame| frame.id == data.frame_id)
        .unwrap();
    ensure_unique_photo(
        tx,
        publication,
        &data.target_page_id,
        frame.asset_id.as_deref(),
        frame.rendition_id.as_deref(),
        Some(&frame.id),
    )
    .await?;
    remove_link(tx, publication, left, &frame.id).await?;
    sqlx::query("UPDATE publication_frames SET page_id=? WHERE id=?")
        .bind(&data.target_page_id)
        .bind(&frame.id)
        .execute(&mut **tx)
        .await?;
    super::update_frame(tx, project, publication, &geometry(frame, &data.geometry)).await?;
    Ok(())
}

/// 同页约束包含右页投影，且允许排除正在修改的同一正式对象。
pub async fn ensure_unique_photo(
    tx: &mut Transaction<'_, Sqlite>,
    publication: &str,
    page: &str,
    asset: Option<&str>,
    rendition: Option<&str>,
    exclude: Option<&str>,
) -> Result<()> {
    let Some(asset) = asset else {
        return Ok(());
    };
    let owners: Vec<(String, Option<String>)> = sqlx::query_as("SELECT id,layout_json FROM publication_pages WHERE publication_id=? AND layout_json IS NOT NULL").bind(publication).fetch_all(&mut **tx).await?;
    let mut projected = Vec::new();
    for (_, raw) in owners {
        projected.extend(
            links_raw(raw.as_deref())?
                .into_iter()
                .filter(|link| link.right_page_id == page)
                .map(|link| link.frame_id),
        );
    }
    let mut query = QueryBuilder::<Sqlite>::new(
        "SELECT EXISTS(SELECT 1 FROM publication_frames f JOIN publication_pages p ON p.id=f.page_id WHERE p.publication_id=",
    );
    query
        .push_bind(publication)
        .push(" AND f.asset_id=")
        .push_bind(asset)
        .push(" AND f.rendition_id IS ")
        .push_bind(rendition)
        .push(" AND (")
        .push_bind(exclude)
        .push(" IS NULL OR f.id<>")
        .push_bind(exclude)
        .push(") AND (p.id=")
        .push_bind(page);
    if !projected.is_empty() {
        query.push(" OR f.id IN (");
        let mut separated = query.separated(",");
        for id in &projected {
            separated.push_bind(id);
        }
        separated.push_unseparated(")");
    }
    query.push("))");
    if query
        .build_query_scalar::<bool>()
        .fetch_one(&mut **tx)
        .await?
    {
        return Err(AppError::new(
            "CONFLICT",
            "本页已使用这张照片的相同版本，请选择其他照片或版本",
        ));
    }
    Ok(())
}
