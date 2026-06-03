use agentbench::benchmark::livecodebench::LiveCodeBenchSuite;
use agentbench::benchmark::swe_bench::SWEBenchSuite;
use agentbench::benchmark::terminal_bench::TerminalBenchSuite;
use agentbench::benchmark::{BenchmarkRunConfig, BenchmarkSuite};
use agentbench::config::{BenchmarkConfig, DatasetConfig};
use agentbench::db::Database;
use agentbench::harness::mock::MockHarness;
use agentbench::harness::{HarnessAdapter, HarnessAdapterConfig};
use agentbench::runner::Runner;
use std::path::PathBuf;
use std::sync::Arc;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn make_dataset_config(path: &str) -> DatasetConfig {
    DatasetConfig {
        source: "local".to_string(),
        path: project_root().join(path).to_string_lossy().to_string(),
        split: None,
        subset: None,
        filter: None,
    }
}

fn make_bench_config(bench_type: &str, dataset: DatasetConfig) -> BenchmarkConfig {
    BenchmarkConfig {
        name: "test".to_string(),
        description: "test".to_string(),
        benchmark_type: bench_type.to_string(),
        dataset,
        harness: agentbench::config::HarnessConfig {
            name: "mock".to_string(),
            adapter: "mock".to_string(),
            endpoint: None,
            api_key: None,
            model: None,
            extra: None,
        },
        runner: agentbench::config::RunnerConfig {
            max_workers: 2,
            timeout_secs: 10,
            retries: 0,
            docker_image: None,
            env: None,
        },
        scoring: agentbench::config::ScoringConfig {
            metric: "pass_rate".to_string(),
            thresholds: None,
        },
    }
}

async fn make_mock_harness() -> Arc<dyn HarnessAdapter> {
    let mut h = MockHarness::new();
    h.init(HarnessAdapterConfig {
        name: "mock".to_string(),
        endpoint: None,
        api_key: None,
        model: None,
        extra: Default::default(),
    })
    .await
    .unwrap();
    Arc::new(h)
}

#[tokio::test]
async fn test_runner_swe_bench() {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let runner = Runner::new(db);
    let harness = make_mock_harness().await;

    let mut suite = SWEBenchSuite::new();
    let dataset = make_dataset_config("data/swe-bench-sample.json");
    suite.load_tasks(&dataset).await.unwrap();

    let run_config = BenchmarkRunConfig {
        harness_name: "mock".to_string(),
        max_tasks: Some(3),
        shuffle: false,
        seed: None,
    };

    let bench_config = make_bench_config("swe_bench", dataset);
    let results = runner
        .run(harness, &suite, &run_config, &bench_config)
        .await
        .unwrap();

    assert_eq!(results.len(), 3);
    // Mock harness returns a patch, so format check passes
    assert!(results.iter().all(|r| r.passed));
}

#[tokio::test]
async fn test_runner_terminal_bench() {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let runner = Runner::new(db);
    let harness = make_mock_harness().await;

    let mut suite = TerminalBenchSuite::new();
    let dataset = make_dataset_config("data/terminal-bench-sample.json");
    suite.load_tasks(&dataset).await.unwrap();

    let run_config = BenchmarkRunConfig {
        harness_name: "mock".to_string(),
        max_tasks: Some(2),
        shuffle: false,
        seed: None,
    };

    let bench_config = make_bench_config("terminal_bench", dataset);
    let results = runner
        .run(harness, &suite, &run_config, &bench_config)
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
    // Mock harness output won't match expected commands, so all fail
    assert!(results.iter().all(|r| !r.passed));
}

#[tokio::test]
async fn test_runner_livecodebench() {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let runner = Runner::new(db);
    let harness = make_mock_harness().await;

    let mut suite = LiveCodeBenchSuite::new();
    let dataset = make_dataset_config("data/livecodebench-sample.json");
    suite.load_tasks(&dataset).await.unwrap();

    let run_config = BenchmarkRunConfig {
        harness_name: "mock".to_string(),
        max_tasks: Some(2),
        shuffle: false,
        seed: None,
    };

    let bench_config = make_bench_config("livecodebench", dataset);
    let results = runner
        .run(harness, &suite, &run_config, &bench_config)
        .await
        .unwrap();

    assert_eq!(results.len(), 2);
    // Mock harness output is not valid Python, so compilation fails
    assert!(results.iter().all(|r| !r.passed));
}
