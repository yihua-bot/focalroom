//! M4 垂直切片 handlers。：登录 → 项目授权 → Web 读取预览 → 撤销即拒绝。
//! S02 验收：撤销后任何预览 URL 返回 403。
//! 本切片仅为 SQLite 本地协议验证；PostgreSQL 需要独立实现与验收。

use crate::{
    auth::{self, CurrentUser},
    collab,
    error::AppError,
    previews,
};
use axum::{
    extract::{DefaultBodyLimit, Path, State},
    http::header,
    middleware::{self, Next},
    response::{Json, Response},
    routing::{delete, get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub preview_dir: std::path::PathBuf,
    pub auth_slots: std::sync::Arc<tokio::sync::Semaphore>,
}

// ===== 项目与授权 =====

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
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
    if req.title.trim().is_empty() || req.title.len() > 500 {
        return Err(AppError::validation("项目名称须为 1–500 字节"));
    }
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
#[serde(deny_unknown_fields)]
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
        .bind(auth::normalized_email(&req.grantee_email)?)
        .fetch_optional(&state.pool)
        .await?;
    let grantee = grantee.ok_or_else(|| AppError::not_found("被邀请者未注册"))?;
    if grantee == user.id {
        return Err(AppError::validation("不能授权给自己"));
    }
    let grant_id = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO project_grants(id,project_id,grantee_id,role,state) VALUES(?,?,?,?,'active') ON CONFLICT(project_id,grantee_id) DO UPDATE SET role=excluded.role,state='active',revoked_at=NULL")
        .bind(&grant_id).bind(&project_id).bind(&grantee).bind(&req.role)
        .execute(&state.pool).await?;
    let grant_id: String =
        sqlx::query_scalar("SELECT id FROM project_grants WHERE project_id=? AND grantee_id=?")
            .bind(&project_id)
            .bind(&grantee)
            .fetch_one(&state.pool)
            .await?;
    Ok(Json(
        json!({"grant_id": grant_id, "grantee": req.grantee_email, "role": req.role, "state": "active"}),
    ))
}

async fn revoke_access(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<CurrentUser>,
    Path((project_id, grant_id)): Path<(String, String)>,
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
    let changed = sqlx::query("UPDATE project_grants SET state='revoked', revoked_at=CURRENT_TIMESTAMP WHERE project_id=? AND id=? AND state='active'")
        .bind(&project_id)
        .bind(&grant_id)
        .execute(&state.pool)
        .await?
        .rows_affected();
    Ok(Json(json!({"revoked": changed})))
}

pub(crate) async fn can_read(
    pool: &SqlitePool,
    user_id: &str,
    project_id: &str,
) -> Result<bool, AppError> {
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

// ===== 服务器 =====

pub async fn build_router(pool: SqlitePool, preview_dir: std::path::PathBuf) -> Router {
    crate::schema::migrate(&pool).await.expect("数据库迁移失败");
    crate::schema::migrate_collab(&pool)
        .await
        .expect("协作表迁移失败");
    let state = AppState {
        pool,
        preview_dir,
        auth_slots: std::sync::Arc::new(tokio::sync::Semaphore::new(4)),
    };
    let protected = Router::new()
        .route("/auth/logout", post(auth::logout))
        .route("/projects", post(create_project).get(list_my_projects))
        .route("/projects/{id}/grants", post(grant_access))
        .route("/projects/{id}/grants/{gid}", delete(revoke_access))
        .route("/projects/{id}/previews", get(previews::list_previews))
        .route(
            "/projects/{id}/previews/{file}",
            get(previews::serve_preview),
        )
        // M5 协作：评论/提案/审批/快照/AI 评论访问
        .route(
            "/projects/{id}/comments",
            post(collab::create_comment).get(collab::list_comments),
        )
        .route(
            "/projects/{id}/comments/{cid}/resolve",
            post(collab::resolve_comment),
        )
        .route("/projects/{id}/proposals", post(collab::create_proposal))
        .route(
            "/projects/{id}/proposals/{pid}/decide",
            post(collab::decide_proposal),
        )
        .route("/projects/{id}/snapshot", get(collab::project_snapshot))
        .route("/ai/comments", get(collab::ai_accessible_comments))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::auth_middleware,
        ));
    Router::new()
        .route("/", get(serve_index))
        .route("/auth/register", post(auth::register))
        .route("/auth/login", post(auth::login))
        .route("/health", get(|| async { Json(json!({"ok": true})) }))
        .merge(protected)
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(middleware::from_fn(private_response))
        .with_state(state)
}

// 认证响应、预览列表及拒绝响应均禁止代理/浏览器持久缓存。
async fn private_response(req: axum::extract::Request, next: Next) -> Response {
    let mut response = next.run(req).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        "private, no-store".parse().expect("静态头有效"),
    );
    response
        .headers_mut()
        .insert(header::VARY, "Authorization".parse().expect("静态头有效"));
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        "nosniff".parse().expect("静态头有效"),
    );
    response
}

async fn serve_index() -> impl axum::response::IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        include_str!("../static/index.html"),
    )
}

async fn list_my_projects(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<CurrentUser>,
) -> Result<Json<serde_json::Value>, AppError> {
    let owned: Vec<(String, String)> = sqlx::query_as(
        "SELECT id, title FROM cloud_projects WHERE owner_id=? ORDER BY created_at DESC",
    )
    .bind(&user.id)
    .fetch_all(&state.pool)
    .await?;
    let granted: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT p.id, p.title, g.role FROM cloud_projects p JOIN project_grants g ON g.project_id=p.id WHERE g.grantee_id=? AND g.state='active'",
    )
    .bind(&user.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(serde_json::json!({
        "owned": owned.iter().map(|(id, t)| json!({"id": id, "title": t, "role": "owner"})).collect::<Vec<_>>(),
        "shared": granted.iter().map(|(id, t, r)| json!({"id": id, "title": t, "role": r})).collect::<Vec<_>>(),
    })))
}
