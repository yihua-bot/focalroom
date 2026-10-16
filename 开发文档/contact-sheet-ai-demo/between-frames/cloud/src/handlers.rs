//! M4 垂直切片 handlers。：登录 → 项目授权 → Web 读取预览 → 撤销即拒绝。
//! S02 验收：撤销后任何预览 URL 返回 403。
//! 本切片用 SQLite 证明逻辑正确；PostgreSQL 只换连接串（sqlx 双方言）。

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Json, Response},
    routing::{delete, get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

#[derive(Clone)]
struct AppState {
    pool: SqlitePool,
    preview_dir: std::path::PathBuf,
}

// ===== 认证 =====

#[derive(Deserialize)]
struct RegisterRequest {
    email: String,
    display_name: String,
    password: String,
}

#[derive(Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct AuthResponse {
    token: String,
    user_id: String,
    display_name: String,
}

fn hash_password(password: &str, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{salt}:{password}"));
    hex::encode(hasher.finalize())
}

async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    if req.email.contains('@').not() || req.password.len() < 8 {
        return Err(AppError::validation("邮箱或密码不合法（密码至少 8 位）"));
    }
    let user_id = uuid::Uuid::new_v4().to_string();
    let salt = uuid::Uuid::new_v4().simple().to_string();
    let hash = hash_password(&req.password, &salt);
    let token = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO users(id,email,salt,password_hash,display_name) VALUES(?,?,?,?,?)")
        .bind(&user_id)
        .bind(&req.email)
        .bind(&salt)
        .bind(&hash)
        .bind(&req.display_name)
        .execute(&state.pool)
        .await?;
    sqlx::query("INSERT INTO sessions(token,user_id) VALUES(?,?)")
        .bind(&token)
        .bind(&user_id)
        .execute(&state.pool)
        .await?;
    Ok(Json(AuthResponse {
        token,
        user_id,
        display_name: req.display_name,
    }))
}

async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let row: Option<(String, String, String, String)> =
        sqlx::query_as("SELECT id,salt,password_hash,display_name FROM users WHERE email=?")
            .bind(&req.email)
            .fetch_optional(&state.pool)
            .await?;
    let (user_id, salt, hash, name) = row.ok_or_else(|| AppError::auth("邮箱或密码错误"))?;
    if hash_password(&req.password, &salt) != hash {
        return Err(AppError::auth("邮箱或密码错误"));
    }
    let token = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO sessions(token,user_id) VALUES(?,?)")
        .bind(&token)
        .bind(&user_id)
        .execute(&state.pool)
        .await?;
    Ok(Json(AuthResponse {
        token,
        user_id,
        display_name: name,
    }))
}

trait Not {
    fn not(&self) -> bool;
}
impl Not for bool {
    fn not(&self) -> bool {
        !self
    }
}
impl Not for &str {
    fn not(&self) -> bool {
        self.is_empty()
    }
}

async fn auth_middleware(
    State(state): State<AppState>,
    req: axum::extract::Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| AppError::auth("缺少认证令牌"))?
        .to_owned();
    let user_id: String = sqlx::query_scalar("SELECT user_id FROM sessions WHERE token=?")
        .bind(&token)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::auth("令牌无效或已过期"))?;
    let mut req = req;
    req.extensions_mut()
        .insert(CurrentUser { id: user_id, token });
    Ok(next.run(req).await)
}

#[derive(Clone)]
struct CurrentUser {
    id: String,
    #[allow(dead_code)]
    token: String,
}

// ===== 项目与授权 =====

#[derive(Deserialize)]
struct CreateProjectRequest {
    title: String,
}

#[derive(Serialize)]
struct ProjectResponse {
    id: String,
    title: String,
    owner: bool,
    role: Option<String>,
}

async fn create_project(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<CurrentUser>,
    Json(req): Json<CreateProjectRequest>,
) -> Result<Json<ProjectResponse>, AppError> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO cloud_projects(id,owner_id,title) VALUES(?,?,?)")
        .bind(&id)
        .bind(&user.id)
        .bind(&req.title)
        .execute(&state.pool)
        .await?;
    Ok(Json(ProjectResponse {
        id,
        title: req.title,
        owner: true,
        role: Some("owner".into()),
    }))
}

#[derive(Deserialize)]
struct GrantRequest {
    grantee_email: String,
    role: String,
}

async fn grant_access(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<CurrentUser>,
    Path(project_id): Path<String>,
    Json(req): Json<GrantRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // 只有 owner 可以授权。
    let owner: Option<String> =
        sqlx::query_scalar("SELECT owner_id FROM cloud_projects WHERE id=?")
            .bind(&project_id)
            .fetch_optional(&state.pool)
            .await?;
    let owner = owner.ok_or_else(|| AppError::not_found("项目不存在"))?;
    if owner != user.id {
        return Err(AppError::forbidden("只有项目所有者可以授权"));
    }
    if !matches!(req.role.as_str(), "viewer" | "reviewer") {
        return Err(AppError::validation("角色不合法"));
    }
    let grantee: Option<String> = sqlx::query_scalar("SELECT id FROM users WHERE email=?")
        .bind(&req.grantee_email)
        .fetch_optional(&state.pool)
        .await?;
    let grantee = grantee.ok_or_else(|| AppError::not_found("被邀请者未注册"))?;
    if grantee == user.id {
        return Err(AppError::validation("不能授权给自己"));
    }
    let grant_id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT OR REPLACE INTO project_grants(id,project_id,grantee_id,role,state) VALUES(?,?,?,?,'active')")
        .bind(&grant_id).bind(&project_id).bind(&grantee).bind(&req.role)
        .execute(&state.pool).await?;
    Ok(Json(
        json!({"grant_id": grant_id, "grantee": req.grantee_email, "role": req.role, "state": "active"}),
    ))
}

