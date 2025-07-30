use anyhow::Result;
use clap::{Parser, Subcommand};

mod check;
mod dev;
mod utils;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Development utility tasks for dissonance-lab")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start development server (log server + trunk serve)
    Dev {
        /// Address to bind servers to
        #[arg(long, default_value = "127.0.0.1")]
        bind: String,
    },
    /// Dump the latest session from the development log file
    DumpLatestLogs,
    /// Check all crates with appropriate targets
    CheckAll,
    /// Run clippy on all crates with appropriate targets
    ClippyAll,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Dev { bind } => dev::run_dev(bind),
        Commands::DumpLatestLogs => dev::dump_log(),
        Commands::CheckAll => check::check_all_crates(),
        Commands::ClippyAll => check::clippy_all_crates(),
    }
}


