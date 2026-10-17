//! 本地作者记忆：授权经 CommandHandler 审计；任务绑定不可变来源摘要。
//! 当前桌面仅一个本地作者；不向云模型或共享项目开放此授权。
use super::{store, types::Task};
use crate::{
    command_audit::fingerprint,
    db::Store,
    domain::{self, AppError, Result},
};
use betweenframes_local_domain::LocalState;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Sqlite, Transaction};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryScopeSettings {
    pub cross_project_enabled: bool,
    pub updated_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Binding {
    version: u32,
    pub cross_project: bool,
    fingerprint: String,
}

async fn grant_epoch(tx: &mut Transaction<'_, Sqlite>) -> Result<i64> {
    Ok(sqlx::query_scalar("SELECT COUNT(*) FROM memory_commands WHERE changed=1 AND json_extract(command_json,'$.type')='SetCrossProjectMemory'")
        .fetch_one(&mut **tx).await?)
}
pub(crate) async fn settings(tx: &mut Transaction<'_, Sqlite>) -> Result<MemoryScopeSettings> {
    let raw: Option<(i64, String)> = sqlx::query_as(
        "SELECT cross_project_enabled,updated_at FROM memory_scope_settings WHERE user_id='local'",
    )
    .fetch_optional(&mut **tx)
    .await?;
    // 历史直接写入的 true 不能作为授权；最后一次有效审计必须明确开启。
    let approved: Option<bool> = sqlx::query_scalar("SELECT json_extract(command_json,'$.enabled') FROM memory_commands WHERE changed=1 AND json_extract(command_json,'$.type')='SetCrossProjectMemory' ORDER BY rowid DESC LIMIT 1")
        .fetch_optional(&mut **tx).await?;
    Ok(MemoryScopeSettings {
        cross_project_enabled: raw.as_ref().is_some_and(|r| r.0 == 1) && approved == Some(true),
        updated_at: raw.map(|r| r.1).unwrap_or_default(),
    })
}
pub async fn get_settings(db: &Store, user_id: &str) -> Result<MemoryScopeSettings> {
    if user_id != "local" {
        return Ok(MemoryScopeSettings {
            cross_project_enabled: false,
            updated_at: String::new(),
        });
    }
    let mut tx = db.pool.begin().await?;
    settings(&mut tx).await
}
pub(crate) async fn set_enabled(tx: &mut Transaction<'_, Sqlite>, enabled: bool) -> Result<bool> {
    let current = settings(tx).await?;
    let raw: i64 = sqlx::query_scalar("SELECT COALESCE((SELECT cross_project_enabled FROM memory_scope_settings WHERE user_id='local'),0)")
        .fetch_one(&mut **tx).await?;
    let changed = (raw != 0) != enabled || (enabled && !current.cross_project_enabled);
    if changed {
        sqlx::query("INSERT INTO memory_scope_settings(user_id,cross_project_enabled,updated_at) VALUES('local',?,?) ON CONFLICT(user_id) DO UPDATE SET cross_project_enabled=excluded.cross_project_enabled,updated_at=excluded.updated_at")
            .bind(enabled).bind(domain::timestamp()).execute(&mut **tx).await?;
    }
    Ok(changed)
}
pub(crate) async fn sources(tx: &mut Transaction<'_, Sqlite>, target: &str) -> Result<Vec<String>> {
    if super::promotion::local_state(tx, target).await? != LocalState::LOCAL {
        return Ok(vec![]);
    }
    let ids: Vec<String> = sqlx::query_scalar("SELECT p.id FROM projects p WHERE p.id<>? AND p.deleted_at IS NULL AND NOT EXISTS(SELECT 1 FROM v2_promotions v WHERE v.project_id=p.id AND v.state IN('pending','uploading','verifying','completed')) ORDER BY p.id LIMIT 101")
        .bind(target).fetch_all(&mut **tx).await?;
    if ids.len() > 100 {
        return Err(AppError::new(
            "MEMORY_SCOPE_LIMIT",
            "本地来源项目超过 100 个；请关闭跨项目记忆后重新委托",
        ));
    }
    Ok(ids)
}
async fn snapshot(tx: &mut Transaction<'_, Sqlite>, task: &Task) -> Result<Binding> {
    let grant = settings(tx).await?;
    let target_local =
        super::promotion::local_state(tx, &task.project_id).await? == LocalState::LOCAL;
    let cross =
        grant.cross_project_enabled && task.provider_id == super::PROVIDER_ID && target_local;
    let mut dependencies = vec![];
    if cross {
        for id in sources(tx, &task.project_id).await? {
            let evidence: i64 = sqlx::query_scalar(
                "SELECT COALESCE(MAX(id),0) FROM preference_evidence WHERE project_id=?",
            )
            .bind(&id)
            .fetch_one(&mut **tx)
            .await?;
            let revision = super::memory_control::revision(tx, &id).await?;
            dependencies.push(json!([id, evidence, revision]));
        }
    }
    Ok(Binding {
        version: 1,
        cross_project: cross,
        fingerprint: fingerprint(
            &json!({"grant":grant,"epoch":grant_epoch(tx).await?,"targetLocal":target_local,"provider":task.provider_id,"sources":dependencies}),
        ),
    })
}
pub(crate) async fn load(tx: &mut Transaction<'_, Sqlite>, id: &str) -> Result<Option<Binding>> {
    let raw: Option<String> = sqlx::query_scalar("SELECT body_json FROM agent_steps WHERE task_id=? AND kind='author_memory_binding' ORDER BY id LIMIT 1")
        .bind(id).fetch_optional(&mut **tx).await?;
    raw.map(|r| serde_json::from_str(&r).map_err(crate::command_audit::serialization))
        .transpose()
}
pub(crate) async fn bind(tx: &mut Transaction<'_, Sqlite>, task: &Task) -> Result<()> {
    let binding = snapshot(tx, task).await?;
    store::step(
        tx,
        &task.id,
        "author_memory_binding",
        serde_json::to_value(binding).unwrap(),
        "绑定本地作者记忆授权和来源版本；不含证据正文",
    )
    .await
}
pub(crate) async fn validate(
    tx: &mut Transaction<'_, Sqlite>,
    task: &Task,
) -> Result<Option<Binding>> {
    let bound = load(tx, &task.id).await?;
    let valid = match &bound {
        Some(binding) => *binding == snapshot(tx, task).await?,
        // 旧任务仅在从未开启过授权时保留项目内恢复能力。
        None => !settings(tx).await?.cross_project_enabled && grant_epoch(tx).await? == 0,
    };
    if !valid {
        return Err(AppError::new(
            "MEMORY_SCOPE_CHANGED",
            "作者记忆授权或来源已变化，请基于当前记忆重新委托",
        ));
    }
    Ok(bound)
}
pub(crate) fn binding_value(binding: Option<Binding>) -> Value {
    serde_json::to_value(binding).unwrap()
}
pub(crate) async fn may_use_author(tx: &mut Transaction<'_, Sqlite>, task: &Task) -> Result<bool> {
    Ok(task.provider_id == super::PROVIDER_ID
        && settings(tx).await?.cross_project_enabled
        && super::promotion::local_state(tx, &task.project_id).await? == LocalState::LOCAL)
}
