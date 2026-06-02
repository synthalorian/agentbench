use serde::{Deserialize, Serialize};

/// Per-task metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskMetrics {
    pub latency_ms: u64,
    pub tokens_input: u64,
    pub tokens_output: u64,
    pub cost_usd: f64,
}

/// Per-run aggregate metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunMetrics {
    pub total_tasks: usize,
    pub passed_tasks: usize,
    pub failed_tasks: usize,
    pub total_latency_ms: u64,
    pub total_tokens_input: u64,
    pub total_tokens_output: u64,
    pub total_cost_usd: f64,
    pub pass_rate: f64,
    pub avg_latency_ms: f64,
}

impl RunMetrics {
    pub fn from_results(results: &[crate::benchmark::BenchmarkResult]) -> Self {
        let total = results.len();
        let passed = results.iter().filter(|r| r.passed).count();

        let total_latency: u64 = results.iter().map(|r| r.response.latency_ms).sum();
        let total_input: u64 = results.iter().map(|r| r.response.tokens_input).sum();
        let total_output: u64 = results.iter().map(|r| r.response.tokens_output).sum();

        Self {
            total_tasks: total,
            passed_tasks: passed,
            failed_tasks: total - passed,
            total_latency_ms: total_latency,
            total_tokens_input: total_input,
            total_tokens_output: total_output,
            total_cost_usd: 0.0, // TODO: cost model
            pass_rate: if total > 0 {
                passed as f64 / total as f64
            } else {
                0.0
            },
            avg_latency_ms: if total > 0 {
                total_latency as f64 / total as f64
            } else {
                0.0
            },
        }
    }
}

/// Cost model for different providers/models
#[derive(Debug, Clone)]
pub struct CostModel {
    pub cost_per_1k_input: f64,
    pub cost_per_1k_output: f64,
}

impl CostModel {
    pub fn estimate(&self, tokens_input: u64, tokens_output: u64) -> f64 {
        let input_cost = (tokens_input as f64 / 1000.0) * self.cost_per_1k_input;
        let output_cost = (tokens_output as f64 / 1000.0) * self.cost_per_1k_output;
        input_cost + output_cost
    }
}

pub fn get_cost_model(model: &str) -> Option<CostModel> {
    match model {
        "gpt-4" | "gpt-4-turbo" => Some(CostModel {
            cost_per_1k_input: 0.03,
            cost_per_1k_output: 0.06,
        }),
        "gpt-3.5-turbo" => Some(CostModel {
            cost_per_1k_input: 0.0005,
            cost_per_1k_output: 0.0015,
        }),
        "claude-3-opus" => Some(CostModel {
            cost_per_1k_input: 0.015,
            cost_per_1k_output: 0.075,
        }),
        "claude-3-sonnet" => Some(CostModel {
            cost_per_1k_input: 0.003,
            cost_per_1k_output: 0.015,
        }),
        _ => None,
    }
}
