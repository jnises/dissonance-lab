use anyhow::{Context, Result};
use cargo_metadata::MetadataCommand;
use std::env;
use std::process::{Child, Command, ExitStatus};
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

/// Get all crates in the workspace with their manifest paths
pub fn get_workspace_crates(project_root: &std::path::Path) -> Result<Vec<(String, String)>> {
    let metadata = MetadataCommand::new()
        .current_dir(project_root)
        .no_deps()
        .exec()
        .context("Failed to run cargo metadata")?;

    let mut crates = Vec::new();
    for package in metadata.packages {
        crates.push((package.name, package.manifest_path.to_string()));
    }

    Ok(crates)
}

/// Run a cargo command on a specific crate
pub fn run_cargo_command(
    command: &str,
    crate_name: &str,
    manifest_path: &str,
    target: Option<&str>,
    action_description: &str,
) -> Result<()> {
    println!("  {action_description} {crate_name} ...");

    let mut args = vec![command, "--quiet", "--manifest-path", manifest_path];

    if let Some(target) = target {
        args.extend_from_slice(&["--target", target]);
    }

    let output = Command::new("cargo")
        .args(&args)
        .output()
        .with_context(|| format!("Failed to run cargo {command} on {crate_name}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("cargo {} failed on {}: {}", command, crate_name, stderr);
    }

    Ok(())
}
