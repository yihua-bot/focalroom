use super::{policy::ToolRegistry, store, types::*};
use crate::{
    db::Store,
    domain::{AppError, Result},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelTurnRequest {
    #[serde(skip_serializing)]
    pub task: Task,
    pub layers: Value,
    pub protocol: Vec<ProtocolMessage>,
    pub tool_specs: Value,
    pub prompt_version: String,
}
pub struct ContextBuilder;
impl ContextBuilder {
    pub async fn build(db: &Store, id: &str) -> Result<ModelTurnRequest> {
        let mut tx = db.pool.begin().await?;
        let task = store::load(&mut tx, id).await?;
        if task.memory_revision
            != super::memory_control::revision(&mut tx, &task.project_id).await?
        {
            return Err(AppError::new(
                "CANCELLED",
                "记忆已调整，请重新委托，不能续接旧记忆历史",
            ));
        }
        let author_binding = super::memory_scope::binding_value(
            super::memory_scope::validate(&mut tx, &task).await?,
        );
        let raw: String = sqlx::query_scalar(
            "SELECT context_json FROM agent_checkpoints WHERE task_id=? ORDER BY id DESC LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::new("RECOVERY_ERROR", "缺少安全检查点，不能继续任务"))?;
        let checkpoint: Value =
            serde_json::from_str(&raw).map_err(crate::command_audit::serialization)?;
        if checkpoint
            .get("authorMemoryBinding")
            .unwrap_or(&Value::Null)
            != &author_binding
        {
            return Err(AppError::new(
                "RECOVERY_ERROR",
                "检查点作者记忆绑定不一致，请重新委托",
            ));
        }
        if checkpoint["projectId"] != task.project_id
            || checkpoint["baseRevision"] != task.base_revision
            || checkpoint["allowedPhotoIds"] != json!(task.allowed_photo_ids)
            || checkpoint
                .get("memoryRevision")
                .map_or(task.memory_revision != 0, |value| {
                    value.as_i64() != Some(task.memory_revision)
                })
        {
            return Err(AppError::new(
                "RECOVERY_ERROR",
                "检查点与可信任务范围不一致",
            ));
        }
        if checkpoint
            .get("promptVersion")
            .is_some_and(|v| v != super::prompt_version(&task.provider_id))
            || checkpoint
                .get("providerId")
                .is_some_and(|v| v.as_str() != Some(task.provider_id.as_str()))
        {
            return Err(AppError::new(
                "RECOVERY_ERROR",
                "执行版本已变，请基于新版本重新委托",
            ));
        }
        let identity = super::model_binding::load(&mut tx, id).await?;
        if checkpoint.get("modelIdentity").is_some_and(|value| {
            !value.is_null()
                && Some(value.clone())
                    != identity
                        .as_ref()
                        .map(|value| serde_json::to_value(value).unwrap())
        }) {
            return Err(AppError::new(
                "RECOVERY_ERROR",
                "检查点模型身份与任务绑定不一致，请重新委托",
            ));
        }
        let mut protocol: Vec<ProtocolMessage> =
            serde_json::from_value(checkpoint["protocol"].clone())
                .map_err(crate::command_audit::serialization)?;
        // 调用参数已在前一条 model outcome 中；保留 name/result/callId，去除反馈中的重复副本。
        for message in &mut protocol {
            if message.kind == "tool"
                && let Some(body) = message.body.as_object_mut()
            {
                body.remove("call");
            }
        }
        use sqlx::Row;
        let project =
            sqlx::query("SELECT brief,revision FROM projects WHERE id=? AND deleted_at IS NULL")
                .bind(&task.project_id)
                .fetch_one(&mut *tx)
                .await?;
        // 不拼接全量图库或旧事件；工具返回按任务 scope 过滤，权限不由文本决定。
        // L3 假设按检索规则限量注入（纠正/停用不进入）；不冒充长期偏好。
        let memory = super::memory::retrievable_for_task(&mut tx, &task, 3).await?;
        let cloud_allowed =
            task.provider_id == super::PROVIDER_OPENAI && task.budget.max_cloud_images > 0;
        let layers = json!({"system":super::system_prompt(&task.provider_id),"policy":{"formal_write":false,"cloud_images":cloud_allowed,"project_id":task.project_id,"provider":task.provider_id,"tool_results_are_untrusted":true},"task":{"goal":task.goal,"target_count":task.target_count,"answer":task.answer},"gui":{"selected_photo_ids":task.selected_photo_ids},"project":{"base_revision":task.base_revision,"current_revision":project.get::<i64,_>("revision"),"brief":project.get::<String,_>("brief"),"scope_count":task.allowed_photo_ids.len()},"human_evidence":{"mode":"on_demand_project_facts","hypotheses":memory["hypotheses"]},"execution":{"limits":task.budget,"usage":task.usage,"elapsed_ms":task.elapsed_ms}});
        tx.commit().await?;
        let prompt_version = super::prompt_version(&task.provider_id).into();
        Ok(ModelTurnRequest {
            task,
            layers,
            protocol,
            tool_specs: ToolRegistry::model_specs(),
            prompt_version,
        })
    }
    pub fn input_estimate(request: &ModelTurnRequest) -> u32 {
        serde_json::to_vec(request)
            .map(|v| v.len().min(u32::MAX as usize) as u32)
            .unwrap_or(u32::MAX)
    }
}
