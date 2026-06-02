use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

use super::{HarnessAdapter, HarnessAdapterConfig, Task, TaskResponse, ToolCall};
use crate::error::{BenchError, BenchResult};

pub struct GenericOpenAIHarness {
    config: Option<HarnessAdapterConfig>,
    client: Client,
}

impl GenericOpenAIHarness {
    pub fn new() -> Self {
        Self {
            config: None,
            client: Client::new(),
        }
    }
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u64,
    completion_tokens: u64,
}

#[async_trait]
impl HarnessAdapter for GenericOpenAIHarness {
    fn name(&self) -> &str {
        "generic"
    }

    fn description(&self) -> &str {
        "Generic OpenAI-compatible API harness"
    }

    async fn init(&mut self, config: HarnessAdapterConfig) -> BenchResult<()> {
        self.config = Some(config);
        Ok(())
    }

    async fn execute_task(&self, task: &Task) -> BenchResult<TaskResponse> {
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| BenchError::Harness("Harness not initialized".to_string()))?;

        let endpoint = config
            .endpoint
            .as_ref()
            .ok_or_else(|| BenchError::Harness("No endpoint configured".to_string()))?;

        let model = config
            .model
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("local-model");

        let max_tokens = config
            .extra
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(4096) as u32;

        let temperature = config
            .extra
            .get("temperature")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32;

        let request_body = ChatCompletionRequest {
            model: model.to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: task.prompt.clone(),
            }],
            max_tokens,
            temperature,
        };

        let start = std::time::Instant::now();

        let mut req = self
            .client
            .post(format!("{}/chat/completions", endpoint))
            .json(&request_body);

        if let Some(key) = &config.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let response = req.send().await?;
        let status = response.status();

        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(BenchError::Harness(format!(
                "API error {}: {}",
                status, text
            )));
        }

        let completion: ChatCompletionResponse = response.json().await?;
        let latency_ms = start.elapsed().as_millis() as u64;

        let output = completion
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        let (tokens_input, tokens_output) = completion
            .usage
            .map(|u| (u.prompt_tokens, u.completion_tokens))
            .unwrap_or((0, 0));

        Ok(TaskResponse {
            task_id: task.id.clone(),
            output,
            patch: None,
            tool_calls: vec![],
            metadata: HashMap::new(),
            latency_ms,
            tokens_input,
            tokens_output,
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
            .get(format!("{}/models", endpoint))
            .send()
            .await?;

        Ok(resp.status().is_success())
    }

    async fn shutdown(&self) -> BenchResult<()> {
        Ok(())
    }
}
