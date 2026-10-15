// v2.0 M4 Promotion 状态机（S01）：LOCAL→SHARED 显式迁移，失败可恢复，绝不双主。
// 本地为协议层实现：真实上传在 M4 云服务部署后接入；状态机与不变量现在就可验证。
use crate::{
    db::Store,
    domain::{self, AppError, Result},
};
use serde_json::{Value, json};
use sqlx::Row;

/// Promotion 开始：项目必须处于 LOCAL 且无进行中的 promotion；
/// 迁移期间本地写入被暂停（mode 进入 PROMOTING——由调用方在业务层执行）。
pub async fn begin(store: &Store, project_id: &str, initiator: &str) -> Result<Value> {
    let project = store.get(project_id).await?;
    let mut tx = store.pool.begin_with("BEGIN IMMEDIATE").await?;
    // 不变量 1：同一项目同时只能有一个未终止的 promotion。
    let active: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM v2_promotions WHERE project_id=? AND state IN('pending','uploading','verifying')",
    )
    .bind(project_id)
    .fetch_one(&mut *tx)
    .await?;
    if active > 0 {
        return Err(AppError::new(
            "INVALID_TRANSITION",
            "已有进行中的共享迁移；须等待完成或显式中止",
        ));
    }
    let now = domain::timestamp();
    let id = domain::id();
    sqlx::query("INSERT INTO v2_promotions(id, project_id, initiated_by, state, from_revision, created_at, updated_at) VALUES(?,?,?,'pending',?,?,?)")
        .bind(&id).bind(project_id).bind(initiator).bind(project.revision).bind(&now).bind(&now)
        .execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(json!({"promotion_id": id, "state": "pending", "from_revision": project.revision}))
}

/// 状态推进：pending→uploading→verifying→completed；任何未终止态可→failed/aborted。
/// completed 前置条件：云侧确认回执（M4 真实接入后为签名回执；测试注入确认值）。
pub async fn advance(
    store: &Store,
    promotion_id: &str,
    to: &str,
    cloud_confirmation: Option<&str>,
) -> Result<Value> {
    let mut tx = store.pool.begin_with("BEGIN IMMEDIATE").await?;
    let row = sqlx::query("SELECT project_id, state, from_revision FROM v2_promotions WHERE id=?")
        .bind(promotion_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::new("NOT_FOUND", "迁移不存在"))?;
    let (project_id, state, from_revision): (String, String, i64) = (
        row.get("project_id"),
        row.get("state"),
        row.get("from_revision"),
    );
    let legal = match (state.as_str(), to) {
        ("pending", "uploading") | ("uploading", "verifying") => true,
        (_, "failed") if !matches!(state.as_str(), "completed" | "failed" | "aborted") => true,
        (_, "aborted") if !matches!(state.as_str(), "completed" | "failed" | "aborted") => true,
        ("verifying", "completed") => cloud_confirmation.is_some_and(|c| !c.is_empty()),
        _ => false,
    };
    if !legal {
        return Err(AppError::new(
            "INVALID_TRANSITION",
            format!("{state}→{to} 不合法；completed 需要云侧确认回执"),
        ));
    }
    sqlx::query("UPDATE v2_promotions SET state=?, updated_at=?, detail_json=json_set(detail_json,'$.last_transition',json_quote(?)) WHERE id=?")
        .bind(to).bind(domain::timestamp()).bind(format!("{state}->{to}")).bind(promotion_id)
        .execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(
        json!({"promotion_id": promotion_id, "state": to, "project_id": project_id, "from_revision": from_revision}),
    )
}

/// S01 核心断言：查询某项目当前的权威端。completed 前为 local（迁移中本地暂停写由业务层
/// 用 PROMOTING 门槛实现），completed 后 shared；failed/aborted 回落 local。
pub async fn authority_of(store: &Store, project_id: &str) -> Result<Value> {
    let state: Option<String> = sqlx::query_scalar(
        "SELECT state FROM v2_promotions WHERE project_id=? AND state IN('pending','uploading','verifying','completed') ORDER BY updated_at DESC LIMIT 1",
    )
    .bind(project_id)
    .fetch_optional(&store.pool)
    .await?;
    let authority = match state.as_deref() {
        Some("completed") => "shared",
        Some(_) => "local-promoting", // 迁移中：本地写入暂停，等待完成或中止
        None => "local",
    };
    Ok(json!({"project_id": project_id, "authority": authority}))
}
