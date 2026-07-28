//! Hardening tests for AgentBench: deadlock regression, timeout/retry behavior,
//! concurrency caps, cost math on unknown models, and malformed dataset handling.
//! All hermetic — no network, no external APIs.

use agentbench::benchmark::swe_bench::SWEBenchSuite;
use agentbench::benchmark::{BenchmarkRunConfig, BenchmarkSuite};
use agentbench::config::{BenchmarkConfig, DatasetConfig};
use agentbench::db::Database;
use agentbench::harness::{HarnessAdapter, HarnessAdapterConfig, Task, TaskResponse};
use agentbench::runner::Runner;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn make_bench_config(max_workers: usize, timeout_secs: u64, retries: u32) -> BenchmarkConfig {
    BenchmarkConfig {
        name: "hardening".to_string(),
        description: "hardening tests".to_string(),
        benchmark_type: "swe_bench".to_string(),
        dataset: DatasetConfig {
            source: "local".to_string(),
            path: project_root()
                .join("data/swe-bench-sample.json")
                .to_string_lossy()
                .to_string(),
            split: None,
            subset: None,
            filter: None,
        },
        harness: agentbench::config::HarnessConfig {
            name: "test-double".to_string(),
            adapter: "mock".to_string(),
            endpoint: None,
            api_key: None,
            model: None,
            extra: None,
        },
        runner: agentbench::config::RunnerConfig {
            max_workers,
            timeout_secs,
            retries,
            docker_image: None,
            env: None,
        },
        scoring: agentbench::config::ScoringConfig {
            metric: "pass_rate".to_string(),
            thresholds: None,
        },
    }
}

fn run_config() -> BenchmarkRunConfig {
    BenchmarkRunConfig {
        harness_name: "test-double".to_string(),
        max_tasks: None,
        shuffle: false,
        seed: None,
    }
}

async fn load_suite() -> SWEBenchSuite {
    let mut suite = SWEBenchSuite::new();
    let dataset = DatasetConfig {
        source: "local".to_string(),
        path: project_root()
            .join("data/swe-bench-sample.json")
            .to_string_lossy()
            .to_string(),
        split: None,
        subset: None,
        filter: None,
    };
    suite.load_tasks(&dataset).await.unwrap();
    suite
}

fn ok_response(task: &Task) -> TaskResponse {
    TaskResponse {
        task_id: task.id.clone(),
        output: "ok".to_string(),
        patch: Some(format!(
            "--- a/file.txt\n+++ b/file.txt\n@@ -1 +1 @@\n-mock patch for {}\n",
            task.id
        )),
        tool_calls: vec![],
        metadata: Default::default(),
        latency_ms: 1,
        tokens_input: 1,
        tokens_output: 1,
    }
}

/// Harness that sleeps a configurable duration per task and tracks
/// the maximum number of concurrently in-flight executions.
struct SlowTrackingHarness {
    delay: Duration,
    in_flight: Arc<AtomicUsize>,
    max_seen: Arc<AtomicUsize>,
}

#[async_trait]
impl HarnessAdapter for SlowTrackingHarness {
    fn name(&self) -> &str {
        "slow-tracking"
    }
    fn description(&self) -> &str {
        "test double"
    }
    async fn init(&mut self, _config: HarnessAdapterConfig) -> agentbench::error::BenchResult<()> {
        Ok(())
    }
    async fn execute_task(&self, task: &Task) -> agentbench::error::BenchResult<TaskResponse> {
        let now = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_seen.fetch_max(now, Ordering::SeqCst);
        tokio::time::sleep(self.delay).await;
        self.in_flight.fetch_sub(1, Ordering::SeqCst);
        Ok(ok_response(task))
    }
    async fn health_check(&self) -> agentbench::error::BenchResult<bool> {
        Ok(true)
    }
    async fn shutdown(&self) -> agentbench::error::BenchResult<()> {
        Ok(())
    }
}

