use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{HarnessAdapter, HarnessAdapterConfig, Task, TaskResponse, ToolCall};
use crate::error::{BenchError, BenchResult};

pub struct HermesHarness {
    config: Option<HarnessAdapterConfig>,
    client: Client,
}

impl Default for HermesHarness {
    fn default() -> Self {
        Self::new()
    }
}

impl HermesHarness {
    pub fn new() -> Self {
        Self {
            config: None,
            client: Client::new(),
        }
    }
}

#[derive(Debug, Serialize)]
struct HermesRequest {
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct HermesResponse {
    reply: String,
    #[serde(default)]
    tool_calls: Vec<ToolCall>,
    #[serde(default)]
    metadata: HashMap<String, String>,
}

#[async_trait]
impl HarnessAdapter for HermesHarness {
    fn name(&self) -> &str {
        "hermes"
    }
    fn description(&self) -> &str {
        "Hermes Agent harness — connects to Hermes API"
    }

    async fn init(&mut self, config: HarnessAdapterConfig) -> BenchResult<()> {
        self.config = Some(config);
        Ok(())
    }

    async fn execute_task(&self, task: &Task) -> BenchResult<TaskResponse> {
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| BenchError::Harness("Hermes not initialized".to_string()))?;

        let endpoint = config
            .endpoint
            .as_ref()
            .ok_or_else(|| BenchError::Harness("No Hermes endpoint configured".to_string()))?;

        let start = std::time::Instant::now();

        let request_body = HermesRequest {
            message: task.prompt.clone(),
            session_id: Some(task.id.clone()),
            context: Some(task.context.clone()),
        };

        let mut req = self
            .client
            .post(format!("{}/api/chat", endpoint))
            .json(&request_body);

        if let Some(key) = &config.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(BenchError::Harness(format!("Hermes API error: {}", text)));
        }

        let hermes_resp: HermesResponse = resp.json().await?;
        let latency_ms = start.elapsed().as_millis() as u64;

        // Extract patch from reply if present
        let patch = extract_patch(&hermes_resp.reply);

        Ok(TaskResponse {
            task_id: task.id.clone(),
            output: hermes_resp.reply,
            patch,
            tool_calls: hermes_resp.tool_calls,
            metadata: hermes_resp.metadata,
            latency_ms,
            tokens_input: 0,
            tokens_output: 0,
        })
    }

    async fn health_check(&self) -> BenchResult<bool> {
        let config = match &self.config {
            Some(c) => c,
            None => return Ok(false),
        };

        let endpoint = match &config.endpoint {
            Some(e) => e,
            None => return Ok(false),
        };

        let resp = self
            .client
            .get(format!("{}/health", endpoint))
            .send()
            .await?;
        Ok(resp.status().is_success())
    }

    async fn shutdown(&self) -> BenchResult<()> {
        Ok(())
    }
}

/// Extract a unified diff patch from text output
fn extract_patch(text: &str) -> Option<String> {
    // Look for diff blocks in the output
    if let Some(start) = text.find("diff --git") {
        let patch = &text[start..];
        // Find end of patch (next section or end of text)
        let end = patch.find("\n\n").unwrap_or(patch.len());
        Some(patch[..end].to_string())
    } else if let Some(start) = text.find("--- ") {
        let patch = &text[start..];
        let end = patch.find("\n\n").unwrap_or(patch.len());
        Some(patch[..end].to_string())
    } else {
        None
    }
}
