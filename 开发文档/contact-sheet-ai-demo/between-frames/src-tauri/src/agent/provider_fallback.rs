//! M3 多 Provider 注册与授权回退：主/副提供方配置 + 故障自动降级 + 实例身份保留。
//! 回退不是静默的——每次切换记录到 model_attempt 并标记 provider 来源。
use crate::domain::{AppError, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

pub const FALLBACK_KEY_ENV: &str = "BETWEEN_FRAMES_OPENAI_FALLBACK_KEY";
pub const FALLBACK_BASE_URL_ENV: &str = "BETWEEN_FRAMES_OPENAI_FALLBACK_BASE_URL";
pub const FALLBACK_MODEL_ENV: &str = "BETWEEN_FRAMES_OPENAI_FALLBACK_MODEL";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSpec {
    pub provider_id: String,
    pub base_url: String,
    pub model: String,
    pub api_key: String,
    pub priority: u8, // 0 = 主提供方
}

/// 从环境变量读取全部已配置的提供方（主 + 可选副）。
pub fn configured_providers() -> Vec<ProviderSpec> {
    let mut providers = Vec::new();
    // 主提供方（现有配置路径）。
    if let Some(key) = std::env::var("BETWEEN_FRAMES_OPENAI_API_KEY")
        .ok()
        .filter(|k| k.starts_with("sk-"))
    {
        providers.push(ProviderSpec {
            provider_id: "openai".into(),
            base_url: std::env::var("BETWEEN_FRAMES_AI_BASE_URL")
                .ok()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "https://api.openai.com/v1".into()),
            model: std::env::var("BETWEEN_FRAMES_AI_MODEL")
                .ok()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "gpt-4o-mini".into()),
            api_key: key,
            priority: 0,
        });
    }
    // 副提供方（回退）。
    if let Some(key) = std::env::var(FALLBACK_KEY_ENV)
        .ok()
        .filter(|k| k.starts_with("sk-"))
    {
        providers.push(ProviderSpec {
            provider_id: "openai-fallback".into(),
            base_url: std::env::var(FALLBACK_BASE_URL_ENV)
                .ok()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "https://api.deepseek.com/v1".into()),
            model: std::env::var(FALLBACK_MODEL_ENV)
                .ok()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "deepseek-chat".into()),
            api_key: key,
            priority: 1,
        });
    }
    providers
}

/// 故障回退决策：主提供方可重试错误时返回副提供方。
/// 不静默切换——返回切换记录供 model_attempt 持久化。
pub fn should_fallback(error: &AppError, primary: &ProviderSpec, fallback: &ProviderSpec) -> Option<FallbackDecision> {
    if !error.retriable {
        return None; // 不可重试错误不回退（如认证失败）。
    }
    Some(FallbackDecision {
        from_provider: primary.provider_id.clone(),
        to_provider: fallback.provider_id.clone(),
        to_base_url: fallback.base_url.clone(),
        to_model: fallback.model.clone(),
        reason: error.message.chars().take(120).collect(),
        at: crate::domain::timestamp(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FallbackDecision {
    pub from_provider: String,
    pub to_provider: String,
    pub to_base_url: String,
    pub to_model: String,
    pub reason: String,
    pub at: String,
}

/// 授权门槛：副提供方必须显式配置（环境变量），不能自动猜测或使用主提供方密钥。
pub fn fallback_authorized() -> bool {
    std::env::var(FALLBACK_KEY_ENV)
        .map(|k| k.starts_with("sk-"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_fallback_without_explicit_config() {
        unsafe { std::env::remove_var(FALLBACK_KEY_ENV) };
        assert!(!fallback_authorized(), "未配置副提供方时不应授权回退");
        let providers = configured_providers();
        assert!(providers.len() <= 1, "不应有未配置的副提供方");
    }

    #[test]
    fn fallback_only_for_retriable_errors() {
        let primary = ProviderSpec {
            provider_id: "openai".into(), base_url: "https://a.com".into(),
            model: "m1".into(), api_key: "sk-xxx".into(), priority: 0,
        };
        let fallback = ProviderSpec {
            provider_id: "openai-fallback".into(), base_url: "https://b.com".into(),
            model: "m2".into(), api_key: "sk-yyy".into(), priority: 1,
        };
        // 可重试错误→回退。
        let mut retriable = AppError::new("MODEL_TIMEOUT", "连接超时");
        retriable.retriable = true;
        assert!(should_fallback(&retriable, &primary, &fallback).is_some());
        // 不可重试→不回退。
        let fatal = AppError::new("UNAUTHORIZED", "认证失败");
        assert!(should_fallback(&fatal, &primary, &fallback).is_none(), "认证失败不应回退");
    }
}
