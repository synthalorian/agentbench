use serde::Serialize;

use crate::benchmark::BenchmarkResult;
use crate::metrics::RunMetrics;

#[derive(Debug, Serialize)]
pub struct Report {
    pub run_id: String,
    pub harness_name: String,
    pub benchmark_name: String,
    pub metrics: RunMetrics,
    pub results: Vec<BenchmarkResult>,
}

impl Report {
    pub fn to_markdown(&self) -> String {
        let mut md = format!(
            "# AgentBench Report\n\n**Run ID:** {}\n**Harness:** {}\n**Benchmark:** {}\n\n",
            self.run_id, self.harness_name, self.benchmark_name
        );

        md.push_str("## Summary\n\n");
        md.push_str(&format!(
            "- **Total Tasks:** {}\n",
            self.metrics.total_tasks
        ));
        md.push_str(&format!("- **Passed:** {}\n", self.metrics.passed_tasks));
        md.push_str(&format!("- **Failed:** {}\n", self.metrics.failed_tasks));
        md.push_str(&format!(
            "- **Pass Rate:** {:.1}%\n",
            self.metrics.pass_rate * 100.0
        ));
        md.push_str(&format!(
            "- **Avg Latency:** {:.0}ms\n",
            self.metrics.avg_latency_ms
        ));
        md.push_str(&format!(
            "- **Total Tokens:** {}\n",
            self.metrics.total_tokens_input + self.metrics.total_tokens_output
        ));
        md.push_str("\n");

        md.push_str("## Results\n\n");
        md.push_str("| Task | Status | Score | Latency | Input | Output |\n");
        md.push_str("|------|--------|-------|---------|-------|--------|\n");

        for r in &self.results {
            let status = if r.passed { "✅ PASS" } else { "❌ FAIL" };
            md.push_str(&format!(
                "| {} | {} | {:.2} | {}ms | {} | {} |\n",
                r.task_id,
                status,
                r.score,
                r.response.latency_ms,
                r.response.tokens_input,
                r.response.tokens_output
            ));
        }

        md
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn to_html(&self) -> String {
        format!(
            r#"<!DOCTYPE html>
<html>
<head><title>AgentBench Report</title></head>
<body>
<h1>AgentBench Report</h1>
<p>Run ID: {}</p>
<p>Harness: {}</p>
<p>Benchmark: {}</p>
<h2>Summary</h2>
<ul>
<li>Total Tasks: {}</li>
<li>Passed: {}</li>
<li>Pass Rate: {:.1}%</li>
</ul>
<p><em>HTML report generation is a stub. Full dashboard coming in v0.2.0.</em></p>
</body>
</html>"#,
            self.run_id,
            self.harness_name,
            self.benchmark_name,
            self.metrics.total_tasks,
            self.metrics.passed_tasks,
            self.metrics.pass_rate * 100.0
        )
    }
}
