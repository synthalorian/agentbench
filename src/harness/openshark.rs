use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::Command;

use super::{HarnessAdapter, HarnessAdapterConfig, Task, TaskResponse, ToolCall};
use crate::error::{BenchError, BenchResult};

pub struct OpenSharkHarness {
    config: Option<HarnessAdapterConfig>,
    client: Client,
    process: Option<tokio::process::Child>,
}

impl OpenSharkHarness {
    pub fn new() -> Self {
        Self {
            config: None,
            client: Client::new(),
            process: None,
        }
    }
}

#[derive(Debug, Serialize)]
struct OpenSharkRequest {
    prompt: String,
    context: HashMap<String, String>,
    files: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct OpenSharkResponse {
    output: String,
    patch: Option<String>,
    tool_calls: Vec<ToolCall>,
    latency_ms: u64,
    tokens_input: u64,
    tokens_output: u64,
}

#[async_trait]
impl HarnessAdapter for OpenSharkHarness {
    fn name(&self) -> &str {
        "openshark"
    }
    fn description(&self) -> &str {
        "OpenShark harness — connects to OpenShark CLI or API"
    }

    async fn init(&mut self, config: HarnessAdapterConfig) -> BenchResult<()> {
        self.config = Some(config.clone());

        // If endpoint is provided, use API mode
        // Otherwise, try to spawn OpenShark CLI process
        if config.endpoint.is_none() {
            let mut cmd = Command::new("openshark");
            cmd.arg("--server")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            match cmd.spawn() {
                Ok(child) => {
                    self.process = Some(child);
                    // Give it a moment to start
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
                Err(e) => {
                    return Err(BenchError::Harness(format!(
                        "Failed to start OpenShark process: {}. Make sure 'openshark' is in PATH or provide an endpoint.",
                        e
                    )));
                }
            }
        }

        Ok(())
    }

    async fn execute_task(&self, task: &Task) -> BenchResult<TaskResponse> {
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| BenchError::Harness("Harness not initialized".to_string()))?;

        let start = std::time::Instant::now();

        let response = if let Some(endpoint) = &config.endpoint {
            // API mode
            let request_body = OpenSharkRequest {
                prompt: task.prompt.clone(),
                context: task.context.clone(),
                files: task.files.clone(),
            };

            let mut req = self
                .client
                .post(format!("{}/execute", endpoint))
                .json(&request_body);

            if let Some(key) = &config.api_key {
                req = req.header("Authorization", format!("Bearer {}", key));
            }

            let resp = req.send().await?;
            if !resp.status().is_success() {
                let text = resp.text().await.unwrap_or_default();
                return Err(BenchError::Harness(format!(
                    "OpenShark API error: {}",
                    text
                )));
            }

            let shark_resp: OpenSharkResponse = resp.json().await?;
            TaskResponse {
                task_id: task.id.clone(),
                output: shark_resp.output,
                patch: shark_resp.patch,
                tool_calls: shark_resp.tool_calls,
                metadata: HashMap::new(),
                latency_ms: shark_resp.latency_ms,
                tokens_input: shark_resp.tokens_input,
                tokens_output: shark_resp.tokens_output,
            }
        } else {
            // CLI mode — use the running process
            return Err(BenchError::Harness(
                "CLI mode not yet implemented for OpenShark. Use API endpoint.".to_string(),
            ));
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
            let resp = self
                .client
                .get(format!("{}/health", endpoint))
                .send()
                .await?;
            Ok(resp.status().is_success())
        } else {
            Ok(self.process.is_some())
        }
    }

    async fn shutdown(&self) -> BenchResult<()> {
        // Process will be killed when dropped
        Ok(())
    }
}
