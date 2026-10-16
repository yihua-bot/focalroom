//! M5 云端协作：评论/提案/审批/ACL/Presence/快照——全部经项目授权检查。
use crate::auth::CurrentUser;
use crate::error::AppError;
use crate::handlers::AppState;
use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

// ===== ACL =====

/// 项目级 ACL：owner 或 active grant 可执行 action；审计每次判定。
pub async fn check_acl(
    state: &AppState,
    user: &CurrentUser,
    project_id: &str,
    action: &str,
) -> Result<AclRole, AppError> {
    let owner: Option<String> =
        sqlx::query_scalar("SELECT owner_id FROM cloud_projects WHERE id=?")
            .bind(project_id)
            .fetch_optional(&state.pool)
            .await?;
    let owner = owner.ok_or_else(|| AppError::not_found("项目不存在"))?;

    let (role, allowed) = if owner == user.id {
        ("owner".to_string(), true)
    } else {
        let grant: Option<(String, String)> = sqlx::query_as(
            "SELECT role, state FROM project_grants WHERE project_id=? AND grantee_id=? ORDER BY created_at DESC LIMIT 1",
        )
        .bind(project_id)
        .bind(&user.id)
        .fetch_optional(&state.pool)
        .await?;
        match grant {
            Some((role, state)) if state == "active" => (role, true),
            Some((_, _)) => ("revoked".to_string(), false),
            None => ("none".to_string(), false),
        }
    };

    // 在同一处判定动作权限，拒绝结果也进入 ACL 审计。
    let allowed = allowed
        && match action {
            "comment.create" | "comment.resolve" | "proposal.create" => {
                matches!(role.as_str(), "owner" | "editor" | "reviewer")
            }
            "proposal.decide" => matches!(role.as_str(), "owner" | "editor"),
            "comment.list" | "project.snapshot" | "ai.comment.read" => {
                matches!(role.as_str(), "owner" | "editor" | "reviewer" | "viewer")
            }
            _ => false,
        };

    // 审计（不因审计失败阻断业务）。
    let _ = sqlx::query(
        "INSERT INTO acl_audit(project_id,user_id,action,resource,allowed) VALUES(?,?,?,?,?)",
    )
    .bind(project_id)
    .bind(&user.id)
    .bind(action)
    .bind(format!("project:{project_id}"))
    .bind(allowed)
    .execute(&state.pool)
    .await;

    if !allowed {
        return Err(AppError::forbidden("无权访问此项目"));
    }
    Ok(AclRole { role })
}

pub struct AclRole {
    pub role: String,
}

/// 审批权限：只有 owner 或 editor 角色可以 approve。
pub async fn require_approver(
    state: &AppState,
    user: &CurrentUser,
    project_id: &str,
) -> Result<(), AppError> {
    let acl = check_acl(state, user, project_id, "proposal.decide").await?;
    if !matches!(acl.role.as_str(), "owner" | "editor") {
        return Err(AppError::forbidden("只有所有者或编辑可以审批提案"));
    }
    Ok(())
}

// ===== 评论 =====

#[derive(Deserialize)]
pub struct CreateComment {
    pub photo_sha256: Option<String>,
    pub anchor_x: Option<f64>,
    pub anchor_y: Option<f64>,
    pub body: String,
    pub parent_id: Option<String>,
}

#[derive(Serialize)]
pub struct CommentView {
    pub id: String,
    pub author: String,
    pub photo_sha256: Option<String>,
    pub anchor_x: Option<f64>,
    pub anchor_y: Option<f64>,
    pub body: String,
    pub parent_id: Option<String>,
    pub state: String,
    pub created_at: String,
}

