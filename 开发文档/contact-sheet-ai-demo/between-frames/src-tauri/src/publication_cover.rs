//! 封底、书脊和封面组成一个真实展开页，沿用原对象、来源和版面历史。
use super::{chapters, invalid, layers, versions};
use crate::{
    command_audit::{encode, serialization},
    domain::{self, AppError, Result},
};
use serde::{Deserialize, Deserializer, Serialize};
use sqlx::{Sqlite, Transaction};
pub const KEY: &str = "focalroomCoverV1";
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Cover {
    pub panel_width_mm: f64,
    pub spine_width_mm: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetCover {
    pub page_id: String,
    #[serde(deserialize_with = "nullable_id")]
    pub back_page_id: Option<String>,
    pub settings: Cover,
    pub height_mm: f64,
    pub margin_mm: f64,
    pub bleed_mm: f64,
    pub content_placement: String,
    pub clear_page_settings_confirmed: bool,
}
fn nullable_id<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<Option<String>, D::Error> {
    Option::<String>::deserialize(d)
}
fn valid(value: &Cover) -> bool {
    value.panel_width_mm.is_finite()
        && (50.0..=1000.0).contains(&value.panel_width_mm)
        && value.spine_width_mm.is_finite()
        && (0.0..=200.0).contains(&value.spine_width_mm)
        && value.panel_width_mm * 2.0 + value.spine_width_mm <= 2000.0
}
pub fn read(page: &versions::LayoutPage) -> Result<Option<Cover>> {
    let raw = page.layout_json.as_deref().unwrap_or("{}");
    let metadata: serde_json::Value =
        serde_json::from_str(raw).map_err(|_| invalid("封面设置无法读取，请恢复之前的版本"))?;
    if !metadata.is_object() {
        return Err(invalid("封面设置无法读取，请恢复之前的版本"));
    }
    let Some(value) = metadata.get(KEY) else {
        return Ok(None);
    };
    let cover: Cover = serde_json::from_value(value.clone())
        .map_err(|_| invalid("封面设置无法读取，请恢复之前的版本"))?;
    if !valid(&cover)
        || page.page_number != 0
        || page.side != "cover"
        || (page.width_mm - cover.panel_width_mm * 2.0 - cover.spine_width_mm).abs() > 1e-6
    {
        return Err(invalid(
            "封面尺寸已变化，请通过封面设置调整或恢复之前的版本",
        ));
    }
    Ok(Some(cover))
}
pub fn validate(layout: &versions::PublicationLayout) -> Result<()> {
    let count = layout
        .pages
        .iter()
        .try_fold(0, |n, p| read(p).map(|c| n + usize::from(c.is_some())))?;
    if count > 0
        && (layout.publication.pub_type != "photobook"
            || count != 1
            || layout.pages.iter().any(|p| p.page_number == -1))
    {
        return Err(invalid("展开封面需要使用摄影书，且不能保留另一张独立封底"));
    }
    Ok(())
}
pub fn plan(
    layout: &versions::PublicationLayout,
    data: &SetCover,
) -> Result<versions::PublicationLayout> {
    if !chapters::valid_id(&data.page_id)
        || data
            .back_page_id
            .as_ref()
            .is_some_and(|id| !chapters::valid_id(id))
        || !valid(&data.settings)
        || !data.height_mm.is_finite()
        || !(50.0..=2000.0).contains(&data.height_mm)
        || !data.margin_mm.is_finite()
        || !(0.0..=999.0).contains(&data.margin_mm)
        || !data.bleed_mm.is_finite()
        || !(0.0..=25.0).contains(&data.bleed_mm)
        || !matches!(
            data.content_placement.as_str(),
            "keepPosition" | "followPanels"
        )
        || layout.publication.pub_type != "photobook"
    {
        return Err(invalid("请检查封面尺寸和内容安排"));
    }
    validate(layout)?;
    let front = layout.pages.iter().find(|p| p.page_number == 0);
    let back = layout.pages.iter().find(|p| p.page_number == -1);
    if front.is_some_and(|p| p.id != data.page_id || p.side != "cover")
        || back.map(|p| &p.id) != data.back_page_id.as_ref()
        || back.is_some_and(|p| p.side != "back")
        || front.is_none()
            && layout.pages.iter().any(|p| {
                p.id == data.page_id
                    || p.frames.iter().any(|f| f.id == data.page_id)
                    || p.texts.iter().any(|t| t.id == data.page_id)
            })
    {
        return Err(invalid("封面或封底已变化，请重新打开封面设置"));
    }
    let old = front.map(read).transpose()?.flatten();
    if old.is_none() && data.content_placement != "followPanels" {
        return Err(invalid("首次展开请按封底和封面的位置保留内容"));
    }
    if old.is_none()
        && [front, back]
            .iter()
            .flatten()
            .any(|p| p.layout_json.is_some())
        && !data.clear_page_settings_confirmed
    {
        return Err(AppError::new(
            "CONFLICT",
            "请先确认替换封面和封底原有的整页设置，照片与文字将保留",
        ));
    }
    let affected: Vec<_> = [front, back]
        .iter()
        .flatten()
        .map(|p| p.id.clone())
        .collect();
    layers::guard_pages(layout, &layers::read(layout)?, &affected)?;
    let width = data.settings.panel_width_mm;
    let spine = data.settings.spine_width_mm;
    let total = width * 2.0 + spine;
    let offset = width + spine;
    let delta = |x: f64, w: f64| -> Result<f64> {
        let Some(old) = &old else {
            return Ok(offset);
        };
        if data.content_placement == "keepPosition"
            || old.panel_width_mm == width && old.spine_width_mm == spine
        {
            return Ok(0.0);
        }
        if x >= -front.unwrap().bleed_mm && x + w <= old.panel_width_mm {
            return Ok(0.0);
        }
        if x >= old.panel_width_mm && x + w <= old.panel_width_mm + old.spine_width_mm {
            return Ok(width - old.panel_width_mm);
        }
        if x >= old.panel_width_mm + old.spine_width_mm
            && x + w <= front.unwrap().width_mm + front.unwrap().bleed_mm
        {
            return Ok(offset - old.panel_width_mm - old.spine_width_mm);
        }
        Err(AppError::new(
            "CONFLICT",
            "部分内容跨越封面分界，请选择保留绝对位置，或先调整这些内容",
        ))
    };
    let mut page = front.cloned().unwrap_or(versions::LayoutPage {
        id: data.page_id.clone(),
        page_number: 0,
        side: "cover".into(),
        width_mm: total,
        height_mm: data.height_mm,
        margin_mm: data.margin_mm,
        bleed_mm: data.bleed_mm,
        layout_json: None,
        frames: vec![],
        texts: vec![],
    });
    for f in &mut page.frames {
        f.x_mm += delta(f.x_mm, f.width_mm)?;
    }
    for t in &mut page.texts {
        t.x_mm += delta(t.x_mm, t.width_mm)?;
    }
    if let Some(back) = back {
        page.frames.extend(back.frames.clone());
        page.texts.extend(back.texts.clone());
    }
    page.frames.sort_by(|a, b| a.id.cmp(&b.id));
    page.texts.sort_by(|a, b| a.id.cmp(&b.id));
    page.width_mm = total;
    page.height_mm = data.height_mm;
    page.margin_mm = data.margin_mm;
    page.bleed_mm = data.bleed_mm;
    let mut metadata = if old.is_some() && !data.clear_page_settings_confirmed {
        serde_json::from_str(front.unwrap().layout_json.as_deref().unwrap_or("{}"))
            .map_err(serialization)?
    } else {
        serde_json::json!({})
    };
    metadata[KEY] = serde_json::to_value(&data.settings).map_err(serialization)?;
    page.layout_json = Some(encode(&metadata)?);
    let mut next = layout.clone();
    next.pages
        .retain(|p| front.is_none_or(|f| p.id != f.id) && back.is_none_or(|b| p.id != b.id));
    next.pages.insert(0, page);
    layers::reconcile_layout(&mut next)?;
    // 保存默认图层，确保合并前后的对象顺序与分组仍由同一套权威数据管理。
    if !layers::has(&next.publication.spec_json)? {
        let defaults = layers::read(&next)?;
        layers::put(&mut next, &defaults)?;
    }
    versions::validate_layout(&next)?;
    if &next == layout {
        return Err(invalid("封面设置没有变化"));
    }
    Ok(next)
}
pub(super) async fn apply(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    publication: &str,
    data: &SetCover,
) -> Result<()> {
    let before = versions::layout_tx(tx, project, publication).await?;
    let next = plan(&before, data)?;
    let page = &next.pages[0];
    let now = domain::timestamp();
    if before.pages.iter().any(|p| p.id == page.id) {
        sqlx::query("UPDATE publication_pages SET width_mm=?,height_mm=?,margin_mm=?,bleed_mm=?,layout_json=? WHERE id=? AND publication_id=?").bind(page.width_mm).bind(page.height_mm).bind(page.margin_mm).bind(page.bleed_mm).bind(&page.layout_json).bind(&page.id).bind(publication).execute(&mut **tx).await?;
    } else {
        sqlx::query("INSERT INTO publication_pages(id,publication_id,page_number,side,width_mm,height_mm,margin_mm,bleed_mm,layout_json,created_at,updated_at) VALUES(?,?,0,'cover',?,?,?,?,?,?,?)").bind(&page.id).bind(publication).bind(page.width_mm).bind(page.height_mm).bind(page.margin_mm).bind(page.bleed_mm).bind(&page.layout_json).bind(&now).bind(&now).execute(&mut **tx).await?;
    }
    for frame in &page.frames {
        sqlx::query("UPDATE publication_frames SET page_id=?,x_mm=? WHERE id=?")
            .bind(&page.id)
            .bind(frame.x_mm)
            .bind(&frame.id)
            .execute(&mut **tx)
            .await?;
    }
    for text in &page.texts {
        sqlx::query("UPDATE publication_texts SET page_id=?,x_mm=? WHERE id=?")
            .bind(&page.id)
            .bind(text.x_mm)
            .bind(&text.id)
            .execute(&mut **tx)
            .await?;
    }
    if let Some(back) = &data.back_page_id {
        sqlx::query("DELETE FROM publication_pages WHERE id=? AND publication_id=?")
            .bind(back)
            .bind(publication)
            .execute(&mut **tx)
            .await?;
    }
    sqlx::query("UPDATE publications SET spec_json=? WHERE id=? AND project_id=?")
        .bind(&next.publication.spec_json)
        .bind(publication)
        .bind(project)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
