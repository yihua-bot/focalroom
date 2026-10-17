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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observation: Option<ObservationIdentity>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ObservationIdentity {
    pub model_id: String,
    pub capability_profile_sha256: Option<String>,
}
impl ModelIdentity {
    /// 本切片只允许同一适配器、同一服务端点分工，不授权跨服务传输。
    pub fn with_observation(mut self, observation: &Self) -> Result<Self> {
        if self.provider_id != observation.provider_id
            || self.endpoint_sha256 != observation.endpoint_sha256
            || observation.observation.is_some()
        {
            return Err(AppError::new(
                "MODEL_ROLE_CONFIG_INVALID",
                "图像观察与策展必须使用同一服务端点；未发送数据",
            ));
        }
        self.observation = (self != *observation).then(|| ObservationIdentity {
            model_id: observation.model_id.clone(),
            capability_profile_sha256: observation.capability_profile_sha256.clone(),
        });
        Ok(self)
    }
    pub fn observation_identity(&self) -> Self {
        let mut identity = self.clone();
        if let Some(observation) = identity.observation.take() {
            identity.model_id = observation.model_id;
            identity.capability_profile_sha256 = observation.capability_profile_sha256;
        }
        identity
    }
    pub fn observation_model_id(&self) -> &str {
        self.observation
            .as_ref()
            .map_or(self.model_id.as_str(), |model| model.model_id.as_str())
    }
    pub fn consent_provider(&self) -> String {
        format!(
            "openai@{}",
            crate::command_audit::fingerprint(&serde_json::json!({
                "version":1,"adapter":self.provider_id,"endpoint":self.endpoint_sha256,"model":self.observation_model_id()
            }))
        )
    }

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
