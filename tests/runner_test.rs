use agentbench::benchmark::swe_bench::SWEBenchSuite;
use agentbench::benchmark::{BenchmarkRunConfig, BenchmarkSuite};
use agentbench::config::{BenchmarkConfig, DatasetConfig};
use agentbench::db::Database;
use agentbench::harness::generic::GenericOpenAIHarness;
use agentbench::runner::Runner;
use std::path::PathBuf;
use std::sync::Arc;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[tokio::test]
async fn test_runner_with_mock_harness() {
    let db = Arc::new(Database::open_in_memory().unwrap());
    let runner = Runner::new(db);

    let harness = Arc::new(GenericOpenAIHarness::new());
    let mut suite = SWEBenchSuite::new();
    let config = DatasetConfig {
        source: "local".to_string(),
        path: project_root()
            .join("data/swe-bench-sample.json")
            .to_string_lossy()
            .to_string(),
        split: None,
        subset: None,
        filter: None,
    };
    suite.load_tasks(&config).await.unwrap();

    let run_config = BenchmarkRunConfig {
        harness_name: "generic".to_string(),
        max_tasks: Some(2),
        shuffle: false,
        seed: None,
    };

    let bench_config = BenchmarkConfig {
        name: "test".to_string(),
        description: "test".to_string(),
        benchmark_type: "swe_bench".to_string(),
        dataset: config,
        harness: agentbench::config::HarnessConfig {
            name: "generic".to_string(),
            adapter: "generic".to_string(),
            endpoint: None,
            api_key: None,
            model: None,
            extra: None,
        },
        runner: agentbench::config::RunnerConfig {
            max_workers: 1,
            timeout_secs: 5,
            retries: 0,
            docker_image: None,
            env: None,
        },
        scoring: agentbench::config::ScoringConfig {
            metric: "pass_rate".to_string(),
            thresholds: None,
        },
    };

    let result = runner
        .run(harness, &suite, &run_config, &bench_config)
        .await;
    // Should complete even if harness isn't initialized (will error on each task)
    assert!(result.is_ok());
    let results = result.unwrap();
    assert_eq!(results.len(), 2);
}
