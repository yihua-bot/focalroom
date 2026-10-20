//! G7 Publishing Studio：出版物 CRUD + 从序列自动生成初稿页面。
use crate::{
    command_handler::CommandHandler,
    db::Store,
    domain::{self, AppError, Result},
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Publication {
    pub id: String,
    pub project_id: String,
    pub pub_type: String,
    pub title: String,
    pub state: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePublicationCommand {
    pub title: String,
    pub pub_type: Option<String>,
    /// 开本（mm）
    pub width_mm: Option<f64>,
    pub height_mm: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateFromSequenceCommand {
    pub publication_id: String,
    /// 每页照片数（1=单图大页 / 2=双图 / 4=四图网格）
    pub photos_per_page: Option<i64>,
}

impl CommandHandler<'_> {
    /// 创建出版物。
    pub async fn create_publication(
        &self,
        project: &str,
        cmd: &CreatePublicationCommand,
    ) -> Result<Publication> {
        let title = cmd.title.trim();
        if title.is_empty() || title.chars().count() > 200 {
            return Err(AppError::new("VALIDATION_ERROR", "标题须为 1–200 字符"));
        }
        let pub_type = cmd.pub_type.as_deref().unwrap_or("photobook");
        if !matches!(pub_type, "photobook" | "exhibition" | "portfolio") {
            return Err(AppError::new(
                "VALIDATION_ERROR",
                "出版物类型须为 photobook/exhibition/portfolio",
            ));
        }
        let mut tx = self.store.pool.begin_with("BEGIN IMMEDIATE").await?;
        crate::agent::promotion::ensure_local_write(&mut tx, project).await?;
        let id = domain::id();
        let now = domain::timestamp();
        let spec = json!({
            "width_mm": cmd.width_mm.unwrap_or(210.0),
            "height_mm": cmd.height_mm.unwrap_or(297.0),
        });
        sqlx::query("INSERT INTO publications(id,project_id,pub_type,title,spec_json,state,created_at,updated_at) VALUES(?,?,?,?,?,'draft',?,?)")
            .bind(&id).bind(project).bind(pub_type).bind(title)
            .bind(spec.to_string()).bind(&now).bind(&now)
            .execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(Publication {
            id,
            project_id: project.to_owned(),
            pub_type: pub_type.to_owned(),
            title: title.to_owned(),
            state: "draft".to_owned(),
            created_at: now,
        })
    }

    /// 从作品序列自动生成初稿页面布局。
    pub async fn generate_from_sequence(
        &self,
        project: &str,
        cmd: &GenerateFromSequenceCommand,
    ) -> Result<i64> {
        let per_page = cmd.photos_per_page.unwrap_or(1).clamp(1, 4);
        let mut tx = self.store.pool.begin_with("BEGIN IMMEDIATE").await?;
        crate::agent::promotion::ensure_local_write(&mut tx, project).await?;
        // 验证出版物
        let pub_exists: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM publications WHERE id=? AND project_id=?")
                .bind(&cmd.publication_id)
                .bind(project)
                .fetch_one(&mut *tx)
                .await?;
        if pub_exists == 0 {
            return Err(AppError::new("NOT_FOUND", "出版物不存在"));
        }
        // 读取作品序列
        let photo_ids: Vec<String> = sqlx::query_scalar(
            "SELECT photo_id FROM sequence_items WHERE project_id=? ORDER BY position",
        )
        .bind(project)
        .fetch_all(&mut *tx)
        .await?;
        if photo_ids.is_empty() {
            return Err(AppError::new(
                "VALIDATION_ERROR",
                "作品序列为空，无法生成页面",
            ));
        }
        // 清除已有页面（重新生成）
        sqlx::query("DELETE FROM publication_frames WHERE page_id IN (SELECT id FROM publication_pages WHERE publication_id=?)")
            .bind(&cmd.publication_id)
            .execute(&mut *tx).await?;
        sqlx::query("DELETE FROM publication_pages WHERE publication_id=?")
            .bind(&cmd.publication_id)
            .execute(&mut *tx)
            .await?;

        // 读取规格
        let spec: String = sqlx::query_scalar("SELECT spec_json FROM publications WHERE id=?")
            .bind(&cmd.publication_id)
            .fetch_one(&mut *tx)
            .await?;
        let spec: serde_json::Value = serde_json::from_str(&spec)
            .map_err(|e| AppError::new("VALIDATION_ERROR", e.to_string()))?;
        let w = spec["width_mm"].as_f64().unwrap_or(210.0);
        let h = spec["height_mm"].as_f64().unwrap_or(297.0);
        let margin = 15.0;
        let now = domain::timestamp();

        // 封面（page 0）
        let cover_id = domain::id();
        sqlx::query("INSERT INTO publication_pages(id,publication_id,page_number,side,width_mm,height_mm,margin_mm,bleed_mm,created_at,updated_at) VALUES(?,?,0,'cover',?,?,?,?,3,?,?)")
            .bind(&cover_id).bind(&cmd.publication_id).bind(w).bind(h).bind(margin).bind(&now).bind(&now)
            .execute(&mut *tx).await?;
        // 封面放第一张
        if let Some(first) = photo_ids.first() {
            let asset: Option<String> = sqlx::query_scalar(
                "SELECT id FROM source_assets WHERE id=? OR id=(SELECT asset_id FROM asset_aliases WHERE photo_id=?)",
            ).bind(first).bind(first).fetch_optional(&mut *tx).await?;
            if let Some(asset_id) = asset {
                sqlx::query("INSERT INTO publication_frames(id,page_id,x_mm,y_mm,width_mm,height_mm,fit_mode,asset_id,created_at,updated_at) VALUES(?,?,?,5,5,?,?,'fill',?,?,?)")
                    .bind(domain::id()).bind(&cover_id).bind(w - margin * 2.0).bind(h - margin * 2.0)
                    .bind(&asset_id).bind(&now).bind(&now)
                    .execute(&mut *tx).await?;
            }
        }

        // 内页
        let mut page_num = 1i64;
        for chunk in photo_ids.chunks(per_page as usize) {
            let page_id = domain::id();
            let side = if page_num % 2 == 0 { "left" } else { "right" };
            sqlx::query("INSERT INTO publication_pages(id,publication_id,page_number,side,width_mm,height_mm,margin_mm,bleed_mm,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,3,?,?)")
                .bind(&page_id).bind(&cmd.publication_id).bind(page_num).bind(side)
                .bind(w).bind(h).bind(margin).bind(&now).bind(&now)
                .execute(&mut *tx).await?;

            // 计算每张图的位置
            let _count = chunk.len() as f64;
            let avail_w = w - margin * 2.0;
            let avail_h = h - margin * 2.0;
            for (i, photo_id) in chunk.iter().enumerate() {
                let asset: Option<String> = sqlx::query_scalar(
                    "SELECT id FROM source_assets WHERE id=? OR id=(SELECT asset_id FROM asset_aliases WHERE photo_id=?)",
                ).bind(photo_id).bind(photo_id).fetch_optional(&mut *tx).await?;
                let Some(asset_id) = asset else { continue };

                let (fx, fy, fw, fh) = match per_page {
                    1 => (margin, margin, avail_w, avail_h), // 全页
                    2 => (
                        margin,
                        margin + (avail_h / 2.0) * i as f64,
                        avail_w,
                        avail_h / 2.0 - 5.0,
                    ), // 上下
                    4 => (
                        margin + (avail_w / 2.0) * (i % 2) as f64,
                        margin + (avail_h / 2.0) * (i / 2) as f64,
                        avail_w / 2.0 - 5.0,
                        avail_h / 2.0 - 5.0,
                    ), // 2x2
                    _ => (margin, margin, avail_w, avail_h),
                };
                sqlx::query("INSERT INTO publication_frames(id,page_id,x_mm,y_mm,width_mm,height_mm,fit_mode,asset_id,created_at,updated_at) VALUES(?,?,?,?,?,?, 'fit',?,?,?)")
                    .bind(domain::id()).bind(&page_id)
                    .bind(fx + margin).bind(fy + margin).bind(fw).bind(fh)
                    .bind(&asset_id).bind(&now).bind(&now)
                    .execute(&mut *tx).await?;
            }
            page_num += 1;
        }

        tx.commit().await?;
        Ok(page_num - 1) // 返回生成的内页数
    }

    /// 列出项目的出版物。
    pub async fn list_publications(&self, project: &str) -> Result<Vec<Publication>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, String, String)>(
            "SELECT id, project_id, pub_type, title, state, created_at FROM publications WHERE project_id=? ORDER BY created_at",
        )
        .bind(project)
        .fetch_all(&self.store.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(id, project_id, pub_type, title, state, created_at)| Publication {
                    id,
                    project_id,
                    pub_type,
                    title,
                    state,
                    created_at,
                },
            )
            .collect())
    }
}

