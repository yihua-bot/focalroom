// v2.0 S02 签名媒体链接（协议层）：生成（token=HMAC(项目+照片+过期)）、
// 校验（签名有效 + 未过期 + 未撤销三重）、撤销（即使签名未过期也拒绝）。
// 真实 CDN/URL 服务在 M4 部署后用同一校验逻辑做边缘网关。
use crate::{
    db::Store,
    domain::{self, AppError, Result},
};
use hmac::{Hmac, Mac};
use serde_json::{Value, json};
use sha2::Sha256;
use sqlx::Row;

pub const MEDIA_KEY_ENV: &str = "BETWEEN_FRAMES_MEDIA_HMAC_KEY";

fn media_secret() -> Result<String> {
    std::env::var(MEDIA_KEY_ENV)
        .ok()
        .filter(|key| !key.trim().is_empty())
        .ok_or_else(|| {
            AppError::new(
                "PERMISSION_DENIED",
                "媒体链接密钥未配置；共享链接功能不可用（安全默认）",
            )
        })
}

fn compute_token(project: &str, photo: &str, expires_ms: i64, secret: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC");
    mac.update(format!("{project}|{photo}|{expires_ms}").as_bytes());
    format!("{:x}", mac.finalize().into_bytes())
}

/// 生成签名链接授权：token 内嵌过期时间；同一照片可有多条不重复授权。
pub async fn issue(
    store: &Store,
    project_id: &str,
    photo_id: &str,
    expires_at_ms: i64,
) -> Result<Value> {
    let secret = media_secret()?;
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    if expires_at_ms <= now_ms {
        return Err(AppError::new("VALIDATION_ERROR", "过期时间必须在未来"));
    }
    let owned: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM photos WHERE id=? AND project_id=?")
        .bind(photo_id)
        .bind(project_id)
        .fetch_one(&store.pool)
        .await
        .unwrap_or(0);
    if owned != 1 {
        return Err(AppError::new("PERMISSION_DENIED", "照片不属于此项目"));
    }
    let token = compute_token(project_id, photo_id, expires_at_ms, &secret);
    let id = domain::id();
    sqlx::query("INSERT OR IGNORE INTO v2_media_grants(id, project_id, photo_id, token, expires_at_ms, created_at) VALUES(?,?,?,?,?,?)")
        .bind(&id).bind(project_id).bind(photo_id).bind(&token).bind(expires_at_ms).bind(domain::timestamp())
        .execute(&store.pool).await?;
    Ok(json!({"grant_id": id, "token": token, "expires_at_ms": expires_at_ms}))
}

/// 校验访问（S02 核心）：签名正确 + 未过期 + 授权 active，三者缺一拒绝。
/// 撤销后的拒绝明确区分于过期——审计与 UX 可分辨。
pub async fn verify(
    store: &Store,
    project_id: &str,
    photo_id: &str,
    token: &str,
    now_ms: i64,
) -> Result<Value> {
    let secret = media_secret()?;
    let row = sqlx::query("SELECT expires_at_ms, state FROM v2_media_grants WHERE project_id=? AND photo_id=? AND token=?")
        .bind(project_id).bind(photo_id).bind(token)
        .fetch_optional(&store.pool)
        .await?;
    let (expires, state): (i64, String) = match row {
        Some(row) => (row.get("expires_at_ms"), row.get("state")),
        None => {
            // 无记录也可能是伪造 token：签名不匹配同样拒绝。
            let expected = compute_token(project_id, photo_id, 0, &secret);
            let _ = expected;
            return Err(AppError::new("PERMISSION_DENIED", "链接无效"));
        }
    };
    if state == "revoked" {
        return Err(AppError::new(
            "PERMISSION_DENIED",
            "链接已被撤销；访问被拒绝",
        ));
    }
    if now_ms >= expires {
        return Err(AppError::new(
            "PERMISSION_DENIED",
            "链接已过期；请请求新的授权",
        ));
    }
    let expected = compute_token(project_id, photo_id, expires, &secret);
    if token != expected {
        return Err(AppError::new("PERMISSION_DENIED", "链接签名不匹配"));
    }
    Ok(json!({"allowed": true, "photo_id": photo_id, "expires_at_ms": expires}))
}

/// 撤销授权：即使签名与有效期均正常，撤销后立即拒绝。
pub async fn revoke(store: &Store, grant_id: &str) -> Result<()> {
    let changed =
        sqlx::query("UPDATE v2_media_grants SET state='revoked' WHERE id=? AND state='active'")
            .bind(grant_id)
            .execute(&store.pool)
            .await?
            .rows_affected();
    if changed == 0 {
        return Err(AppError::new("NOT_FOUND", "授权不存在或已撤销"));
    }
    Ok(())
}

/// 撤销某项目的全部媒体授权（项目降级/成员移除时调用方使用）。
pub async fn revoke_project(store: &Store, project_id: &str) -> Result<u64> {
    let changed = sqlx::query(
        "UPDATE v2_media_grants SET state='revoked' WHERE project_id=? AND state='active'",
    )
    .bind(project_id)
    .execute(&store.pool)
    .await?
    .rows_affected();
    Ok(changed)
}
