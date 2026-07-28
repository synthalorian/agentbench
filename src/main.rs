#![allow(dead_code)]

use clap::Parser;
use std::sync::Arc;

mod benchmark;
mod cli;
mod config;
mod db;
mod error;
mod harness;
mod metrics;
mod report;
mod runner;
mod tui;
mod web;

use crate::benchmark::{
    livecodebench::LiveCodeBenchSuite, swe_bench::SWEBenchSuite,
    terminal_bench::TerminalBenchSuite, BenchmarkRunConfig, BenchmarkSuite,
};
use crate::cli::{Cli, Commands};
use crate::config::BenchmarkConfig;
use crate::db::Database;
use crate::harness::{
    claude_code::ClaudeCodeHarness, codex::CodexHarness, generic::GenericOpenAIHarness,
    hermes::HermesHarness, openshark::OpenSharkHarness, HarnessAdapter, HarnessAdapterConfig,
};
use crate::runner::Runner;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let args = Cli::parse();

    match args.command {
        Commands::Run {
            config,
            harness,
            output,
            dry_run,
            max_tasks,
        } => {
            let bench_config = BenchmarkConfig::from_file(&config)?;
            let db = Arc::new(Database::new("agentbench.db")?);

            // Harness resolution: --harness flag wins; --dry-run forces mock;
            // otherwise fall back to the config file's harness.adapter.
            let harness = match (harness, dry_run) {
                (Some(h), _) => h,
                (None, true) => "mock".to_string(),
                (None, false) => bench_config.harness.adapter.clone(),
            };

            let harness_adapter: Box<dyn HarnessAdapter> = if dry_run {
                let mut h = crate::harness::mock::MockHarness::new();
                h.init(build_harness_config(&harness, &bench_config))
                    .await?;
                Box::new(h)
            } else {
                match harness.as_str() {
                    "generic" => {
                        let mut h = GenericOpenAIHarness::new();
                        h.init(build_harness_config(&harness, &bench_config))
                            .await?;
                        Box::new(h)
                    }
                    "mock" => {
                        let mut h = crate::harness::mock::MockHarness::new();
                        h.init(build_harness_config(&harness, &bench_config))
                            .await?;
                        Box::new(h)
                    }
                    "openshark" => {
                        let mut h = OpenSharkHarness::new();
                        h.init(build_harness_config(&harness, &bench_config))
                            .await?;
                        Box::new(h)
                    }
                    "hermes" => {
                        let mut h = HermesHarness::new();
                        h.init(build_harness_config(&harness, &bench_config))
                            .await?;
                        Box::new(h)
                    }
                    "claude_code" => {
                        let mut h = ClaudeCodeHarness::new();
                        h.init(build_harness_config(&harness, &bench_config))
                            .await?;
                        Box::new(h)
                    }
                    "codex" => {
                        let mut h = CodexHarness::new();
                        h.init(build_harness_config(&harness, &bench_config))
                            .await?;
                        Box::new(h)
                    }
                    "opencode" => {
                        let mut h = crate::harness::opencode::OpenCodeHarness::new();
                        h.init(build_harness_config(&harness, &bench_config))
                            .await?;
                        Box::new(h)
                    }
                    _ => {
                        return Err(anyhow::anyhow!(
                            "Unknown harness: '{}'. Available: generic, mock, openshark, hermes, claude_code, codex, opencode",
                            harness
                        ));
                    }
                }
            };

            let harness_arc: Arc<dyn HarnessAdapter> = harness_adapter.into();

            // Build benchmark suite
            let suite_box: Box<dyn BenchmarkSuite> = match bench_config.benchmark_type.as_str() {
                "swe_bench" => {
                    let mut s = SWEBenchSuite::new();
                    s.load_tasks(&bench_config.dataset).await?;
                    Box::new(s)
                }
                "terminal_bench" => {
                    let mut s = TerminalBenchSuite::new();
                    s.load_tasks(&bench_config.dataset).await?;
                    Box::new(s)
                }
                "livecodebench" => {
                    let mut s = LiveCodeBenchSuite::new();
                    s.load_tasks(&bench_config.dataset).await?;
                    Box::new(s)
                }
                _ => {
                    return Err(anyhow::anyhow!(
                        "Unknown benchmark: {}",
                        bench_config.benchmark_type
                    ));
                }
            };

            let run_config = BenchmarkRunConfig {
                harness_name: harness.clone(),
                max_tasks,
                shuffle: false,
                seed: None,
            };

            let runner = Runner::new(db.clone());
            let results = runner
                .run(harness_arc, suite_box.as_ref(), &run_config, &bench_config)
                .await?;

            match output.as_str() {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&results)?);
                }
                "markdown" => {
                    println!("# AgentBench Results\n");
                    println!("| Task | Passed | Score | Latency | Tokens |");
                    println!("|------|--------|-------|---------|--------|");
                    for r in &results {
                        println!(
                            "| {} | {} | {:.2} | {}ms | {} |",
                            r.task_id,
                            r.passed,
                            r.score,
                            r.response.latency_ms,
                            r.response.tokens_input + r.response.tokens_output
                        );
                    }
                }
                _ => {
                    let metrics = crate::metrics::RunMetrics::from_results(&results);
                    println!("AgentBench Results — {} tasks", metrics.total_tasks);
                    println!(
                        "Passed: {}/{} ({:.1}%)",
                        metrics.passed_tasks,
                        metrics.total_tasks,
                        metrics.pass_rate * 100.0
                    );
                    println!(
                        "Tokens: {} ({} in / {} out)",
                        metrics.total_tokens_input + metrics.total_tokens_output,
                        metrics.total_tokens_input,
                        metrics.total_tokens_output
                    );
                    println!("Estimated cost: ${:.4}", metrics.total_cost_usd);
                }
            }
        }
        Commands::List => {
            println!("Available harnesses: generic, mock, openshark, hermes, claude_code, codex, opencode");
            println!("Available benchmarks: swe_bench, terminal_bench, livecodebench");
        }
        Commands::Tui => {
            tui::run_tui().await?;
        }
        Commands::Web { port } => {
            let db = Arc::new(Database::new("agentbench.db")?);
            web::serve(db, port).await?;
        }
        Commands::Report {
            run_id,
            format,
            output,
        } => {
            let db = Arc::new(Database::new("agentbench.db")?);
            let runs = db.get_runs(1000)?;
            let run = runs.iter().find(|r| r.id == run_id).ok_or_else(|| {
                anyhow::anyhow!(
                    "Run '{}' not found. Check the TUI or web dashboard for available runs.",
                    run_id
                )
            })?;

            let db_results = db.get_results(&run_id)?;

            // Reconstruct full results from the DB rows
            let results: Vec<crate::benchmark::BenchmarkResult> = db_results
                .into_iter()
                .map(|r| crate::benchmark::BenchmarkResult {
                    task_id: r.task_id,
                    harness_name: run.harness_name.clone(),
                    benchmark_name: run.benchmark_name.clone(),
                    passed: r.passed,
                    score: r.score,
                    response: crate::harness::TaskResponse {
                        task_id: String::new(),
                        output: r.output.unwrap_or_default(),
                        patch: r.patch,
                        tool_calls: vec![],
                        metadata: Default::default(),
                        latency_ms: r.latency_ms.unwrap_or(0) as u64,
                        tokens_input: r.tokens_input.unwrap_or(0) as u64,
                        tokens_output: r.tokens_output.unwrap_or(0) as u64,
                    },
                    validation_output: None,
                    error: r.error,
                    started_at: chrono::Utc::now(),
                    finished_at: chrono::Utc::now(),
                })
                .collect();

            let metrics = crate::metrics::RunMetrics::from_results(&results);
            let report = crate::report::Report {
                run_id: run.id.clone(),
                harness_name: run.harness_name.clone(),
                benchmark_name: run.benchmark_name.clone(),
                metrics,
                results,
            };

            let rendered = match format.as_str() {
                "markdown" | "md" => report.to_markdown(),
                "json" => report.to_json(),
                "html" => report.to_html(),
                other => {
                    return Err(anyhow::anyhow!(
                        "Unknown report format: '{}'. Use markdown, json, or html.",
                        other
                    ))
                }
            };

            if let Some(out_path) = output {
                std::fs::write(&out_path, &rendered)?;
                println!("Report written to {}", out_path);
            } else {
                println!("{}", rendered);
            }
        }
    }

    Ok(())
}

fn build_harness_config(
    harness_name: &str,
    bench_config: &BenchmarkConfig,
) -> HarnessAdapterConfig {
    HarnessAdapterConfig {
        name: harness_name.to_string(),
        endpoint: bench_config.harness.endpoint.clone(),
        api_key: bench_config.harness.api_key.clone(),
        model: bench_config.harness.model.clone(),
        extra: bench_config.harness.extra.clone().unwrap_or_default(),
    }
}
