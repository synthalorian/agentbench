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
        let content = std::fs::read_to_string(path)?;
        let config: BenchmarkConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }
}
