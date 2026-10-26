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
    pub revision: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicationPage {
    pub id: String,
    pub page_number: i64,
    pub width_mm: f64,
    pub height_mm: f64,
    pub frames: Vec<PublicationFrame>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PublicationFrame {
    pub id: String,
    pub x_mm: f64,
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
    pub fit_mode: String,
    pub thumbnail_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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

fn validate_page_size(width: f64, height: f64) -> Result<()> {
    if !width.is_finite()
        || !height.is_finite()
        || !(50.0..=2000.0).contains(&width)
        || !(50.0..=2000.0).contains(&height)
    {
        return Err(AppError::new(
            "VALIDATION_ERROR",
            "页面宽高须为 50–2000 毫米",
        ));
    }
    Ok(())
}

impl CommandHandler<'_> {
    /// 创建出版物。
    pub(crate) async fn create_publication_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        project: &str,
        cmd: &CreatePublicationCommand,
        id: &str,
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
        let width = cmd.width_mm.unwrap_or(210.0);
        let height = cmd.height_mm.unwrap_or(297.0);
        validate_page_size(width, height)?;
        let now = domain::timestamp();
        let spec = json!({
            "width_mm": width,
            "height_mm": height,
        });
        sqlx::query("INSERT INTO publications(id,project_id,pub_type,title,spec_json,state,created_at,updated_at) VALUES(?,?,?,?,?,'draft',?,?)")
            .bind(id).bind(project).bind(pub_type).bind(title)
            .bind(spec.to_string()).bind(&now).bind(&now)
            .execute(&mut **tx).await?;
        Ok(Publication {
            id: id.to_owned(),
            project_id: project.to_owned(),
            pub_type: pub_type.to_owned(),
            title: title.to_owned(),
            state: "draft".to_owned(),
            created_at: now,
            revision: 1,
        })
    }

    /// 从作品序列自动生成初稿页面布局。
    pub(crate) async fn generate_from_sequence_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        project: &str,
        cmd: &GenerateFromSequenceCommand,
    ) -> Result<i64> {
        let per_page = cmd.photos_per_page.unwrap_or(1);
        if !matches!(per_page, 1 | 2 | 4) {
            return Err(AppError::new(
                "VALIDATION_ERROR",
                "每页照片数须为 1、2 或 4",
            ));
        }
        // 验证出版物
        let state: Option<String> =
            sqlx::query_scalar("SELECT state FROM publications WHERE id=? AND project_id=?")
                .bind(&cmd.publication_id)
                .bind(project)
                .fetch_optional(&mut **tx)
                .await?;
        let Some(state) = state else {
            return Err(AppError::new("NOT_FOUND", "出版物不存在"));
        };
        if state != "draft" {
            return Err(AppError::new("CONFLICT", "只有草稿出版物可以生成页面"));
        }
        let page_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM publication_pages WHERE publication_id=?")
                .bind(&cmd.publication_id)
                .fetch_one(&mut **tx)
                .await?;
        if page_count > 0 {
            return Err(AppError::new(
                "CONFLICT",
                "该出版物已有页面，请新建出版物生成另一版",
            ));
        }
        // 读取作品序列
        let photo_ids: Vec<String> = sqlx::query_scalar(
            "SELECT photo_id FROM sequence_items WHERE project_id=? ORDER BY position",
        )
        .bind(project)
        .fetch_all(&mut **tx)
        .await?;
        if photo_ids.is_empty() {
            return Err(AppError::new(
                "VALIDATION_ERROR",
                "作品序列为空，无法生成页面",
            ));
        }
        // 生成前校验全部引用；不能静默跳过缺失资产，产生空白页。
        let mut asset_ids = Vec::with_capacity(photo_ids.len());
        for photo_id in &photo_ids {
            let asset: Option<String> = sqlx::query_scalar(
                "SELECT a.id FROM source_assets a JOIN asset_aliases alias ON alias.asset_id=a.id JOIN photos p ON p.id=alias.photo_id WHERE alias.photo_id=? AND a.project_id=? AND p.project_id=?",
            ).bind(photo_id).bind(project).bind(project).fetch_optional(&mut **tx).await?;
            asset_ids.push(asset.ok_or_else(|| {
                AppError::new(
                    "ASSET_NOT_FOUND",
                    "作品序列中的照片缺少项目资产，请重新关联后再生成",
                )
            })?);
        }

        // 读取规格
        let spec: String = sqlx::query_scalar("SELECT spec_json FROM publications WHERE id=?")
            .bind(&cmd.publication_id)
            .fetch_one(&mut **tx)
            .await?;
        let spec: serde_json::Value = serde_json::from_str(&spec)
            .map_err(|e| AppError::new("VALIDATION_ERROR", e.to_string()))?;
        let w = spec["width_mm"]
            .as_f64()
            .ok_or_else(|| AppError::new("VALIDATION_ERROR", "出版规格缺少页面宽度"))?;
        let h = spec["height_mm"]
            .as_f64()
            .ok_or_else(|| AppError::new("VALIDATION_ERROR", "出版规格缺少页面高度"))?;
        validate_page_size(w, h)?;
        let margin = 15.0;
        let now = domain::timestamp();

        // 封面（page 0）
        let cover_id = domain::id();
        sqlx::query("INSERT INTO publication_pages(id,publication_id,page_number,side,width_mm,height_mm,margin_mm,bleed_mm,created_at,updated_at) VALUES(?,?,0,'cover',?,?,?,3,?,?)")
            .bind(&cover_id).bind(&cmd.publication_id).bind(w).bind(h).bind(margin).bind(&now).bind(&now)
            .execute(&mut **tx).await?;
        // 封面放第一张
        sqlx::query("INSERT INTO publication_frames(id,page_id,x_mm,y_mm,width_mm,height_mm,fit_mode,asset_id,created_at,updated_at) VALUES(?,?,?,?,?,?,'fill',?,?,?)")
            .bind(domain::id()).bind(&cover_id).bind(margin).bind(margin)
            .bind(w - margin * 2.0).bind(h - margin * 2.0)
            .bind(&asset_ids[0]).bind(&now).bind(&now)
            .execute(&mut **tx).await?;

        // 内页
        let mut page_num = 1i64;
        for chunk in asset_ids.chunks(per_page as usize) {
            let page_id = domain::id();
            let side = if page_num % 2 == 0 { "left" } else { "right" };
            sqlx::query("INSERT INTO publication_pages(id,publication_id,page_number,side,width_mm,height_mm,margin_mm,bleed_mm,created_at,updated_at) VALUES(?,?,?,?,?,?,?,3,?,?)")
                .bind(&page_id).bind(&cmd.publication_id).bind(page_num).bind(side)
                .bind(w).bind(h).bind(margin).bind(&now).bind(&now)
                .execute(&mut **tx).await?;

            // 计算每张图的位置
            let avail_w = w - margin * 2.0;
            let avail_h = h - margin * 2.0;
            for (i, asset_id) in chunk.iter().enumerate() {
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
                    .bind(fx).bind(fy).bind(fw).bind(fh)
                    .bind(asset_id).bind(&now).bind(&now)
                    .execute(&mut **tx).await?;
            }
            page_num += 1;
        }

        sqlx::query("UPDATE publications SET updated_at=? WHERE id=? AND project_id=?")
            .bind(&now)
            .bind(&cmd.publication_id)
            .bind(project)
            .execute(&mut **tx)
            .await?;
        Ok(page_num - 1) // 返回生成的内页数
    }

    /// 列出项目的出版物。
    pub async fn list_publications(&self, project: &str) -> Result<Vec<Publication>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, String, String,i64)>(
            "SELECT id, project_id, pub_type, title, state, created_at,revision FROM publications WHERE project_id=? ORDER BY created_at",
        )
        .bind(project)
        .fetch_all(&self.store.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(id, project_id, pub_type, title, state, created_at, revision)| Publication {
                    id,
                    project_id,
                    pub_type,
                    title,
                    state,
                    created_at,
                    revision,
                },
            )
            .collect())
    }
}

