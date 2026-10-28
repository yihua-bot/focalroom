//! 出版照片版本目录。只返回当前项目、指定照片框的有界缓存预览，不暴露原文件路径。
use crate::{
    db::Store,
    domain::{AppError, Result},
    publication_versions as versions,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionCursor {
    pub created_at: String,
    pub id: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhotoVersion {
    pub rendition_id: Option<String>,
    pub variant_name: Option<String>,
    pub external_software: Option<String>,
    pub created_at: Option<String>,
    pub source: versions::LayoutSource,
    pub thumbnail_path: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhotoVersionsPage {
    pub revision: i64,
    pub frame_id: String,
    pub asset_id: String,
    pub original: PhotoVersion,
    pub current: PhotoVersion,
    pub choices: Vec<PhotoVersion>,
    pub next: Option<VersionCursor>,
}

impl Store {
    pub async fn publication_photo_versions(
        &self,
        project: &str,
        publication: &str,
        frame_id: &str,
        base_revision: i64,
        cursor: Option<VersionCursor>,
    ) -> Result<PhotoVersionsPage> {
        if base_revision < 1
            || uuid::Uuid::parse_str(frame_id).is_err()
            || cursor.as_ref().is_some_and(|c| {
                c.created_at.is_empty()
                    || c.created_at.len() > 80
                    || uuid::Uuid::parse_str(&c.id).is_err()
            })
        {
            return Err(AppError::new("VALIDATION_ERROR", "请重新打开照片版本"));
        }
        let mut tx = self.pool.begin().await?;
        let mut layout = versions::layout_tx(&mut tx, project, publication).await?;
        let revision: i64 =
            sqlx::query_scalar("SELECT revision FROM publications WHERE id=? AND project_id=?")
                .bind(publication)
                .bind(project)
                .fetch_one(&mut *tx)
                .await?;
        if revision != base_revision {
            return Err(AppError::new("CONFLICT", "版面已更新，请重新打开照片版本"));
        }
        let mut page = layout
            .pages
            .iter()
            .find(|p| p.frames.iter().any(|f| f.id == frame_id))
            .cloned()
            .ok_or_else(|| AppError::new("NOT_FOUND", "照片框已不存在"))?;
        let frame = page
            .frames
            .iter()
            .find(|f| f.id == frame_id)
            .cloned()
            .unwrap();
        let asset = frame
            .asset_id
            .clone()
            .ok_or_else(|| AppError::new("VALIDATION_ERROR", "请先为照片框选择照片"))?;
        let alias:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM photos p JOIN asset_aliases x ON x.photo_id=p.id JOIN source_assets a ON a.id=x.asset_id WHERE p.project_id=? AND a.project_id=? AND a.id=? AND p.content_sha256=a.content_sha256)").bind(project).bind(project).bind(&asset).fetch_one(&mut *tx).await?;
        if !alias {
            return Err(AppError::new(
                "ASSET_NOT_FOUND",
                "照片来源已变化，请重新关联",
            ));
        }
        let rows=sqlx::query("SELECT r.id,r.created_at,v.name,v.external_software FROM renditions r JOIN edit_variants v ON v.id=r.variant_id WHERE v.asset_id=? AND (? IS NULL OR r.created_at<? OR (r.created_at=? AND r.id<?)) ORDER BY r.created_at DESC,r.id DESC LIMIT 51")
            .bind(&asset).bind(cursor.as_ref().map(|c|&c.created_at)).bind(cursor.as_ref().map(|c|&c.created_at)).bind(cursor.as_ref().map(|c|&c.created_at)).bind(cursor.as_ref().map(|c|&c.id)).fetch_all(&mut *tx).await?;
        let more = rows.len() > 50;
        let mut descriptors = Vec::new();
        for id in [None, frame.rendition_id.clone()] {
            let meta = if let Some(id) = &id {
                sqlx::query("SELECT v.name,v.external_software,r.created_at FROM renditions r JOIN edit_variants v ON v.id=r.variant_id WHERE r.id=? AND v.asset_id=?").bind(id).bind(&asset).fetch_optional(&mut *tx).await?
            } else {
                None
            };
            descriptors.push((
                id,
                meta.as_ref().map(|r| r.get::<String, _>("name")),
                meta.as_ref()
                    .and_then(|r| r.get::<Option<String>, _>("external_software")),
                meta.as_ref().map(|r| r.get::<String, _>("created_at")),
            ));
        }
        for row in rows.iter().take(50) {
            descriptors.push((
                Some(row.get::<String, _>("id")),
                Some(row.get::<String, _>("name")),
                row.get::<Option<String>, _>("external_software"),
                Some(row.get::<String, _>("created_at")),
            ));
        }
        page.frames.clear();
        page.texts.clear();
        for (index, (id, _, _, _)) in descriptors.iter().enumerate() {
            let mut item = frame.clone();
            item.id = format!("choice-{index}");
            item.rendition_id = id.clone();
            item.source =
                versions::source_tx(&mut tx, project, Some(&asset), id.as_deref()).await?;
            page.frames.push(item);
        }
        layout.pages = vec![page];
        crate::publication_previews::capture(&mut tx, &mut layout).await?;
        let previews = crate::publication_previews::project(&mut tx, &layout).await?;
        let mut choices = Vec::new();
        for ((id, name, software, date), item) in
            descriptors.into_iter().zip(layout.pages.remove(0).frames)
        {
            let thumbnail_path = previews
                .iter()
                .find(|p| p.frame_id == item.id)
                .and_then(|p| p.thumbnail_path.clone());
            choices.push(PhotoVersion {
                rendition_id: id,
                variant_name: name,
                external_software: software,
                created_at: date,
                source: item
                    .source
                    .ok_or_else(|| AppError::new("ASSET_NOT_FOUND", "照片版本已不存在"))?,
                thumbnail_path,
            });
        }
        let original = choices.remove(0);
        let current = choices.remove(0);
        let next = if more {
            let row = &rows[49];
            Some(VersionCursor {
                created_at: row.get("created_at"),
                id: row.get("id"),
            })
        } else {
            None
        };
        tx.commit().await?;
        Ok(PhotoVersionsPage {
            revision,
            frame_id: frame_id.into(),
            asset_id: asset,
            original,
            current,
            choices,
            next,
        })
    }
}
