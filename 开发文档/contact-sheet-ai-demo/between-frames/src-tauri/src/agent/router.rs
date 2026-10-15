// v2.0 M3 ModelRouter：按能力/区域/预算/授权过滤并按质量-成本-延迟排序。
// 错误可读：MODEL_CAPABILITY_MISSING / PRIVACY_REGION_REJECTED / BUDGET_EXCEEDED。
// 路由永不扩大图像授权范围（never_auto_expand_image_scope）；普通 UI 不要求用户选模型。
// 内置默认路由表（部署可用 JSON 覆盖文件扩展）；不引入 YAML 依赖（ADR 0018）。
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
    pub quality: &'static str,
    pub est_cost_usd: f64,
    pub capabilities: &'static [&'static str],
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
}

fn required_capability(task: TaskType) -> &'static str {
    match task {
        TaskType::Index => "embedding_or_multimodal",
        TaskType::VisionObserve | TaskType::Curation => "vision",
        TaskType::AgentOrchestrate => "tool_calling",
        TaskType::MemoryExtract => "structured_output",
        TaskType::ReviewSummary => "instruction_following",
    }
}

/// 内置路由目录：mock 本地零成本；openai 云端（视觉+工具）。
/// 成本为保守估算系数，真实计费由 M3 质量与计费数据校准（不冒充报价）。
fn catalog() -> Vec<Route> {
    vec![
        Route {
            provider_id: "mock".into(),
            model_id: "metadata-mock-1".into(),
            quality: "baseline",
            est_cost_usd: 0.0,
            capabilities: &[
                "embedding_or_multimodal",
                "tool_calling",
                "structured_output",
                "instruction_following",
            ],
        },
        Route {
            provider_id: "openai".into(),
            model_id: "gpt-4o-mini".into(),
            quality: "professional",
            est_cost_usd: 0.02,
            capabilities: &[
                "vision",
                "tool_calling",
                "structured_output",
                "instruction_following",
                "embedding_or_multimodal",
            ],
        },
    ]
}

pub fn route(request: &RouteRequest<'_>, openai_available: bool) -> Result<Route> {
    let need = required_capability(request.task_type);
    // 过滤顺序遵循模板：region_eligibility → privacy_consent → required_capabilities → usage_budget。
    let mut candidates: Vec<Route> = catalog()
        .into_iter()
        .filter(|route| {
            // 区域：受限部署中，云 provider 不满足驻留要求即拒（mock 恒本地）。
            let cloud = route.provider_id != "mock";
            if cloud
                && !request.region.restricted.is_empty()
                && !request
                    .region
                    .allowed_residencies
                    .iter()
                    .any(|allowed| route.residency() == allowed)
            {
                return false;
            }
            // 隐私：需要图像但任务未获授权 → 拒绝所有候选（不自动扩大范围）。
            if request.images_required && !request.image_consent && route.provider_id != "mock" {
                return false;
            }
            // 能力。
            route.capabilities.contains(&need)
        })
        .collect();
    // 可用性：openai 需要 key；不可用时从候选剔除（保持可读错误语义）。
    let openai_in_candidates = candidates.iter().any(|route| route.provider_id == "openai");
    if !openai_available && openai_in_candidates {
        candidates.retain(|route| route.provider_id != "openai");
    }
    if candidates.is_empty() {
        // 区分三类拒绝原因（可读错误）。
        let any_capability_match = catalog()
            .iter()
            .filter(|route| openai_available || route.provider_id == "mock")
            .any(|route| route.capabilities.contains(&need));
        if request.images_required && !request.image_consent && any_capability_match {
            return Err(AppError::new(
                "PRIVACY_REGION_REJECTED",
                "此任务需要图像，但当前没有云端图像授权；路由不会自动扩大授权范围。请先在授权界面明确授权，或改用本地模式。",
            ));
        }
        if !any_capability_match {
            return Err(AppError::new(
                "MODEL_CAPABILITY_MISSING",
                format!("当前可用提供方均不具备「{need}」能力；请检查模型配置或更换服务档位。"),
            ));
        }
        return Err(AppError::new(
            "PRIVACY_REGION_REJECTED",
            "部署区域策略不允许当前提供方；请在设置中调整区域/驻留策略。",
        ));
    }
    // 排序：质量档 → 估算成本 → （同分保持目录顺序=延迟稳定近似）。
    candidates.sort_by(|a, b| {
        let rank = |quality: &str| if quality == "professional" { 0 } else { 1 };
        rank(a.quality)
            .cmp(&rank(b.quality))
            .then(a.est_cost_usd.partial_cmp(&b.est_cost_usd).unwrap())
    });
    let best = candidates.remove(0);
    // 预算：单任务估算成本不得超出档位上限，也不得吃穿剩余预算。
    let ceiling = request
        .tier
        .max_cost_usd_per_task()
        .min(request.budget_remaining_usd.max(0.0));
    if best.est_cost_usd > ceiling && best.est_cost_usd > 0.0 {
        return Err(AppError::new(
            "BUDGET_EXCEEDED",
            format!(
                "估算成本 ${:.2} 超出可用预算 ${ceiling:.2}（服务档位上限与剩余预算取小）；请提高预算或降低任务规模。",
                best.est_cost_usd
            ),
        ));
    }
    Ok(best)
}

impl Route {
    fn residency(&self) -> &'static str {
        if self.provider_id == "mock" {
            "local"
        } else {
            "global"
        }
    }
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
    json!({
        "taskType": request.task_type,
        "tier": request.tier,
        "requiredCapability": required_capability(request.task_type),
        "imagesRequired": request.images_required,
        "imageConsent": request.image_consent,
        "outcome": outcome,
        "policy": {"neverAutoExpandImageScope": true, "providerSelectionInUI": false},
    })
}