impl Store {
    pub async fn create_publication(
        &self,
        project: &str,
        cmd: &CreatePublicationCommand,
    ) -> Result<Publication> {
        CommandHandler::new(self)
            .create_publication(project, cmd)
            .await
    }
    pub async fn generate_from_sequence(
        &self,
        project: &str,
        cmd: &GenerateFromSequenceCommand,
    ) -> Result<i64> {
        CommandHandler::new(self)
            .generate_from_sequence(project, cmd)
            .await
    }
    pub async fn list_publications(&self, project: &str) -> Result<Vec<Publication>> {
        CommandHandler::new(self).list_publications(project).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn publication_create_generate_and_list() {
        let temp = tempfile::tempdir().unwrap();
        let db = Store::open(temp.path().join("pub-test.sqlite").as_path())
            .await
            .unwrap();
        let project = db.create("pub-test", "出版测试").await.unwrap();

        // 创建
        let pub1 = db
            .create_publication(
                &project.id,
                &CreatePublicationCommand {
                    title: "我的第一本书".to_owned(),
                    pub_type: Some("photobook".to_owned()),
                    width_mm: Some(190.0),
                    height_mm: Some(257.0),
                },
            )
            .await
            .unwrap();
        assert_eq!(pub1.state, "draft");
        assert_eq!(pub1.pub_type, "photobook");

        // 列表
        let list = db.list_publications(&project.id).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].title, "我的第一本书");

        // 空序列生成应报错
        let result = db
            .generate_from_sequence(
                &project.id,
                &GenerateFromSequenceCommand {
                    publication_id: pub1.id.clone(),
                    photos_per_page: Some(1),
                },
            )
            .await;
        assert!(result.is_err());
    }
}
