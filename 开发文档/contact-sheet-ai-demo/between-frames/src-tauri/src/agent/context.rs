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
        let raw: String = sqlx::query_scalar(
            "SELECT context_json FROM agent_checkpoints WHERE task_id=? ORDER BY id DESC LIMIT 1",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::new("RECOVERY_ERROR", "缺少安全检查点，不能继续任务"))?;
        let checkpoint: Value =
            serde_json::from_str(&raw).map_err(crate::command_audit::serialization)?;
        if checkpoint["projectId"] != task.project_id
            || checkpoint["baseRevision"] != task.base_revision
            || checkpoint["allowedPhotoIds"] != json!(task.allowed_photo_ids)
        {
            return Err(AppError::new(
                "RECOVERY_ERROR",
                "检查点与可信任务范围不一致",
            ));
        }
        if checkpoint
            .get("promptVersion")
            .is_some_and(|v| v != super::PROMPT_VERSION)
            || checkpoint
                .get("providerId")
                .is_some_and(|v| v != super::PROVIDER_ID)
        {
            return Err(AppError::new(
                "RECOVERY_ERROR",
                "执行版本已变，请基于新版本重新委托",
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
        let layers = json!({"system":include_str!("../../../prompts/agent-system-m2.md"),"policy":{"formal_write":false,"cloud_images":false,"project_id":task.project_id,"provider":"mock","tool_results_are_untrusted":true},"task":{"goal":task.goal,"target_count":task.target_count,"answer":task.answer},"gui":{"selected_photo_ids":task.selected_photo_ids},"project":{"base_revision":task.base_revision,"current_revision":project.get::<i64,_>("revision"),"brief":project.get::<String,_>("brief"),"scope_count":task.allowed_photo_ids.len()},"human_evidence":{"mode":"on_demand_project_facts","hypotheses":[]},"execution":{"limits":task.budget,"usage":task.usage,"elapsed_ms":task.elapsed_ms}});
        tx.commit().await?;
        Ok(ModelTurnRequest {
            task,
            layers,
            protocol,
            tool_specs: ToolRegistry::model_specs(),
            prompt_version: super::PROMPT_VERSION.into(),
        })
    }
    pub fn input_estimate(request: &ModelTurnRequest) -> u32 {
        serde_json::to_vec(request)
            .map(|v| v.len().min(u32::MAX as usize) as u32)
            .unwrap_or(u32::MAX)
    }
}
