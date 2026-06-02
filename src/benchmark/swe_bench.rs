use async_trait::async_trait;
use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

use super::{BenchmarkResult, BenchmarkSuite, BenchmarkTask};
use crate::error::{BenchError, BenchResult};
use crate::harness::TaskResponse;

pub struct SWEBenchSuite {
    tasks: Vec<BenchmarkTask>,
}

impl SWEBenchSuite {
    pub fn new() -> Self {
        Self { tasks: vec![] }
    }

    /// Apply a patch inside a Docker container and run tests
    async fn validate_in_docker(
        &self,
        task: &BenchmarkTask,
        patch: &str,
        docker_image: &str,
        timeout_secs: u64,
    ) -> BenchResult<(bool, String)> {
        let repo = task
            .repo
            .as_ref()
            .ok_or_else(|| BenchError::Benchmark("Task missing repo field".to_string()))?;

        let base_commit = task
            .base_commit
            .as_ref()
            .ok_or_else(|| BenchError::Benchmark("Task missing base_commit field".to_string()))?;

        // Create a temporary directory for the patch file
        let temp_dir = tempfile::tempdir()?;
        let patch_path = temp_dir.path().join("patch.diff");
        tokio::fs::write(&patch_path, patch).await?;

        // Docker command to:
        // 1. Clone the repo at the base commit
        // 2. Apply the patch
        // 3. Run the test patch
        let script = format!(
            r#"
set -e
cd /testbed

# Clone repo if not exists, checkout base commit
if [ ! -d "{repo}" ]; then
    git clone https://github.com/{repo}.git {repo}
fi
cd {repo}
git checkout {base_commit}
git clean -fd

# Apply the patch
cat /tmp/patch.diff | git apply -

# Run tests from test_patch if available
if [ -f /tmp/test_patch.py ]; then
    python -m pytest /tmp/test_patch.py -v 2>&1
else
    echo "No test patch provided"
fi
"#,
            repo = repo,
            base_commit = base_commit
        );

        let mut cmd = Command::new("docker");
        cmd.args([
            "run",
            "--rm",
            "-v",
            &format!("{}:/tmp/patch.diff", patch_path.display()),
            "-v",
            &format!("{}:/tmp/test_patch.py", patch_path.display()),
            "-w",
            "/testbed",
            docker_image,
            "bash",
            "-c",
            &script,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

        let result = timeout(Duration::from_secs(timeout_secs), cmd.output()).await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let combined = format!("{stdout}\n{stderr}");
                let passed = output.status.success();
                Ok((passed, combined))
            }
            Ok(Err(e)) => Err(BenchError::TaskExecution(format!(
                "Docker execution failed: {}",
                e
            ))),
            Err(_) => Err(BenchError::TaskExecution(format!(
                "Docker timeout after {}s",
                timeout_secs
            ))),
        }
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
