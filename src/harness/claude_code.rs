use super::{HarnessAdapter, HarnessAdapterConfig, Task, TaskResponse};
use crate::error::BenchResult;
use async_trait::async_trait;

pub struct ClaudeCodeHarness;

#[async_trait]
impl HarnessAdapter for ClaudeCodeHarness {
    fn name(&self) -> &str {
        "claude_code"
    }
    fn description(&self) -> &str {
        "Claude Code harness (TODO: implement)"
    }
    async fn init(&mut self, _config: HarnessAdapterConfig) -> BenchResult<()> {
        Ok(())
    }
    async fn execute_task(&self, _task: &Task) -> BenchResult<TaskResponse> {
        todo!("Claude Code adapter not yet implemented")
    }
    async fn health_check(&self) -> BenchResult<bool> {
        Ok(false)
    }
    async fn shutdown(&self) -> BenchResult<()> {
        Ok(())
    }
}
