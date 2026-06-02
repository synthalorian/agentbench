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

        // Calculate cost using cost model
        let total_cost: f64 = results
            .iter()
            .map(|r| {
                let model = r
                    .response
                    .metadata
                    .get("model")
                    .map(|s| s.as_str())
                    .unwrap_or("");
                let cost_model = get_cost_model(model);
                cost_model
                    .map(|cm| cm.estimate(r.response.tokens_input, r.response.tokens_output))
                    .unwrap_or(0.0)
            })
            .sum();

        Self {
            total_tasks: total,
            passed_tasks: passed,
            failed_tasks: total - passed,
            total_latency_ms: total_latency,
            total_tokens_input: total_input,
            total_tokens_output: total_output,
            total_cost_usd: total_cost,
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

    /// Check if cost exceeds budget threshold
    pub fn check_budget(&self, budget_usd: f64) -> Option<String> {
        if self.total_cost_usd >= budget_usd {
            Some(format!(
                "BUDGET ALERT: ${:.4} spent of ${:.4} budget ({:.1}%)",
                self.total_cost_usd,
                budget_usd,
                (self.total_cost_usd / budget_usd) * 100.0
            ))
        } else if self.total_cost_usd >= budget_usd * 0.8 {
            Some(format!(
                "BUDGET WARNING: ${:.4} spent of ${:.4} budget ({:.1}%)",
                self.total_cost_usd,
                budget_usd,
                (self.total_cost_usd / budget_usd) * 100.0
            ))
        } else {
            None
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
        "claude-3-haiku" => Some(CostModel {
            cost_per_1k_input: 0.00025,
            cost_per_1k_output: 0.00125,
        }),
        "gemini-pro" => Some(CostModel {
            cost_per_1k_input: 0.0005,
            cost_per_1k_output: 0.0015,
        }),
        "local" | "local-model" | "" => Some(CostModel {
            cost_per_1k_input: 0.0,
            cost_per_1k_output: 0.0,
        }),
        _ => None,
    }
}

/// Budget tracker for a benchmark run
#[derive(Debug, Clone)]
pub struct BudgetTracker {
    pub budget_usd: f64,
    pub spent_usd: f64,
    pub alerts: Vec<String>,
}

impl BudgetTracker {
    pub fn new(budget_usd: f64) -> Self {
        Self {
            budget_usd,
            spent_usd: 0.0,
            alerts: vec![],
        }
    }

    pub fn add_cost(&mut self, cost: f64) {
        self.spent_usd += cost;

        if self.spent_usd >= self.budget_usd {
            self.alerts.push(format!(
                "BUDGET EXCEEDED: ${:.4} / ${:.4}",
                self.spent_usd, self.budget_usd
            ));
        } else if self.spent_usd >= self.budget_usd * 0.9 {
            self.alerts.push(format!(
                "BUDGET CRITICAL: ${:.4} / ${:.4} ({:.1}%)",
                self.spent_usd,
                self.budget_usd,
                (self.spent_usd / self.budget_usd) * 100.0
            ));
        } else if self.spent_usd >= self.budget_usd * 0.75 {
            self.alerts.push(format!(
                "BUDGET WARNING: ${:.4} / ${:.4} ({:.1}%)",
                self.spent_usd,
                self.budget_usd,
                (self.spent_usd / self.budget_usd) * 100.0
            ));
        }
    }

    pub fn remaining(&self) -> f64 {
        (self.budget_usd - self.spent_usd).max(0.0)
    }

    pub fn is_exceeded(&self) -> bool {
        self.spent_usd >= self.budget_usd
    }
}