impl Store {
    pub async fn publication_pages(
        &self,
        project: &str,
        publication_id: &str,
    ) -> Result<Vec<PublicationPage>> {
        let mut tx = self.pool.begin().await?;
        let exists: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM publications WHERE id=? AND project_id=?")
                .bind(publication_id)
                .bind(project)
                .fetch_one(&mut *tx)
                .await?;
        if exists == 0 {
            return Err(AppError::new("NOT_FOUND", "出版物不存在或不属于此项目"));
        }
        let pages: Vec<(String, i64, f64, f64)> = sqlx::query_as(
            "SELECT id,page_number,width_mm,height_mm FROM publication_pages WHERE publication_id=? ORDER BY page_number",
        ).bind(publication_id).fetch_all(&mut *tx).await?;
        let mut result = Vec::with_capacity(pages.len());
        for (id, page_number, width_mm, height_mm) in pages {
            let frames = sqlx::query_as::<_, PublicationFrame>(
                "SELECT f.id,f.x_mm,f.y_mm,f.width_mm,f.height_mm,f.fit_mode,(SELECT MIN(p.thumbnail_path) FROM source_assets a JOIN asset_aliases alias ON alias.asset_id=a.id JOIN photos p ON p.id=alias.photo_id AND p.project_id=a.project_id WHERE a.id=f.asset_id AND a.project_id=?) AS thumbnail_path FROM publication_frames f WHERE f.page_id=? ORDER BY f.y_mm,f.x_mm,f.id",
            ).bind(project).bind(&id).fetch_all(&mut *tx).await?;
            result.push(PublicationPage {
                id,
                page_number,
                width_mm,
                height_mm,
                frames,
            });
        }
        tx.commit().await?;
        Ok(result)
    }

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

