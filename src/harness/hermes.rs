use super::{HarnessAdapter, HarnessAdapterConfig, Task, TaskResponse};
use crate::error::BenchResult;
use async_trait::async_trait;

pub struct HermesHarness;

#[async_trait]
impl HarnessAdapter for HermesHarness {
    fn name(&self) -> &str {
        "hermes"
    }
    fn description(&self) -> &str {
        "Hermes Agent harness (TODO: implement)"
    }
    async fn init(&mut self, _config: HarnessAdapterConfig) -> BenchResult<()> {
        Ok(())
    }
    async fn execute_task(&self, _task: &Task) -> BenchResult<TaskResponse> {
        todo!("Hermes adapter not yet implemented")
    }
    async fn health_check(&self) -> BenchResult<bool> {
        Ok(false)
    }
    async fn shutdown(&self) -> BenchResult<()> {
        Ok(())
    }
}
