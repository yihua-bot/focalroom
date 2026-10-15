mod cloud;
pub mod comments;
pub mod consent;
pub mod context;
mod drafts;
pub(crate) mod execution;
pub mod lifecycle;
pub mod memory;
pub mod memory_control;
mod memory_search;
pub mod openai;
pub mod policy;
pub mod proposals;
pub mod provider;
pub mod router;
pub mod runtime;
pub(crate) mod store;
mod tools;
pub mod types;
pub mod vision;
pub(crate) use drafts::human as apply_human_draft;

use crate::{command_handler::CommandHandler, domain::Result};
use types::*;
impl CommandHandler<'_> {
    pub(crate) async fn reserve_vision(
        &self,
        task: &str,
        call: &ToolCall,
        images: u32,
        cost: u32,
    ) -> Result<(i64, u64)> {
        cloud::reserve(self.store, task, call, images, cost).await
    }
    pub(crate) async fn settle_vision(
        &self,
        task: &str,
        call: &ToolCall,
        data: Option<&vision::VisionPrefetch>,
        cost: Option<u32>,
        elapsed: u64,
    ) -> Result<()> {
        cloud::settle(self.store, task, call, data, cost, elapsed).await
    }
    pub async fn agent_request(&self, r: AgentRequest) -> Result<Task> {
        lifecycle::request(self.store, r).await
    }
    pub async fn recover_agents(&self) -> Result<()> {
        lifecycle::recover(self.store).await
    }
    pub(crate) async fn agent_attempt(&self, id: &str, input: u32) -> Result<()> {
        execution::attempt(self.store, id, input).await
    }
    pub(crate) async fn agent_elapsed(&self, id: &str, elapsed: u64) -> Result<()> {
        execution::elapsed(self.store, id, elapsed).await
    }
    pub(crate) async fn agent_state(&self, id: &str, state: &str, reason: &str) -> Result<()> {
        execution::state(self.store, id, state, reason).await
    }
    pub(crate) async fn agent_outcome(
        &self,
        id: &str,
        o: ModelOutcome,
        input: u32,
        elapsed: u64,
    ) -> Result<()> {
        execution::outcome(self.store, id, o, input, elapsed).await
    }
    pub async fn agent_tool(
        &self,
        id: &str,
        c: &ToolCall,
        vision_data: Option<&vision::VisionPrefetch>,
    ) -> Result<ToolResult> {
        execution::tool(self.store, id, c, vision_data).await
    }
    pub(crate) async fn agent_clear(&self, id: &str) -> Result<()> {
        execution::clear(self.store, id).await
    }
    pub(crate) async fn agent_final(&self, id: &str, o: ModelOutcome) -> Result<()> {
        execution::final_outcome(self.store, id, o).await
    }
}

pub async fn projection(db: &crate::db::Store, project: &str) -> Result<AgentProjection> {
    store::projection(db, project).await
}

pub const PROMPT_VERSION: &str = "m2-mock-1";
pub const PROVIDER_ID: &str = "mock";
pub const MODEL_ID: &str = "metadata-mock-1";
pub const PROVIDER_OPENAI: &str = "openai";
pub fn prompt_version(provider: &str) -> &'static str {
    if provider == PROVIDER_OPENAI {
        "m3-openai-1"
    } else {
        PROMPT_VERSION
    }
}
pub fn system_prompt(provider: &str) -> &'static str {
    if provider == PROVIDER_OPENAI {
        include_str!("../../../prompts/agent-system-m3.md")
    } else {
        include_str!("../../../prompts/agent-system-m2.md")
    }
}
