//! 作者级跨项目记忆授权（M3）：显式开启后，同一本地作者的项目间共享偏好记忆检索。
//! 默认关闭（M02 隐私保守）；开启也不等于跨创作者——本桌面应用当前只有一位本地作者。
use crate::{
    db::Store,
    domain::{self, Result},
};
use serde::{Deserialize, Serialize};
use sqlx::Row;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryScopeSettings {
    pub cross_project_enabled: bool,
    pub updated_at: String,
}

/// 读取跨项目设置；无记录时返回保守默认（关闭）。
pub async fn get_settings(store: &Store, user_id: &str) -> Result<MemoryScopeSettings> {
    let row = sqlx::query(
        "SELECT cross_project_enabled,updated_at FROM memory_scope_settings WHERE user_id=?",
    )
    .bind(user_id)
    .fetch_optional(&store.pool)
    .await?;
    Ok(match row {
        Some(row) => MemoryScopeSettings {
            cross_project_enabled: row.get::<i64, _>("cross_project_enabled") == 1,
            updated_at: row.get("updated_at"),
        },
        None => MemoryScopeSettings {
            cross_project_enabled: false,
            updated_at: String::new(),
        },
    })
}

/// 设置跨项目授权；幂等。
pub async fn set_cross_project(
    store: &Store,
    user_id: &str,
    enabled: bool,
) -> Result<MemoryScopeSettings> {
    let now = domain::timestamp();
    sqlx::query(
        "INSERT INTO memory_scope_settings(user_id,cross_project_enabled,updated_at) VALUES(?,?,?) \
         ON CONFLICT(user_id) DO UPDATE SET cross_project_enabled=excluded.cross_project_enabled, updated_at=excluded.updated_at",
    )
    .bind(user_id)
    .bind(if enabled { 1 } else { 0 })
    .bind(&now)
    .execute(&store.pool)
    .await?;
    get_settings(store, user_id).await
}

/// 快捷判断（供检索路径内联调用）。
pub async fn cross_project_enabled(store: &Store) -> bool {
    get_settings(store, "local")
        .await
        .map(|s| s.cross_project_enabled)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn default_is_isolated_and_setting_is_idempotent() {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::open(&temp.path().join("m.sqlite")).await.unwrap();
        // 默认关闭。
        assert!(
            !get_settings(&store, "local")
                .await
                .unwrap()
                .cross_project_enabled
        );
        // 开启。
        let enabled = set_cross_project(&store, "local", true).await.unwrap();
        assert!(enabled.cross_project_enabled);
        assert!(cross_project_enabled(&store).await);
        // 幂等重复。
        let again = set_cross_project(&store, "local", true).await.unwrap();
        assert!(again.cross_project_enabled);
        // 关闭恢复。
        set_cross_project(&store, "local", false).await.unwrap();
        assert!(!cross_project_enabled(&store).await);
    }
}
