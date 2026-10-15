// v2.0 M4 Promotion 状态机（S01）：LOCAL→SHARED 显式迁移，失败可恢复，绝不双主。
// 本地为协议层实现：真实上传在 M4 云服务部署后接入；状态机与不变量现在就可验证。
use crate::{
    db::Store,
    domain::{self, AppError, Result},
};
use serde_json::{Value, json};
use sqlx::Row;

/// Promotion 开始：项目必须处于 LOCAL 且无进行中的 promotion；
/// 迁移状态与本地写入口共用写事务锁，旧 GUI 也不能绕过。
pub async fn begin(store: &Store, project_id: &str, initiator: &str) -> Result<Value> {
    let mut tx = store.pool.begin_with("BEGIN IMMEDIATE").await?;
    let project = crate::db::snapshot_tx(&mut tx, project_id, None).await?;
    if project.archived {
        return Err(AppError::new("NOT_FOUND", "项目已归档"));
    }
    ensure_local_write(&mut tx, project_id)
        .await
        .map_err(|_| AppError::new("INVALID_TRANSITION", "已有迁移或项目已共享，不能重新发起"))?;
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

/// 统一查询本地权威状态，completed 不因失败重试记录而回退。
pub(crate) async fn local_state(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    project_id: &str,
) -> Result<betweenframes_local_domain::LocalState> {
    let states: Vec<String> = sqlx::query_scalar("SELECT state FROM v2_promotions WHERE project_id=? AND state IN('pending','uploading','verifying','completed')")
        .bind(project_id).fetch_all(&mut **tx).await?;
    use betweenframes_local_domain::LocalState;
    Ok(if states.iter().any(|s| s == "completed") {
        LocalState::SHARED_CACHE
    } else if !states.is_empty() {
        LocalState::PROMOTING
    } else {
        LocalState::LOCAL
    })
}
pub(crate) async fn ensure_local_write(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    project_id: &str,
) -> Result<()> {
    if local_state(tx, project_id).await? != betweenframes_local_domain::LocalState::LOCAL {
        return Err(AppError::new(
            "AUTHORITY_MISMATCH",
            "迁移中的项目暂停本地写入；共享项目由云端持有写入权威",
        ));
    }
    Ok(())
}
pub async fn authority_of(store: &Store, project_id: &str) -> Result<Value> {
    let mut tx = store.pool.begin().await?;
    let state = local_state(&mut tx, project_id).await?;
    tx.commit().await?;
    use betweenframes_local_domain::LocalState;
    let authority = match state {
        LocalState::LOCAL => "local",
        LocalState::PROMOTING => "local-promoting",
        LocalState::SHARED_CACHE => "shared",
    };
    Ok(json!({"project_id":project_id,"authority":authority}))
}
