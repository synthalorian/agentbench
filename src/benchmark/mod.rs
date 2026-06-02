use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::BenchResult;
use crate::harness::TaskResponse;

pub mod huggingface;
pub mod livecodebench;
pub mod swe_bench;
pub mod terminal_bench;

/// A single benchmark task (loaded from dataset)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkTask {
    pub id: String,
    pub task_type: String,
    pub repo: Option<String>,
    pub base_commit: Option<String>,
    pub problem_statement: String,
    pub hints: Vec<String>,
    pub test_patch: Option<String>,
    pub expected_files: Vec<String>,
    pub metadata: HashMap<String, String>,
}

/// Result of running a single benchmark task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub task_id: String,
    pub harness_name: String,
    pub benchmark_name: String,
    pub passed: bool,
    pub score: f64,
    pub response: TaskResponse,
    pub validation_output: Option<String>,
    pub error: Option<String>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub finished_at: chrono::DateTime<chrono::Utc>,
}

/// Configuration for a benchmark run
#[derive(Debug, Clone)]
pub struct BenchmarkRunConfig {
    pub harness_name: String,
    pub max_tasks: Option<usize>,
    pub shuffle: bool,
    pub seed: Option<u64>,
}

/// Core trait for all benchmark suites
#[async_trait]
pub trait BenchmarkSuite: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn task_count(&self) -> usize;

    /// Load tasks from the dataset source
    async fn load_tasks(&mut self, config: &crate::config::DatasetConfig) -> BenchResult<()>;

    /// Get all loaded tasks
    fn tasks(&self) -> &[BenchmarkTask];

    /// Validate a harness response against the expected solution
    async fn validate(
        &self,
        task: &BenchmarkTask,
        response: &TaskResponse,
    ) -> BenchResult<BenchmarkResult>;

    /// Compute aggregate score from individual results
    fn aggregate_score(&self, results: &[BenchmarkResult]) -> f64;
}

/// Registry of available benchmark suites
pub struct BenchmarkRegistry {
    suites: HashMap<String, Box<dyn BenchmarkSuite>>,
}

impl BenchmarkRegistry {
    pub fn new() -> Self {
        Self {
            suites: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: String, suite: Box<dyn BenchmarkSuite>) {
        self.suites.insert(name, suite);
    }

    pub fn get(&self, name: &str) -> Option<&dyn BenchmarkSuite> {
        self.suites.get(name).map(|s| s.as_ref())
    }

    pub fn list(&self) -> Vec<&str> {
        self.suites.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for BenchmarkRegistry {
    fn default() -> Self {
        Self::new()
    }
}
