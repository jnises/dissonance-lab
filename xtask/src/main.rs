use anyhow::{Context, Result};
use cargo_metadata::MetadataCommand;
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
    /// Check all crates with appropriate targets
    CheckAll,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Dev { bind } => run_dev(bind),
        Commands::DumpLatestLogs => dump_log(),
        Commands::CheckAll => check_all_crates(),
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

fn check_all_crates() -> Result<()> {
    println!("🔧 Checking all crates with appropriate targets...");
    
    // Ensure we're in the project root
    let project_root = find_project_root()?;
    env::set_current_dir(&project_root).context("Failed to change to project root directory")?;

    // Get all crates in the workspace
    let crates = get_workspace_crates(&project_root)?;
    
    // Define which crates should use which target
    const NATIVE_CRATES: &[&str] = &["xtask", "dev-log-server"];
    const WASM_CRATES: &[&str] = &["dissonance-lab", "audio-worklet", "shared-types"];

    // Check native crates
    println!("📦 Checking native crates...");
    for crate_name in &crates {
        if NATIVE_CRATES.contains(&crate_name.as_str()) {
            check_native_crate(crate_name)?;
        }
    }

    // Check WASM crates
    println!("🌐 Checking WASM crates...");
    for crate_name in &crates {
        if WASM_CRATES.contains(&crate_name.as_str()) {
            check_wasm_crate(crate_name)?;
        }
    }

    // Verify all crates were checked
    let mut all_expected_crates = NATIVE_CRATES.iter().chain(WASM_CRATES.iter()).collect::<std::collections::HashSet<_>>();
    let mut missing_crates = Vec::new();
    let mut uncategorized_crates = Vec::new();

    for crate_name in &crates {
        if all_expected_crates.remove(&crate_name.as_str()) {
            // Crate was expected and found
        } else {
            uncategorized_crates.push(crate_name.clone());
        }
    }

    // Check for missing expected crates
    for missing in all_expected_crates {
        missing_crates.push(missing.to_string());
    }

    if !missing_crates.is_empty() {
        anyhow::bail!("Expected crates not found in workspace: {}", missing_crates.join(", "));
    }

    if !uncategorized_crates.is_empty() {
        println!("⚠️  Warning: Found uncategorized crates (not checked): {}", uncategorized_crates.join(", "));
        println!("   Consider adding them to NATIVE_CRATES or WASM_CRATES in check_all_crates()");
    }

    println!("✅ All crates checked successfully!");
    println!("   📦 Native crates checked: {}", NATIVE_CRATES.len());
    println!("   🌐 WASM crates checked: {}", WASM_CRATES.len());
    
    Ok(())
}

fn get_workspace_crates(project_root: &std::path::Path) -> Result<Vec<String>> {
    let metadata = MetadataCommand::new()
        .manifest_path(project_root.join("Cargo.toml"))
        .exec()
        .context("Failed to get cargo metadata")?;

    let crates: Vec<String> = metadata
        .workspace_packages()
        .iter()
        .map(|package| package.name.clone())
        .collect();

    if crates.is_empty() {
        anyhow::bail!("No crates found in workspace");
    }

    println!("📋 Found {} crates in workspace: {}", crates.len(), crates.join(", "));
    
    Ok(crates)
}

fn check_native_crate(crate_name: &str) -> Result<()> {
    println!("  Checking {crate_name} (native target)...");
    
    let mut cmd = Command::new("cargo");
    cmd.args(["check", "-p", crate_name]);
    cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());

    let status = cmd
        .status()
        .with_context(|| format!("Failed to run cargo check for {crate_name}"))?;

    if !status.success() {
        anyhow::bail!("Failed to check {crate_name}");
    }

    Ok(())
}

fn check_wasm_crate(crate_name: &str) -> Result<()> {
    println!("  Checking {crate_name} (WASM target)...");
    
    let mut cmd = Command::new("cargo");
    cmd.args(["check", "-p", crate_name, "--target", "wasm32-unknown-unknown"]);
    cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());

    let status = cmd
        .status()
        .with_context(|| format!("Failed to run cargo check for {crate_name} with WASM target"))?;

    if !status.success() {
        anyhow::bail!("Failed to check {crate_name} for WASM target");
    }

    Ok(())
}