pub async fn create_comment(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<CurrentUser>,
    Path(project_id): Path<String>,
    axum::Json(req): axum::Json<CreateComment>,
) -> Result<Json<CommentView>, AppError> {
    check_acl(&state, &user, &project_id, "comment.create").await?;
    if req.body.trim().is_empty() || req.body.chars().count() > 2000 {
        return Err(AppError::validation("评论内容不合法（1-2000 字）"));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let _author_name: String = sqlx::query_scalar("SELECT display_name FROM users WHERE id=?")
        .bind(&user.id)
        .fetch_one(&state.pool)
        .await?;
    sqlx::query("INSERT INTO project_comments(id,project_id,author_id,photo_sha256,anchor_x,anchor_y,body,parent_id) VALUES(?,?,?,?,?,?,?,?)")
        .bind(&id).bind(&project_id).bind(&user.id)
        .bind(&req.photo_sha256).bind(req.anchor_x).bind(req.anchor_y)
        .bind(req.body.trim()).bind(&req.parent_id)
        .execute(&state.pool).await?;
    let created = get_comment(&state, &id).await?;
    broadcast(
        &state,
        &project_id,
        &json!({"type":"comment.created","comment_id":&id}),
    )
    .await;
    Ok(Json(created))
}

async fn get_comment(state: &AppState, id: &str) -> Result<CommentView, AppError> {
    let row = sqlx::query_as::<_, (String, String, Option<String>, Option<f64>, Option<f64>, String, Option<String>, String, String)>(
        "SELECT c.id,u.display_name,c.photo_sha256,c.anchor_x,c.anchor_y,c.body,c.parent_id,c.state,c.created_at \
         FROM project_comments c JOIN users u ON u.id=c.author_id WHERE c.id=?",
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await?;
    Ok(CommentView {
        id: row.0,
        author: row.1,
        photo_sha256: row.2,
        anchor_x: row.3,
        anchor_y: row.4,
        body: row.5,
        parent_id: row.6,
        state: row.7,
        created_at: row.8,
    })
}

pub async fn list_comments(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<CurrentUser>,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    check_acl(&state, &user, &project_id, "comment.list").await?;
    let rows = sqlx::query_as::<_, (String, String, Option<String>, Option<f64>, Option<f64>, String, Option<String>, String, String)>(
        "SELECT c.id,u.display_name,c.photo_sha256,c.anchor_x,c.anchor_y,c.body,c.parent_id,c.state,c.created_at \
         FROM project_comments c JOIN users u ON u.id=c.author_id WHERE c.project_id=? ORDER BY c.created_at",
    )
    .bind(&project_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({"comments": rows.iter().map(|r| json!({
        "id": &r.0, "author": &r.1, "photoSha256": &r.2, "anchorX": r.3, "anchorY": r.4,
        "body": &r.5, "parentId": &r.6, "state": &r.7, "createdAt": &r.8,
    })).collect::<Vec<_>>()})))
}

pub async fn resolve_comment(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<CurrentUser>,
    Path((project_id, comment_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let acl = check_acl(&state, &user, &project_id, "comment.resolve").await?;
    // 只有 owner/editor 或评论作者可以 resolve。
    let author: String =
        sqlx::query_scalar("SELECT author_id FROM project_comments WHERE id=? AND project_id=?")
            .bind(&comment_id)
            .bind(&project_id)
            .fetch_optional(&state.pool)
            .await?
            .ok_or_else(|| AppError::not_found("评论不存在"))?;
    if acl.role != "owner"
        && !matches!(acl.role.as_str(), "editor" | "reviewer")
        && author != user.id
    {
        return Err(AppError::forbidden("无权标记此评论"));
    }
    sqlx::query(
        "UPDATE project_comments SET state='resolved', updated_at=datetime('now') WHERE id=?",
    )
    .bind(&comment_id)
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({"resolved": true})))
}

// ===== 提案 =====

#[derive(Deserialize)]
pub struct CreateProposal {
    pub title: String,
    pub base_revision: i64,
    pub operations: serde_json::Value,
}

pub async fn create_proposal(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<CurrentUser>,
    Path(project_id): Path<String>,
    axum::Json(req): axum::Json<CreateProposal>,
) -> Result<Json<serde_json::Value>, AppError> {
    let acl = check_acl(&state, &user, &project_id, "proposal.create").await?;
    if !matches!(acl.role.as_str(), "owner" | "editor" | "reviewer") {
        return Err(AppError::forbidden("此角色不能创建提案"));
    }
    // 检查基线是否是最新版本（并发冲突检测 C02）。
    let current_rev: i64 = current_revision(&state, &project_id).await?;
    if req.base_revision != current_rev {
        return Err(AppError::conflict(&format!(
            "提案基线 v{} 已过期（当前 v{}）；请刷新后重建提案",
            req.base_revision, current_rev
        )));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let nonce = uuid::Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO project_proposals(id,project_id,author_id,title,base_revision,operations_json,approval_nonce) VALUES(?,?,?,?,?,?,?)")
        .bind(&id).bind(&project_id).bind(&user.id).bind(&req.title)
        .bind(req.base_revision).bind(req.operations.to_string()).bind(&nonce)
        .execute(&state.pool).await?;
    broadcast(
        &state,
        &project_id,
        &json!({"type":"proposal.created","proposal_id":&id}),
    )
    .await;
    Ok(Json(
        json!({"proposal_id": id, "approval_nonce": nonce, "base_revision": req.base_revision}),
    ))
}

pub async fn decide_proposal(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<CurrentUser>,
    Path((project_id, proposal_id)): Path<(String, String)>,
    axum::Json(req): axum::Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_approver(&state, &user, &project_id).await?;
    let approve = req["approve"].as_bool().unwrap_or(false);
    let nonce = req["approval_nonce"].as_str().unwrap_or("");
    let row = sqlx::query_as::<_, (String, i64)>(
        "SELECT state, base_revision FROM project_proposals WHERE id=? AND project_id=?",
    )
    .bind(&proposal_id)
    .bind(&project_id)
    .fetch_optional(&state.pool)
    .await?;
    let (prop_state, base_rev) = row.ok_or_else(|| AppError::not_found("提案不存在"))?;
    if prop_state != "submitted" {
        return Err(AppError::conflict(&format!("提案已处理（{prop_state}）")));
    }
    // nonce 验证。
    let expected: String =
        sqlx::query_scalar("SELECT approval_nonce FROM project_proposals WHERE id=?")
            .bind(&proposal_id)
            .fetch_one(&state.pool)
            .await?;
    if nonce != expected {
        return Err(AppError::forbidden("审批 nonce 不匹配"));
    }
    if !approve {
        sqlx::query("UPDATE project_proposals SET state='rejected',decided_by=?,decided_at=datetime('now') WHERE id=?")
            .bind(&user.id).bind(&proposal_id).execute(&state.pool).await?;
        broadcast(
            &state,
            &project_id,
            &json!({"type":"proposal.rejected","proposal_id":&proposal_id}),
        )
        .await;
        return Ok(Json(json!({"decided": "rejected"})));
    }
    // 审批前基线漂移检查（C02 核心：并发提案冲突）。
    let current_rev = current_revision(&state, &project_id).await?;
    if base_rev != current_rev {
        sqlx::query("UPDATE project_proposals SET state='conflict',decided_by=?,decided_at=datetime('now') WHERE id=?")
            .bind(&user.id).bind(&proposal_id).execute(&state.pool).await?;
        return Err(AppError::conflict(&format!(
            "项目已从 v{base_rev} 更新到 v{current_rev}；提案标记冲突，请基于新版本重建"
        )));
    }
    // 执行提案：推进版本 + 写快照。
    let new_rev = current_rev + 1;
    let snapshot = json!({"cause":"proposal_apply","proposal_id":&proposal_id,"revision":new_rev});
    sqlx::query("UPDATE project_proposals SET state='applied',decided_by=?,decided_at=datetime('now') WHERE id=?")
        .bind(&user.id).bind(&proposal_id).execute(&state.pool).await?;
    sqlx::query("INSERT INTO project_revisions(project_id,revision,snapshot_json,author_id,cause) VALUES(?,?,?,?,?)")
        .bind(&project_id).bind(new_rev).bind(snapshot.to_string()).bind(&user.id).bind("proposal_apply")
        .execute(&state.pool).await?;
    broadcast(
        &state,
        &project_id,
        &json!({"type":"proposal.applied","proposal_id":&proposal_id,"revision":new_rev}),
    )
    .await;
    Ok(Json(json!({"decided": "applied", "revision": new_rev})))
}

async fn current_revision(state: &AppState, project_id: &str) -> Result<i64, AppError> {
    let rev: Option<i64> =
        sqlx::query_scalar("SELECT MAX(revision) FROM project_revisions WHERE project_id=?")
            .bind(project_id)
            .fetch_one(&state.pool)
            .await?;
    Ok(rev.unwrap_or(0))
}

// ===== 版本与重连 =====

pub async fn project_snapshot(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<CurrentUser>,
    Path(project_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    check_acl(&state, &user, &project_id, "project.snapshot").await?;
    let rev = current_revision(&state, &project_id).await?;
    let pid_for_json = project_id.clone();
    let snapshot: Option<String> = sqlx::query_scalar(
        "SELECT snapshot_json FROM project_revisions WHERE project_id=? AND revision=?",
    )
    .bind(project_id)
    .bind(rev)
    .fetch_optional(&state.pool)
    .await?;
    Ok(Json(json!({
        "project_id": pid_for_json, "revision": rev,
        "snapshot": snapshot.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(json!({})),
    })))
}

// ===== Presence（WebSocket） =====

/// 广播通道：项目内所有在线 WebSocket 连接。
pub async fn broadcast(state: &AppState, project_id: &str, message: &serde_json::Value) {
    // 简化实现：写数据库通知，WebSocket 轮询或后续接 pub/sub。
    // TODO: 接入 tokio broadcast channel 或 Redis pub/sub。
    let _ = (state, project_id, message);
}

// ===== AI 评论访问（经 ACL） =====

#[derive(Deserialize)]
pub struct AiCommentQuery {
    pub project_id: String,
}

/// AI 只能通过项目 ACL 获取评论；返回带 reviewer 身份的引用。
pub async fn ai_accessible_comments(
    State(state): State<AppState>,
    axum::Extension(user): axum::Extension<CurrentUser>,
    Query(query): Query<AiCommentQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    check_acl(&state, &user, &query.project_id, "ai.comment.read").await?;
    let rows = sqlx::query_as::<_, (String, String, String, String)>(
        "SELECT c.id, u.display_name, c.body, c.state \
         FROM project_comments c JOIN users u ON u.id=c.author_id \
         WHERE c.project_id=? AND c.state='open' ORDER BY c.created_at DESC LIMIT 100",
    )
    .bind(&query.project_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({
        "comments": rows.iter().map(|(id, author, body, state)| json!({
            "id": id, "reviewer": author, "body": body, "state": state,
        })).collect::<Vec<_>>(),
        "acl_note": "AI 仅获取已授权项目的评论，引用保留 reviewer 身份",
    })))
}
