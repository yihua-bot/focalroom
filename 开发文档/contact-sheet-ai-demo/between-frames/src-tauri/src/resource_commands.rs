use crate::{
    command_audit::internal_event,
    command_handler::CommandHandler,
    db,
    domain::{self, AppError, Project, Result},
    image_pipeline::PreparedPhoto,
};
use serde_json::json;
use sqlx::Row;
impl CommandHandler<'_> {
    pub async fn import_photo(&self, project: &str, prepared: &PreparedPhoto) -> Result<bool> {
        let mut tx = self.store.pool.begin_with("BEGIN IMMEDIATE").await?;
        crate::agent::promotion::ensure_local_write(&mut tx, project).await?;
        let revision: i64 =
            sqlx::query_scalar("SELECT revision FROM projects WHERE id=? AND deleted_at IS NULL")
                .bind(project)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| AppError::new("NOT_FOUND", "项目不存在或已归档"))?;
        let exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM photos WHERE project_id=? AND content_sha256=?",
        )
        .bind(project)
        .bind(&prepared.hash)
        .fetch_one(&mut *tx)
        .await?;
        if exists > 0 {
            tx.commit().await?;
            return Ok(false);
        }
        let id = domain::id();
        let now = domain::timestamp();
        let index: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(import_index),0)+1 FROM photos WHERE project_id=?",
        )
        .bind(project)
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO photos(id,project_id,import_index,original_path,filename,file_size,content_sha256,width,height,thumbnail_path,preview_path,created_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?)").bind(&id).bind(project).bind(index).bind(prepared.path.to_string_lossy().as_ref()).bind(&prepared.filename).bind(prepared.size as i64).bind(&prepared.hash).bind(prepared.width as i64).bind(prepared.height as i64).bind(prepared.thumbnail.to_string_lossy().as_ref()).bind(prepared.preview.to_string_lossy().as_ref()).bind(&now).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO photo_decisions(project_id,photo_id,updated_at) VALUES(?,?,?)")
            .bind(project)
            .bind(&id)
            .bind(&now)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE projects SET revision=revision+1,updated_at=? WHERE id=?")
            .bind(&now)
            .bind(project)
            .execute(&mut *tx)
            .await?;
        let after =
            json!({"photoId":id,"hash":prepared.hash,"originalPath":prepared.path,"index":index});
        internal_event(
            &mut tx,
            project,
            Some(&id),
            "import",
            revision,
            &json!(null),
            &after,
            "system_import",
        )
        .await?;
        tx.commit().await?;
        Ok(true)
    }
    pub async fn relink_photo(
        &self,
        project: &str,
        photo: &str,
        prepared: &PreparedPhoto,
        expected: i64,
    ) -> Result<Project> {
        let mut tx = self.store.pool.begin_with("BEGIN IMMEDIATE").await?;
        crate::agent::promotion::ensure_local_write(&mut tx, project).await?;
        let previous = sqlx::query(
            "SELECT original_path,content_sha256 FROM photos WHERE project_id=? AND id=?",
        )
        .bind(project)
        .bind(photo)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::new("NOT_FOUND", "照片不属于项目"))?;
        if previous.get::<String, _>("content_sha256") != prepared.hash {
            return Err(AppError::new("HASH_MISMATCH", "所选文件与原图指纹不同"));
        }
        if sqlx::query("UPDATE projects SET revision=revision+1,updated_at=? WHERE id=? AND revision=? AND deleted_at IS NULL").bind(domain::timestamp()).bind(project).bind(expected).execute(&mut *tx).await?.rows_affected()!=1 {return Err(AppError::new("REVISION_CONFLICT","项目已更新，请重新加载"));}
        sqlx::query("UPDATE photos SET original_path=?,thumbnail_path=?,preview_path=? WHERE project_id=? AND id=?").bind(prepared.path.to_string_lossy().as_ref()).bind(prepared.thumbnail.to_string_lossy().as_ref()).bind(prepared.preview.to_string_lossy().as_ref()).bind(project).bind(photo).execute(&mut *tx).await?;
        internal_event(
            &mut tx,
            project,
            Some(photo),
            "relink",
            expected,
            &json!({"originalPath":previous.get::<String,_>("original_path")}),
            &json!({"originalPath":prepared.path}),
            "gui_direct",
        )
        .await?;
        let result = db::snapshot_tx(&mut tx, project, Some(false)).await?;
        tx.commit().await?;
        Ok(result)
    }
}