/// Harness that fails a configurable number of times before succeeding.
struct FlakyHarness {
    failures_before_success: usize,
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl HarnessAdapter for FlakyHarness {
    fn name(&self) -> &str {
        "flaky"
    }
    fn description(&self) -> &str {
        "test double"
    }
    async fn init(&mut self, _config: HarnessAdapterConfig) -> agentbench::error::BenchResult<()> {
        Ok(())
    }
    async fn execute_task(&self, task: &Task) -> agentbench::error::BenchResult<TaskResponse> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        if n < self.failures_before_success {
            return Err(agentbench::error::BenchError::TaskExecution(
                "simulated transient failure".to_string(),
            ));
        }
        Ok(ok_response(task))
    }
    async fn health_check(&self) -> agentbench::error::BenchResult<bool> {
        Ok(true)
    }
    async fn shutdown(&self) -> agentbench::error::BenchResult<()> {
        Ok(())
    }
}

/// REGRESSION TEST for the runner deadlock fixed in d844e15:
/// with more tasks than max_workers, workers blocked on a full results
/// channel while holding semaphore permits deadlocked the run.
/// This test fails (via overall timeout) on the pre-fix code.
#[tokio::test]
async fn test_no_deadlock_when_tasks_exceed_workers() {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let runner = Runner::new(db);
    let suite = load_suite().await;
    assert!(suite.tasks().len() >= 2, "sample dataset must have tasks");

    let harness = Arc::new(SlowTrackingHarness {
        delay: Duration::from_millis(50),
        in_flight: Arc::new(AtomicUsize::new(0)),
        max_seen: Arc::new(AtomicUsize::new(0)),
    });

    // max_workers = 1 forces maximum contention on the old design
    let bench_config = make_bench_config(1, 30, 0);
    let run_cfg = run_config();
    let run = runner.run(harness, &suite, &run_cfg, &bench_config);

    let results = tokio::time::timeout(Duration::from_secs(20), run)
        .await
        .expect("runner deadlocked: did not finish within 20s")
        .unwrap();
    assert_eq!(results.len(), suite.tasks().len());
}

#[tokio::test]
async fn test_concurrency_never_exceeds_max_workers() {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let runner = Runner::new(db);
    let suite = load_suite().await;

    let in_flight = Arc::new(AtomicUsize::new(0));
    let max_seen = Arc::new(AtomicUsize::new(0));
    let harness = Arc::new(SlowTrackingHarness {
        delay: Duration::from_millis(100),
        in_flight: in_flight.clone(),
        max_seen: max_seen.clone(),
    });

    let bench_config = make_bench_config(2, 30, 0);
    let results = runner
        .run(harness, &suite, &run_config(), &bench_config)
        .await
        .unwrap();

    assert_eq!(results.len(), suite.tasks().len());
    assert!(
        max_seen.load(Ordering::SeqCst) <= 2,
        "concurrency exceeded max_workers: saw {}",
        max_seen.load(Ordering::SeqCst)
    );
    assert_eq!(
        in_flight.load(Ordering::SeqCst),
        0,
        "tasks leaked in-flight"
    );
}

#[tokio::test]
async fn test_timeout_produces_failed_result_not_hang() {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let runner = Runner::new(db);
    let suite = load_suite().await;

    // Harness sleeps 3s, runner timeout is 1s
    let harness = Arc::new(SlowTrackingHarness {
        delay: Duration::from_secs(3),
        in_flight: Arc::new(AtomicUsize::new(0)),
        max_seen: Arc::new(AtomicUsize::new(0)),
    });

    let bench_config = make_bench_config(4, 1, 0);
    let run_cfg = run_config();
    let run = runner.run(harness, &suite, &run_cfg, &bench_config);
    let results = tokio::time::timeout(Duration::from_secs(30), run)
        .await
        .expect("run hung despite per-task timeout")
        .unwrap();

    assert!(
        results.iter().all(|r| !r.passed && r.error.is_some()),
        "timed-out tasks must be recorded as failures with errors"
    );
    assert!(
        results
            .iter()
            .any(|r| r.error.as_deref().unwrap_or("").contains("Timeout")),
        "expected a timeout error message"
    );
}

