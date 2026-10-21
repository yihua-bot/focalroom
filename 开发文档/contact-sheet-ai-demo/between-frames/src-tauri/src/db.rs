use crate::{
    command_handler::{CommandHandler, GuiCommandRequest},
    commands_generated::GuiCommand,
    domain::{self, AppError, HistoryState, Photo, Project, Result, SequenceVersion},
    image_pipeline::PreparedPhoto,
};
use sqlx::{
    Row, Sqlite, SqlitePool, Transaction,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use std::{path::Path, str::FromStr};

#[derive(Clone)]
pub struct Store {
    pub pool: SqlitePool,
}

impl Store {
    pub async fn open(path: &Path) -> Result<Self> {
        let options = SqliteConnectOptions::from_str("sqlite:")?
            .filename(path)
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .busy_timeout(std::time::Duration::from_secs(10));
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        let migrator = sqlx::migrate!("./migrations");
        let latest = migrator
            .iter()
            .map(|migration| migration.version)
            .max()
            .unwrap_or(0);
        let migrations_exist: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations'",
        )
        .fetch_one(&pool)
        .await?;
        if migrations_exist > 0 {
            let version: i64 = sqlx::query_scalar(
                "SELECT COALESCE(MAX(version),0) FROM _sqlx_migrations WHERE success=1",
            )
            .fetch_one(&pool)
            .await?;
            if version > 0 && version < latest {
                // VACUUM INTO 包含已提交 WAL，备份成功后才升级；不覆盖既有文件。
                let backup = path.with_extension(format!(
                    "backup-before-{}-{}.sqlite",
                    if version < 3 {
                        "m1".to_owned()
                    } else if version == 3 {
                        "m2".to_owned()
                    } else {
                        format!("v{version}")
                    },
                    domain::id()
                ));
                sqlx::query("VACUUM INTO ?")
                    .bind(backup.to_string_lossy().as_ref())
                    .execute(&pool)
                    .await?;
                let check = SqliteConnectOptions::new()
                    .filename(&backup)
                    .read_only(true);
                let verification = SqlitePoolOptions::new()
                    .max_connections(1)
                    .connect_with(check)
                    .await?;
                let integrity: String = sqlx::query_scalar("PRAGMA integrity_check")
                    .fetch_one(&verification)
                    .await?;
                verification.close().await;
                if integrity != "ok" {
                    pool.close().await;
                    return Err(AppError::new("BACKUP_FAILED", "升级前备份完整性校验失败"));
                }
                let invalid:i64=sqlx::query_scalar("SELECT (SELECT COUNT(*) FROM photo_decisions d LEFT JOIN photos p ON p.id=d.photo_id WHERE p.id IS NULL OR p.project_id<>d.project_id) + (SELECT COUNT(*) FROM sequence_items s LEFT JOIN photo_decisions d ON d.project_id=s.project_id AND d.photo_id=s.photo_id WHERE d.photo_id IS NULL OR d.status<>'keep') + (SELECT COUNT(*) FROM comparison_events c WHERE (SELECT COUNT(*) FROM photos p WHERE p.project_id=c.project_id AND p.id IN(c.left_photo_id,c.right_photo_id))<>2)").fetch_one(&pool).await?;
                if invalid > 0
                    || !sqlx::query("PRAGMA foreign_key_check")
                        .fetch_all(&pool)
                        .await?
                        .is_empty()
                {
                    pool.close().await;
                    return Err(AppError::new(
                        "MIGRATION",
                        "旧库存在归属或外键错误；已保存备份，停止升级",
                    ));
                }
            }
        }
        migrator
            .run(&pool)
            .await
            .map_err(|error| AppError::new("MIGRATION", error.to_string()))?;
        // G1 回填：photos → source_assets + alias + representations（幂等：已回填跳过）。
        Self::backfill_asset_domain(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn list(&self, archived: bool) -> Result<Vec<Project>> {
        let ids: Vec<String> = sqlx::query_scalar(
            "SELECT id FROM projects WHERE (deleted_at IS NOT NULL)=? ORDER BY updated_at DESC,id",
        )
        .bind(archived)
        .fetch_all(&self.pool)
        .await?;
        let mut projects = Vec::new();
        for id in ids {
            projects.push(self.snapshot(&id, archived).await?);
        }
        Ok(projects)
    }
    pub async fn get(&self, id: &str) -> Result<Project> {
        self.snapshot(id, false).await
    }
    pub(crate) async fn snapshot(&self, id: &str, archived: bool) -> Result<Project> {
        let mut tx = self.pool.begin().await?;
        let project = snapshot_tx(&mut tx, id, Some(archived)).await?;
        tx.commit().await?;
        Ok(project)
    }
    async fn gui(&self, id: &str, revision: i64, command: GuiCommand) -> Result<Project> {
        let request = GuiCommandRequest::new(id, revision, command);
        CommandHandler::new(self)
            .execute_gui(request)
            .await?
            .project
            .ok_or_else(|| AppError::new("INTERNAL", "命令未返回项目"))
    }
    pub async fn create(&self, title: &str, brief: &str) -> Result<Project> {
        self.gui(
            &domain::id(),
            0,
            GuiCommand::CreateProject {
                title: title.into(),
                brief: brief.into(),
            },
        )
        .await
    }
    pub async fn edit(&self, id: &str, title: &str, brief: &str, revision: i64) -> Result<Project> {
        self.gui(
            id,
            revision,
            GuiCommand::UpdateProject {
                title: title.into(),
                brief: brief.into(),
            },
        )
        .await
    }
    pub async fn archive(&self, id: &str, revision: i64) -> Result<()> {
        self.gui(id, revision, GuiCommand::ArchiveProject {})
            .await
            .map(|_| ())
    }
    pub async fn restore(&self, id: &str) -> Result<Project> {
        let project = self.snapshot(id, true).await?;
        self.gui(id, project.revision, GuiCommand::RestoreProject {})
            .await
    }
    pub async fn decide(
        &self,
        id: &str,
        photo: &str,
        status: &str,
        revision: i64,
    ) -> Result<Project> {
        self.decide_many(id, &[photo.into()], status, revision)
            .await
    }
    pub async fn decide_many(
        &self,
        id: &str,
        photos: &[String],
        status: &str,
        revision: i64,
    ) -> Result<Project> {
        self.gui(
            id,
            revision,
            GuiCommand::SetPhotoDecision {
                photo_ids: photos.to_vec(),
                status: status.into(),
            },
        )
        .await
    }
    pub async fn rate_many(
        &self,
        id: &str,
        photos: &[String],
        rating: i64,
        revision: i64,
    ) -> Result<Project> {
        self.gui(
            id,
            revision,
            GuiCommand::SetPhotoRating {
                photo_ids: photos.to_vec(),
                rating,
            },
        )
        .await
    }
    pub async fn note(&self, id: &str, photo: &str, text: &str, revision: i64) -> Result<Project> {
        self.gui(
            id,
            revision,
            GuiCommand::SetPhotoNote {
                photo_id: photo.into(),
                note: text.into(),
            },
        )
        .await
    }
    pub async fn reorder(&self, id: &str, ids: &[String], revision: i64) -> Result<Project> {
        self.gui(
            id,
            revision,
            GuiCommand::ReorderSequence {
                ordered_photo_ids: ids.to_vec(),
            },
        )
        .await
    }
    pub async fn compare(
        &self,
        id: &str,
        left: &str,
        right: &str,
        preferred: Option<&str>,
        reason: &str,
        revision: i64,
    ) -> Result<Project> {
        if preferred.is_some_and(|photo| photo != left && photo != right) {
            return Err(AppError::new("INVALID_INPUT", "比较偏好不属于两张照片"));
        }
        self.gui(
            id,
            revision,
            GuiCommand::RecordComparison {
                left_id: left.into(),
                right_id: right.into(),
                preference: match preferred {
                    Some(p) if p == left => "left",
                    Some(_) => "right",
                    None => "uncertain",
                }
                .into(),
                reason: reason.into(),
            },
        )
        .await
    }
    pub async fn insert_photo(&self, id: &str, prepared: &PreparedPhoto) -> Result<bool> {
        CommandHandler::new(self).import_photo(id, prepared).await
    }
    /// 全部照片的内容指纹（缓存清理的 live 集合；缓存按 hash 跨项目共享）。
    /// 项目标签（v1.1）：组织元数据走独立通道——不占人工撤销栈、不改
    /// revision（不使草案 stale）。校验：1–24 字符、去重、每项目 ≤12 个。
    pub async fn set_project_tags(&self, project_id: &str, tags: &[String]) -> Result<Vec<String>> {
        let mut cleaned: Vec<String> = vec![];
        for tag in tags {
            let trimmed = tag.trim();
            if trimmed.is_empty() || trimmed.chars().count() > 24 {
                return Err(AppError::new("VALIDATION_ERROR", "标签须为 1–24 个字符"));
            }
            if !cleaned.iter().any(|existing| existing == trimmed) {
                cleaned.push(trimmed.to_owned());
            }
        }
        if cleaned.len() > 12 {
            return Err(AppError::new("VALIDATION_ERROR", "每个项目最多 12 个标签"));
        }
        let changed = sqlx::query(
            "UPDATE projects SET tags_json=?,updated_at=? WHERE id=? AND deleted_at IS NULL",
        )
        .bind(serde_json::to_string(&cleaned).unwrap())
        .bind(crate::domain::timestamp())
        .bind(project_id)
        .execute(&self.pool)
        .await?
        .rows_affected();
        if changed == 0 {
            return Err(AppError::new("NOT_FOUND", "项目不存在"));
        }
        Ok(cleaned)
    }
    pub async fn photo_hashes(&self) -> Result<Vec<String>> {
        Ok(
            sqlx::query_scalar("SELECT DISTINCT content_sha256 FROM photos")
                .fetch_all(&self.pool)
                .await?,
        )
    }
    pub async fn relink(
        &self,
        id: &str,
        photo: &str,
        prepared: &PreparedPhoto,
        revision: i64,
    ) -> Result<Project> {
        CommandHandler::new(self)
            .relink_photo(id, photo, prepared, revision)
            .await
    }
}

pub(crate) async fn snapshot_tx(
    tx: &mut Transaction<'_, Sqlite>,
    id: &str,
    archived: Option<bool>,
) -> Result<Project> {
    let row = sqlx::query(
        "SELECT id,title,brief,revision,updated_at,deleted_at,tags_json FROM projects WHERE id=?",
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or_else(|| AppError::new("NOT_FOUND", "项目不存在"))?;
    if archived
        .is_some_and(|expected| row.get::<Option<String>, _>("deleted_at").is_some() != expected)
    {
        return Err(AppError::new("NOT_FOUND", "项目不存在或已归档"));
    }
    let mut photos:Vec<Photo>=sqlx::query_as("SELECT p.id,p.import_index,p.filename AS name,p.original_path,p.thumbnail_path,p.preview_path,p.content_sha256,p.width,p.height,p.file_size,d.status,d.user_note AS note,d.rating,d.book_mark FROM photos p JOIN photo_decisions d ON d.project_id=p.project_id AND d.photo_id=p.id WHERE p.project_id=? ORDER BY p.import_index").bind(id).fetch_all(&mut **tx).await?;
    for photo in &mut photos {
        photo.is_missing = !Path::new(&photo.original_path).is_file();
    }
    let sequence = sequence(tx, id).await?;
    let events = crate::command_audit::events_through(tx, id, i64::MAX).await?;
    let cursor: i64 = sqlx::query_scalar("SELECT cursor FROM history_heads WHERE project_id=?")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?
        .unwrap_or(0);
    let undo_group_id = sqlx::query_scalar(
        "SELECT group_id FROM command_history WHERE project_id=? AND position=? AND active=1",
    )
    .bind(id)
    .bind(cursor)
    .fetch_optional(&mut **tx)
    .await?;
    let redo_group_id = sqlx::query_scalar(
        "SELECT group_id FROM command_history WHERE project_id=? AND position=? AND active=1",
    )
    .bind(id)
    .bind(cursor + 1)
    .fetch_optional(&mut **tx)
    .await?;
    let redo_count = sqlx::query_scalar(
        "SELECT COUNT(*) FROM command_history WHERE project_id=? AND position>? AND active=1",
    )
    .bind(id)
    .bind(cursor)
    .fetch_one(&mut **tx)
    .await?;
    let mut versions = Vec::new();
    for version in sqlx::query("SELECT id,label,based_on_revision,snapshot_json,created_at FROM sequence_versions WHERE project_id=? ORDER BY created_at DESC,rowid DESC").bind(id).fetch_all(&mut **tx).await? {
        let state:crate::edit_state::EditState=serde_json::from_str(&version.get::<String,_>("snapshot_json")).map_err(|_|AppError::new("DATABASE","版本快照格式无效"))?;
        versions.push(SequenceVersion{id:version.get("id"),label:version.get("label"),based_on_revision:version.get("based_on_revision"),created_at:version.get("created_at"),photo_ids:state.sequence,photos:state.photos});
    }
    Ok(Project {
        id: row.get("id"),
        archived: row.get::<Option<String>, _>("deleted_at").is_some(),
        title: row.get("title"),
        brief: row.get("brief"),
        revision: row.get("revision"),
        updated_at: row.get("updated_at"),
        tags: serde_json::from_str(&row.get::<String, _>("tags_json")).unwrap_or_default(),
        photos,
        sequence,
        events,
        history: HistoryState {
            undo_group_id,
            redo_group_id,
            undo_count: cursor,
            redo_count,
        },
        versions,
    })
}
pub(crate) async fn sequence(tx: &mut Transaction<'_, Sqlite>, id: &str) -> Result<Vec<String>> {
    Ok(sqlx::query_scalar(
        "SELECT photo_id FROM sequence_items WHERE project_id=? ORDER BY position",
    )
    .bind(id)
    .fetch_all(&mut **tx)
    .await?)
}
pub(crate) async fn write_sequence(
    tx: &mut Transaction<'_, Sqlite>,
    id: &str,
    order: &[String],
) -> Result<()> {
    sqlx::query("DELETE FROM sequence_items WHERE project_id=?")
        .bind(id)
        .execute(&mut **tx)
        .await?;
    for (position, photo) in order.iter().enumerate() {
        sqlx::query("INSERT INTO sequence_items VALUES(?,?,?,?)")
            .bind(id)
            .bind(photo)
            .bind(position as i64)
            .bind(domain::timestamp())
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

impl Store {
    /// G1 expand→backfill：把 photos 行提取到资产域五张表。
    /// 幂等：photo id 已在 source_assets 中则跳过；不修改 photos 任何列。
    async fn backfill_asset_domain(pool: &SqlitePool) -> Result<()> {
        let migrated: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM source_assets")
            .fetch_one(pool)
            .await
            .unwrap_or(0);
        let photos: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM photos")
            .fetch_one(pool)
            .await
            .unwrap_or(0);
        if migrated >= photos {
            return Ok(());
        }
        let rows = sqlx::query(
            "SELECT id, project_id, content_sha256, original_path, file_size, width, height, thumbnail_path, preview_path, created_at FROM photos",
        )
        .fetch_all(pool)
        .await?;
        let now = crate::domain::timestamp();
        for row in rows {
            let id: String = row.get("id");
            // 已迁移（部分回填后重跑）
            let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM source_assets WHERE id=?")
                .bind(&id)
                .fetch_one(pool)
                .await
                .unwrap_or(0);
            if exists > 0 {
                continue;
            }
            let project_id: String = row.get("project_id");
            let hash: String = row.get("content_sha256");
            let original_path: String = row.get("original_path");
            let file_size: i64 = row.get("file_size");
            let width: i64 = row.get("width");
            let height: i64 = row.get("height");
            let thumbnail: String = row.get("thumbnail_path");
            let preview: String = row.get("preview_path");
            let created_at: String = row.get("created_at");
            let locator = format!(
                r#"[{{"path":{path},"kind":"file"}}]"#,
                path = serde_json::json!(original_path)
            );
            let format = original_path
                .rsplit('.')
                .next()
                .unwrap_or("")
                .to_lowercase();
            let format = if format.len() > 10 {
                "unknown".to_owned()
            } else {
                format
            };
            let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;
            sqlx::query(
                "INSERT INTO source_assets(id, project_id, content_sha256, original_locator, file_size, width, height, format, source_profile_digest, source_hash_verified_at, created_at) VALUES(?,?,?,?,?,?,?,?,NULL,NULL,?)",
            )
            .bind(&id).bind(&project_id).bind(&hash).bind(&locator)
            .bind(file_size).bind(width).bind(height).bind(&format).bind(&created_at)
            .execute(&mut *tx).await?;
            sqlx::query("INSERT INTO asset_aliases(photo_id, asset_id) VALUES(?,?)")
                .bind(&id)
                .bind(&id)
                .execute(&mut *tx)
                .await?;
            // 默认编辑变体（原名）
            let variant_id = crate::domain::id();
            sqlx::query(
                "INSERT INTO edit_variants(id, asset_id, name, external_software, external_variant_id, is_current, created_at) VALUES(?,?,?,NULL,NULL,1,?)",
            )
            .bind(&variant_id).bind(&id).bind("original").bind(&now)
            .execute(&mut *tx).await?;
            // 两条 representation：thumbnail + display_proxy
            // G1 修复：读缓存文件的真实尺寸，不再用 width/4 猜测
            for (kind, path) in [("thumbnail", &thumbnail), ("display_proxy", &preview)] {
                let size = std::fs::metadata(path.as_str())
                    .map(|m| m.len() as i64)
                    .unwrap_or(0);
                let (rep_w, rep_h) = image::image_dimensions(path.as_str())
                    .map(|(w, h)| (w as i64, h as i64))
                    .unwrap_or((width, height)); // 文件损坏时退回源尺寸
                sqlx::query(
                    "INSERT INTO asset_representations(id, asset_id, kind, algorithm, width, height, file_path, file_size, profile_digest, created_at) VALUES(?,?,?,?,?,?,?,?,NULL,?)",
                )
                .bind(crate::domain::id()).bind(&id).bind(kind).bind("image_pipeline_v1")
                .bind(rep_w).bind(rep_h).bind(path.as_str()).bind(size).bind(&now)
                .execute(&mut *tx).await?;
            }
            tx.commit().await?;
        }
        Ok(())
    }
}
