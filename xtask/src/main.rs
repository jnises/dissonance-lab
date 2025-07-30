use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::env;
use std::fs;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Shutdown signal types
#[derive(Debug)]
enum ShutdownSignal {
    CtrlC,
    ProcessExit { name: String, status: ExitStatus },
}

/// A wrapper around Child that automatically kills the process when dropped
/// and can monitor the process in a separate thread
struct ManagedProcess {
    name: String,
    child: Child,
}

impl ManagedProcess {
    fn new(name: String, child: Child) -> Self {
        Self { name, child }
    }

    /// Spawn a monitoring thread that sends a shutdown signal when the process exits
    fn spawn_monitor(mut self, tx: mpsc::Sender<ShutdownSignal>) {
        let name = self.name.clone();
        thread::spawn(move || {
            match self.child.wait() {
                Ok(status) => {
                    let _ = tx.send(ShutdownSignal::ProcessExit { name, status });
                }
                Err(e) => {
                    eprintln!("Error waiting for {name}: {e}");
                }
            }
        });
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
    Dev {
        /// Address to bind servers to
        #[arg(long, default_value = "127.0.0.1")]
        bind: String,
    },
    /// Dump the latest session from the development log file
    DumpLatestLogs,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Dev { bind } => run_dev(bind),
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
            if !line.trim().is_empty() {
                println!("{line}");
            }
        }
    } else {
        // Process full log if no session marker found
        for line in content.lines() {
            if !line.trim().is_empty() {
                println!("{line}");
            }
        }
    }

    Ok(())
}

fn run_dev(bind_address: String) -> Result<()> {
    // Ensure we're in the project root first
    let project_root = find_project_root()?;
    env::set_current_dir(&project_root).context("Failed to change to project root directory")?;

    // Build the log server and main project before starting anything
    build_log_server()?;
    build_main_project()?;

    println!("🚀 Starting dissonance-lab development environment...");

    // Project root is already set above

    // Start the log server in the background (silently)
    let log_server = start_log_server()?;

    // Wait a moment for the log server to start
    thread::sleep(Duration::from_millis(500));

    // Start trunk serve
    println!("🌐 Starting trunk development server...");
    let trunk_server = start_trunk_serve(&bind_address)?;

    // Wait a bit for the initial trunk output
    thread::sleep(Duration::from_secs(4));

    println!();
    println!("✅ Development environment is ready!");
    println!("   📊 Frontend: http://{bind_address}:8080/#dev");
    println!("   🛑 Press Ctrl+C to stop all servers");
    println!();

    // Set up shutdown signal channel
    let (tx, rx) = mpsc::channel::<ShutdownSignal>();

    // Set up Ctrl+C handler
    let ctrl_c_tx = tx.clone();
    ctrlc::set_handler(move || {
        println!("\n🛑 Received Ctrl+C, shutting down...");
        let _ = ctrl_c_tx.send(ShutdownSignal::CtrlC);
    })
    .expect("Error setting Ctrl-C handler");

    // Spawn monitoring threads for both servers
    log_server.spawn_monitor(tx.clone());
    trunk_server.spawn_monitor(tx.clone());

    // Wait for any shutdown signal
    match rx.recv() {
        Ok(ShutdownSignal::CtrlC) => {
            // User requested shutdown - this is normal
        }
        Ok(ShutdownSignal::ProcessExit { name, status }) => {
            if status.success() {
                eprintln!("ℹ️  {name} exited cleanly");
            } else {
                eprintln!("❌ {name} exited with error: {status}");
                anyhow::bail!("{name} failed");
            }
        }
        Err(_) => {
            // Channel closed - shouldn't happen but handle gracefully
            eprintln!("Warning: Shutdown channel closed unexpectedly");
        }
    }

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

fn start_trunk_serve(bind_address: &str) -> Result<ManagedProcess> {
    // Check if trunk is available
    if which::which("trunk").is_err() {
        anyhow::bail!("trunk command not found - please install trunk with: cargo install trunk");
    }

    let mut cmd = Command::new("trunk");
    cmd.arg("serve");
    cmd.arg("--address");
    cmd.arg(bind_address);
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
