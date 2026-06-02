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
    livecodebench::LiveCodeBenchSuite,
    swe_bench::SWEBenchSuite,
    terminal_bench::TerminalBenchSuite,
    BenchmarkRegistry, BenchmarkRunConfig, BenchmarkSuite,
};
use crate::cli::{Cli, Commands};
use crate::config::BenchmarkConfig;
use crate::db::Database;
use crate::harness::{
    claude_code::ClaudeCodeHarness,
    codex::CodexHarness,
    generic::GenericOpenAIHarness,
    hermes::HermesHarness,
    openshark::OpenSharkHarness,
    HarnessAdapter, HarnessAdapterConfig, HarnessRegistry,
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
        } => {
            let bench_config = BenchmarkConfig::from_file(&config)?;
            let db = Arc::new(Database::new("agentbench.db")?);

            // Build harness
            let harness_adapter: Box<dyn HarnessAdapter> = match harness.as_str() {
                "generic" => {
                    let mut h = GenericOpenAIHarness::new();
                    h.init(build_harness_config(&harness, &bench_config)).await?;
                    Box::new(h)
                }
                "openshark" => {
                    let mut h = OpenSharkHarness::new();
                    h.init(build_harness_config(&harness, &bench_config)).await?;
                    Box::new(h)
                }
                "hermes" => {
                    let mut h = HermesHarness::new();
                    h.init(build_harness_config(&harness, &bench_config)).await?;
                    Box::new(h)
                }
                "claude_code" => {
                    let mut h = ClaudeCodeHarness::new();
                    h.init(build_harness_config(&harness, &bench_config)).await?;
                    Box::new(h)
                }
                "codex" => {
                    let mut h = CodexHarness::new();
                    h.init(build_harness_config(&harness, &bench_config)).await?;
                    Box::new(h)
                }
                "opencode" => {
                    let mut h = crate::harness::opencode::OpenCodeHarness::new();
                    h.init(build_harness_config(&harness, &bench_config)).await?;
                    Box::new(h)
                }
                _ => {
                    return Err(anyhow::anyhow!("Unknown harness: {}", harness));
                }
            };

            let harness_arc: Arc<dyn HarnessAdapter> = harness_adapter.into();

            // Build benchmark suite
            let mut suite_box: Box<dyn BenchmarkSuite> = match bench_config.benchmark_type.as_str() {
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
                max_tasks: None,
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
                    println!("AgentBench Results — {} tasks", results.len());
                    let passed = results.iter().filter(|r| r.passed).count();
                    println!(
                        "Passed: {}/{} ({:.1}%)",
                        passed,
                        results.len(),
                        if !results.is_empty() {
                            passed as f64 / results.len() as f64 * 100.0
                        } else {
                            0.0
                        }
                    );
                }
            }
        }
        Commands::List => {
            println!("Available harnesses: generic, openshark, hermes, claude_code, codex, opencode");
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
            run_id: _,
            format: _,
            output: _,
        } => {
            println!("Report generation not yet implemented.");
        }
    }

    Ok(())
}

fn build_harness_config(harness_name: &str, bench_config: &BenchmarkConfig) -> HarnessAdapterConfig {
    HarnessAdapterConfig {
        name: harness_name.to_string(),
        endpoint: bench_config.harness.endpoint.clone(),
        api_key: bench_config.harness.api_key.clone(),
        model: bench_config.harness.model.clone(),
        extra: bench_config.harness.extra.clone().unwrap_or_default(),
    }
}
