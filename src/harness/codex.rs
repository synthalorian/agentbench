use super::{HarnessAdapter, HarnessAdapterConfig, Task, TaskResponse};
use crate::error::BenchResult;
use async_trait::async_trait;

pub struct CodexHarness;

#[async_trait]
impl HarnessAdapter for CodexHarness {
    fn name(&self) -> &str {
        "codex"
    }
    fn description(&self) -> &str {
        "OpenAI Codex harness (TODO: implement)"
    }
    async fn init(&mut self, _config: HarnessAdapterConfig) -> BenchResult<()> {
        Ok(())
    }
    async fn execute_task(&self, _task: &Task) -> BenchResult<TaskResponse> {
        todo!("Codex adapter not yet implemented")
    }
    async fn health_check(&self) -> BenchResult<bool> {
        Ok(false)
    }
    async fn shutdown(&self) -> BenchResult<()> {
        Ok(())
    }
}
