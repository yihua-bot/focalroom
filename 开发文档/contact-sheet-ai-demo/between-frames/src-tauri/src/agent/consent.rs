use crate::domain::{AppError, Result};
use sqlx::{Row, Sqlite, Transaction};

/// 生效授权：任务显式引用的 consent（task 模式经 consent_id 绑定）优先；
/// 否则回退项目级 consent；deny 优先于一切；已撤回的不生效。
/// 返回 Some(consent_id) 表示允许上传，quota 剩余量在 used/quota 中校验。
pub(crate) async fn authorize(
    tx: &mut Transaction<'_, Sqlite>,
    task: &super::types::Task,
    count: i64,
) -> Result<Option<i64>> {
    let provider = task.provider_id.as_str();
    let matches_project = |row: &sqlx::sqlite::SqliteRow| {
        row.get::<String, _>("project_id") == task.project_id
            && row.get::<String, _>("provider") == provider
            && row.get::<Option<String>, _>("revoked_at").is_none()
    };
    if let Some(id) = task.consent_id {
        if let Some(row) = by_id(tx, id).await?
            && matches_project(&row)
        {
            return grant(&row, count);
        }
        return Ok(None);
    }
    if let Some(row) = latest(tx, &task.project_id, provider, None).await? {
        return grant(&row, count);
    }
    Ok(None)
}
fn grant(row: &sqlx::sqlite::SqliteRow, count: i64) -> Result<Option<i64>> {
    let quota = row.get::<i64, _>("quota");
    let used = row.get::<i64, _>("used");
    if row.get::<String, _>("mode") == "deny" || used + count > quota {
        return Ok(None);
    }
    Ok(Some(row.get::<i64, _>("id")))
}
async fn by_id(
    tx: &mut Transaction<'_, Sqlite>,
    id: i64,
) -> Result<Option<sqlx::sqlite::SqliteRow>> {
    Ok(sqlx::query("SELECT * FROM privacy_consents WHERE id=?")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?)
}
async fn latest(
    tx: &mut Transaction<'_, Sqlite>,
    project: &str,
    provider: &str,
    mode: Option<&str>,
) -> Result<Option<sqlx::sqlite::SqliteRow>> {
    let sql = match mode {
        Some(_mode) => {
            "SELECT * FROM privacy_consents WHERE project_id=? AND provider=? AND mode=? AND revoked_at IS NULL ORDER BY id DESC LIMIT 1"
        }
        None => {
            "SELECT * FROM privacy_consents WHERE project_id=? AND provider=? AND revoked_at IS NULL ORDER BY CASE mode WHEN 'deny' THEN 0 ELSE 1 END, id DESC LIMIT 1"
        }
    };
    Ok(sqlx::query(sql)
        .bind(project)
        .bind(provider)
        .bind(mode)
        .fetch_optional(&mut **tx)
        .await?)
}
/// 上传成功后在同一事务消耗配额并累计任务用量/费用。
pub(crate) async fn consume(
    tx: &mut Transaction<'_, Sqlite>,
    task: &mut super::types::Task,
    consent_id: i64,
    images: u32,
    text_tokens: u32,
    cost_minor_usd: u32,
) -> Result<()> {
    sqlx::query("UPDATE privacy_consents SET used=used+? WHERE id=? AND used+?<=quota")
        .bind(images as i64)
        .bind(consent_id)
        .bind(images as i64)
        .execute(&mut **tx)
        .await?;
    task.usage.cloud_images = task.usage.cloud_images.saturating_add(images);
    task.usage.cost_minor_usd = task.usage.cost_minor_usd.saturating_add(cost_minor_usd);
    super::store::save(tx, task).await?;
    sqlx::query(
        "UPDATE model_usage SET cloud_images=cloud_images+?, cost_minor_usd=cost_minor_usd+? WHERE task_id=? AND request_id=?",
    )
    .bind(images as i64)
    .bind(cost_minor_usd as i64)
    .bind(&task.id)
    .bind(format!("turn-{}", task.usage.model_turns))
    .execute(&mut **tx)
    .await?;
    let _ = text_tokens;
    Ok(())
}
pub struct SetConsent {
    pub request_id: String,
    pub project_id: String,
    pub mode: String,
    pub provider: String,
    pub quota: i64,
}
pub async fn set(db: &crate::db::Store, r: SetConsent) -> Result<super::types::ConsentView> {
    if !matches!(r.mode.as_str(), "task" | "project" | "deny")
        || !matches!(r.provider.as_str(), "openai" | "mock")
        || r.quota < 0
        || r.quota > 10000
        || r.request_id.is_empty()
        || r.request_id.len() > 160
    {
        return Err(AppError::new("VALIDATION_ERROR", "授权参数不合法"));
    }
    if r.mode != "deny" && r.provider == "mock" {
        return Err(AppError::new(
            "VALIDATION_ERROR",
            "Mock 不上传图像，无需授权",
        ));
    }
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let existing: Option<i64> =
        sqlx::query_scalar("SELECT consent_id FROM consent_receipts WHERE request_id=?")
            .bind(&r.request_id)
            .fetch_optional(&mut *tx)
            .await?;
    if let Some(id) = existing {
        let view = view(&mut tx, id).await?;
        tx.commit().await?;
        return Ok(view);
    }
    // 同项目同 provider 旧授权自动失效；deny 撤回已有授权。
    sqlx::query(
        "UPDATE privacy_consents SET revoked_at=? WHERE project_id=? AND provider=? AND revoked_at IS NULL",
    )
    .bind(crate::domain::timestamp())
    .bind(&r.project_id)
    .bind(&r.provider)
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO privacy_consents(project_id,task_id,mode,provider,quota,used,granted_at) VALUES(?,NULL,?,?,?,0,?)")
        .bind(&r.project_id)
        .bind(&r.mode)
        .bind(&r.provider)
        .bind(r.quota)
        .bind(crate::domain::timestamp())
        .execute(&mut *tx)
        .await?;
    let id = sqlx::query_scalar("SELECT last_insert_rowid()")
        .fetch_one(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO consent_receipts VALUES(?,?)")
        .bind(&r.request_id)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let view = view(&mut tx, id).await?;
    tx.commit().await?;
    Ok(view)
}
async fn view(tx: &mut Transaction<'_, Sqlite>, id: i64) -> Result<super::types::ConsentView> {
    let row = by_id(tx, id)
        .await?
        .ok_or_else(|| AppError::new("NOT_FOUND", "授权不存在"))?;
    Ok(super::types::ConsentView {
        id: row.get("id"),
        project_id: row.get("project_id"),
        mode: row.get("mode"),
        provider: row.get("provider"),
        quota: row.get("quota"),
        used: row.get("used"),
        granted_at: row.get("granted_at"),
        revoked_at: row.get("revoked_at"),
    })
}
pub async fn effective_view(
    db: &crate::db::Store,
    project: &str,
    provider: &str,
) -> Result<Option<super::types::ConsentView>> {
    let mut tx = db.pool.begin().await?;
    let row = latest(&mut tx, project, provider, None).await?;
    let result = row
        .map(|row| super::types::ConsentView {
            id: row.get("id"),
            project_id: row.get("project_id"),
            mode: row.get("mode"),
            provider: row.get("provider"),
            quota: row.get("quota"),
            used: row.get("used"),
            granted_at: row.get("granted_at"),
            revoked_at: row.get("revoked_at"),
        })
        .filter(|view| view.revoked_at.is_none());
    tx.commit().await?;
    Ok(result)
}
