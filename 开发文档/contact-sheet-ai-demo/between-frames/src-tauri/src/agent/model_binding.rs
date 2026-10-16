//! 可信实例身份绑定到任务，恢复时拒绝静默更换端点/模型。无密钥或明文端点落库。
use super::{store, types::Task};
use crate::{
    command_audit::serialization,
    domain::{AppError, Result},
};
use serde::{Deserialize, Serialize};
use sqlx::{Sqlite, Transaction};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelIdentity {
    pub provider_id: String,
    pub model_id: String,
    pub endpoint_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_profile_sha256: Option<String>,
}
impl ModelIdentity {
    pub fn cache_version(&self) -> String {
        format!(
            "{}@{}",
            super::vision::VISION_PROMPT_VERSION,
            self.endpoint_sha256
        )
    }
}
pub(crate) async fn load(
    tx: &mut Transaction<'_, Sqlite>,
    task: &str,
) -> Result<Option<ModelIdentity>> {
    let raw: Option<String> = sqlx::query_scalar("SELECT body_json FROM agent_steps WHERE task_id=? AND kind='model_binding' ORDER BY id LIMIT 1")
        .bind(task).fetch_optional(&mut **tx).await?;
    raw.map(|raw| serde_json::from_str(&raw).map_err(serialization))
        .transpose()
}
pub(crate) async fn ensure(
    tx: &mut Transaction<'_, Sqlite>,
    task: &Task,
    identity: &ModelIdentity,
) -> Result<bool> {
    if identity.provider_id != task.provider_id {
        return Err(changed());
    }
    if let Some(existing) = load(tx, &task.id).await? {
        return if existing == *identity {
            Ok(false)
        } else {
            Err(changed())
        };
    }
    let history: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agent_steps WHERE task_id=? AND kind IN('model','tool','vision_reserved')")
        .bind(&task.id).fetch_one(&mut **tx).await?;
    if task.usage.model_turns > 0 || task.pending.is_some() || history > 0 {
        return Err(changed());
    }
    store::step(
        tx,
        &task.id,
        "model_binding",
        serde_json::to_value(identity).map_err(serialization)?,
        "已绑定实际模型与端点身份；质量尚未评测",
    )
    .await?;
    Ok(true)
}
fn changed() -> AppError {
    AppError::new(
        "RECOVERY_ERROR",
        "模型、服务端点或能力配置已变化，或旧任务缺少可信身份绑定；请重新委托，不能自动更换服务方续跑",
    )
}
