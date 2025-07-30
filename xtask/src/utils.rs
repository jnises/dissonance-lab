use anyhow::{Context, Result};
use cargo_metadata::MetadataCommand;
use std::env;
use std::process::{Child, Command, ExitStatus, Stdio};
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

/// Get all crates in the workspace
pub fn get_workspace_crates(project_root: &std::path::Path) -> Result<Vec<String>> {
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

    println!(
        "📋 Found {} crates in workspace: {}",
        crates.len(),
        crates.join(", ")
    );

    Ok(crates)
}

/// Run a cargo command for a specific crate with given target
pub fn run_cargo_command(
    command: &str,
    crate_name: &str,
    target: Option<&str>,
    description: &str,
) -> Result<()> {
    println!(
        "  {description} {crate_name}{}...",
        if let Some(t) = target {
            format!(" ({t} target)")
        } else {
            " (native target)".to_string()
        }
    );

    let mut cmd = Command::new("cargo");
    cmd.args([command, "-p", crate_name]);

    if let Some(target) = target {
        cmd.args(["--target", target]);
    }

    cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());

    let status = cmd.status().with_context(|| {
        format!(
            "Failed to run cargo {command} for {crate_name}{}",
            if let Some(t) = target {
                format!(" with {t} target")
            } else {
                String::new()
            }
        )
    })?;

    if !status.success() {
        anyhow::bail!(
            "Failed to {command} {crate_name}{}",
            if let Some(t) = target {
                format!(" for {t} target")
            } else {
                String::new()
            }
        );
    }

    Ok(())
}
