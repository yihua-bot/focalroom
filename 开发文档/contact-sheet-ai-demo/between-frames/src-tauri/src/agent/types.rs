use crate::domain::{AppError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Budget {
    pub max_model_turns: u32,
    pub max_tool_calls: u32,
    pub max_duration_seconds: u32,
    pub max_cloud_images: u32,
    pub max_cost_minor_usd: u32,
    pub max_context_tokens: u32,
    pub max_total_tokens: u32,
    pub max_output_tokens: u32,
}
impl Default for Budget {
    fn default() -> Self {
        Self {
            max_model_turns: 16,
            max_tool_calls: 40,
            max_duration_seconds: 480,
            max_cloud_images: 0,
            max_cost_minor_usd: 100,
            max_context_tokens: 48000,
            max_total_tokens: 500000,
            max_output_tokens: 8192,
        }
    }
}
impl Budget {
    pub fn validate(&self) -> Result<()> {
        if !(1..=200).contains(&self.max_model_turns)
            || !(1..=1000).contains(&self.max_tool_calls)
            || !(1..=7200).contains(&self.max_duration_seconds)
            || self.max_cloud_images > 100
            || self.max_cost_minor_usd > 100000
            || !(128..=256000).contains(&self.max_context_tokens)
            || !(128..=2000000).contains(&self.max_total_tokens)
            || !(128..=16000).contains(&self.max_output_tokens)
        {
            return Err(AppError::new("VALIDATION_ERROR", "预算超出允许范围"));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    pub model_turns: u32,
    pub tool_calls: u32,
    pub input_estimate: u32,
    pub output_estimate: u32,
    pub cloud_images: u32,
    pub cost_minor_usd: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentRequest {
    pub request_id: String,
    pub project_id: String,
    pub command: TaskCommand,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all_fields = "camelCase", deny_unknown_fields)]
pub enum TaskCommand {
    StartTask {
        goal: String,
        selected_photo_ids: Vec<String>,
        target_count: u32,
        budget: Budget,
        consent_id: Option<i64>,
    },
    PauseTask {
        task_id: String,
    },
    CancelTask {
        task_id: String,
    },
    ResumeTask {
        task_id: String,
        budget: Budget,
    },
    SubmitFeedback {
        task_id: String,
        answer: String,
    },
    CloseTask {
        task_id: String,
    },
}
impl TaskCommand {
    pub fn task_id(&self) -> Option<&str> {
        match self {
            Self::StartTask { .. } => None,
            Self::PauseTask { task_id }
            | Self::CancelTask { task_id }
            | Self::ResumeTask { task_id, .. }
            | Self::SubmitFeedback { task_id, .. }
            | Self::CloseTask { task_id } => Some(task_id),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub project_id: String,
    pub goal: String,
    pub base_revision: i64,
    #[serde(default)]
    pub memory_revision: i64,
    pub allowed_photo_ids: Vec<String>,
    pub selected_photo_ids: Vec<String>,
    pub target_count: u32,
    pub state: String,
    pub reason: String,
    pub budget: Budget,
    pub usage: Usage,
    pub elapsed_ms: u64,
    pub question: Option<String>,
    pub answer: Option<String>,
    pub final_draft_ids: Vec<String>,
    pub pending: Option<ModelOutcome>,
    pub provider_id: String,
    pub consent_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Draft {
    pub id: String,
    pub project_id: String,
    pub task_id: String,
    pub title: String,
    pub base_revision: i64,
    pub revision: i64,
    pub state: String,
    pub stale: bool,
    pub summary: String,
    pub items: Vec<DraftItem>,
    pub created_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftItem {
    pub photo_id: String,
    pub proposed_action: String,
    pub position: i64,
    pub rationale: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ToolCall {
    pub call_id: String,
    pub name: String,
    pub args: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResult {
    pub call_id: String,
    pub ok: bool,
    pub result: Option<Value>,
    pub error: Option<ToolError>,
    pub observation_refs: Vec<String>,
    pub duration_ms: u64,
    pub billing_estimate: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolError {
    pub code: String,
    pub retryable: bool,
    pub message: String,
}
impl ToolResult {
    pub fn success(id: &str, value: Value) -> Self {
        Self {
            call_id: id.into(),
            ok: true,
            result: Some(value),
            error: None,
            observation_refs: vec![],
            duration_ms: 0,
            billing_estimate: 0,
        }
    }
    pub fn failure(id: &str, error: AppError) -> Self {
        Self {
            call_id: id.into(),
            ok: false,
            result: None,
            error: Some(ToolError {
                code: error.code,
                retryable: error.retriable,
                message: error.message,
            }),
            observation_refs: vec![],
            duration_ms: 0,
            billing_estimate: 0,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ModelOutcome {
    ToolCalls {
        calls: Vec<ToolCall>,
    },
    NeedsInput {
        question: String,
    },
    FinalDrafts {
        draft_ids: Vec<String>,
        summary: String,
    },
    NoAction {
        reason: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolMessage {
    pub kind: String,
    pub body: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub id: i64,
    pub kind: String,
    pub summary: String,
    pub created_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentProjection {
    pub tasks: Vec<TaskView>,
    pub drafts: Vec<Draft>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskView {
    pub id: String,
    pub goal: String,
    pub base_revision: i64,
    pub state: String,
    pub reason: String,
    pub budget: Budget,
    pub usage: Usage,
    pub elapsed_ms: u64,
    pub question: Option<String>,
    pub final_draft_ids: Vec<String>,
    pub provider_id: String,
    pub steps: Vec<Step>,
    pub created_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsentView {
    pub id: i64,
    pub project_id: String,
    pub mode: String,
    pub provider: String,
    pub quota: i64,
    pub used: i64,
    pub granted_at: String,
    pub revoked_at: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConfig {
    pub provider_id: String,
    pub model_id: String,
    pub ready: bool,
    pub api_key_tail: Option<String>,
    pub key_source: Option<String>,
    pub base_url: String,
    pub prompt_version: String,
    pub capabilities: Vec<String>,
    pub capability_source: Option<String>,
    pub configuration_error: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConsentRequest {
    pub request_id: String,
    pub project_id: String,
    pub mode: String,
    pub provider: String,
    pub quota: i64,
}
