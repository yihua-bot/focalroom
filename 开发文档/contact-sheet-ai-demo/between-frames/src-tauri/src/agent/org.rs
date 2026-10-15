// v2.0 M6 组织/席位本地骨架：席位上限、角色撤回、审计写入（ADR 0021）。
// 本地默认单组织单所有者；SHARED 云同步在 M4 落地，本模块不冒充云能力。
use crate::{
    db::Store,
    domain::{self, AppError, Result},
};
use serde_json::{Value, json};
use sqlx::Row;

pub async fn ensure_workspace(
    db: &Store,
    workspace: &str,
    owner: &str,
    name: &str,
    seats: i64,
) -> Result<()> {
    if !(1..=500).contains(&seats) {
        return Err(AppError::new("VALIDATION_ERROR", "席位上限须为 1–500"));
    }
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    sqlx::query("INSERT OR IGNORE INTO v2_creators(id, display_name, kind, created_at) VALUES(?,?, 'individual', ?)")
        .bind(owner).bind(name).bind(domain::timestamp())
        .execute(&mut *tx).await?;
    sqlx::query("INSERT OR IGNORE INTO v2_workspaces(id, name, owner_creator_id, seats_limit, created_at) VALUES(?,?,?,?,?)")
        .bind(workspace).bind(name).bind(owner).bind(seats).bind(domain::timestamp())
        .execute(&mut *tx).await?;
    sqlx::query("INSERT OR IGNORE INTO v2_memberships(workspace_id, creator_id, role, created_at) VALUES(?,?,'OWNER', ?)")
        .bind(workspace).bind(owner).bind(domain::timestamp())
        .execute(&mut *tx).await?;
    audit(
        &mut tx,
        workspace,
        "workspace_ensured",
        json!({"owner": owner, "seats": seats}),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// 加入席位：受席位上限约束；重复加入幂等；被撤回者不可自行恢复。
pub async fn add_member(db: &Store, workspace: &str, creator: &str, role: &str) -> Result<()> {
    if !matches!(role, "ADMIN" | "EDITOR" | "REVIEWER" | "VIEWER") {
        return Err(AppError::new(
            "VALIDATION_ERROR",
            "角色不合法（OWNER 由创建产生）",
        ));
    }
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM v2_workspaces WHERE id=?")
        .bind(workspace)
        .fetch_one(&mut *tx)
        .await?;
    if exists == 0 {
        return Err(AppError::new("NOT_FOUND", "工作区不存在"));
    }
    let revoked: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM v2_memberships WHERE workspace_id=? AND creator_id=? AND state='revoked'",
    )
    .bind(workspace).bind(creator).fetch_one(&mut *tx).await?;
    if revoked > 0 {
        return Err(AppError::new(
            "PERMISSION_DENIED",
            "成员已被撤回；恢复须由所有者执行",
        ));
    }
    let seats: (i64,) = sqlx::query_as("SELECT seats_limit FROM v2_workspaces WHERE id=?")
        .bind(workspace)
        .fetch_one(&mut *tx)
        .await?;
    let active: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM v2_memberships WHERE workspace_id=? AND state='active'",
    )
    .bind(workspace)
    .fetch_one(&mut *tx)
    .await?;
    if active >= seats.0 {
        return Err(AppError::new(
            "BUDGET_EXCEEDED",
            format!("席位已满（{}/{}）；请先扩容或移除成员", active, seats.0),
        ));
    }
    sqlx::query("INSERT INTO v2_creators(id, display_name, created_at) VALUES(?, ?, ?) ON CONFLICT(id) DO NOTHING")
        .bind(creator).bind(creator).bind(domain::timestamp())
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO v2_memberships(workspace_id, creator_id, role, created_at) VALUES(?,?,?,?) ON CONFLICT(workspace_id, creator_id) DO UPDATE SET role=excluded.role, state='active'")
        .bind(workspace).bind(creator).bind(role).bind(domain::timestamp())
        .execute(&mut *tx).await?;
    audit(
        &mut tx,
        workspace,
        "member_added",
        json!({"creator": creator, "role": role}),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

/// 撤回成员席位：OWNER 不可被撤回（须先转移所有权——M4 云端流程）。
pub async fn revoke_member(db: &Store, workspace: &str, creator: &str) -> Result<()> {
    let mut tx = db.pool.begin_with("BEGIN IMMEDIATE").await?;
    let row: Option<(String,)> =
        sqlx::query_as("SELECT role FROM v2_memberships WHERE workspace_id=? AND creator_id=?")
            .bind(workspace)
            .bind(creator)
            .fetch_optional(&mut *tx)
            .await?;
    let (role,) = row.ok_or_else(|| AppError::new("NOT_FOUND", "成员不存在"))?;
    if role == "OWNER" {
        return Err(AppError::new(
            "PERMISSION_DENIED",
            "所有者席位不可撤回；请先转移所有权",
        ));
    }
    sqlx::query("UPDATE v2_memberships SET state='revoked' WHERE workspace_id=? AND creator_id=?")
        .bind(workspace)
        .bind(creator)
        .execute(&mut *tx)
        .await?;
    audit(
        &mut tx,
        workspace,
        "member_revoked",
        json!({"creator": creator}),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn members(db: &Store, workspace: &str) -> Result<Value> {
    let rows: Vec<(String, String, String)> = sqlx::query(
        "SELECT creator_id, role, state FROM v2_memberships WHERE workspace_id=? ORDER BY created_at",
    )
    .bind(workspace)
    .fetch_all(&db.pool)
    .await?
    .iter()
    .map(|r| (r.get("creator_id"), r.get("role"), r.get("state")))
    .collect();
    Ok(
        json!({"members": rows.iter().map(|(id, role, state)| json!({"creator": id, "role": role, "state": state})).collect::<Vec<_>>()}),
    )
}

async fn audit(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    workspace: &str,
    event: &str,
    detail: Value,
) -> Result<()> {
    sqlx::query("INSERT INTO v2_workspace_audit(workspace_id, event, detail_json, created_at) VALUES(?,?,?,?)")
        .bind(workspace).bind(event).bind(detail.to_string()).bind(domain::timestamp())
        .execute(&mut **tx).await?;
    Ok(())
}
