use async_trait::async_trait;
use std::collections::HashMap;

use super::{HarnessAdapter, HarnessAdapterConfig, Task, TaskResponse};
use crate::error::BenchResult;

/// Mock harness for testing and dry-runs.
/// Returns deterministic responses without calling any external API.
pub struct MockHarness {
    config: Option<HarnessAdapterConfig>,
}

impl Default for MockHarness {
    fn default() -> Self {
        Self::new()
    }
}

impl MockHarness {
    pub fn new() -> Self {
        Self { config: None }
    }
}

#[async_trait]
impl HarnessAdapter for MockHarness {
    fn name(&self) -> &str {
        "mock"
    }

    fn description(&self) -> &str {
        "Mock harness for testing — returns deterministic responses without external API calls"
    }

    async fn init(&mut self, config: HarnessAdapterConfig) -> BenchResult<()> {
        self.config = Some(config);
        Ok(())
    }

    async fn execute_task(&self, task: &Task) -> BenchResult<TaskResponse> {
        // Return a deterministic mock response based on task content
        let output = format!(
            "[MOCK] Processed task {} (type: {})\nPrompt length: {} chars",
            task.id,
            task.task_type,
            task.prompt.len()
        );

        Ok(TaskResponse {
            task_id: task.id.clone(),
            output,
            patch: Some(format!("--- a/file.txt\n+++ b/file.txt\n@@ -1 +1 @@\n-mock patch for {}\n", task.id)),
            tool_calls: vec![],
            metadata: HashMap::new(),
            latency_ms: 42,
            tokens_input: task.prompt.len() as u64 / 4,
            tokens_output: 64,
        })
    }

    async fn health_check(&self) -> BenchResult<bool> {
        Ok(true)
    }

    async fn shutdown(&self) -> BenchResult<()> {
        Ok(())
    }
}
