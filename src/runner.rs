use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};
use tokio::time::{timeout, Duration};

use crate::benchmark::{BenchmarkResult, BenchmarkRunConfig, BenchmarkSuite};
use crate::db::Database;
use crate::error::{BenchError, BenchResult};
use crate::harness::{HarnessAdapter, Task as HarnessTask, TaskResponse};

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

        // Phase 1: Execute tasks concurrently to get responses
        let (tx, mut rx) = mpsc::channel::<(String, BenchResult<TaskResponse>)>(max_workers);
        let mut handles = vec![];

        for task in tasks_to_run.iter() {
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

            let handle = tokio::spawn(async move {
                let result = Self::execute_with_retry(
                    harness.as_ref(),
                    &harness_task,
                    &task,
                    &harness_name,
                    timeout_secs,
                    retries,
                )
                .await;

                drop(permit);
                let _ = tx.send((task.id.clone(), result)).await;
            });

            handles.push(handle);
        }

        drop(tx);

        // Phase 2: Collect responses and validate serially (suite.validate is &self)
        let mut results = vec![];
        while let Some((task_id, response_result)) = rx.recv().await {
            let result = match response_result {
     Ok(response) => {
         // Find the task to validate against
         match tasks_to_run.iter().find(|t| t.id == task_id) {
             Some(task) => suite.validate(task, &response).await,
             None => Err(BenchError::TaskExecution(format!(
                 "Task {} not found for validation",
                 task_id
             ))),
         }
     }
     Err(e) => Ok(BenchmarkResult {
         task_id: task_id.clone(),
                    harness_name: config.harness_name.clone(),
                    benchmark_name: suite.name().to_string(),
                    passed: false,
                    score: 0.0,
                    response: TaskResponse {
                        task_id: String::new(),
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
                }),
            };

            match result {
                Ok(r) => {
                    self.db.save_result(&run_id, &r)?;
                    results.push(r);
                }
                Err(e) => {
                    let fallback = BenchmarkResult {
                        task_id,
                        harness_name: config.harness_name.clone(),
                        benchmark_name: suite.name().to_string(),
                        passed: false,
                        score: 0.0,
                        response: TaskResponse {
                            task_id: String::new(),
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
                    };
                    self.db.save_result(&run_id, &fallback)?;
                    results.push(fallback);
                }
            }
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
        _benchmark_task: &crate::benchmark::BenchmarkTask,
        _harness_name: &str,
        timeout_secs: u64,
        retries: u32,
    ) -> BenchResult<TaskResponse> {
        let mut last_error = None;

        for _attempt in 0..=retries {
            let result = timeout(
                Duration::from_secs(timeout_secs),
                harness.execute_task(harness_task),
            )
            .await;

            match result {
                Ok(Ok(response)) => {
                    return Ok(response);
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
