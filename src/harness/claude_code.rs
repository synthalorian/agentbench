use async_trait::async_trait;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::Command;

use super::{HarnessAdapter, HarnessAdapterConfig, Task, TaskResponse, ToolCall};
use crate::error::{BenchError, BenchResult};

pub struct ClaudeCodeHarness {
    config: Option<HarnessAdapterConfig>,
    workspace_dir: Option<String>,
}

impl ClaudeCodeHarness {
    pub fn new() -> Self {
        Self {
            config: None,
            workspace_dir: None,
        }
    }
}

#[async_trait]
impl HarnessAdapter for ClaudeCodeHarness {
    fn name(&self) -> &str {
        "claude_code"
    }
    fn description(&self) -> &str {
        "Claude Code harness — runs Claude Code CLI in a workspace"
    }

    async fn init(&mut self, config: HarnessAdapterConfig) -> BenchResult<()> {
        self.config = Some(config.clone());

        // Set up workspace directory
        let workspace = config
            .extra
            .get("workspace_dir")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                std::env::temp_dir()
                    .join("agentbench-claude-code")
                    .to_string_lossy()
                    .to_string()
            });

        tokio::fs::create_dir_all(&workspace).await?;
        self.workspace_dir = Some(workspace);
        Ok(())
    }

    async fn execute_task(&self, task: &Task) -> BenchResult<TaskResponse> {
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| BenchError::Harness("Claude Code not initialized".to_string()))?;

        let workspace = self
            .workspace_dir
            .as_ref()
            .ok_or_else(|| BenchError::Harness("No workspace configured".to_string()))?;

        let start = std::time::Instant::now();

        // Build the prompt file
        let prompt_path = format!("{}/prompt.txt", workspace);
        tokio::fs::write(&prompt_path, &task.prompt).await?;

        // Run Claude Code with the prompt
        let mut cmd = Command::new("claude");
        cmd.args(["code", "--prompt", &prompt_path, "--workspace", workspace])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(model) = &config.model {
            cmd.arg("--model").arg(model);
        }

        let output = cmd.output().await?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined = format!("{stdout}\n{stderr}");

        let latency_ms = start.elapsed().as_millis() as u64;

        // Extract patch from output
        let patch = extract_patch_from_claude_output(&combined);

        Ok(TaskResponse {
            task_id: task.id.clone(),
            output: combined,
            patch,
            tool_calls: vec![],
            metadata: HashMap::new(),
            latency_ms,
            tokens_input: 0,
            tokens_output: 0,
        })
    }

    async fn health_check(&self) -> BenchResult<bool> {
        match Command::new("claude").arg("--version").output().await {
            Ok(output) => Ok(output.status.success()),
            Err(_) => Ok(false),
        }
    }

    async fn shutdown(&self) -> BenchResult<()> {
        Ok(())
    }
}

fn extract_patch_from_claude_output(text: &str) -> Option<String> {
    // Claude Code often outputs patches in code blocks
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
