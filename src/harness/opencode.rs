use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::{BenchError, BenchResult};
use super::{HarnessAdapter, HarnessAdapterConfig, Task, TaskResponse, ToolCall};

pub struct OpenCodeHarness {
    config: Option<HarnessAdapterConfig>,
    client: Client,
}

impl OpenCodeHarness {
    pub fn new() -> Self {
        Self {
            config: None,
            client: Client::new(),
        }
    }
}

#[derive(Debug, Serialize)]
struct OpenCodeRequest {
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    files: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct OpenCodeResponse {
    output: String,
    #[serde(default)]
    patch: Option<String>,
    #[serde(default)]
    tool_calls: Vec<ToolCall>,
    #[serde(default)]
    metadata: HashMap<String, String>,
}

#[async_trait]
impl HarnessAdapter for OpenCodeHarness {
    fn name(&self) -> &str { "opencode" }
    fn description(&self) -> &str { "OpenCode harness — connects to OpenCode CLI or API" }

    async fn init(&mut self, config: HarnessAdapterConfig) -> BenchResult<()> {
        self.config = Some(config);
        Ok(())
    }

    async fn execute_task(&self, task: &Task) -> BenchResult<TaskResponse> {
        let config = self.config.as_ref()
            .ok_or_else(|| BenchError::Harness("OpenCode not initialized".to_string()))?;

        let start = std::time::Instant::now();

        let response = if let Some(endpoint) = &config.endpoint {
            // API mode
            let request_body = OpenCodeRequest {
                prompt: task.prompt.clone(),
                context: Some(task.context.clone()),
                files: Some(task.files.clone()),
            };

            let mut req = self.client
                .post(format!("{}/execute", endpoint))
                .json(&request_body);

            if let Some(key) = &config.api_key {
                req = req.header("Authorization", format!("Bearer {}", key));
            }

            let resp = req.send().await?;
            if !resp.status().is_success() {
                let text = resp.text().await.unwrap_or_default();
                return Err(BenchError::Harness(format!("OpenCode API error: {}", text)));
            }

            let code_resp: OpenCodeResponse = resp.json().await?;
            TaskResponse {
                task_id: task.id.clone(),
                output: code_resp.output,
                patch: code_resp.patch,
                tool_calls: code_resp.tool_calls,
                metadata: code_resp.metadata,
                latency_ms: 0,
                tokens_input: 0,
                tokens_output: 0,
            }
        } else {
            // CLI mode — run opencode command
            let output = std::process::Command::new("opencode")
                .arg("--prompt")
                .arg(&task.prompt)
                .output()
                .map_err(|e| BenchError::Harness(format!(
                    "Failed to run opencode CLI: {}. Make sure 'opencode' is installed.",
                    e
                )))?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let combined = format!("{stdout}\n{stderr}");

            let patch = extract_patch_from_opencode_output(&combined);

            TaskResponse {
                task_id: task.id.clone(),
                output: combined,
                patch,
                tool_calls: vec![],
                metadata: HashMap::new(),
                latency_ms: 0,
                tokens_input: 0,
                tokens_output: 0,
            }
        };

        let latency_ms = start.elapsed().as_millis() as u64;

        Ok(TaskResponse {
            latency_ms,
            ..response
        })
    }

    async fn health_check(&self) -> BenchResult<bool> {
        let config = match &self.config {
            Some(c) => c,
            None => return Ok(false),
        };

        if let Some(endpoint) = &config.endpoint {
            let resp = self.client.get(format!("{}/health", endpoint))
                .send().await?;
            Ok(resp.status().is_success())
        } else {
            // Check if opencode CLI is available
            match std::process::Command::new("opencode").arg("--version").output() {
                Ok(output) => Ok(output.status.success()),
                Err(_) => Ok(false),
            }
        }
    }

    async fn shutdown(&self) -> BenchResult<()> {
        Ok(())
    }
}

fn extract_patch_from_opencode_output(text: &str) -> Option<String> {
    if let Some(start) = text.find("```diff") {
        let patch_start = start + 7;
        let patch_end = text[patch_start..].find("```").unwrap_or(text.len() - patch_start);
        Some(text[patch_start..patch_start + patch_end].trim().to_string())
    } else if let Some(start) = text.find("diff --git") {
        let patch = &text[start..];
        let end = patch.find("\n\n").unwrap_or(patch.len());
        Some(patch[..end].to_string())
    } else {
        None
    }
}
