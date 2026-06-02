use agentbench::config::BenchmarkConfig;
use std::path::PathBuf;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn test_load_sample_benchmark_config() {
    let path = project_root().join("benches/sample-benchmark.yml");
    let config = BenchmarkConfig::from_file(&path);
    assert!(
        config.is_ok(),
        "Failed to load sample benchmark: {:?}",
        config.err()
    );

    let config = config.unwrap();
    assert_eq!(config.name, "AgentBench Sample");
    assert_eq!(config.benchmark_type, "swe_bench");
    assert_eq!(config.dataset.source, "local");
    assert_eq!(config.harness.adapter, "generic");
    assert_eq!(config.runner.max_workers, 2);
    assert_eq!(config.runner.timeout_secs, 60);
}

#[test]
fn test_load_config_file_not_found() {
    let path = project_root().join("benches/nonexistent.yml");
    let result = BenchmarkConfig::from_file(&path);
    assert!(result.is_err());
}

#[test]
fn test_load_config_invalid_yaml() {
    let tmpdir = std::env::temp_dir();
    let path = tmpdir.join("invalid-bench.yml");
    std::fs::write(&path, "this is not: valid: yaml: [").unwrap();
    let result = BenchmarkConfig::from_file(&path);
    assert!(result.is_err());
    let _ = std::fs::remove_file(&path);
}
