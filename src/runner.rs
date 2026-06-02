use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};
use tokio::time::{timeout, Duration};

use crate::benchmark::{BenchmarkResult, BenchmarkRunConfig, BenchmarkSuite};
use crate::db::Database;
use crate::error::{BenchError, BenchResult};
use crate::harness::{HarnessAdapter, Task as HarnessTask};

pub struct Runner {
    db: Arc<Database>,
}

impl Runner {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub async fn run(
        &self,
        harness: Arc<dyn HarnessAdapter>,
        suite: &dyn BenchmarkSuite,
        config: &BenchmarkRunConfig,
        bench_config: &crate::config::BenchmarkConfig,
    ) -> BenchResult<Vec<BenchmarkResult>> {
        let run_id = uuid::Uuid::new_v4().to_string();
        let started_at = chrono::Utc::now();

        self.db
            .create_run(&run_id, &config.harness_name, suite.name(), started_at)?;

        let tasks = suite.tasks();
        let task_count = config.max_tasks.unwrap_or(tasks.len()).min(tasks.len());
        let tasks_to_run = &tasks[..task_count];

        let max_workers = bench_config.runner.max_workers.max(1);
        let semaphore = Arc::new(Semaphore::new(max_workers));
        let timeout_secs = bench_config.runner.timeout_secs;
        let retries = bench_config.runner.retries;

        let (tx, mut rx) = mpsc::channel::<BenchmarkResult>(max_workers);
        let mut handles = vec![];

        for task in tasks_to_run {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let tx = tx.clone();
            let task = task.clone();
            let harness_task = HarnessTask {
                id: task.id.clone(),
                task_type: task.task_type.clone(),
                prompt: task.problem_statement.clone(),
                context: task.metadata.clone(),
                files: task.expected_files.clone(),
                expected_output: None,
            };
            let harness = harness.clone();
            let harness_name = config.harness_name.clone();
            let benchmark_name = suite.name().to_string();
            let timeout_secs = timeout_secs;
            let retries = retries;

            let handle = tokio::spawn(async move {
                let _permit = permit;

                let result = Self::execute_with_retry(
                    harness.as_ref(),
                    &harness_task,
                    &task,
                    &benchmark_name,
                    &harness_name,
                    timeout_secs,
                    retries,
                )
                .await;

                match result {
                    Ok(r) => {
                        let _ = tx.send(r).await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(BenchmarkResult {
                                task_id: task.id.clone(),
                                harness_name: harness_name.clone(),
                                benchmark_name: benchmark_name.clone(),
                                passed: false,
                                score: 0.0,
                                response: crate::harness::TaskResponse {
                                    task_id: task.id.clone(),
                                    output: String::new(),
                                    patch: None,
                                    tool_calls: vec![],
                                    metadata: Default::default(),
                                    latency_ms: 0,
                                    tokens_input: 0,
                                    tokens_output: 0,
                                },
                                validation_output: None,
                                error: Some(e.to_string()),
                                started_at: chrono::Utc::now(),
                                finished_at: chrono::Utc::now(),
                            })
                            .await;
                    }
                }
            });

            handles.push(handle);
        }

        drop(tx);

        let mut results = vec![];
        while let Some(result) = rx.recv().await {
            self.db.save_result(&run_id, &result)?;
            results.push(result);
        }

        for h in handles {
            let _ = h.await;
        }

        let finished_at = chrono::Utc::now();
        let aggregate = suite.aggregate_score(&results);
        self.db
            .finish_run(&run_id, finished_at, aggregate, results.len())?;

        Ok(results)
    }

    async fn execute_with_retry(
        harness: &dyn HarnessAdapter,
        harness_task: &HarnessTask,
        benchmark_task: &crate::benchmark::BenchmarkTask,
        benchmark_name: &str,
        harness_name: &str,
        timeout_secs: u64,
        retries: u32,
    ) -> BenchResult<BenchmarkResult> {
        let mut last_error = None;

        for _attempt in 0..=retries {
            let started_at = chrono::Utc::now();

            let result = timeout(
                Duration::from_secs(timeout_secs),
                harness.execute_task(harness_task),
            )
            .await;

            match result {
                Ok(Ok(response)) => {
                    let passed = !response.output.is_empty();
                    let finished_at = chrono::Utc::now();

                    return Ok(BenchmarkResult {
                        task_id: benchmark_task.id.clone(),
                        harness_name: harness_name.to_string(),
                        benchmark_name: benchmark_name.to_string(),
                        passed,
                        score: if passed { 1.0 } else { 0.0 },
                        response,
                        validation_output: None,
                        error: None,
                        started_at,
                        finished_at,
                    });
                }
                Ok(Err(e)) => {
                    last_error = Some(e);
                }
                Err(_) => {
                    last_error = Some(BenchError::TaskExecution(format!(
                        "Timeout after {}s",
                        timeout_secs
                    )));
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| BenchError::TaskExecution("All retries exhausted".to_string())))
    }
}
