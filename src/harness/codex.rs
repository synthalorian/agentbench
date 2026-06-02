use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{HarnessAdapter, HarnessAdapterConfig, Task, TaskResponse};
use crate::error::{BenchError, BenchResult};

pub struct CodexHarness {
    config: Option<HarnessAdapterConfig>,
    client: Client,
}

impl Default for CodexHarness {
    fn default() -> Self {
        Self::new()
    }
}

impl CodexHarness {
    pub fn new() -> Self {
        Self {
            config: None,
            client: Client::new(),
        }
    }
}

#[derive(Debug, Serialize)]
struct CodexRequest {
    model: String,
    instructions: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct CodexResponse {
    id: String,
    output: Vec<CodexOutput>,
}

#[derive(Debug, Deserialize)]
struct CodexOutput {
    #[serde(rename = "type")]
    output_type: String,
    content: Vec<CodexContent>,
}

#[derive(Debug, Deserialize)]
struct CodexContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

#[async_trait]
impl HarnessAdapter for CodexHarness {
    fn name(&self) -> &str {
        "codex"
    }
    fn description(&self) -> &str {
        "OpenAI Codex harness — connects to Codex API"
    }

    async fn init(&mut self, config: HarnessAdapterConfig) -> BenchResult<()> {
        self.config = Some(config);
        Ok(())
    }

    async fn execute_task(&self, task: &Task) -> BenchResult<TaskResponse> {
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| BenchError::Harness("Codex not initialized".to_string()))?;

        let endpoint = config
            .endpoint
            .as_ref()
            .ok_or_else(|| BenchError::Harness("No Codex endpoint configured".to_string()))?;

        let model = config
            .model.as_deref()
            .unwrap_or("codex-latest");

        let start = std::time::Instant::now();

        let request_body = CodexRequest {
            model: model.to_string(),
            instructions: task.prompt.clone(),
            file_ids: None,
        };

        let mut req = self
            .client
            .post(format!("{}/v1/responses", endpoint))
            .json(&request_body);

        if let Some(key) = &config.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(BenchError::Harness(format!("Codex API error: {}", text)));
        }

        let codex_resp: CodexResponse = resp.json().await?;
        let latency_ms = start.elapsed().as_millis() as u64;

        // Extract text from response
        let output = codex_resp
            .output
            .iter()
            .flat_map(|o| o.content.iter())
            .map(|c| c.text.clone())
            .collect::<Vec<_>>()
            .join("\n");

        // Extract patch if present
        let patch = extract_patch_from_codex_output(&output);

        Ok(TaskResponse {
            task_id: task.id.clone(),
            output,
            patch,
            tool_calls: vec![],
            metadata: HashMap::new(),
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
            .get(format!("{}/v1/models", endpoint))
            .send()
            .await?;
        Ok(resp.status().is_success())
    }

    async fn shutdown(&self) -> BenchResult<()> {
        Ok(())
    }
}

fn extract_patch_from_codex_output(text: &str) -> Option<String> {
    if let Some(start) = text.find("```diff") {
        let patch_start = start + 7;
        let patch_end = text[patch_start..]
            .find("```")
            .unwrap_or(text.len() - patch_start);
        Some(
            text[patch_start..patch_start + patch_end]
                .trim()
                .to_string(),
        )
    } else if let Some(start) = text.find("diff --git") {
        let patch = &text[start..];
        let end = patch.find("\n\n").unwrap_or(patch.len());
        Some(patch[..end].to_string())
    } else {
        None
    }
}
