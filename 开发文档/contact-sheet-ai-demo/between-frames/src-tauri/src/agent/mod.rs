pub mod context;
mod drafts;
mod execution;
mod lifecycle;
pub mod policy;
pub mod provider;
pub mod runtime;
pub(crate) mod store;
mod tools;
pub mod types;
pub(crate) use drafts::human as apply_human_draft;

use crate::{command_handler::CommandHandler, domain::Result};
use types::*;
impl CommandHandler<'_> {
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
    pub async fn agent_tool(&self, id: &str, c: &ToolCall) -> Result<ToolResult> {
        execution::tool(self.store, id, c).await
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
