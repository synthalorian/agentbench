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
    swe_bench::SWEBenchSuite, BenchmarkRegistry, BenchmarkRunConfig, BenchmarkSuite,
};
use crate::cli::{Cli, Commands};
use crate::config::BenchmarkConfig;
use crate::db::Database;
use crate::harness::{
    generic::GenericOpenAIHarness, HarnessAdapter, HarnessAdapterConfig, HarnessRegistry,
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

            let mut benchmark_registry = BenchmarkRegistry::new();
            benchmark_registry.register("swe_bench".to_string(), Box::new(SWEBenchSuite::new()));

            let harness_adapter = harness_registry
                .get(&harness)
                .ok_or_else(|| anyhow::anyhow!("Harness '{}' not found", harness))?;

            let mut harness_box = GenericOpenAIHarness::new();
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

            let suite = benchmark_registry
                .get(&bench_config.benchmark_type)
                .ok_or_else(|| {
                    anyhow::anyhow!("Benchmark '{}' not found", bench_config.benchmark_type)
                })?;

            let mut suite_box = SWEBenchSuite::new();
            suite_box.load_tasks(&bench_config.dataset).await?;

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
            println!("Available harnesses: generic");
            println!("Available benchmarks: swe_bench");
        }
        Commands::Tui => {
            println!("TUI not yet implemented. Use 'run' command instead.");
        }
        Commands::Web { port } => {
            println!("Web dashboard not yet implemented. Port: {}", port);
        }
        Commands::Report {
            run_id,
            format,
            output,
        } => {
            println!("Report generation not yet implemented. Run ID: {}", run_id);
        }
    }

    Ok(())
}
