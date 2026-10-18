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
    observation: Option<Arc<OpenAiProvider>>,
    /// 多供应商（M3）：Anthropic/Gemini 原生文本编排槽位；视觉仍走 OpenAI 兼容链路。
    native: Option<Arc<dyn ModelProvider>>,
    native_route: Option<router::Route>,
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
        let mut engine = Self::with_providers(
            db,
            provider,
            OpenAiConfig::from_env().map(|c| Arc::new(OpenAiProvider::new(c))),
        );
        if let Some(primary) = &engine.openai {
            let model = std::env::var(super::openai::OBSERVATION_MODEL_ENV)
                .ok()
                .map(|model| model.trim().to_owned())
                .filter(|model| !model.is_empty())
                .unwrap_or_else(|| primary.model_id().into());
            if model != primary.model_id()
                || std::env::var_os(super::capabilities::OBSERVATION_PROFILE_PATH_ENV).is_some()
            {
                engine.observation = Some(Arc::new(primary.for_observation_model(model)));
            }
        }
        engine
    }
    /// 测试与注入用：显式提供双 provider，不经环境变量。
    pub fn with_providers(
        db: Store,
        mock: Arc<dyn ModelProvider>,
        openai: Option<Arc<OpenAiProvider>>,
    ) -> Self {
        // 多供应商（M3）：环境显式配置 Anthropic/Gemini 时装配原生文本编排；
        // 视觉观察仍走 OpenAI 兼容链路（能力目录未声明 vision，路由自动拒绝并指引）。
        let native: Option<Arc<dyn ModelProvider>> =
            if let Some(provider) = super::anthropic::AnthropicProvider::from_env() {
                Some(Arc::new(provider) as Arc<dyn ModelProvider>)
            } else if let Some(provider) = super::gemini::GeminiProvider::from_env() {
                Some(Arc::new(provider) as Arc<dyn ModelProvider>)
            } else {
                None
            };
        let native_route = native.as_ref().and_then(|provider| {
            provider
                .native_route()
                .map_err(|e| {
                    eprintln!("[agent] 原生供应商能力配置无效：{}", e.message);
                })
                .ok()
        });
        Self {
            db,
            mock,
            observation: openai.clone(),
            openai,
            native,
            native_route,
            active: Default::default(),
            notify: Arc::new(|_| {}),
            region: RegionPolicy::default(),
            tier: ServiceTier::Standard,
        }
    }
    pub fn with_observation_provider(mut self, provider: Arc<OpenAiProvider>) -> Result<Self> {
        let primary = self
            .openai
            .as_ref()
            .ok_or_else(|| AppError::new("MODEL_ROLE_CONFIG_INVALID", "策展服务未配置"))?;
        primary.identity().with_observation(&provider.identity())?;
        self.observation = Some(provider);
        Ok(self)
    }
    fn model_binding(&self) -> Result<super::model_binding::ModelIdentity> {
        let primary = self
            .openai
            .as_ref()
            .ok_or_else(|| AppError::new("MODEL_CAPABILITY_MISSING", "云端提供方不可用"))?;
        let observation = self.observation.as_ref().unwrap_or(primary);
        primary.identity().with_observation(&observation.identity())
    }
    pub fn ai_config(&self) -> AiConfig {
        match &self.openai {
            Some(provider) => {
                let resolved = crate::keychain::resolve_api_key();
                let observation = self.observation.as_ref().unwrap_or(provider);
                AiConfig {
                    observation: Some(ObservationModelConfig {
                        model_id: observation.model_id().into(),
                        capabilities: observation
                            .route()
                            .map(|route| {
                                route.capabilities.into_iter().map(str::to_owned).collect()
                            })
                            .unwrap_or_default(),
                        capability_source: observation.capability_source().map(str::to_owned),
                        configuration_error: observation
                            .configuration_error_for(TaskType::VisionObserve),
                    }),
                    consent_provider: Some(observation.identity().consent_provider()),
                    provider_id: super::PROVIDER_OPENAI.into(),
                    model_id: provider.model_id().into(),
                    ready: true,
                    api_key_tail: Some(provider.api_key_tail()),
                    key_source: resolved
                        .as_ref()
                        .filter(|(key, _)| provider.matches_key(key))
                        .map(|(_, source)| (*source).to_owned()),
                    base_url: provider.base_url().into(),
                    prompt_version: super::prompt_version(super::PROVIDER_OPENAI).into(),
                    capabilities: provider
                        .route()
                        .map(|route| route.capabilities.into_iter().map(str::to_owned).collect())
                        .unwrap_or_default(),
                    capability_source: provider.capability_source().map(str::to_owned),
                    configuration_error: provider.configuration_error(),
                }
            }
            None => AiConfig {
                observation: None,
                consent_provider: None,
                provider_id: super::PROVIDER_ID.into(),
                model_id: super::MODEL_ID.into(),
                ready: false,
                api_key_tail: None,
                key_source: None,
                base_url: String::new(),
                prompt_version: super::PROMPT_VERSION.into(),
                capabilities: vec![],
                capability_source: None,
                configuration_error: None,
            },
        }
    }
    /// 部署端注入可信策略，任务/模型输出不能修改区域或档位。
    pub fn with_routing_policy(mut self, region: RegionPolicy, tier: ServiceTier) -> Self {
        self.region = region;
        self.tier = tier;
        self
    }
    fn provider_for(
        &self,
        task: &Task,
        quality: &[super::router::QualityRecord],
    ) -> Result<Arc<dyn ModelProvider>> {
        let route = router::route_for_provider(
            &RouteRequest {
                task_type: TaskType::AgentOrchestrate,
                tier: self.tier,
                region: &self.region,
                budget_remaining_usd: super::router::remaining_budget(task, self.tier),
                images_required: false,
                image_consent: false,
                quality_scores: quality,
            },
            &task.provider_id,
            if task.provider_id == super::PROVIDER_OPENAI {
                self.openai
                    .as_ref()
                    .map(|provider| provider.route())
                    .transpose()?
            } else if matches!(
                task.provider_id.as_str(),
                super::anthropic::PROVIDER_ID | super::gemini::PROVIDER_ID
            ) {
                self.native_route.clone()
            } else {
                None
            },
        )?;
        match route.provider_id.as_str() {
            super::PROVIDER_OPENAI => self
                .openai
                .clone()
                .map(|p| p as Arc<dyn ModelProvider>)
                .ok_or_else(|| AppError::new("MODEL_CAPABILITY_MISSING", "云端提供方不可用")),
            super::anthropic::PROVIDER_ID | super::gemini::PROVIDER_ID => self
                .native
                .clone()
                .ok_or_else(|| AppError::new("MODEL_CAPABILITY_MISSING", "原生提供方不可用")),
            _ => Ok(self.mock.clone()),
        }
    }
    pub async fn set_consent(
        &self,
        request: super::types::ConsentRequest,
    ) -> Result<super::types::ConsentView> {
        if request.mode != "deny" {
            let provider = self
                .observation
                .as_ref()
                .ok_or_else(super::consent::target_changed)?;
            if request.provider != provider.identity().consent_provider() {
                return Err(super::consent::target_changed());
            }
            let route = provider.route()?;
            if !router::required_capabilities(TaskType::VisionObserve)
                .iter()
                .all(|cap| route.capabilities.contains(cap))
            {
                return Err(AppError::new(
                    "MODEL_CAPABILITY_MISSING",
                    "当前服务不支持图像观察，未创建授权",
                ));
            }
        } else if request.provider != super::PROVIDER_OPENAI {
            return Err(super::consent::target_changed());
        }
        super::consent::set(
            &self.db,
            super::consent::SetConsent {
                request_id: request.request_id,
                project_id: request.project_id,
                mode: request.mode,
                provider: request.provider,
                quota: request.quota,
            },
        )
        .await
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
                .agent_state(
                    id,
                    if ["MEMORY_SCOPE_CHANGED", "MEMORY_SCOPE_LIMIT"].contains(&e.code.as_str()) {
                        "paused"
                    } else {
                        "failed"
                    },
                    &format!("{}：{}", e.code, e.message),
                )
                .await;
        }
        result
    }
    async fn run_inner(&self, id: &str) -> Result<()> {
        let handler = CommandHandler::new(&self.db);
        let initial = ContextBuilder::build(&self.db, id).await?;
        // 质量驱动选模：加载该 provider 的摄影师盲评记录（只读；无记录则回退默认优先级）。
        let quality =
            super::quality::load_quality_records(&self.db.pool, &initial.task.provider_id)
                .await
                .unwrap_or_default();
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
            if input.task.provider_id == super::PROVIDER_OPENAI {
                let provider = self
                    .openai
                    .as_ref()
                    .ok_or_else(|| AppError::new("MODEL_CAPABILITY_MISSING", "云端提供方不可用"))?;
                let binding = self.model_binding()?;
                handler.agent_bind_model(id, &binding).await?;
                input.layers["execution"]["modelId"] = provider.model_id().into();
                input.layers["execution"]["observationModelId"] =
                    binding.observation_model_id().into();
                input.layers["execution"]["modelQuality"] = "unmeasured".into();
            }
            if let Some(outcome) = input.task.pending.clone() {
                match outcome {
                    ModelOutcome::ToolCalls { calls } => {
                        for call in calls {
                            let openai = self
                                .observation
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
                                        &quality,
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
            let provider = match self.provider_for(&input.task, &quality) {
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
            input.layers["execution"]["configuredMaxOutputTokens"] =
                input.task.budget.max_output_tokens.into();
            let rate = self
                .openai
                .as_ref()
                .filter(|_| input.task.provider_id == super::PROVIDER_OPENAI)
                .map(|p| p.rate_card())
                .unwrap_or_default();
            input.layers["execution"]["rateCard"] = serde_json::to_value(rate).unwrap();
            let estimate = ContextBuilder::input_estimate(&input);
            let output_limit = match super::policy::PolicyGate::output_limit(
                &input.task,
                estimate,
                cost_limit,
                rate,
            ) {
                Ok(limit) => limit,
                Err(error) => {
                    handler.agent_state(id, "paused", &error.message).await?;
                    return Ok(());
                }
            };
            // 收紧为不超过原值的整数，序列化长度不会增加，保留原估算作为上界。
            input.task.budget.max_output_tokens = output_limit;
            input.layers["execution"]["limits"]["maxOutputTokens"] = output_limit.into();
            if let Err(e) =
                super::policy::PolicyGate::model_budget(&input.task, estimate, cost_limit, rate)
            {
                handler.agent_state(id, "paused", &e.message).await?;
                return Ok(());
            }
            handler
                .agent_attempt(id, estimate, cost_limit, output_limit, rate)
                .await?;
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
