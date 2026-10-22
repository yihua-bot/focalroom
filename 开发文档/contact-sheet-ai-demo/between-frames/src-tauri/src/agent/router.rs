// v2.0 M3 ModelRouter：按适配器声明能力、区域、预算和授权过滤；实测质量排名待补。
// 错误可读：MODEL_CAPABILITY_MISSING / PRIVACY_REGION_REJECTED / BUDGET_EXCEEDED。
// 路由永不扩大图像授权范围（never_auto_expand_image_scope）；普通 UI 不要求用户选模型。
// 运行时使用可信适配器实际配置；质量尚未测量，不自动切换提供方。
use crate::domain::{AppError, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskType {
    Index,
    VisionObserve,
    AgentOrchestrate,
    Curation,
    MemoryExtract,
    ReviewSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceTier {
    Standard,
    Pro,
}
impl ServiceTier {
    pub fn max_cost_usd_per_task(self) -> f64 {
        match self {
            Self::Standard => 0.50,
            Self::Pro => 1.00,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Route {
    pub provider_id: String,
    pub model_id: String,
    pub residency: String,
    pub quality: String,
    pub est_cost_usd: f64,
    pub capabilities: Vec<&'static str>,
}

/// 摄影师盲评质量记录（AGENT-QUALITY-EVAL 协议产出；运行时只读消费）。
/// mean 为五维均分（0–5）；hard_fail = 评测中出现过越权写入或幻觉 ID 等硬门槛违规。
#[derive(Debug, Clone, PartialEq)]
pub struct QualityRecord {
    pub provider_id: String,
    pub model_id: String,
    pub task_type: TaskType,
    pub mean_score: f64,
    pub sample_count: u32,
    pub score_version: String,
    pub hard_fail: bool,
}

impl QualityRecord {
    fn matches(&self, route: &Route, task_type: TaskType) -> bool {
        self.provider_id == route.provider_id
            && self.model_id == route.model_id
            && self.task_type == task_type
    }
    /// 排序键：无分 None；有分为 (mean, -cost_per_point) 视档位而定。
    fn effective_mean<'a>(
        records: &'a [QualityRecord],
        route: &Route,
        task_type: TaskType,
    ) -> Option<&'a QualityRecord> {
        records
            .iter()
            .rev()
            .find(|record| record.matches(route, task_type) && !record.hard_fail)
    }
}

/// 部署区域约束：默认本地部署无区域限制；受限区域由配置注入。
#[derive(Debug, Clone, Default)]
pub struct RegionPolicy {
    /// 拒绝出网的区域标记（如 "EU_NO_EXPORT"）；空 = 不限制。
    pub restricted: Vec<String>,
    /// 各 provider 的数据驻留区域；不在允许集合内的 provider 被拒。
    pub allowed_residencies: Vec<String>,
}

pub struct RouteRequest<'a> {
    pub task_type: TaskType,
    pub tier: ServiceTier,
    pub region: &'a RegionPolicy,
    pub budget_remaining_usd: f64,
    pub images_required: bool,
    /// 当前任务已获图像授权（consent）；需要图像但无授权时路由拒绝而不是扩大范围。
    pub image_consent: bool,
    /// 实测质量记录（摄影师盲评）；空 = 全部候选 unmeasured，排序回退默认优先级。
    pub quality_scores: &'a [QualityRecord],
}

pub(crate) fn required_capabilities(task: TaskType) -> &'static [&'static str] {
    match task {
        TaskType::Index => &["embedding_or_multimodal"],
        TaskType::VisionObserve | TaskType::Curation => {
            &["vision", "structured_output", "instruction_following"]
        }
        TaskType::AgentOrchestrate => &["tool_calling", "instruction_following"],
        TaskType::MemoryExtract => &["structured_output", "instruction_following"],
        TaskType::ReviewSummary => &["instruction_following"],
    }
}

/// 内置路由目录：mock 本地零成本；openai 云端（视觉+工具）。
/// 成本为保守估算系数，真实计费由 M3 质量与计费数据校准（不冒充报价）。
impl Route {
    pub fn responses(model: &str, capabilities: Vec<&'static str>) -> Self {
        Self {
            provider_id: "openai".into(),
            model_id: model.into(),
            residency: "unknown".into(),
            quality: "unmeasured".into(),
            est_cost_usd: 0.02,
            capabilities,
        }
    }
    fn mock() -> Self {
        Self {
            provider_id: "mock".into(),
            model_id: super::MODEL_ID.into(),
            residency: "local".into(),
            quality: "mock".into(),
            est_cost_usd: 0.0,
            capabilities: vec!["tool_calling", "structured_output", "instruction_following"],
        }
    }
}

/// 默认目录示例；实际执行必须用 route_for_provider 传入已实例化的配置。
pub fn route(request: &RouteRequest<'_>, openai_available: bool) -> Result<Route> {
    let mut routes = vec![Route::mock()];
    if openai_available {
        let profile = super::capabilities::builtin(
            super::openai::DEFAULT_BASE_URL,
            super::openai::DEFAULT_MODEL,
        )?;
        routes.push(Route::responses(
            super::openai::DEFAULT_MODEL,
            profile.profile.names(),
        ));
    }
    route_candidates(request, routes)
}
/// 任务绑定 provider；不会因为环境新增适配器而自动切换。
pub fn route_for_provider(
    request: &RouteRequest<'_>,
    provider: &str,
    configured: Option<Route>,
) -> Result<Route> {
    let routes = match provider {
        "mock" => vec![Route::mock()],
        "openai" | "anthropic" | "gemini" => configured
            .filter(|route| route.provider_id == provider)
            .into_iter()
            .collect(),
        _ => vec![],
    };
    route_candidates(request, routes)
}

/// 显式目录路由：质量评测/诊断与多候选比较用；生产路径走 route / route_for_provider。
pub fn route_with_catalog(request: &RouteRequest<'_>, catalog: Vec<Route>) -> Result<Route> {
    route_candidates(request, catalog)
}
fn route_candidates(request: &RouteRequest<'_>, catalog: Vec<Route>) -> Result<Route> {
    let mut required = required_capabilities(request.task_type).to_vec();
    if request.images_required && !required.contains(&"vision") {
        required.push("vision");
    }
    let capable = |route: &Route| required.iter().all(|cap| route.capabilities.contains(cap));
    // 过滤顺序遵循模板：region_eligibility → privacy_consent → required_capabilities → usage_budget。
    let mut candidates: Vec<Route> = catalog
        .clone()
        .into_iter()
        .filter(|route| {
            // 硬门槛：评测记录存在越权/幻觉 ID 违规的模型直接排除，不参与任何档位。
            if request
                .quality_scores
                .iter()
                .any(|record| record.matches(route, request.task_type) && record.hard_fail)
            {
                return false;
            }
            // 区域：受限部署中，云 provider 不满足驻留要求即拒（mock 恒本地）。
            let cloud = route.provider_id != "mock";
            if cloud
                && (!request.region.restricted.is_empty()
                    || !request.region.allowed_residencies.is_empty())
                && !request
                    .region
                    .allowed_residencies
                    .iter()
                    .any(|allowed| route.residency != "unknown" && route.residency == *allowed)
            {
                return false;
            }
            // 隐私：需要图像但任务未获授权 → 拒绝所有候选（不自动扩大范围）。
            if request.images_required && !request.image_consent && route.provider_id != "mock" {
                return false;
            }
            // 能力。
            capable(route)
        })
        .collect();
    if candidates.is_empty() {
        // 区分三类拒绝原因（可读错误）。
        let any_capability_match = catalog.iter().any(capable);
        if request.images_required && !request.image_consent && any_capability_match {
            return Err(AppError::new(
                "PRIVACY_REGION_REJECTED",
                "此任务需要图像，但当前没有云端图像授权；路由不会自动扩大授权范围。请先在授权界面明确授权，或改用本地模式。",
            ));
        }
        if !any_capability_match {
            return Err(AppError::new(
                "MODEL_CAPABILITY_MISSING",
                format!(
                    "当前可用提供方缺少所需能力：{}；请检查模型能力配置，服务档位不会自动赋予能力。",
                    required.join(", ")
                ),
            ));
        }
        return Err(AppError::new(
            "PRIVACY_REGION_REJECTED",
            "部署区域策略不允许当前提供方；请在设置中调整区域/驻留策略。",
        ));
    }
    // 质量驱动排序：有实测分的候选按档位策略优先；无分候选保持"配置云端优先、本地兜底"。
    // Pro 档按均分降序（质量优先）；Standard 档按均分/成本性价比降序（成本敏感）。
    let quality_of = |route: &Route| {
        QualityRecord::effective_mean(request.quality_scores, route, request.task_type)
    };
    let value_key = |tier: ServiceTier, record: &QualityRecord, cost: f64| match tier {
        ServiceTier::Pro => record.mean_score,
        ServiceTier::Standard => record.mean_score / cost.max(0.01),
    };
    candidates.sort_by(|a, b| {
        let rank = |route: &Route| if route.provider_id == "mock" { 1 } else { 0 };
        match (quality_of(a), quality_of(b)) {
            (Some(ra), Some(rb)) => value_key(request.tier, ra, a.est_cost_usd)
                .partial_cmp(&value_key(request.tier, rb, b.est_cost_usd))
                .unwrap()
                .reverse(),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => rank(a)
                .cmp(&rank(b))
                .then(a.est_cost_usd.partial_cmp(&b.est_cost_usd).unwrap()),
        }
    });
    let mut best = candidates.remove(0);
    if let Some(record) = quality_of(&best) {
        best.quality = format!(
            "measured:{:.1}@{}(n={})",
            record.mean_score, record.score_version, record.sample_count
        );
    }
    // 预算：单任务估算成本不得超出档位上限，也不得吃穿剩余预算。
    let ceiling = request
        .tier
        .max_cost_usd_per_task()
        .min(request.budget_remaining_usd.max(0.0));
    if best.est_cost_usd > ceiling && best.est_cost_usd > 0.0 {
        return Err(super::budget::exceeded(
            super::budget::LimitKind::Cost,
            0,
            (best.est_cost_usd * 100.0).ceil() as u64,
            (ceiling * 100.0).floor() as u64,
            if request.task_type == TaskType::VisionObserve {
                "vision"
            } else {
                "model"
            },
        ));
    }
    Ok(best)
}

/// 路由决策的可解释投影（UI/诊断展示；不含密钥或端点）。
pub fn explain(request: &RouteRequest<'_>, openai_available: bool) -> Value {
    let decision = route(request, openai_available);
    let outcome = match &decision {
        Ok(route) => {
            json!({"providerId": route.provider_id, "modelId": route.model_id, "quality": route.quality, "estimatedCostUsd": route.est_cost_usd})
        }
        Err(error) => json!({"code": error.code, "message": error.message}),
    };
    let basis = match &decision {
        Ok(route) if route.quality.starts_with("measured:") => {
            format!(
                "quality_ranked_{}",
                match request.tier {
                    ServiceTier::Pro => "pro_mean",
                    ServiceTier::Standard => "standard_value",
                }
            )
        }
        _ => "capability_cost_fallback_unmeasured".to_string(),
    };
    json!({
        "decisionBasis": basis,
        "taskType": request.task_type,
        "tier": request.tier,
        "requiredCapabilities": required_capabilities(request.task_type),
        "imagesRequired": request.images_required,
        "imageConsent": request.image_consent,
        "qualityRecords": request.quality_scores.len(),
        "outcome": outcome,
        "policy": {"neverAutoExpandImageScope": true, "providerSelectionInUI": false},
    })
}

/// v1 费用单位为美分；档位额度按整项任务的累计用量扣减。
pub(crate) fn remaining_budget(task: &super::types::Task, tier: ServiceTier) -> f64 {
    let used = f64::from(task.usage.cost_minor_usd) / 100.0;
    (f64::from(task.budget.max_cost_minor_usd) / 100.0 - used)
        .min(tier.max_cost_usd_per_task() - used)
        .max(0.0)
}