    async fn seed_sequence(db: &Store, project: &str, count: usize) -> Vec<String> {
        let mut assets = Vec::new();
        for i in 0..count {
            let photo = domain::id();
            let asset = domain::id();
            let hash = format!("{i:064x}");
            sqlx::query("INSERT INTO photos(id,project_id,import_index,original_path,filename,file_size,content_sha256,width,height,orientation,thumbnail_path,preview_path,created_at) VALUES(?,?,?,'/fixture/source.jpg','source.jpg',100,?,100,100,1,'/fixture/thumb.jpg','/fixture/preview.jpg','now')")
                .bind(&photo).bind(project).bind(i as i64).bind(&hash).execute(&db.pool).await.unwrap();
            sqlx::query("INSERT INTO source_assets(id,project_id,content_sha256,original_locator,file_size,width,height,format,created_at) VALUES(?,?,?,'[]',100,100,100,'jpeg','now')")
                .bind(&asset).bind(project).bind(&hash).execute(&db.pool).await.unwrap();
            sqlx::query("INSERT INTO asset_aliases(photo_id,asset_id) VALUES(?,?)")
                .bind(&photo)
                .bind(&asset)
                .execute(&db.pool)
                .await
                .unwrap();
            sqlx::query("INSERT INTO photo_decisions(project_id,photo_id,status,updated_at) VALUES(?,?,'keep','now')")
                .bind(project).bind(&photo).execute(&db.pool).await.unwrap();
            sqlx::query("INSERT INTO sequence_items VALUES(?,?,?,'now')")
                .bind(project)
                .bind(photo)
                .bind(i as i64)
                .execute(&db.pool)
                .await
                .unwrap();
            assets.push(asset);
        }
        assets
    }

    fn publication_command() -> CreatePublicationCommand {
        CreatePublicationCommand {
            title: "初稿".to_owned(),
            pub_type: None,
            width_mm: Some(190.0),
            height_mm: Some(257.0),
        }
    }

