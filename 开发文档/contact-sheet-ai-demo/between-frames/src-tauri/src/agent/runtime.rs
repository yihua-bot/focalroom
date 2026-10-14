use super::{
    context::ContextBuilder,
    provider::{MockProvider, ModelProvider},
    types::*,
};
use crate::{
    command_handler::CommandHandler,
    db::Store,
    domain::{AppError, Result},
};
use std::{collections::HashSet, sync::Arc};
use tokio::sync::Mutex;
type Notifier = Arc<dyn Fn(&str) + Send + Sync>;
#[derive(Clone)]
pub struct AgentEngine {
    db: Store,
    provider: Arc<dyn ModelProvider>,
    active: Arc<Mutex<HashSet<String>>>,
    notify: Notifier,
}
impl AgentEngine {
    pub fn new(db: Store) -> Self {
        Self::with_provider(db, Arc::new(MockProvider))
    }
    pub fn with_provider(db: Store, provider: Arc<dyn ModelProvider>) -> Self {
        Self {
            db,
            provider,
            active: Default::default(),
            notify: Arc::new(|_| {}),
        }
    }
    pub fn with_notify(mut self, notify: Notifier) -> Self {
        self.notify = notify;
        self
    }
    pub async fn dispatch(&self, id: &str) {
        let mut active = self.active.lock().await;
        if !active.insert(id.into()) {
            return;
        }
        drop(active);
        let engine = self.clone();
        let id = id.to_owned();
        tokio::spawn(async move {
            loop {
                let project = ContextBuilder::build(&engine.db, &id)
                    .await
                    .ok()
                    .map(|c| c.task.project_id);
                let _ = engine.run(&id).await;
                if let Some(p) = project {
                    (engine.notify)(&p);
                }
                // 恢复可能与旧 worker 退出同时发生；在调度锁内检查 queued，避免幽灵排队。
                let mut active = engine.active.lock().await;
                let queued = ContextBuilder::build(&engine.db, &id)
                    .await
                    .is_ok_and(|c| c.task.state == "queued");
                if queued {
                    drop(active);
                    continue;
                }
                active.remove(&id);
                break;
            }
        });
    }
    pub async fn run(&self, id: &str) -> Result<()> {
        let result = self.run_inner(id).await;
        if let Err(e) = &result
            && !["CANCELLED", "BUDGET_EXCEEDED"].contains(&e.code.as_str())
        {
            let _ = CommandHandler::new(&self.db)
                .agent_state(id, "failed", &format!("{}：{}", e.code, e.message))
                .await;
        }
        result
    }
    async fn run_inner(&self, id: &str) -> Result<()> {
        let handler = CommandHandler::new(&self.db);
        let initial = ContextBuilder::build(&self.db, id).await?;
        if initial.task.state != "queued" {
            return Err(AppError::new("CANCELLED", "任务未排队"));
        }
        handler
            .agent_state(id, "planning", "加载可信 scope、预算与安全检查点")
            .await?;
        handler
            .agent_state(id, "running", "本地 Mock 执行中，不读取或上传像素")
            .await?;
        let mut retries = 0;
        loop {
            let input = ContextBuilder::build(&self.db, id).await?;
            (self.notify)(&input.task.project_id);
            if input.task.state != "running" {
                return Ok(());
            }
            if let Some(outcome) = input.task.pending.clone() {
                match outcome {
                    ModelOutcome::ToolCalls { calls } => {
                        for call in calls {
                            handler.agent_tool(id, &call).await?;
                            (self.notify)(&input.task.project_id);
                            let current = ContextBuilder::build(&self.db, id).await?;
                            if current.task.state != "running" {
                                return Ok(());
                            }
                        }
                        handler.agent_clear(id).await?;
                    }
                    other => {
                        handler.agent_final(id, other).await?;
                        return Ok(());
                    }
                }
                continue;
            }
            let estimate = ContextBuilder::input_estimate(&input);
            if let Err(e) = super::policy::PolicyGate::model_budget(&input.task, estimate) {
                handler.agent_state(id, "paused", &e.message).await?;
                return Ok(());
            }
            handler.agent_attempt(id, estimate).await?;
            let started = std::time::Instant::now();
            let remaining = (u64::from(input.task.budget.max_duration_seconds) * 1000)
                .saturating_sub(input.task.elapsed_ms)
                .max(1);
            let result = tokio::time::timeout(
                std::time::Duration::from_millis(remaining),
                self.provider.generate(&input),
            )
            .await;
            match result {
                Ok(Ok(outcome)) => {
                    handler
                        .agent_outcome(id, outcome, estimate, started.elapsed().as_millis() as u64)
                        .await?;
                    retries = 0;
                }
                Ok(Err(error)) if error.retriable && retries < 2 => {
                    handler
                        .agent_elapsed(id, started.elapsed().as_millis() as u64)
                        .await?;
                    retries += 1;
                    tokio::time::sleep(std::time::Duration::from_millis(50 * (1 << retries))).await;
                }
                Ok(Err(error)) => {
                    handler
                        .agent_elapsed(id, started.elapsed().as_millis() as u64)
                        .await?;
                    return Err(error);
                }
                Err(_) => {
                    handler
                        .agent_elapsed(id, started.elapsed().as_millis() as u64)
                        .await?;
                    handler
                        .agent_state(id, "paused", "模型等待达到时间上限；未执行未确认工具")
                        .await?;
                    return Ok(());
                }
            }
        }
    }
}
