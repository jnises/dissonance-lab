use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::env;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;
use std::io;

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
    /// Build the project for release
    Build,
    /// Build the project for debug
    BuildDebug,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Dev => run_dev(),
        Commands::Build => run_build(false),
        Commands::BuildDebug => run_build(true),
    }
}

fn run_dev() -> Result<()> {
    println!("🚀 Starting dissonance-lab development environment...");
    
    // Ensure we're in the project root
    let project_root = find_project_root()?;
    env::set_current_dir(&project_root)
        .context("Failed to change to project root directory")?;

    // Generate config.js first (needed before trunk starts)
    generate_config_js(true)?;

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
    println!("   🛑 Press Enter or Ctrl+C to stop all servers");
    println!();

    // Simple approach: wait for user input (Enter key or Ctrl+C will both work)
    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();

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

fn run_build(debug: bool) -> Result<()> {
    let mode = if debug { "debug" } else { "release" };
    println!("🔨 Building dissonance-lab in {mode} mode...");
    
    // Ensure we're in the project root
    let project_root = find_project_root()?;
    env::set_current_dir(&project_root)
        .context("Failed to change to project root directory")?;

    // Generate config.js first
    generate_config_js(debug)?;

    // Build audio worklet
    build_audio_worklet(debug)?;

    // Run trunk build
    let mut cmd = Command::new("trunk");
    cmd.arg("build");
    
    if !debug {
        cmd.arg("--release");
    }

    let status = cmd.status()
        .context("Failed to run trunk build - make sure trunk is installed")?;

    if !status.success() {
        anyhow::bail!("Trunk build failed");
    }

    println!("✅ Build completed successfully!");
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

fn generate_config_js(debug: bool) -> Result<()> {
    println!("📄 Generating build/config.js...");
    
    // Create build directory if it doesn't exist
    std::fs::create_dir_all("build")
        .context("Failed to create build directory")?;
    
    // Generate the config.js content based on debug mode
    let dev_flag = if debug { "true" } else { "false" };
    let mode = if debug { "DEBUG" } else { "RELEASE" };
    
    println!("{mode} build detected - generating config.js with dev_flag = {dev_flag}");
    
    let config_content = format!(
        "// Build configuration\nwindow.dev_flag = {dev_flag};\n"
    );
    
    // Write the config file
    std::fs::write("build/config.js", config_content)
        .context("Failed to write build/config.js")?;
    
    println!("✅ Successfully generated build/config.js");
    Ok(())
}

fn build_audio_worklet(debug: bool) -> Result<()> {
    println!("🎵 Building audio worklet...");
    
    let mut cmd = Command::new("./build-audio-worklet.sh");
    
    if debug {
        cmd.arg("debug");
    }

    let status = cmd.status()
        .context("Failed to run build-audio-worklet.sh - make sure it exists and is executable")?;

    if !status.success() {
        anyhow::bail!("build-audio-worklet.sh failed");
    }

    Ok(())
}

fn start_log_server() -> Result<Child> {
    // The dev-log-server package has its own .cargo/config.toml that ensures
    // it builds for the correct native target, so we don't need to specify it here
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
