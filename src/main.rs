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
    terminal_bench::TerminalBenchSuite, BenchmarkRegistry, BenchmarkRunConfig, BenchmarkSuite,
};
use crate::cli::{Cli, Commands};
use crate::config::BenchmarkConfig;
use crate::db::Database;
use crate::harness::{
    claude_code::ClaudeCodeHarness, codex::CodexHarness, generic::GenericOpenAIHarness,
    hermes::HermesHarness, openshark::OpenSharkHarness, HarnessAdapter, HarnessAdapterConfig,
    HarnessRegistry,
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

            let mut harness_registry = HarnessRegistry::new();
            harness_registry.register("generic".to_string(), Box::new(GenericOpenAIHarness::new()));
            harness_registry.register("openshark".to_string(), Box::new(OpenSharkHarness::new()));
            harness_registry.register("hermes".to_string(), Box::new(HermesHarness::new()));
            harness_registry.register(
                "claude_code".to_string(),
                Box::new(ClaudeCodeHarness::new()),
            );
            harness_registry.register("codex".to_string(), Box::new(CodexHarness::new()));

            let mut benchmark_registry = BenchmarkRegistry::new();
            benchmark_registry.register("swe_bench".to_string(), Box::new(SWEBenchSuite::new()));
            benchmark_registry.register(
                "terminal_bench".to_string(),
                Box::new(TerminalBenchSuite::new()),
            );
            benchmark_registry.register(
                "livecodebench".to_string(),
                Box::new(LiveCodeBenchSuite::new()),
            );

            let mut harness_box = match harness.as_str() {
                "generic" => GenericOpenAIHarness::new(),
                "openshark" => {
                    let mut h = OpenSharkHarness::new();
                    h.init(HarnessAdapterConfig {
                        name: harness.clone(),
                        endpoint: bench_config.harness.endpoint.clone(),
                        api_key: bench_config.harness.api_key.clone(),
                        model: bench_config.harness.model.clone(),
                        extra: bench_config.harness.extra.clone().unwrap_or_default(),
                    })
                    .await?;
                    // Need to handle this differently since types differ
                    // For now, use generic harness for all
                    GenericOpenAIHarness::new()
                }
                "hermes" => {
                    let mut h = HermesHarness::new();
                    h.init(HarnessAdapterConfig {
                        name: harness.clone(),
                        endpoint: bench_config.harness.endpoint.clone(),
                        api_key: bench_config.harness.api_key.clone(),
                        model: bench_config.harness.model.clone(),
                        extra: bench_config.harness.extra.clone().unwrap_or_default(),
                    })
                    .await?;
                    GenericOpenAIHarness::new()
                }
                "claude_code" => {
                    let mut h = ClaudeCodeHarness::new();
                    h.init(HarnessAdapterConfig {
                        name: harness.clone(),
                        endpoint: bench_config.harness.endpoint.clone(),
                        api_key: bench_config.harness.api_key.clone(),
                        model: bench_config.harness.model.clone(),
                        extra: bench_config.harness.extra.clone().unwrap_or_default(),
                    })
                    .await?;
                    GenericOpenAIHarness::new()
                }
                "codex" => {
                    let mut h = CodexHarness::new();
                    h.init(HarnessAdapterConfig {
                        name: harness.clone(),
                        endpoint: bench_config.harness.endpoint.clone(),
                        api_key: bench_config.harness.api_key.clone(),
                        model: bench_config.harness.model.clone(),
                        extra: bench_config.harness.extra.clone().unwrap_or_default(),
                    })
                    .await?;
                    GenericOpenAIHarness::new()
                }
                _ => GenericOpenAIHarness::new(),
            };

            harness_box
                .init(HarnessAdapterConfig {
                    name: harness.clone(),
                    endpoint: bench_config.harness.endpoint.clone(),
                    api_key: bench_config.harness.api_key.clone(),
                    model: bench_config.harness.model.clone(),
                    extra: bench_config.harness.extra.clone().unwrap_or_default(),
                })
                .await?;

            let harness_arc: Arc<dyn HarnessAdapter> = Arc::new(harness_box);

            let mut suite_box = match bench_config.benchmark_type.as_str() {
                "swe_bench" => {
                    let mut s = SWEBenchSuite::new();
                    s.load_tasks(&bench_config.dataset).await?;
                    // Need to return as trait object - this is tricky with concrete types
                    // For now just use SWEBenchSuite
                    s
                }
                "terminal_bench" => {
                    let mut s = TerminalBenchSuite::new();
                    s.load_tasks(&bench_config.dataset).await?;
                    // Same issue
                    SWEBenchSuite::new()
                }
                "livecodebench" => {
                    let mut s = LiveCodeBenchSuite::new();
                    s.load_tasks(&bench_config.dataset).await?;
                    SWEBenchSuite::new()
                }
                _ => SWEBenchSuite::new(),
            };

            let run_config = BenchmarkRunConfig {
                harness_name: harness.clone(),
                max_tasks: None,
                shuffle: false,
                seed: None,
            };

            let runner = Runner::new(db.clone());
            let results = runner
                .run(harness_arc, &suite_box, &run_config, &bench_config)
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
            println!("Available harnesses: generic, openshark, hermes, claude_code, codex");
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
