//! 页面排版辅助元数据；不改几何、像素、裁切或其他页面的参考线。
use super::{invalid, versions};
use crate::domain::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Sqlite, Transaction};
const KEY: &str = "focalroomPageGuidesV1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PageGuides {
    pub columns: i64,
    pub rows: i64,
    pub column_gap_mm: f64,
    pub row_gap_mm: f64,
    pub vertical_guides_mm: Vec<f64>,
    pub horizontal_guides_mm: Vec<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetPageGuides {
    pub page_id: String,
    pub settings: PageGuides,
}
fn metadata(page: &versions::LayoutPage) -> Result<Value> {
    let raw = page.layout_json.as_deref().unwrap_or("{}");
    if raw.len() > versions::MAX_SNAPSHOT {
        return Err(invalid("页面设置过大，请减少参考线后重试"));
    }
    let value: Value =
        serde_json::from_str(raw).map_err(|_| invalid("页面设置无法读取，请恢复一个有效版本"))?;
    if !value.is_object() {
        return Err(invalid("页面设置无法读取，请恢复一个有效版本"));
    }
    Ok(value)
}
pub fn validate(page: &versions::LayoutPage, settings: &PageGuides) -> Result<()> {
    let inner_width = page.width_mm - 2. * page.margin_mm;
    let inner_height = page.height_mm - 2. * page.margin_mm;
    if !(1..=24).contains(&settings.columns)
        || !(1..=24).contains(&settings.rows)
        || ![settings.column_gap_mm, settings.row_gap_mm]
            .iter()
            .all(|n| n.is_finite() && (0. ..=100.).contains(n))
        || inner_width - (settings.columns - 1) as f64 * settings.column_gap_mm <= 0.
        || inner_height - (settings.rows - 1) as f64 * settings.row_gap_mm <= 0.
    {
        return Err(invalid("网格间距超出版心，请减少行列或间距"));
    }
    for (lines, length) in [
        (&settings.vertical_guides_mm, page.width_mm),
        (&settings.horizontal_guides_mm, page.height_mm),
    ] {
        if lines.len() > 32
            || lines
                .iter()
                .any(|n| !n.is_finite() || *n < 0. || *n > length)
            || lines
                .iter()
                .enumerate()
                .any(|(i, n)| lines[..i].contains(n))
        {
            return Err(invalid("请检查参考线位置：须在页面范围内，且不能重复"));
        }
    }
    Ok(())
}
pub fn read(page: &versions::LayoutPage) -> Result<Option<PageGuides>> {
    let value = metadata(page)?;
    if value.get(KEY).is_some()
        && page
            .layout_json
            .as_ref()
            .is_some_and(|raw| raw.len() > 32768)
    {
        return Err(invalid("页面参考线设置过大，请恢复一个有效版本"));
    }
    let Some(raw) = value.get(KEY) else {
        return Ok(None);
    };
    let settings: PageGuides = serde_json::from_value(raw.clone())
        .map_err(|_| invalid("页面参考线无法读取，请恢复一个有效版本"))?;
    validate(page, &settings)?;
    Ok(Some(settings))
}
pub fn apply_to_page(page: &mut versions::LayoutPage, settings: &PageGuides) -> Result<()> {
    validate(page, settings)?;
    let mut meta = metadata(page)?;
    meta[KEY] = serde_json::to_value(settings).map_err(|_| invalid("页面参考线无法保存"))?;
    let raw = meta.to_string();
    if raw.len() > 32768 {
        return Err(invalid(
            "这页已有较多设置，暂时无法添加网格或参考线。原版面已保留。",
        ));
    }
    page.layout_json = Some(raw);
    Ok(())
}
pub async fn set(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    publication: &str,
    data: &SetPageGuides,
) -> Result<()> {
    let layout = versions::layout_tx(tx, project, publication).await?;
    let mut page = layout
        .pages
        .into_iter()
        .find(|p| p.id == data.page_id)
        .ok_or_else(|| invalid("这页已变化，请重新选择"))?;
    apply_to_page(&mut page, &data.settings)?;
    sqlx::query(
        "UPDATE publication_pages SET layout_json=?,updated_at=? WHERE id=? AND publication_id=?",
    )
    .bind(page.layout_json)
    .bind(crate::domain::timestamp())
    .bind(&data.page_id)
    .bind(publication)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
