use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::env;
use std::fs;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

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
    Dev,
    /// Dump the latest session from the development log file
    DumpLatestLogs,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Dev => run_dev(),
        Commands::DumpLatestLogs => dump_log(),
    }
}

fn dump_log() -> Result<()> {
    let project_root = find_project_root()?;
    let log_file_path = project_root.join("tmp").join("dev-log-server.log");

    if !log_file_path.exists() {
        anyhow::bail!("Log file not found: {}", log_file_path.display());
    }

    let content = fs::read_to_string(&log_file_path).context(format!(
        "Failed to read log file at: {}",
        log_file_path.display()
    ))?;

    const SESSION_START_MARKER: &str = "=== DISSONANCE_LAB_SESSION_START ===";

    if let Some(start_index) = content.rfind(SESSION_START_MARKER) {
        // Skip the "=== DISSONANCE_LAB_SESSION_START ===" line itself and process each line
        for line in content[start_index..].lines().skip(1) {
            // Clean up the line for agent consumption
            let cleaned_line = clean_log_line(line);
            if !cleaned_line.trim().is_empty() {
                println!("{cleaned_line}");
            }
        }
    } else {
        // Process full log if no session marker found
        for line in content.lines() {
            let cleaned_line = clean_log_line(line);
            if !cleaned_line.trim().is_empty() {
                println!("{cleaned_line}");
            }
        }
    }

    Ok(())
}

fn clean_log_line(line: &str) -> String {
    // With simplified log format (no timestamp, target, or module_path),
    // we can just return the line as-is since it should now be clean
    line.to_string()
}

fn run_dev() -> Result<()> {
    println!("🚀 Starting dissonance-lab development environment...");

    // Ensure we're in the project root
    let project_root = find_project_root()?;
    env::set_current_dir(&project_root).context("Failed to change to project root directory")?;

    // Start the log server in the background
    println!("📡 Starting development log server...");
    let mut log_server = start_log_server()?;

    // Wait a moment for the log server to start
    thread::sleep(Duration::from_millis(500));

    // Start trunk serve
    println!("🌐 Starting trunk development server...");
    let mut trunk_server = start_trunk_serve()?;

    println!();
    println!("✅ Development environment is ready!");
    println!("   📊 Frontend: http://localhost:8080");
    println!("   📡 Log server: http://localhost:3001");
    println!("   🛑 Press Ctrl+C to stop all servers");
    println!();

    // Set up Ctrl+C handling with channel
    let (tx, rx) = mpsc::channel();

    ctrlc::set_handler(move || {
        println!("\n🛑 Received Ctrl+C, shutting down...");
        let _ = tx.send(()); // Ignore send errors - if receiver is dropped, we're already shutting down
    })
    .expect("Error setting Ctrl-C handler");

    // Wait for Ctrl+C signal
    let _ = rx.recv(); // Ignore recv errors - any error means we should proceed to shutdown

    println!("🛑 Shutting down development environment...");

    // Kill trunk server
    if let Err(e) = trunk_server.kill() {
        eprintln!("Warning: Failed to kill trunk server: {e}");
    }

    // Kill log server
    if let Err(e) = log_server.kill() {
        eprintln!("Warning: Failed to kill log server: {e}");
    }

    println!("👋 Development environment stopped.");
    Ok(())
}

fn find_project_root() -> Result<std::path::PathBuf> {
    let current = env::current_dir().context("Failed to get current directory")?;

    // Look for Cargo.toml in current dir or parent dirs
    let mut path = current.as_path();
    loop {
        if path.join("Cargo.toml").exists() && path.join("Trunk.toml").exists() {
            return Ok(path.to_path_buf());
        }

        match path.parent() {
            Some(parent) => path = parent,
            None => {
                anyhow::bail!("Could not find project root (looking for Cargo.toml and Trunk.toml)")
            }
        }
    }
}

fn start_log_server() -> Result<Child> {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "dev-log-server"]);

    cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());

    let child = cmd
        .spawn()
        .context("Failed to start dev-log-server - make sure cargo is available")?;

    Ok(child)
}

fn start_trunk_serve() -> Result<Child> {
    // Check if trunk is available
    if which::which("trunk").is_err() {
        anyhow::bail!("trunk command not found - please install trunk with: cargo install trunk");
    }

    let mut cmd = Command::new("trunk");
    cmd.arg("serve");
    cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());

    let child = cmd.spawn().context("Failed to start trunk serve")?;

    Ok(child)
}