#[tokio::test]
async fn test_retries_eventually_succeed() {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let runner = Runner::new(db);
    let suite = load_suite().await;

    let calls = Arc::new(AtomicUsize::new(0));
    let harness = Arc::new(FlakyHarness {
        failures_before_success: 2,
        calls: calls.clone(),
    });

    let bench_config = make_bench_config(1, 30, 3);
    let config = BenchmarkRunConfig {
        max_tasks: Some(1),
        ..run_config()
    };
    let results = runner
        .run(harness, &suite, &config, &bench_config)
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        3,
        "expected 1 initial + 2 retries"
    );
}

#[tokio::test]
async fn test_retries_exhausted_records_failure() {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let runner = Runner::new(db);
    let suite = load_suite().await;

    let calls = Arc::new(AtomicUsize::new(0));
    let harness = Arc::new(FlakyHarness {
        failures_before_success: usize::MAX,
        calls: calls.clone(),
    });

    let bench_config = make_bench_config(1, 30, 2);
    let config = BenchmarkRunConfig {
        max_tasks: Some(1),
        ..run_config()
    };
    let results = runner
        .run(harness, &suite, &config, &bench_config)
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert!(!results[0].passed);
    assert!(results[0].error.is_some());
    assert_eq!(calls.load(Ordering::SeqCst), 3, "expected 1 + 2 attempts");
}

#[test]
fn test_unknown_model_cost_is_zero_not_panic() {
    assert!(agentbench::metrics::get_cost_model("definitely-not-a-model").is_none());
    let known = agentbench::metrics::get_cost_model("gpt-4o");
    assert!(known.is_some());
    let est = known.unwrap().estimate(1000, 1000);
    assert!(est > 0.0);
}

#[tokio::test]
async fn test_malformed_dataset_errors_not_panics() {
    let dir = std::env::temp_dir().join("agentbench-hardening");
    std::fs::create_dir_all(&dir).unwrap();

    // Not JSON at all
    let bad = dir.join("bad.json");
    std::fs::write(&bad, "{ this is not json !!!").unwrap();
    let mut suite = SWEBenchSuite::new();
    let dataset = DatasetConfig {
        source: "local".to_string(),
        path: bad.to_string_lossy().to_string(),
        split: None,
        subset: None,
        filter: None,
    };
    let err = suite.load_tasks(&dataset).await;
    assert!(err.is_err(), "malformed JSON must produce an error");

    // Valid JSON, but empty array → 0 tasks must error
    let empty = dir.join("empty.json");
    std::fs::write(&empty, "[]").unwrap();
    let dataset = DatasetConfig {
        path: empty.to_string_lossy().to_string(),
        ..DatasetConfig {
            source: "local".to_string(),
            path: String::new(),
            split: None,
            subset: None,
            filter: None,
        }
    };
    let err = suite.load_tasks(&dataset).await;
    assert!(err.is_err(), "empty dataset must produce an error");

    // Missing file
    let dataset = DatasetConfig {
        source: "local".to_string(),
        path: dir
            .join("does-not-exist.json")
            .to_string_lossy()
            .to_string(),
        split: None,
        subset: None,
        filter: None,
    };
    let err = suite.load_tasks(&dataset).await;
    assert!(err.is_err(), "missing file must produce an error");
}

#[test]
fn test_config_rejects_unknown_adapter() {
    let mut config = make_bench_config(1, 10, 0);
    config.harness.adapter = "skynet".to_string();
    assert!(config.validate().is_err());

    config.harness.adapter = "mock".to_string();
    assert!(config.validate().is_ok());
}

/// Runner must reject a suite with zero tasks instead of producing
/// a vacuous "completed" run.
#[tokio::test]
async fn test_zero_task_run_is_an_error() {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let runner = Runner::new(db);
    let suite = SWEBenchSuite::new(); // never loaded → 0 tasks

    let harness = Arc::new(FlakyHarness {
        failures_before_success: 0,
        calls: Arc::new(AtomicUsize::new(0)),
    });
    let bench_config = make_bench_config(1, 10, 0);
    let err = runner
        .run(harness, &suite, &run_config(), &bench_config)
        .await;
    assert!(err.is_err(), "zero-task run must be an error");
}