    #[tokio::test]
    async fn nonempty_sequence_generates_ordered_pages_and_frames_within_margins() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("layout.sqlite");
        let db = Store::open(&path).await.unwrap();
        let project = db.create("布局", "").await.unwrap();
        let assets = seed_sequence(&db, &project.id, 5).await;
        for per_page in [1, 2, 4] {
            let publication = db
                .create_publication(&project.id, &publication_command())
                .await
                .unwrap();
            let command = GenerateFromSequenceCommand {
                publication_id: publication.id.clone(),
                photos_per_page: Some(per_page),
            };
            let pages = db
                .generate_from_sequence(&project.id, &command)
                .await
                .unwrap();
            assert_eq!(pages, (5 + per_page - 1) / per_page);
            let frames: Vec<(String, i64, f64, f64, f64, f64, String)> = sqlx::query_as(
                "SELECT f.asset_id,p.page_number,f.x_mm,f.y_mm,f.width_mm,f.height_mm,f.fit_mode FROM publication_frames f JOIN publication_pages p ON p.id=f.page_id WHERE p.publication_id=? ORDER BY p.page_number,f.y_mm,f.x_mm")
                .bind(&publication.id).fetch_all(&db.pool).await.unwrap();
            assert_eq!(frames.len(), 6);
            let preview = db
                .publication_pages(&project.id, &publication.id)
                .await
                .unwrap();
            assert_eq!(preview.len(), pages as usize + 1);
            assert_eq!(preview[0].page_number, 0);
            assert_eq!(
                preview[0].frames[0].thumbnail_path.as_deref(),
                Some("/fixture/thumb.jpg")
            );
            assert_eq!(
                preview.iter().map(|page| page.frames.len()).sum::<usize>(),
                6
            );
            assert_eq!(frames[0].0, assets[0]);
            assert_eq!(frames[0].1, 0);
            assert_eq!(frames[0].6, "fill");
            assert_eq!(
                frames
                    .iter()
                    .skip(1)
                    .map(|f| f.0.clone())
                    .collect::<Vec<_>>(),
                assets
            );
            for (_, _, x, y, w, h, _) in &frames {
                assert!(*x >= 15.0 && *y >= 15.0 && *w > 0.0 && *h > 0.0);
                assert!(x + w <= 175.0 && y + h <= 242.0, "图片框不得超出页面边距");
            }
            let before: Vec<String> = sqlx::query_scalar(
                "SELECT id FROM publication_pages WHERE publication_id=? ORDER BY page_number",
            )
            .bind(&publication.id)
            .fetch_all(&db.pool)
            .await
            .unwrap();
            let error = db
                .generate_from_sequence(&project.id, &command)
                .await
                .unwrap_err();
            assert_eq!(error.code, "CONFLICT");
            let after: Vec<String> = sqlx::query_scalar(
                "SELECT id FROM publication_pages WHERE publication_id=? ORDER BY page_number",
            )
            .bind(&publication.id)
            .fetch_all(&db.pool)
            .await
            .unwrap();
            assert_eq!(before, after, "重复生成不能销毁已有页面");
        }
        db.pool.close().await;
        let reopened = Store::open(&path).await.unwrap();
        assert_eq!(
            reopened.list_publications(&project.id).await.unwrap().len(),
            3
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM publication_frames")
                .fetch_one(&reopened.pool)
                .await
                .unwrap(),
            18
        );
    }

    #[tokio::test]
    async fn invalid_specs_states_missing_assets_and_foreign_projects_leave_no_pages() {
        let temp = tempfile::tempdir().unwrap();
        let db = Store::open(&temp.path().join("invalid.sqlite"))
            .await
            .unwrap();
        let project = db.create("校验", "").await.unwrap();
        let other = db.create("另一个项目", "").await.unwrap();
        seed_sequence(&db, &project.id, 1).await;
        for dimension in [f64::NAN, f64::INFINITY, -1.0, 0.0, 30.0, 2001.0] {
            let mut command = publication_command();
            command.width_mm = Some(dimension);
            assert_eq!(
                db.create_publication(&project.id, &command)
                    .await
                    .unwrap_err()
                    .code,
                "VALIDATION_ERROR"
            );
            command.width_mm = Some(190.0);
            command.height_mm = Some(dimension);
            assert!(db.create_publication(&project.id, &command).await.is_err());
        }
        let publication = db
            .create_publication(&project.id, &publication_command())
            .await
            .unwrap();
        for per_page in [0, 3, 5, i64::MAX] {
            assert!(
                db.generate_from_sequence(
                    &project.id,
                    &GenerateFromSequenceCommand {
                        publication_id: publication.id.clone(),
                        photos_per_page: Some(per_page),
                    }
                )
                .await
                .is_err()
            );
        }
        let command = GenerateFromSequenceCommand {
            publication_id: publication.id.clone(),
            photos_per_page: None,
        };
        assert_eq!(
            db.publication_pages(&other.id, &publication.id)
                .await
                .unwrap_err()
                .code,
            "NOT_FOUND"
        );
        assert_eq!(
            db.generate_from_sequence(&other.id, &command)
                .await
                .unwrap_err()
                .code,
            "NOT_FOUND"
        );
        sqlx::query("UPDATE publications SET state='finalized' WHERE id=?")
            .bind(&publication.id)
            .execute(&db.pool)
            .await
            .unwrap();
        assert_eq!(
            db.generate_from_sequence(&project.id, &command)
                .await
                .unwrap_err()
                .code,
            "CONFLICT"
        );
        sqlx::query("UPDATE publications SET state='draft',spec_json='{}' WHERE id=?")
            .bind(&publication.id)
            .execute(&db.pool)
            .await
            .unwrap();
        assert_eq!(
            db.generate_from_sequence(&project.id, &command)
                .await
                .unwrap_err()
                .code,
            "VALIDATION_ERROR"
        );
        sqlx::query(
            "UPDATE publications SET spec_json='{\"width_mm\":190,\"height_mm\":257}' WHERE id=?",
        )
        .bind(&publication.id)
        .execute(&db.pool)
        .await
        .unwrap();
        sqlx::query("UPDATE source_assets SET project_id=?")
            .bind(&other.id)
            .execute(&db.pool)
            .await
            .unwrap();
        assert_eq!(
            db.generate_from_sequence(&project.id, &command)
                .await
                .unwrap_err()
                .code,
            "ASSET_NOT_FOUND"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM publication_pages")
                .fetch_one(&db.pool)
                .await
                .unwrap(),
            0
        );
    }

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
