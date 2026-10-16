//! 本地认证验证切片。PHC 密码哈希、令牌摘要和会话时限复用现有列，不改变 schema。
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{error::AppError, handlers::AppState};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegisterRequest {
    email: String,
    display_name: String,
    password: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LoginRequest {
    email: String,
    password: String,
}
#[derive(Serialize)]
pub(crate) struct AuthResponse {
    token: String,
    user_id: String,
    display_name: String,
}

pub(crate) fn normalized_email(email: &str) -> Result<String, AppError> {
    let email = email.trim().to_ascii_lowercase();
    let (local, domain) = email
        .split_once('@')
        .ok_or_else(|| AppError::validation("邮箱格式不合法"))?;
    if email.len() > 254
        || local.is_empty()
        || domain.is_empty()
        || domain.contains('@')
        || email.chars().any(char::is_whitespace)
        || email.chars().any(char::is_control)
    {
        return Err(AppError::validation("邮箱格式不合法"));
    }
    Ok(email)
}
fn valid_password(password: &str) -> bool {
    (8..=1024).contains(&password.len())
}
fn token_digest(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}
fn new_token() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

pub(crate) async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let email = normalized_email(&req.email)?;
    if !valid_password(&req.password)
        || req.display_name.trim().is_empty()
        || req.display_name.len() > 200
    {
        return Err(AppError::validation(
            "密码须为 8–1024 字节，姓名须为 1–200 字节",
        ));
    }
    // 限制并发内存与阻塞线程；不是按账户/IP 的生产限流。
    let permit = state
        .auth_slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| AppError::busy())?;
    let hash = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        Argon2::default()
            .hash_password(req.password.as_bytes(), &SaltString::generate(&mut OsRng))
            .map(|hash| hash.to_string())
            .map_err(|_| AppError::internal())
    })
    .await
    .map_err(|_| AppError::internal())??;
    let user_id = uuid::Uuid::new_v4().to_string();
    let token = new_token();
    let mut tx = state.pool.begin().await?;
    // salt 保留旧列；随机盐和参数已包含在 PHC 串，不修改历史 schema。
    sqlx::query("INSERT INTO users(id,email,salt,password_hash,display_name) VALUES(?,?,'',?,?)")
        .bind(&user_id)
        .bind(email)
        .bind(hash)
        .bind(req.display_name.trim())
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO sessions(token,user_id) VALUES(?,?)")
        .bind(token_digest(&token))
        .bind(&user_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(AuthResponse {
        token,
        user_id,
        display_name: req.display_name.trim().into(),
    }))
}

pub(crate) async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let email = normalized_email(&req.email).map_err(|_| AppError::auth("邮箱或密码错误"))?;
    if !valid_password(&req.password) {
        return Err(AppError::auth("邮箱或密码错误"));
    }
    let row: Option<(String, String, String)> =
        sqlx::query_as("SELECT id,password_hash,display_name FROM users WHERE email=?")
            .bind(email)
            .fetch_optional(&state.pool)
            .await?;
    // 不支持旧快速 SHA-256 自动降级；使用固定 Argon2 参数执行等量工作后统一拒绝。
    let hash = row.as_ref().map(|row| row.1.clone());
    let permit = state
        .auth_slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| AppError::busy())?;
    let valid = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let salt = SaltString::encode_b64(b"bf-dummy-auth-v1").expect("固定盐有效");
        let argon = Argon2::default();
        if let Some(hash) = hash
            .as_deref()
            .filter(|hash| hash.starts_with("$argon2id$"))
        {
            if let Ok(parsed) = PasswordHash::new(hash) {
                return argon
                    .verify_password(req.password.as_bytes(), &parsed)
                    .is_ok();
            }
        }
        let _ = argon.hash_password(req.password.as_bytes(), &salt);
        false
    })
    .await
    .map_err(|_| AppError::internal())?;
    if !valid {
        return Err(AppError::auth("邮箱或密码错误"));
    }
    let (user_id, _, name) = row.ok_or_else(|| AppError::auth("邮箱或密码错误"))?;
    let token = new_token();
    sqlx::query("INSERT INTO sessions(token,user_id) VALUES(?,?)")
        .bind(token_digest(&token))
        .bind(&user_id)
        .execute(&state.pool)
        .await?;
    Ok(Json(AuthResponse {
        token,
        user_id,
        display_name: name,
    }))
}

#[derive(Clone)]
pub(crate) struct CurrentUser {
    pub id: String,
    token_hash: String,
}

pub(crate) async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|token| token.len() == 64 && token.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(|| AppError::auth("缺少或无效的认证令牌"))?;
    let token_hash = token_digest(token);
    let user_id: String = sqlx::query_scalar("SELECT user_id FROM sessions WHERE token=? AND datetime(created_at)>datetime('now','-24 hours') AND datetime(created_at)<=datetime('now')")
        .bind(&token_hash).fetch_optional(&state.pool).await?
        .ok_or_else(|| AppError::auth("令牌无效或已过期"))?;
    req.extensions_mut().insert(CurrentUser {
        id: user_id,
        token_hash,
    });
    Ok(next.run(req).await)
}

pub(crate) async fn logout(
    State(state): State<AppState>,
    Extension(user): Extension<CurrentUser>,
) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query("DELETE FROM sessions WHERE token=?")
        .bind(user.token_hash)
        .execute(&state.pool)
        .await?;
    Ok(Json(serde_json::json!({"logged_out": true})))
}
