use agentbench::benchmark::swe_bench::SWEBenchSuite;
use agentbench::benchmark::{BenchmarkRegistry, BenchmarkSuite};
use agentbench::config::DatasetConfig;
use std::path::PathBuf;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[tokio::test]
async fn test_swe_bench_load_tasks() {
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

    let result = suite.load_tasks(&config).await;
    assert!(result.is_ok(), "Failed to load tasks: {:?}", result.err());
    assert_eq!(suite.task_count(), 5);

    let tasks = suite.tasks();
    assert_eq!(tasks[0].id, "swe-bench-sample-1");
    assert_eq!(tasks[0].repo, Some("psf/requests".to_string()));
}

#[test]
fn test_benchmark_registry() {
    let mut registry = BenchmarkRegistry::new();
    assert!(registry.list().is_empty());

    let suite = SWEBenchSuite::new();
    registry.register("swe_bench".to_string(), Box::new(suite));

    let list = registry.list();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0], "swe_bench");

    let retrieved = registry.get("swe_bench");
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name(), "swe_bench");
}
