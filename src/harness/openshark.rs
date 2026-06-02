use super::{HarnessAdapter, HarnessAdapterConfig, Task, TaskResponse};
use crate::error::BenchResult;
use async_trait::async_trait;

pub struct OpenSharkHarness;

#[async_trait]
impl HarnessAdapter for OpenSharkHarness {
    fn name(&self) -> &str {
        "openshark"
    }
    fn description(&self) -> &str {
        "OpenShark harness (TODO: implement)"
    }
    async fn init(&mut self, _config: HarnessAdapterConfig) -> BenchResult<()> {
        Ok(())
    }
    async fn execute_task(&self, _task: &Task) -> BenchResult<TaskResponse> {
        todo!("OpenShark adapter not yet implemented")
    }
    async fn health_check(&self) -> BenchResult<bool> {
        Ok(false)
    }
    async fn shutdown(&self) -> BenchResult<()> {
        Ok(())
    }
}
