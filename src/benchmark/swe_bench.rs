use async_trait::async_trait;
use std::collections::HashMap;

use super::{BenchmarkResult, BenchmarkSuite, BenchmarkTask};
use crate::error::BenchResult;
use crate::harness::TaskResponse;

pub struct SWEBenchSuite {
    tasks: Vec<BenchmarkTask>,
}

impl SWEBenchSuite {
    pub fn new() -> Self {
        Self { tasks: vec![] }
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
                let path = std::path::Path::new(&config.path);
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
        let now = chrono::Utc::now();
        let passed = response.patch.is_some();
        let score = if passed { 1.0 } else { 0.0 };

        Ok(BenchmarkResult {
            task_id: task.id.clone(),
            harness_name: "generic".to_string(),
            benchmark_name: self.name().to_string(),
            passed,
            score,
            response: response.clone(),
            validation_output: None,
            error: None,
            started_at: now,
            finished_at: now,
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
