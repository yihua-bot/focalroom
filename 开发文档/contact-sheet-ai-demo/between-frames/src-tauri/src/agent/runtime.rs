use super::{
    context::ContextBuilder,
    openai::{OpenAiConfig, OpenAiProvider},
    provider::{MockProvider, ModelProvider},
    router::{self, RegionPolicy, RouteRequest, ServiceTier, TaskType},
    types::*,
    vision,
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
    mock: Arc<dyn ModelProvider>,
    openai: Option<Arc<OpenAiProvider>>,
    active: Arc<Mutex<HashSet<String>>>,
    notify: Notifier,
    region: RegionPolicy,
    tier: ServiceTier,
}
impl AgentEngine {
    pub fn new(db: Store) -> Self {
        Self::with_provider(db, Arc::new(MockProvider))
    }
    pub fn with_provider(db: Store, provider: Arc<dyn ModelProvider>) -> Self {
        Self::with_providers(
            db,
            provider,
            OpenAiConfig::from_env().map(|c| Arc::new(OpenAiProvider::new(c))),
        )
    }
    /// 测试与注入用：显式提供双 provider，不经环境变量。
    pub fn with_providers(
        db: Store,
        mock: Arc<dyn ModelProvider>,
        openai: Option<Arc<OpenAiProvider>>,
    ) -> Self {
        Self {
            db,
            mock,
            openai,
            active: Default::default(),
            notify: Arc::new(|_| {}),
            region: RegionPolicy::default(),
            tier: ServiceTier::Standard,
        }
    }
    pub fn ai_config(&self) -> AiConfig {
        match &self.openai {
            Some(provider) => {
                let resolved = crate::keychain::resolve_api_key();
                AiConfig {
                    provider_id: super::PROVIDER_OPENAI.into(),
                    model_id: provider.model_id().into(),
                    ready: true,
                    api_key_tail: resolved.as_ref().map(|(key, _)| crate::keychain::tail(key)),
                    key_source: resolved.as_ref().map(|(_, source)| (*source).to_owned()),
                    base_url: std::env::var(super::openai::BASE_URL_ENV)
                        .unwrap_or_else(|_| super::openai::DEFAULT_BASE_URL.into()),
                    prompt_version: super::prompt_version(super::PROVIDER_OPENAI).into(),
                }
            }
            None => AiConfig {
                provider_id: super::PROVIDER_ID.into(),
                model_id: super::MODEL_ID.into(),
                ready: false,
                api_key_tail: None,
                key_source: None,
                base_url: String::new(),
                prompt_version: super::PROMPT_VERSION.into(),
            },
        }
    }
    /// 部署端注入可信策略，任务/模型输出不能修改区域或档位。
    pub fn with_routing_policy(mut self, region: RegionPolicy, tier: ServiceTier) -> Self {
        self.region = region;
        self.tier = tier;
        self
    }
    fn provider_for(&self, task: &Task) -> Result<Arc<dyn ModelProvider>> {
        let route = router::route_for_provider(
            &RouteRequest {
                task_type: TaskType::AgentOrchestrate,
                tier: self.tier,
                region: &self.region,
                budget_remaining_usd: super::router::remaining_budget(task, self.tier),
                images_required: false,
                image_consent: false,
            },
            &task.provider_id,
            self.openai.is_some(),
        )?;
        match route.provider_id.as_str() {
            super::PROVIDER_OPENAI => self
                .openai
                .clone()
                .map(|p| p as Arc<dyn ModelProvider>)
                .ok_or_else(|| AppError::new("MODEL_CAPABILITY_MISSING", "云端提供方不可用")),
            _ => Ok(self.mock.clone()),
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
            .agent_state(
                id,
                "running",
                &format!(
                    "提供方 {} 执行中；图像仅在明确授权后上传",
                    initial.task.provider_id
                ),
            )
            .await?;
        let mut retries = 0;
        loop {
            let mut input = ContextBuilder::build(&self.db, id).await?;
            (self.notify)(&input.task.project_id);
            if input.task.state != "running" {
                return Ok(());
            }
            if let Some(outcome) = input.task.pending.clone() {
                match outcome {
                    ModelOutcome::ToolCalls { calls } => {
                        for call in calls {
                            let openai = self
                                .openai
                                .as_deref()
                                .filter(|_| input.task.provider_id == super::PROVIDER_OPENAI)
                                .filter(|_| vision::is_vision_tool(&call.name));
                            let prefetch = match openai {
                                Some(provider) => {
                                    match vision::prefetch(
                                        &self.db,
                                        provider,
                                        &input.task,
                                        &call,
                                        &self.region,
                                        self.tier,
                                    )
                                    .await
                                    {
                                        Ok(data) => data,
                                        Err(e) if e.code == "BUDGET_EXCEEDED" => {
                                            handler.agent_state(id, "paused", &e.message).await?;
                                            return Ok(());
                                        }
                                        Err(e) => return Err(e),
                                    }
                                }
                                None => None,
                            };
                            handler.agent_tool(id, &call, prefetch.as_ref()).await?;
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
            let provider = match self.provider_for(&input.task) {
                Ok(provider) => provider,
                Err(error) if error.code == "BUDGET_EXCEEDED" => {
                    handler.agent_state(id, "paused", &error.message).await?;
                    return Ok(());
                }
                Err(error) => return Err(error),
            };
            let cost_limit = (self.tier.max_cost_usd_per_task() * 100.0) as u32;
            let effective_limit = input.task.budget.max_cost_minor_usd.min(cost_limit);
            input.layers["execution"]["effectiveMaxCostMinorUsd"] = effective_limit.into();
            input.layers["execution"]["remainingCostMinorUsd"] = effective_limit
                .saturating_sub(input.task.usage.cost_minor_usd)
                .into();
            let estimate = ContextBuilder::input_estimate(&input);
            if let Err(e) =
                super::policy::PolicyGate::model_budget(&input.task, estimate, cost_limit)
            {
                handler.agent_state(id, "paused", &e.message).await?;
                return Ok(());
            }
            handler.agent_attempt(id, estimate, cost_limit).await?;
            let started = std::time::Instant::now();
            let remaining = (u64::from(input.task.budget.max_duration_seconds) * 1000)
                .saturating_sub(input.task.elapsed_ms)
                .max(1);
            let result = tokio::time::timeout(
                std::time::Duration::from_millis(remaining),
                provider.generate_metered(&input),
            )
            .await;
            match result {
                Ok(Ok(outcome)) => {
                    handler
                        .agent_outcome(
                            id,
                            outcome,
                            estimate,
                            started.elapsed().as_millis() as u64,
                            cost_limit,
                        )
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
