use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use crate::utils::{ManagedProcess, find_project_root};

/// Start development server (log server + trunk serve)
pub fn run_dev(bind_address: String) -> Result<()> {
    // Ensure we're in the project root first
    let project_root = find_project_root()?;
    env::set_current_dir(&project_root).context("Failed to change to project root directory")?;

    build_log_server()?;

    let (tx, rx) = mpsc::channel();
    ctrlc::set_handler(move || {
        // The receiver may already be gone if a child exited first.
        let _ = tx.send(());
    })
    .context("Failed to set Ctrl-C handler")?;

    let mut log_server = start_log_server()?;
    let mut trunk_server = start_trunk_serve(&bind_address)?;
    println!("Starting development server at http://{bind_address}:8080/#dev");
    println!("Press Ctrl+C to stop both servers.");

    const PROCESS_CHECK_INTERVAL: Duration = Duration::from_millis(100);
    loop {
        match rx.recv_timeout(PROCESS_CHECK_INTERVAL) {
            Ok(()) => return Ok(()),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(e @ mpsc::RecvTimeoutError::Disconnected) => return Err(e.into()),
        }

        if log_server.has_exited()? || trunk_server.has_exited()? {
            return Ok(());
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

    Ok(ManagedProcess::new("log server", child))
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

    Ok(ManagedProcess::new("trunk server", child))
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

/// Dump the latest session from the development log file
pub fn dump_log() -> Result<()> {
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
