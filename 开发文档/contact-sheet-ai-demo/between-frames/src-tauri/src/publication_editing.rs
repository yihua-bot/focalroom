//! 页面顺序与文字编辑，复用出版人工命令事务与不可变版面历史。
use super::{PublicationAction, invalid, spreads, versions};
use crate::{
    command_audit::encode,
    domain::{self, AppError, Result},
};
use serde::{Deserialize, Serialize};
use sqlx::{Sqlite, Transaction};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AddPage {
    pub page_id: String,
    pub after_page_id: Option<String>,
    pub width_mm: f64,
    pub height_mm: f64,
    pub margin_mm: f64,
    pub bleed_mm: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PageOrder {
    pub page_ids: Vec<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::chapters::present"
    )]
    pub chapter_changes_confirmed: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PageReference {
    pub page_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextReference {
    pub text_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PhotoFrameAdd {
    pub frame_id: String,
    pub page_id: String,
    pub photo_id: String,
    pub rendition_id: Option<String>,
    pub x_mm: f64,
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrameReference {
    pub frame_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FramePhoto {
    pub frame_id: String,
    pub photo_id: String,
    pub rendition_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextUpdate {
    pub text_id: String,
    pub page_id: String,
    pub x_mm: f64,
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
    pub kind: String,
    pub text: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::chapters::present"
    )]
    pub text_direction: Option<super::typography::Direction>,
    pub font_family: String,
    pub font_size_pt: f64,
    pub line_height: f64,
    pub align: String,
    pub bold: bool,
    pub color: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextGeometry {
    pub text_id: String,
    pub page_id: String,
    pub x_mm: f64,
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
}
fn uuid(id: &str) -> Result<()> {
    uuid::Uuid::parse_str(id).map_err(|_| invalid("这项内容无效，请重新打开页面"))?;
    Ok(())
}
async fn page(tx: &mut Transaction<'_, Sqlite>, publication: &str, id: &str) -> Result<()> {
    uuid(id)?;
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM publication_pages WHERE id=? AND publication_id=?)",
    )
    .bind(id)
    .bind(publication)
    .fetch_one(&mut **tx)
    .await?;
    if !exists {
        return Err(AppError::new("NOT_FOUND", "页面不属于这本出版物"));
    }
    Ok(())
}
async fn fresh(tx: &mut Transaction<'_, Sqlite>, id: &str) -> Result<()> {
    uuid(id)?;
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT id FROM publication_pages WHERE id=? UNION ALL SELECT id FROM publication_texts WHERE id=? UNION ALL SELECT id FROM publication_frames WHERE id=?)")
        .bind(id).bind(id).bind(id).fetch_one(&mut **tx).await?;
    if exists {
        return Err(AppError::new("CONFLICT", "这项内容已存在，请刷新后重试"));
    }
    Ok(())
}
async fn ordered(
    tx: &mut Transaction<'_, Sqlite>,
    publication: &str,
) -> Result<Vec<(String, i64, String)>> {
    Ok(sqlx::query_as("SELECT id,page_number,side FROM publication_pages WHERE publication_id=? ORDER BY CASE WHEN page_number=-1 THEN 10001 ELSE page_number END,id")
        .bind(publication).fetch_all(&mut **tx).await?)
}
pub(super) fn reordered_rows(
    rows: &[(String, i64, String)],
    ids: &[String],
) -> Result<Vec<(String, i64, String)>> {
    let wanted: BTreeSet<_> = ids.iter().collect();
    if ids.len() != rows.len()
        || wanted.len() != ids.len()
        || rows.iter().any(|r| !wanted.contains(&r.0))
    {
        return Err(invalid("页面顺序与当前出版物不一致，请刷新后重试"));
    }
    let cover = rows.iter().find(|r| r.1 == 0);
    let back = rows.iter().find(|r| r.1 == -1);
    if cover.is_some_and(|c| ids.first() != Some(&c.0))
        || back.is_some_and(|b| ids.last() != Some(&b.0))
    {
        return Err(invalid("封面和封底需要保留在书的两端"));
    }
    let mut number = 1_i64;
    let mut result = Vec::new();
    for id in ids {
        let old = rows.iter().find(|r| &r.0 == id).unwrap();
        let (n, side) = if cover.is_some_and(|c| &c.0 == id) {
            (0, "cover")
        } else if back.is_some_and(|b| &b.0 == id) {
            (-1, "back")
        } else {
            let n = number;
            number += 1;
            (
                n,
                if old.2 == "spread" {
                    "spread"
                } else if n % 2 == 0 {
                    "left"
                } else {
                    "right"
                },
            )
        };
        result.push((id.clone(), n, side.to_owned()));
    }
    Ok(result)
}
pub(super) async fn reorder(
    tx: &mut Transaction<'_, Sqlite>,
    publication: &str,
    ids: &[String],
) -> Result<()> {
    let rows = ordered(tx, publication).await?;
    let rows = reordered_rows(&rows, ids)?;
    // 先腾出唯一页码，避免交换两页时发生中间冲突；最终页码在同一事务内恢复。
    sqlx::query(
        "UPDATE publication_pages SET page_number=page_number+20000 WHERE publication_id=?",
    )
    .bind(publication)
    .execute(&mut **tx)
    .await?;
    for (id, n, side) in rows {
        sqlx::query("UPDATE publication_pages SET page_number=?,side=?,updated_at=? WHERE id=? AND publication_id=?")
            .bind(n).bind(side).bind(domain::timestamp()).bind(id).bind(publication).execute(&mut **tx).await?;
    }
    Ok(())
}
pub(super) async fn apply(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    publication: &str,
    action: &PublicationAction,
) -> Result<()> {
    match action {
        PublicationAction::AddFrame(data) => {
            fresh(tx, &data.frame_id).await?;
            page(tx, publication, &data.page_id).await?;
            let asset =
                photo_asset(tx, project, &data.photo_id, data.rendition_id.as_deref()).await?;
            spreads::ensure_unique_photo(
                tx,
                publication,
                &data.page_id,
                Some(&asset),
                data.rendition_id.as_deref(),
                None,
            )
            .await?;
            let now = domain::timestamp();
            sqlx::query("INSERT INTO publication_frames(id,page_id,x_mm,y_mm,width_mm,height_mm,fit_mode,asset_id,rendition_id,crop_x,crop_y,crop_zoom,created_at,updated_at) VALUES(?,?,?,?,?,?,'fit',?,?,0.5,0.5,1,?,?)")
                .bind(&data.frame_id).bind(&data.page_id).bind(data.x_mm).bind(data.y_mm).bind(data.width_mm).bind(data.height_mm).bind(asset).bind(&data.rendition_id).bind(&now).bind(&now).execute(&mut **tx).await?;
        }
        PublicationAction::SetFramePhoto(data) => {
            let target = frame_page(tx, publication, &data.frame_id).await?;
            let asset =
                photo_asset(tx, project, &data.photo_id, data.rendition_id.as_deref()).await?;
            spreads::ensure_unique_photo(
                tx,
                publication,
                &target,
                Some(&asset),
                data.rendition_id.as_deref(),
                Some(&data.frame_id),
            )
            .await?;
            let raw: Option<String> = sqlx::query_scalar(
                "SELECT layout_json FROM publication_pages WHERE id=? AND publication_id=?",
            )
            .bind(&target)
            .bind(publication)
            .fetch_one(&mut **tx)
            .await?;
            if let Some(link) = spreads::links_raw(raw.as_deref())?
                .iter()
                .find(|link| link.frame_id == data.frame_id)
            {
                spreads::ensure_unique_photo(
                    tx,
                    publication,
                    &link.right_page_id,
                    Some(&asset),
                    data.rendition_id.as_deref(),
                    Some(&data.frame_id),
                )
                .await?;
            }
            sqlx::query("UPDATE publication_frames SET asset_id=?,rendition_id=?,crop_x=0.5,crop_y=0.5,crop_zoom=1,updated_at=? WHERE id=?")
                .bind(asset).bind(&data.rendition_id).bind(domain::timestamp()).bind(&data.frame_id).execute(&mut **tx).await?;
        }
        PublicationAction::RemoveFrame(data) => {
            frame_page(tx, publication, &data.frame_id).await?;
            let owner: versions::LayoutPage = sqlx::query_as("SELECT p.id,p.page_number,p.side,p.width_mm,p.height_mm,p.margin_mm,p.bleed_mm,p.layout_json FROM publication_pages p JOIN publication_frames f ON f.page_id=p.id WHERE f.id=? AND p.publication_id=?").bind(&data.frame_id).bind(publication).fetch_one(&mut **tx).await?;
            spreads::remove_link(tx, publication, &owner, &data.frame_id).await?;
            sqlx::query("DELETE FROM publication_frames WHERE id=? AND page_id IN (SELECT id FROM publication_pages WHERE publication_id=?)")
                .bind(&data.frame_id).bind(publication).execute(&mut **tx).await?;
        }
        PublicationAction::AddPage(data) => {
            fresh(tx, &data.page_id).await?;
            let rows = ordered(tx, publication).await?;
            if rows.len() >= 512 {
                return Err(invalid("这本出版物已达到512页，请另建分册"));
            }
            let index = if let Some(after) = &data.after_page_id {
                let index = rows
                    .iter()
                    .position(|r| &r.0 == after)
                    .ok_or_else(|| invalid("插入位置已变化，请重新选择页面"))?;
                if rows[index].1 == -1 {
                    return Err(invalid("请在封底之前插入页面"));
                }
                index + 1
            } else if rows.is_empty() {
                0
            } else {
                rows.iter().position(|r| r.1 == -1).unwrap_or(rows.len())
            };
            let temporary = if rows.is_empty() {
                0
            } else {
                rows.iter().map(|r| r.1).max().unwrap() + 1
            };
            let now = domain::timestamp();
            sqlx::query("INSERT INTO publication_pages(id,publication_id,page_number,side,width_mm,height_mm,margin_mm,bleed_mm,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?)")
                .bind(&data.page_id).bind(publication).bind(temporary).bind(if rows.is_empty(){"cover"}else{"right"})
                .bind(data.width_mm).bind(data.height_mm).bind(data.margin_mm).bind(data.bleed_mm).bind(&now).bind(&now).execute(&mut **tx).await?;
            let mut ids: Vec<_> = rows.into_iter().map(|r| r.0).collect();
            ids.insert(index, data.page_id.clone());
            reorder(tx, publication, &ids).await?;
        }
        PublicationAction::ReorderPages(data) => reorder(tx, publication, &data.page_ids).await?,
        PublicationAction::RemovePage(data) => {
            page(tx, publication, &data.page_id).await?;
            let metadata: Vec<(String,Option<String>)> = sqlx::query_as("SELECT id,layout_json FROM publication_pages WHERE publication_id=? AND layout_json IS NOT NULL").bind(publication).fetch_all(&mut **tx).await?;
            for (owner, raw) in metadata {
                let links = spreads::links_raw(raw.as_deref())?;
                if owner == data.page_id && !links.is_empty()
                    || links.iter().any(|link| link.right_page_id == data.page_id)
                {
                    return Err(AppError::new(
                        "CONFLICT",
                        "请先移除或转为单页照片，再移除关联的跨页页面",
                    ));
                }
            }
            let rows = ordered(tx, publication).await?;
            if rows.iter().any(|r| r.0 == data.page_id && r.1 == 0) {
                return Err(invalid("封面不能移除；可以清空或调整封面内容"));
            }
            sqlx::query("DELETE FROM publication_pages WHERE id=? AND publication_id=?")
                .bind(&data.page_id)
                .bind(publication)
                .execute(&mut **tx)
                .await?;
            let ids: Vec<_> = rows
                .into_iter()
                .filter(|r| r.0 != data.page_id)
                .map(|r| r.0)
                .collect();
            // 移除后即使相对顺序相同，仍需压紧页码。
            reorder(tx, publication, &ids).await?;
        }
        PublicationAction::AddText(data) | PublicationAction::UpdateText(data) => {
            page(tx, publication, &data.page_id).await?;
            uuid(&data.text_id)?;
            if !matches!(data.kind.as_str(), "text" | "page_number")
                || data.text.chars().count() > 10000
                || !matches!(
                    data.font_family.as_str(),
                    "serif" | "sans-serif" | "monospace"
                )
                || !data.font_size_pt.is_finite()
                || !(4.0..=300.0).contains(&data.font_size_pt)
                || !data.line_height.is_finite()
                || !(1.0..=3.0).contains(&data.line_height)
                || !matches!(data.align.as_str(), "left" | "center" | "right")
                || data.color.len() != 7
                || !data.color.starts_with('#')
                || !data.color[1..].bytes().all(|c| c.is_ascii_hexdigit())
            {
                return Err(invalid("文字或样式无效，请检查后重试"));
            }
            let content = encode(&serde_json::json!({"kind":data.kind,"text":data.text}))?;
            let direction = if let Some(direction) = data.text_direction {
                Some(direction)
            } else if matches!(action, PublicationAction::UpdateText(_)) {
                let previous: Option<Option<String>> = sqlx::query_scalar(
                    "SELECT style_json FROM publication_texts WHERE id=? AND page_id=?",
                )
                .bind(&data.text_id)
                .bind(&data.page_id)
                .fetch_optional(&mut **tx)
                .await?;
                super::typography::stored(previous.flatten().as_deref())?
            } else {
                None
            };
            let mut style = serde_json::json!({"fontFamily":data.font_family,"fontSizePt":data.font_size_pt,"lineHeight":data.line_height,"align":data.align,"bold":data.bold,"color":data.color});
            if let Some(direction) = direction {
                style["textDirection"] =
                    serde_json::to_value(direction).map_err(crate::command_audit::serialization)?;
            }
            let style = encode(&style)?;
            if matches!(action, PublicationAction::AddText(_)) {
                fresh(tx, &data.text_id).await?;
                let now = domain::timestamp();
                sqlx::query("INSERT INTO publication_texts(id,page_id,x_mm,y_mm,width_mm,height_mm,content_json,style_json,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?)")
                    .bind(&data.text_id).bind(&data.page_id).bind(data.x_mm).bind(data.y_mm).bind(data.width_mm).bind(data.height_mm).bind(content).bind(style).bind(&now).bind(&now).execute(&mut **tx).await?;
            } else {
                let count=sqlx::query("UPDATE publication_texts SET x_mm=?,y_mm=?,width_mm=?,height_mm=?,content_json=?,style_json=?,updated_at=? WHERE id=? AND page_id=?")
                    .bind(data.x_mm).bind(data.y_mm).bind(data.width_mm).bind(data.height_mm).bind(content).bind(style).bind(domain::timestamp()).bind(&data.text_id).bind(&data.page_id).execute(&mut **tx).await?.rows_affected();
                if count != 1 {
                    return Err(AppError::new("NOT_FOUND", "文字不属于当前页面"));
                }
            }
        }
        PublicationAction::UpdateTextGeometry(data) => {
            page(tx, publication, &data.page_id).await?;
            uuid(&data.text_id)?;
            if ![data.x_mm, data.y_mm, data.width_mm, data.height_mm]
                .iter()
                .all(|v| v.is_finite())
                || data.width_mm <= 0.
                || data.height_mm <= 0.
            {
                return Err(invalid("文字位置或尺寸无效"));
            }
            let changed=sqlx::query("UPDATE publication_texts SET x_mm=?,y_mm=?,width_mm=?,height_mm=?,updated_at=? WHERE id=? AND page_id=?").bind(data.x_mm).bind(data.y_mm).bind(data.width_mm).bind(data.height_mm).bind(domain::timestamp()).bind(&data.text_id).bind(&data.page_id).execute(&mut **tx).await?.rows_affected();
            if changed != 1 {
                return Err(AppError::new("NOT_FOUND", "文字不属于当前页面"));
            }
        }
        PublicationAction::RemoveText(data) => {
            let count=sqlx::query("DELETE FROM publication_texts WHERE id=? AND page_id IN (SELECT id FROM publication_pages WHERE publication_id=?)")
                .bind(&data.text_id).bind(publication).execute(&mut **tx).await?.rows_affected();
            if count != 1 {
                return Err(AppError::new("NOT_FOUND", "文字不属于这本出版物"));
            }
        }
        _ => return Err(invalid("不支持的页面操作")),
    }
    Ok(())
}
async fn frame_page(
    tx: &mut Transaction<'_, Sqlite>,
    publication: &str,
    id: &str,
) -> Result<String> {
    uuid(id)?;
    sqlx::query_scalar("SELECT f.page_id FROM publication_frames f JOIN publication_pages p ON p.id=f.page_id WHERE f.id=? AND p.publication_id=?")
        .bind(id).bind(publication).fetch_optional(&mut **tx).await?.ok_or_else(||AppError::new("NOT_FOUND","照片框不属于这本出版物"))
}
async fn photo_asset(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    photo: &str,
    rendition: Option<&str>,
) -> Result<String> {
    let asset:String=sqlx::query_scalar("SELECT alias.asset_id FROM photos p JOIN asset_aliases alias ON alias.photo_id=p.id JOIN source_assets a ON a.id=alias.asset_id AND a.project_id=p.project_id WHERE p.id=? AND p.project_id=? AND p.content_sha256=a.content_sha256")
        .bind(photo).bind(project).fetch_optional(&mut **tx).await?.ok_or_else(||AppError::new("ASSET_NOT_FOUND","这张照片不属于当前项目或来源已变化"))?;
    versions::source_tx(tx, project, Some(&asset), rendition).await?;
    Ok(asset)
}
