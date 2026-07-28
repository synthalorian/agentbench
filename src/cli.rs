use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "agentbench")]
#[command(about = "Benchmark runner for AI coding agents")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Run a benchmark suite
    Run {
        /// Path to benchmark config YAML
        #[arg(short, long)]
        config: String,
        /// Harness to benchmark (e.g., generic, openshark, hermes, mock). Defaults to the config's harness.adapter; --dry-run implies mock.
        #[arg(short = 'H', long)]
        harness: Option<String>,
        /// Output format: table, json, markdown
        #[arg(short, long, default_value = "table")]
        output: String,
        /// Dry-run mode — use mock harness, no external API calls
        #[arg(long)]
        dry_run: bool,
        /// Maximum number of tasks to run (for quick testing)
        #[arg(short, long)]
        max_tasks: Option<usize>,
    },
    /// Start the TUI dashboard
    Tui,
    /// Start the web dashboard
    Web {
        #[arg(short, long, default_value = "8910")]
        port: u16,
    },
    /// List available harnesses and benchmarks
    List,
    /// Export results to a report
    Report {
        /// Run ID to report on
        run_id: String,
        /// Output format: markdown, json, html
        #[arg(short, long, default_value = "markdown")]
        format: String,
        /// Output file path
        #[arg(short, long)]
        output: Option<String>,
    },
}