async fn revoke_access(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<CurrentUser>,
    Path((project_id, _grant_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let owner: Option<String> =
        sqlx::query_scalar("SELECT owner_id FROM cloud_projects WHERE id=?")
            .bind(&project_id)
            .fetch_optional(&state.pool)
            .await?;
    let owner = owner.ok_or_else(|| AppError::not_found("项目不存在"))?;
    if owner != user.id {
        return Err(AppError::forbidden("只有项目所有者可以撤销"));
    }
    let changed = sqlx::query("UPDATE project_grants SET state='revoked', revoked_at=CURRENT_TIMESTAMP WHERE project_id=? AND state='active'")
        .bind(&project_id)
        .execute(&state.pool)
        .await?
        .rows_affected();
    Ok(Json(json!({"revoked": changed})))
}

// ===== 预览读取（S02 核心：撤销即拒） =====

async fn list_previews(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<CurrentUser>,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !can_read(&state.pool, &user.id, &project_id).await? {
        return Err(AppError::forbidden("无权访问此项目"));
    }
    let files: Vec<(String, i64, i64, i64)> = sqlx::query_as(
        "SELECT sha256,width,height,file_size FROM preview_files WHERE project_id=?",
    )
    .bind(&project_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({
        "project_id": project_id,
        "previews": files.iter().map(|(sha, w, h, size)| json!({
            "sha256_prefix": &sha[..16], "width": w, "height": h, "size": size,
            "url": format!("/projects/{project_id}/previews/{sha}.jpg"),
        })).collect::<Vec<_>>(),
    })))
}

async fn serve_preview(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<CurrentUser>,
    Path((project_id, filename)): Path<(String, String)>,
) -> Result<Response, AppError> {
    // S02 核心：每次请求都检查授权状态——撤销后立即拒绝。
    if !can_read(&state.pool, &user.id, &project_id).await? {
        return Err(AppError::forbidden("访问已被撤销"));
    }
    let sha = filename.trim_end_matches(".jpg");
    let exists: Option<i64> =
        sqlx::query_scalar("SELECT 1 FROM preview_files WHERE project_id=? AND sha256=?")
            .bind(&project_id)
            .bind(sha)
            .fetch_optional(&state.pool)
            .await?;
    if exists.is_none() {
        return Err(AppError::not_found("预览不存在"));
    }
    let path = state.preview_dir.join(format!("{sha}-1400.jpg"));
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|_| AppError::not_found("预览文件缺失"))?;
    Ok((
        [
            (header::CONTENT_TYPE, "image/jpeg"),
            (header::CACHE_CONTROL, "private, no-store"),
        ],
        bytes,
    )
        .into_response())
}

async fn can_read(pool: &SqlitePool, user_id: &str, project_id: &str) -> Result<bool, AppError> {
    let owner: Option<String> =
        sqlx::query_scalar("SELECT owner_id FROM cloud_projects WHERE id=?")
            .bind(project_id)
            .fetch_optional(pool)
            .await?;
    if owner.as_deref() == Some(user_id) {
        return Ok(true);
    }
    let grant: Option<String> = sqlx::query_scalar(
        "SELECT state FROM project_grants WHERE project_id=? AND grantee_id=? ORDER BY created_at DESC LIMIT 1",
    )
    .bind(project_id).bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(grant.as_deref() == Some("active"))
}

// ===== 错误 =====

struct AppError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl AppError {
    fn auth(msg: &str) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: "UNAUTHORIZED",
            message: msg.into(),
        }
    }
    fn forbidden(msg: &str) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            code: "PERMISSION_DENIED",
            message: msg.into(),
        }
    }
    fn not_found(msg: &str) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "NOT_FOUND",
            message: msg.into(),
        }
    }
    fn validation(msg: &str) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "VALIDATION_ERROR",
            message: msg.into(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({"error": {"code": self.code, "message": self.message}})),
        )
            .into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "DATABASE",
            message: e.to_string(),
        }
    }
}

// ===== 服务器 =====

pub async fn build_router(pool: SqlitePool, preview_dir: std::path::PathBuf) -> Router {
    crate::schema::migrate(&pool).await.expect("数据库迁移失败");
    let state = AppState { pool, preview_dir };
    let protected = Router::new()
        .route("/projects", post(create_project))
        .route("/projects/{id}/grants", post(grant_access))
        .route("/projects/{id}/grants/{gid}", delete(revoke_access))
        .route("/projects/{id}/previews", get(list_previews))
        .route("/projects/{id}/previews/{file}", get(serve_preview))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));
    Router::new()
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/health", get(|| async { Json(json!({"ok": true})) }))
        .merge(protected)
        .with_state(state)
}
