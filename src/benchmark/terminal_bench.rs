use async_trait::async_trait;
use std::collections::HashMap;

use super::{BenchmarkResult, BenchmarkSuite, BenchmarkTask};
use crate::error::BenchResult;
use crate::harness::TaskResponse;

pub struct TerminalBenchSuite {
    tasks: Vec<BenchmarkTask>,
}

impl TerminalBenchSuite {
    pub fn new() -> Self {
        Self { tasks: vec![] }
    }
}

#[async_trait]
impl BenchmarkSuite for TerminalBenchSuite {
    fn name(&self) -> &str {
        "terminal_bench"
    }
    fn description(&self) -> &str {
        "Terminal-bench: Command-line task execution"
    }
    fn task_count(&self) -> usize {
        self.tasks.len()
    }

    async fn load_tasks(&mut self, _config: &crate::config::DatasetConfig) -> BenchResult<()> {
        todo!("Terminal-bench loader not yet implemented")
    }

    fn tasks(&self) -> &[BenchmarkTask] {
        &self.tasks
    }

    async fn validate(
        &self,
        _task: &BenchmarkTask,
        _response: &TaskResponse,
    ) -> BenchResult<BenchmarkResult> {
        todo!("Terminal-bench validation not yet implemented")
    }

    fn aggregate_score(&self, results: &[BenchmarkResult]) -> f64 {
        if results.is_empty() {
            return 0.0;
        }
        results.iter().map(|r| r.score).sum::<f64>() / results.len() as f64
    }
}
