use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
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
    DumpLog,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Dev => run_dev(),
        Commands::DumpLog => dump_log(),
    }
}

fn dump_log() -> Result<()> {
    let project_root = find_project_root()?;
    let tmp_dir = project_root.join("tmp");
    
    // Find the most recent log file (check both current and dated versions)
    let base_log_path = tmp_dir.join("dev-log-server.log");
    let mut log_files = vec![];
    
    // Add the base log file if it exists
    if base_log_path.exists() {
        log_files.push(base_log_path.clone());
    }
    
    // Look for dated log files
    if let Ok(entries) = fs::read_dir(&tmp_dir) {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            if let Some(name_str) = file_name.to_str() {
                if name_str.starts_with("dev-log-server.log.") {
                    log_files.push(entry.path());
                }
            }
        }
    }
    
    if log_files.is_empty() {
        anyhow::bail!("No log files found in: {}", tmp_dir.display());
    }
    
    // Sort by modification time, most recent first
    log_files.sort_by_key(|path| {
        fs::metadata(path)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });
    log_files.reverse();
    
    let log_file_path = &log_files[0];
    let content = fs::read_to_string(log_file_path)
        .context(format!("Failed to read log file at: {}", log_file_path.display()))?;

    const SESSION_START_MARKER: &str = "New session started";

    let logs_to_print = if let Some(start_index) = content.rfind(SESSION_START_MARKER) {
        // Skip the "New session started" line itself
        let session_logs = content[start_index..].lines().skip(1).collect::<Vec<_>>().join("\n");
        format!("--- Latest Log Session ---\n{session_logs}\n--- End of Log Session ---")
    } else {
        format!("No 'New session started' marker found. Showing full log.\n--- Full Log ---\n{content}\n--- End of Full Log ---")
    };

    // Colorize the output
    for line in logs_to_print.lines() {
        let colored_line = if line.contains("ERROR") {
            line.red()
        } else if line.contains("WARN") {
            line.yellow()
        } else if line.contains("INFO") {
            line.green()
        } else if line.contains("DEBUG") {
            line.blue()
        } else if line.contains("TRACE") {
            line.purple()
        } else {
            line.normal()
        };
        println!("{colored_line}");
    }

    Ok(())
}


fn run_dev() -> Result<()> {
    println!("🚀 Starting dissonance-lab development environment...");
    
    // Ensure we're in the project root
    let project_root = find_project_root()?;
    env::set_current_dir(&project_root)
        .context("Failed to change to project root directory")?;

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
    }).expect("Error setting Ctrl-C handler");

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
            None => anyhow::bail!("Could not find project root (looking for Cargo.toml and Trunk.toml)"),
        }
    }
}

fn start_log_server() -> Result<Child> {
    let mut cmd = Command::new("cargo");
    cmd.args(["run", "-p", "dev-log-server"]);
    
    cmd.stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let child = cmd.spawn()
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
    cmd.stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let child = cmd.spawn()
        .context("Failed to start trunk serve")?;

    Ok(child)
}