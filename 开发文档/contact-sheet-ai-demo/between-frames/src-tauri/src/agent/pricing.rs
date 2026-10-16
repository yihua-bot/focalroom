use super::types::Task;
use serde::{Deserialize, Serialize};

/// 已核对的预算费率，不是服务商账单。未知端点/模型保留旧保守规则。
/// 来源 https://api-docs.deepseek.com/quick_start/pricing/，2026-10-16。
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RateCard {
    #[default]
    LegacyConservative,
    DeepseekFlashPeak20261016,
}
impl RateCard {
    pub fn for_endpoint(base: &str, model: &str) -> Self {
        if base.trim_end_matches('/') == "https://api.deepseek.com" && model == "deepseek-flash" {
            Self::DeepseekFlashPeak20261016
        } else {
            Self::LegacyConservative
        }
    }
    pub fn model_cost(self, input: u32, output: u32) -> u32 {
        match self {
            Self::LegacyConservative => {
                super::openai::text_cost_minor_usd(input.saturating_add(output))
            }
            // 峰时、非缓存输入 $0.30/M；输出 $1.20/M。以整数有理数计算美分并向上取整。
            Self::DeepseekFlashPeak20261016 => {
                let numerator = u64::from(input) * 300_000 + u64::from(output) * 1_200_000;
                numerator.div_ceil(10_000_000_000).min(u64::from(u32::MAX)) as u32
            }
        }
    }
    pub fn reservation(self, task: &Task, input: u32) -> u32 {
        if task.provider_id == super::PROVIDER_OPENAI {
            self.model_cost(input, task.budget.max_output_tokens)
        } else {
            0
        }
    }
    pub fn vision_reservation(self, images: u32, input: u32, output: u32) -> u32 {
        self.model_cost(input, output)
            .saturating_add(if self == Self::LegacyConservative {
                images.saturating_mul(super::openai::IMAGE_COST_MINOR_USD)
            } else {
                0
            })
    }
    pub fn vision_cost(self, images: u32, total: u32) -> u32 {
        // observe 尚只报告总 token；按更贵的输出单价估算，不能冒充分项计费。
        self.vision_reservation(images, 0, total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rates_are_integer_rounded_versioned_and_endpoint_bound() {
        let rate = RateCard::DeepseekFlashPeak20261016;
        assert_eq!(rate.model_cost(1_000_000, 0), 30);
        assert_eq!(rate.model_cost(0, 1_000_000), 120);
        assert_eq!(rate.model_cost(0, 0), 0);
        assert_eq!(rate.model_cost(1, 0), 1);
        assert_eq!(rate.model_cost(10_000, 8_192), 2);
        assert_eq!(rate.vision_cost(3, 1000), 1);
        assert_eq!(RateCard::LegacyConservative.vision_cost(3, 1000), 4);
        assert_eq!(
            RateCard::for_endpoint("https://api.deepseek.com/", "deepseek-flash"),
            rate
        );
        for (base, model) in [
            ("http://api.deepseek.com", "deepseek-flash"),
            ("https://api.deepseek.com.evil.invalid", "deepseek-flash"),
            ("https://api.deepseek.com", "unknown-model"),
            ("http://localhost:1234", "deepseek-flash"),
        ] {
            assert_eq!(
                RateCard::for_endpoint(base, model),
                RateCard::LegacyConservative
            );
        }
        let legacy = RateCard::LegacyConservative;
        assert_eq!(legacy.model_cost(1000, 1001), 3);
        assert!(rate.model_cost(u32::MAX, u32::MAX) > 0);
    }
}
