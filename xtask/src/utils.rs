use anyhow::{Context, Result};
use std::env;
use std::process::{Child, ExitStatus};
use std::sync::mpsc;
use std::thread;

/// Shutdown signal types
#[derive(Debug)]
pub enum ShutdownSignal {
    CtrlC,
    ProcessExit { name: String, status: ExitStatus },
}

/// A wrapper around Child that automatically kills the process when dropped
/// and can monitor the process in a separate thread
pub struct ManagedProcess {
    name: String,
    child: Child,
}

impl ManagedProcess {
    pub fn new(name: String, child: Child) -> Self {
        Self { name, child }
    }

    /// Spawn a monitoring thread that sends a shutdown signal when the process exits
    pub fn spawn_monitor(mut self, tx: mpsc::Sender<ShutdownSignal>) {
        let name = self.name.clone();
        thread::spawn(move || match self.child.wait() {
            Ok(status) => {
                let _ = tx.send(ShutdownSignal::ProcessExit { name, status });
            }
            Err(e) => {
                eprintln!("Error waiting for {name}: {e}");
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

/// Find the project root by looking for Cargo.toml and Trunk.toml
pub fn find_project_root() -> Result<std::path::PathBuf> {
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
