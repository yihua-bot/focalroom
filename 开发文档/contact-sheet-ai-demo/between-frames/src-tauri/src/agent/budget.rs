//! 任务额度停止原因：保留结构化数据供界面解释，不把内部术语当产品文案。
use super::types::Task;
use crate::domain::AppError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LimitKind {
    Images,
    Cost,
    Steps,
    Time,
    Rounds,
    Context,
    Tokens,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BudgetIssue {
    pub kind: LimitKind,
    pub used: u64,
    pub requested: u64,
    pub limit: u64,
    pub stage: String,
    pub request_not_sent: bool,
}

pub fn exceeded(kind: LimitKind, used: u64, requested: u64, limit: u64, stage: &str) -> AppError {
    let message = match kind {
        LimitKind::Images => "这次分析可用的照片次数不足。已保留进度，请调整照片范围后重新开始。",
        LimitKind::Cost => "这次分析的可用额度不足。已保留进度，请缩小照片范围后重新开始。",
        LimitKind::Steps | LimitKind::Rounds => "这次分析需要更多步骤。已保留进度，可以继续分析。",
        LimitKind::Time => "这次分析已达到等待时间。已保留进度，可以延长时间后继续。",
        LimitKind::Context | LimitKind::Tokens => {
            "这次分析的内容过多。已保留进度，请缩小照片范围后重新开始。"
        }
    };
    let issue = BudgetIssue {
        kind,
        used,
        requested,
        limit,
        stage: stage.into(),
        request_not_sent: true,
    };
    let mut error = AppError::new("BUDGET_EXCEEDED", message);
    error.details = Some(serde_json::json!({"budgetIssue":issue}));
    error
}

pub fn issue(error: &AppError) -> Option<BudgetIssue> {
    serde_json::from_value(error.details.as_ref()?.get("budgetIssue")?.clone()).ok()
}

pub fn vision_limits(
    task: &Task,
    images: u32,
    cost: u32,
    cost_limit: u32,
) -> crate::domain::Result<()> {
    let checks = [
        (
            LimitKind::Time,
            task.elapsed_ms,
            1,
            u64::from(task.budget.max_duration_seconds) * 1000,
        ),
        (
            LimitKind::Steps,
            u64::from(task.usage.tool_calls),
            1,
            u64::from(task.budget.max_tool_calls),
        ),
        (
            LimitKind::Images,
            u64::from(task.usage.cloud_images),
            u64::from(images),
            u64::from(task.budget.max_cloud_images),
        ),
        (
            LimitKind::Cost,
            u64::from(task.usage.cost_minor_usd),
            u64::from(cost),
            u64::from(task.budget.max_cost_minor_usd.min(cost_limit)),
        ),
    ];
    for (kind, used, requested, limit) in checks {
        if used.saturating_add(requested) > limit {
            return Err(exceeded(kind, used, requested, limit, "vision"));
        }
    }
    Ok(())
}

/// 收紧本次回复长度以适配剩余额度；不提高任务上限，也不减少用户请求的照片。
pub fn vision_output_limit(
    task: &Task,
    images: u32,
    input: u32,
    cost_limit: u32,
    rate: super::pricing::RateCard,
) -> crate::domain::Result<u32> {
    let limit = task.budget.max_cost_minor_usd.min(cost_limit);
    let remaining = limit.saturating_sub(task.usage.cost_minor_usd);
    let mut low = images
        .saturating_mul(256)
        .saturating_add(128)
        .min(task.budget.max_output_tokens);
    // 短观察不使用完整策展回复长度，减少等待与截断前的无效输出。
    let mut high = task
        .budget
        .max_output_tokens
        .min(images.saturating_mul(384).saturating_add(512));
    let minimum_cost = rate.vision_reservation(images, input, low);
    if minimum_cost > remaining {
        return Err(exceeded(
            LimitKind::Cost,
            u64::from(task.usage.cost_minor_usd),
            u64::from(minimum_cost),
            u64::from(limit),
            "vision",
        ));
    }
    while low < high {
        let middle = low + (high - low).div_ceil(2);
        if rate.vision_reservation(images, input, middle) <= remaining {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    Ok(low)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::types::{Budget, Usage};
    fn task() -> Task {
        Task {
            id: "task".into(),
            project_id: "project".into(),
            goal: "goal".into(),
            base_revision: 1,
            memory_revision: 0,
            allowed_photo_ids: vec![],
            selected_photo_ids: vec![],
            target_count: 1,
            state: "running".into(),
            reason: String::new(),
            budget: Budget {
                max_cloud_images: 20,
                ..Budget::default()
            },
            usage: Usage::default(),
            elapsed_ms: 0,
            question: None,
            answer: None,
            final_draft_ids: vec![],
            pending: None,
            provider_id: "openai".into(),
            consent_id: None,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }
    #[test]
    fn vision_stops_report_each_limit_without_combining_causes() {
        for kind in [
            LimitKind::Images,
            LimitKind::Cost,
            LimitKind::Steps,
            LimitKind::Time,
        ] {
            let mut task = task();
            match kind {
                LimitKind::Images => task.usage.cloud_images = 20,
                LimitKind::Cost => task.usage.cost_minor_usd = 50,
                LimitKind::Steps => task.usage.tool_calls = 40,
                LimitKind::Time => task.elapsed_ms = 480_000,
                _ => unreachable!(),
            }
            let error = vision_limits(&task, 1, 1, 50).unwrap_err();
            assert_eq!(error.code, "BUDGET_EXCEEDED");
            let issue = issue(&error).unwrap();
            assert_eq!(issue.kind, kind);
            assert_eq!(issue.stage, "vision");
            assert!(issue.request_not_sent);
            assert!(!error.message.contains("工具"));
        }
    }
    #[test]
    fn exact_image_and_cost_limits_are_allowed_and_use_wide_arithmetic() {
        let mut task = task();
        task.usage.cloud_images = 19;
        task.usage.cost_minor_usd = 49;
        vision_limits(&task, 1, 1, 50).unwrap();
        assert_eq!(
            issue(&vision_limits(&task, u32::MAX, 1, 50).unwrap_err())
                .unwrap()
                .kind,
            LimitKind::Images
        );
    }
    #[test]
    fn a_large_default_reply_is_trimmed_instead_of_refusing_an_affordable_visual_request() {
        let mut task = task();
        task.budget.max_cost_minor_usd = 12;
        task.usage.cost_minor_usd = 5;
        let rate = super::super::pricing::RateCard::LegacyConservative;
        assert!(rate.vision_reservation(1, 4096, 8192) > 7);
        let output = vision_output_limit(&task, 1, 4096, 12, rate).unwrap();
        assert!((384..8192).contains(&output));
        assert!(rate.vision_reservation(1, 4096, output) <= 7);
        assert_eq!(task.budget.max_output_tokens, 8192);
        assert_eq!(task.budget.max_cost_minor_usd, 12);
        assert!(vision_output_limit(&task, 1, 4096, 5, rate).is_err());
    }
}
