use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::env;
use std::fs;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// A wrapper around Child that automatically kills the process when dropped
struct ManagedProcess {
    name: String,
    child: Child,
}

impl ManagedProcess {
    fn new(name: String, child: Child) -> Self {
        Self { name, child }
    }
}

impl Drop for ManagedProcess {
    fn drop(&mut self) {
        if let Err(e) = self.child.kill() {
            eprintln!("Warning: Failed to kill {}: {e}", self.name);
        }
    }
}

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

    let content = fs::read_to_string(&log_file_path)
        .with_context(|| format!("Failed to read log file at: {}", log_file_path.display()))?;

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
    // Ensure we're in the project root first
    let project_root = find_project_root()?;
    env::set_current_dir(&project_root).context("Failed to change to project root directory")?;

    // Build the log server and main project before starting anything
    build_log_server()?;
    build_main_project()?;

    println!("🚀 Starting dissonance-lab development environment...");

    // Project root is already set above

    // Start the log server in the background
    println!("📡 Starting development log server...");
    let _log_server = start_log_server()?;

    // Wait a moment for the log server to start
    thread::sleep(Duration::from_millis(500));

    // Start trunk serve
    println!("🌐 Starting trunk development server...");
    let _trunk_server = start_trunk_serve()?;

    // Wait a bit for the initial trunk output
    thread::sleep(Duration::from_secs(4));

    println!();
    println!("✅ Development environment is ready!");
    println!("   📊 Frontend: http://localhost:8080/#dev");
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
    
    // Servers will be automatically killed when they go out of scope via Drop trait
    
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

fn start_log_server() -> Result<ManagedProcess> {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "--release", "-p", "dev-log-server"]);

    cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());

    let child = cmd
        .spawn()
        .context("Failed to start dev-log-server - make sure cargo is available")?;

    Ok(ManagedProcess::new("log server".to_string(), child))
}

fn start_trunk_serve() -> Result<ManagedProcess> {
    // Check if trunk is available
    if which::which("trunk").is_err() {
        anyhow::bail!("trunk command not found - please install trunk with: cargo install trunk");
    }

    let mut cmd = Command::new("trunk");
    cmd.arg("serve");
    cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());

    let child = cmd.spawn().context("Failed to start trunk serve")?;

    Ok(ManagedProcess::new("trunk server".to_string(), child))
}

fn build_log_server() -> Result<()> {
    println!("🔨 Building development log server (release mode)...");
    
    let mut cmd = Command::new("cargo");
    cmd.args(["build", "--release", "-p", "dev-log-server"]);
    cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());

    let status = cmd
        .status()
        .context("Failed to run cargo build for dev-log-server")?;

    if !status.success() {
        anyhow::bail!("Failed to build dev-log-server");
    }

    Ok(())
}

fn build_main_project() -> Result<()> {
    println!("🔨 Building main project...");
    
    // Check if trunk is available
    if which::which("trunk").is_err() {
        anyhow::bail!("trunk command not found - please install trunk with: cargo install trunk");
    }
    
    let mut cmd = Command::new("trunk");
    cmd.args(["build"]);
    cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());

    let status = cmd
        .status()
        .context("Failed to run trunk build for main project")?;

    if !status.success() {
        anyhow::bail!("Failed to build main project with trunk");
    }

    Ok(())
}
