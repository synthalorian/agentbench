use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

use crate::error::BenchResult;

#[derive(Debug, Clone, Deserialize)]
pub struct BenchmarkConfig {
    pub name: String,
    pub description: String,
    pub benchmark_type: String, // "swe_bench", "terminal_bench", "livecodebench"
    pub dataset: DatasetConfig,
    pub harness: HarnessConfig,
    pub runner: RunnerConfig,
    pub scoring: ScoringConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatasetConfig {
    pub source: String, // "huggingface", "local", "git"
    pub path: String,
    pub split: Option<String>,
    pub subset: Option<String>,
    pub filter: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HarnessConfig {
    pub name: String,
    pub adapter: String, // "generic", "openshark", "hermes", "claude_code", "codex"
    pub endpoint: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub extra: Option<HashMap<String, serde_yaml::Value>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RunnerConfig {
    pub max_workers: usize,
    pub timeout_secs: u64,
    pub retries: u32,
    pub docker_image: Option<String>,
    pub env: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScoringConfig {
    pub metric: String, // "pass_rate", "exact_match", "bleu"
    pub thresholds: Option<HashMap<String, f64>>,
}

impl BenchmarkConfig {
    pub fn from_file<P: AsRef<Path>>(path: P) -> BenchResult<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Err(crate::error::BenchError::Config(format!(
                "Config file not found: {}",
                path.display()
            )));
        }

        let content = std::fs::read_to_string(path).map_err(|e| {
            crate::error::BenchError::Config(format!(
                "Failed to read config file '{}': {}",
                path.display(),
                e
            ))
        })?;

        let config: BenchmarkConfig = serde_yaml::from_str(&content).map_err(|e| {
            crate::error::BenchError::Config(format!(
                "Failed to parse config file '{}': {}\n\nHint: Check that all required fields are present (name, description, benchmark_type, dataset, harness, runner, scoring)",
                path.display(),
                e
            ))
        })?;

        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> BenchResult<()> {
        if self.name.trim().is_empty() {
            return Err(crate::error::BenchError::Config(
                "Benchmark 'name' cannot be empty".to_string(),
            ));
        }

        let valid_types = ["swe_bench", "terminal_bench", "livecodebench"];
        if !valid_types.contains(&self.benchmark_type.as_str()) {
            return Err(crate::error::BenchError::Config(format!(
                "Unknown benchmark_type: '{}'. Must be one of: {:?}",
                self.benchmark_type, valid_types
            )));
        }

        let valid_adapters = [
            "generic", "openshark", "hermes", "claude_code", "codex", "opencode",
        ];
        if !valid_adapters.contains(&self.harness.adapter.as_str()) {
            return Err(crate::error::BenchError::Config(format!(
                "Unknown harness adapter: '{}'. Must be one of: {:?}",
                self.harness.adapter, valid_adapters
            )));
        }

        if self.runner.max_workers == 0 {
            return Err(crate::error::BenchError::Config(
                "runner.max_workers must be at least 1".to_string(),
            ));
        }

        if self.runner.timeout_secs == 0 {
            return Err(crate::error::BenchError::Config(
                "runner.timeout_secs must be greater than 0".to_string(),
            ));
        }

        if self.dataset.source == "local" && !Path::new(&self.dataset.path).exists() {
            return Err(crate::error::BenchError::Config(format!(
                "Local dataset path does not exist: {}",
                self.dataset.path
            )));
        }

        Ok(())
    }
}
