use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use tokio::fs;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

use super::{BenchmarkResult, BenchmarkSuite, BenchmarkTask};
use crate::error::{BenchError, BenchResult};
use crate::harness::TaskResponse;

pub struct LiveCodeBenchSuite {
    tasks: Vec<BenchmarkTask>,
}

impl Default for LiveCodeBenchSuite {
    fn default() -> Self {
        Self::new()
    }
}

impl LiveCodeBenchSuite {
    pub fn new() -> Self {
        Self { tasks: vec![] }
    }

    /// Validate code by writing it to a temp file and running tests
    async fn validate_code(
        &self,
        task: &BenchmarkTask,
        code: &str,
        timeout_secs: u64,
    ) -> BenchResult<(bool, String)> {
        // Create temp directory
        let temp_dir = tempfile::tempdir()?;
        let temp_path = temp_dir.path();

        // Determine language from metadata
        let language = task
            .metadata
            .get("language")
            .map(|s| s.as_str())
            .unwrap_or("python");

        // Write the code to a file
        let (_code_file, run_cmd) = match language {
            "python" => {
                let file = temp_path.join("solution.py");
                fs::write(&file, code).await?;
                (file, vec!["python", "-m", "py_compile", "solution.py"])
            }
            "rust" => {
                let file = temp_path.join("solution.rs");
                fs::write(&file, code).await?;
                (file, vec!["rustc", "--edition", "2021", "solution.rs"])
            }
            "javascript" | "js" => {
                let file = temp_path.join("solution.js");
                fs::write(&file, code).await?;
                (file, vec!["node", "--check", "solution.js"])
            }
            _ => {
                return Err(BenchError::Benchmark(format!(
                    "Unsupported language: {}",
                    language
                )))
            }
        };

        // Run compilation/syntax check
        let mut cmd = Command::new(run_cmd[0]);
        cmd.args(&run_cmd[1..])
            .current_dir(temp_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let compile_result = timeout(Duration::from_secs(timeout_secs), cmd.output()).await;

        let compile_ok = match compile_result {
            Ok(Ok(output)) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !output.status.success() {
                    return Ok((false, format!("Compilation failed:\n{}", stderr)));
                }
                true
            }
            Ok(Err(e)) => {
                return Err(BenchError::TaskExecution(format!(
                    "Compilation execution failed: {}",
                    e
                )))
            }
            Err(_) => {
                return Err(BenchError::TaskExecution(format!(
                    "Compilation timeout after {}s",
                    timeout_secs
                )))
            }
        };

        // Run tests if test code is provided
        if let Some(test_code) = &task.test_patch {
            let test_file = temp_path.join("test_solution.py");
            fs::write(&test_file, test_code).await?;

            let mut test_cmd = Command::new("python");
            test_cmd
                .arg("-m")
                .arg("pytest")
                .arg("test_solution.py")
                .arg("-v")
                .current_dir(temp_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            let test_result = timeout(Duration::from_secs(timeout_secs), test_cmd.output()).await;

            match test_result {
                Ok(Ok(output)) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let combined = format!("{stdout}\n{stderr}");
                    let passed = output.status.success();
                    Ok((passed && compile_ok, combined))
                }
                Ok(Err(e)) => Err(BenchError::TaskExecution(format!(
                    "Test execution failed: {}",
                    e
                ))),
                Err(_) => Err(BenchError::TaskExecution(format!(
                    "Test timeout after {}s",
                    timeout_secs
                ))),
            }
        } else {
            // No tests — just compilation success
            Ok((compile_ok, "Compilation successful".to_string()))
        }
    }
}

#[async_trait]
impl BenchmarkSuite for LiveCodeBenchSuite {
    fn name(&self) -> &str {
        "livecodebench"
    }
    fn description(&self) -> &str {
        "LiveCodeBench: Live coding competition problems"
    }
    fn task_count(&self) -> usize {
        self.tasks.len()
    }

    async fn load_tasks(&mut self, config: &crate::config::DatasetConfig) -> BenchResult<()> {
        match config.source.as_str() {
            "local" => {
                let path = Path::new(&config.path);
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    let content = fs::read_to_string(path).await?;
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
                            task_type: "livecodebench".to_string(),
                            repo: None,
                            base_commit: None,
                            problem_statement: v
                                .get("problem")
                                .and_then(|s| s.as_str())
                                .unwrap_or("")
                                .to_string(),
                            hints: vec![],
                            test_patch: v
                                .get("test_code")
                                .and_then(|s| s.as_str())
                                .map(|s| s.to_string()),
                            expected_files: vec![],
                            metadata: {
                                let mut m = HashMap::new();
                                if let Some(lang) = v.get("language").and_then(|s| s.as_str()) {
                                    m.insert("language".to_string(), lang.to_string());
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
            .unwrap_or(60);

        let (passed, validation_output, error) = match self
            .validate_code(task, &response.output, timeout_secs)
            .await
        {
            Ok((passed, output)) => (passed, Some(output), None),
            Err(e) => (false, Some(e.to_string()), Some(e.to_string())),
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
        results.iter().map(|r| r.score).sum::<f64>() / results.len() as f64
    }
}
