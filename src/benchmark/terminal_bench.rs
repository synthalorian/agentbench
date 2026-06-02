use async_trait::async_trait;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

use super::{BenchmarkResult, BenchmarkSuite, BenchmarkTask};
use crate::error::{BenchError, BenchResult};
use crate::harness::TaskResponse;

pub struct TerminalBenchSuite {
    tasks: Vec<BenchmarkTask>,
}

impl Default for TerminalBenchSuite {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalBenchSuite {
    pub fn new() -> Self {
        Self { tasks: vec![] }
    }

    /// Validate a terminal command by executing it and checking output
    async fn validate_command(
        &self,
        task: &BenchmarkTask,
        response: &TaskResponse,
        timeout_secs: u64,
    ) -> BenchResult<(bool, String)> {
        let command = response.output.trim();
        if command.is_empty() {
            return Ok((false, "No command provided".to_string()));
        }

        // Parse command (first word is the binary, rest are args)
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Ok((false, "Empty command".to_string()));
        }

        let mut cmd = Command::new(parts[0]);
        cmd.args(&parts[1..])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set working directory if specified
        if let Some(cwd) = task.metadata.get("working_dir") {
            cmd.current_dir(cwd);
        }

        // Set environment variables if specified
        if let Some(env_json) = task.metadata.get("env") {
            if let Ok(env_map) = serde_json::from_str::<HashMap<String, String>>(env_json) {
                for (k, v) in env_map {
                    cmd.env(k, v);
                }
            }
        }

        let result = timeout(Duration::from_secs(timeout_secs), cmd.output()).await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let combined = format!("{stdout}\n{stderr}").trim().to_string();

                let expected_output = task.metadata.get("expected_output");
                let passed = if let Some(expected) = expected_output {
                    combined.contains(expected)
                } else {
                    output.status.success()
                };

                Ok((passed, combined))
            }
            Ok(Err(e)) => Err(BenchError::TaskExecution(format!(
                "Command execution failed: {}",
                e
            ))),
            Err(_) => Err(BenchError::TaskExecution(format!(
                "Command timeout after {}s",
                timeout_secs
            ))),
        }
    }
}

#[async_trait]
impl BenchmarkSuite for TerminalBenchSuite {
    fn name(&self) -> &str {
        "terminal_bench"
    }
    fn description(&self) -> &str {
        "Terminal-bench: Command-line task execution"
    }
    fn task_count(&self) -> usize {
        self.tasks.len()
    }

    async fn load_tasks(&mut self, config: &crate::config::DatasetConfig) -> BenchResult<()> {
        match config.source.as_str() {
            "local" => {
                let path = std::path::Path::new(&config.path);
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    let content = tokio::fs::read_to_string(path).await?;
                    let raw: Vec<serde_json::Value> = serde_json::from_str(&content)?;
                    self.tasks = raw
                        .into_iter()
                        .enumerate()
                        .map(|(i, v)| BenchmarkTask {
                            id: v
                                .get("id")
                                .and_then(|s| s.as_str())
                                .unwrap_or(&format!("task-{}", i))
                                .to_string(),
                            task_type: "terminal_bench".to_string(),
                            repo: None,
                            base_commit: None,
                            problem_statement: v
                                .get("prompt")
                                .and_then(|s| s.as_str())
                                .unwrap_or("")
                                .to_string(),
                            hints: vec![],
                            test_patch: None,
                            expected_files: vec![],
                            metadata: {
                                let mut m = HashMap::new();
                                if let Some(expected) =
                                    v.get("expected_output").and_then(|s| s.as_str())
                                {
                                    m.insert("expected_output".to_string(), expected.to_string());
                                }
                                if let Some(timeout) =
                                    v.get("timeout_secs").and_then(|s| s.as_u64())
                                {
                                    m.insert("timeout_secs".to_string(), timeout.to_string());
                                }
                                m
                            },
                        })
                        .collect();
                }
            }
            "huggingface" => {
                return Err(crate::error::BenchError::Benchmark(
                    "HuggingFace loading not yet implemented. Download dataset locally and use source: local".to_string()
                ));
            }
            _ => {
                return Err(crate::error::BenchError::Benchmark(format!(
                    "Unknown dataset source: {}",
                    config.source
                )));
            }
        }
        Ok(())
    }

    fn tasks(&self) -> &[BenchmarkTask] {
        &self.tasks
    }

    async fn validate(
        &self,
        task: &BenchmarkTask,
        response: &TaskResponse,
    ) -> BenchResult<BenchmarkResult> {
        let started_at = chrono::Utc::now();

        let timeout_secs = task
            .metadata
            .get("timeout_secs")
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);

        let (passed, validation_output) =
            match self.validate_command(task, response, timeout_secs).await {
                Ok((passed, output)) => (passed, Some(output)),
                Err(e) => (false, Some(e.to_string())),
            };

        let finished_at = chrono::Utc::now();
        let score = if passed { 1.0 } else { 0.0 };

        Ok(BenchmarkResult {
            task_id: task.id.clone(),
            harness_name: "generic".to_string(),
            benchmark_name: self.name().to_string(),
            passed,
            score,
            response: response.clone(),
            validation_output,
            error: None,
            started_at,
            finished_at,
        })
    }

    fn aggregate_score(&self, results: &[BenchmarkResult]) -> f64 {
        if results.is_empty() {
            return 0.0;
        }
        results.iter().map(|r| r.score).sum::<f64>() / results.len() as f64
    }
}
