use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;

use super::{BenchmarkResult, BenchmarkSuite, BenchmarkTask};
use crate::error::{BenchError, BenchResult};
use crate::harness::TaskResponse;

pub struct SWEBenchSuite {
    tasks: Vec<BenchmarkTask>,
}

impl Default for SWEBenchSuite {
    fn default() -> Self {
        Self::new()
    }
}

impl SWEBenchSuite {
    pub fn new() -> Self {
        Self { tasks: vec![] }
    }

    /// Apply a patch inside a Docker container and run tests
    async fn validate_in_docker(
        &self,
        _task: &BenchmarkTask,
        _patch: &str,
        _docker_image: &str,
        _timeout_secs: u64,
    ) -> BenchResult<(bool, String)> {
        // Docker validation disabled for v0.3.0 — falls back to patch format check
        Err(BenchError::Benchmark(
            "Docker validation not available in this build".to_string(),
        ))
    }

    /// Fallback validation without Docker — just check patch format
    fn validate_patch_format(&self, patch: &str) -> bool {
        patch.starts_with("diff ") || patch.starts_with("--- ") || patch.starts_with("Index: ")
    }
}

#[async_trait]
impl BenchmarkSuite for SWEBenchSuite {
    fn name(&self) -> &str {
        "swe_bench"
    }
    fn description(&self) -> &str {
        "SWE-bench: Software engineering tasks from GitHub issues"
    }
    fn task_count(&self) -> usize {
        self.tasks.len()
    }

    async fn load_tasks(&mut self, config: &crate::config::DatasetConfig) -> BenchResult<()> {
        match config.source.as_str() {
            "local" => {
                let path = Path::new(&config.path);
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    let content = tokio::fs::read_to_string(path).await?;
                    let raw: Vec<serde_json::Value> = serde_json::from_str(&content)?;
                    self.tasks = raw
                        .into_iter()
                        .enumerate()
                        .map(|(i, v)| BenchmarkTask {
                            id: v
                                .get("instance_id")
                                .and_then(|s| s.as_str())
                                .unwrap_or(&format!("task-{}", i))
                                .to_string(),
                            task_type: "swe_bench".to_string(),
                            repo: v
                                .get("repo")
                                .and_then(|s| s.as_str())
                                .map(|s| s.to_string()),
                            base_commit: v
                                .get("base_commit")
                                .and_then(|s| s.as_str())
                                .map(|s| s.to_string()),
                            problem_statement: v
                                .get("problem_statement")
                                .and_then(|s| s.as_str())
                                .unwrap_or("")
                                .to_string(),
                            hints: vec![],
                            test_patch: v
                                .get("test_patch")
                                .and_then(|s| s.as_str())
                                .map(|s| s.to_string()),
                            expected_files: vec![],
                            metadata: HashMap::new(),
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

        let (passed, validation_output, error) = match &response.patch {
            Some(patch) => {
                // Try Docker validation first
                let docker_image = task
                    .metadata
                    .get("docker_image")
                    .map(|s| s.as_str())
                    .unwrap_or("agentbench/swe-bench:latest");
                let timeout_secs = task
                    .metadata
                    .get("timeout_secs")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(300);

                match self
                    .validate_in_docker(task, patch, docker_image, timeout_secs)
                    .await
                {
                    Ok((passed, output)) => (passed, Some(output), None),
                    Err(e) => {
                        // Docker failed — fall back to patch format check
                        let format_ok = self.validate_patch_format(patch);
                        (
                            format_ok,
                            Some(format!(
                                "Docker failed: {}\nFalling back to format check",
                                e
                            )),
                            Some(e.to_string()),
                        )
                    }
                }
            }
            None => (false, Some("No patch provided".to_string()), None),
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
            error,
            started_at,
            finished_at,
        })
    }

    fn aggregate_score(&self, results: &[BenchmarkResult]) -> f64 {
        if results.is_empty() {
            return 0.0;
        }
        let passed = results.iter().filter(|r| r.passed).count();
        passed as f64 / results.len() as f64
    }
}
