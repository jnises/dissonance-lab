use anyhow::{Context, Result};
use std::collections::HashSet;
use std::env;
use std::process::Command;

use crate::utils::{find_project_root, get_workspace_crates, run_cargo_command};

/// Define which crates should use which target
const NATIVE_CRATES: &[&str] = &["xtask", "dev-log-server"];
const WASM_CRATES: &[&str] = &["dissonance-lab", "audio-worklet", "shared-types"];
const WASM_TARGET: &str = "wasm32-unknown-unknown";

/// Run comprehensive checks including build, format, clippy, tests, and trunk build
pub fn run_check(skip_fmt: bool) -> Result<()> {
    println!("🔧 Running comprehensive checks...");

    // Ensure we're in the project root
    let project_root = find_project_root()?;
    env::set_current_dir(&project_root).context("Failed to change to project root directory")?;

    // Run check on all crates
    check_all_crates()?;

    // Run formatting check unless skipped
    if !skip_fmt {
        println!("📝 Checking code formatting...");
        let output = Command::new("cargo")
            .args(["fmt", "--all", "--", "--check"])
            .output()
            .context("Failed to run cargo fmt check")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Code formatting check failed: {}", stderr);
        }
    } else {
        println!("📝 Skipping code formatting check (--skip-fmt)");
    }

    // Run clippy on all crates
    clippy_all_crates()?;

    // Run tests
    println!("🧪 Running tests...");
    let output = Command::new("cargo")
        .args(["test", "--quiet", "--workspace", "--all-features"])
        .output()
        .context("Failed to run cargo test")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Tests failed: {}", stderr);
    }

    // Run doc tests
    println!("📚 Running doc tests...");
    let output = Command::new("cargo")
        .args(["test", "--quiet", "--workspace", "--doc"])
        .output()
        .context("Failed to run cargo test --doc")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Doc tests failed: {}", stderr);
    }

    // Run trunk build
    println!("🏗️  Running trunk build...");
    let output = Command::new("trunk")
        .args(["build"])
        .output()
        .context("Failed to run trunk build")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Trunk build failed: {}", stderr);
    }

    println!("✅ All checks passed successfully!");
    Ok(())
}

/// Check all crates with appropriate targets
pub fn check_all_crates() -> Result<()> {
    println!("🔧 Checking all crates with appropriate targets...");

    // Ensure we're in the project root
    let project_root = find_project_root()?;
    env::set_current_dir(&project_root).context("Failed to change to project root directory")?;

    // Get all crates in the workspace
    let crates = get_workspace_crates(&project_root)?;

    // Check native crates
    println!("📦 Checking native crates...");
    for (crate_name, manifest_path) in &crates {
        if NATIVE_CRATES.contains(&crate_name.as_str()) {
            run_cargo_command("check", crate_name, manifest_path, None, "Checking")?;
        }
    }

    // Check WASM crates
    println!("🌐 Checking WASM crates...");
    for (crate_name, manifest_path) in &crates {
        if WASM_CRATES.contains(&crate_name.as_str()) {
            run_cargo_command(
                "check",
                crate_name,
                manifest_path,
                Some(WASM_TARGET),
                "Checking",
            )?;
        }
    }

    let crate_names: Vec<String> = crates.iter().map(|(name, _)| name.clone()).collect();
    verify_crate_coverage(&crate_names)?;

    println!("✅ All crates checked successfully!");
    println!("   📦 Native crates checked: {}", NATIVE_CRATES.len());
    println!("   🌐 WASM crates checked: {}", WASM_CRATES.len());

    Ok(())
}

/// Run clippy on all crates with appropriate targets
pub fn clippy_all_crates() -> Result<()> {
    println!("🔧 Running clippy on all crates with appropriate targets...");

    // Ensure we're in the project root
    let project_root = find_project_root()?;
    env::set_current_dir(&project_root).context("Failed to change to project root directory")?;

    // Get all crates in the workspace
    let crates = get_workspace_crates(&project_root)?;

    // Clippy native crates
    println!("📦 Running clippy on native crates...");
    for (crate_name, manifest_path) in &crates {
        if NATIVE_CRATES.contains(&crate_name.as_str()) {
            run_clippy_on_crate(crate_name, manifest_path, None)?;
        }
    }

    // Clippy WASM crates
    println!("🌐 Running clippy on WASM crates...");
    for (crate_name, manifest_path) in &crates {
        if WASM_CRATES.contains(&crate_name.as_str()) {
            run_clippy_on_crate(crate_name, manifest_path, Some(WASM_TARGET))?;
        }
    }

    let crate_names: Vec<String> = crates.iter().map(|(name, _)| name.clone()).collect();
    verify_crate_coverage(&crate_names)?;

    println!("✅ All crates linted successfully!");
    println!("   📦 Native crates linted: {}", NATIVE_CRATES.len());
    println!("   🌐 WASM crates linted: {}", WASM_CRATES.len());

    Ok(())
}

/// Run clippy on a specific crate with optional target
fn run_clippy_on_crate(
    crate_name: &str,
    manifest_path: &str,
    target: Option<&str>,
) -> Result<()> {
    let target_desc = if target.is_some() { " (WASM)" } else { "" };
    println!("  Running clippy on {crate_name}{target_desc} ...");

    let mut args = vec![
        "clippy",
        "--quiet",
        "--manifest-path",
        manifest_path,
        "--all-targets",
        "--all-features",
    ];

    if let Some(target) = target {
        args.extend_from_slice(&["--target", target]);
    }

    args.extend_from_slice(&["--", "-D", "warnings", "-W", "clippy::all"]);

    let output = Command::new("cargo")
        .args(&args)
        .output()
        .with_context(|| format!("Failed to run clippy on {crate_name}{target_desc}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Clippy failed on {crate_name}{target_desc}: {stderr}");
    }

    Ok(())
}

/// Verify that all crates in the workspace are categorized and handled
fn verify_crate_coverage(crates: &[String]) -> Result<()> {
    let mut all_expected_crates = NATIVE_CRATES
        .iter()
        .chain(WASM_CRATES.iter())
        .collect::<HashSet<_>>();
    let mut missing_crates = Vec::new();
    let mut uncategorized_crates = Vec::new();

    for crate_name in crates {
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
        anyhow::bail!(
            "Expected crates not found in workspace: {}",
            missing_crates.join(", ")
        );
    }

    if !uncategorized_crates.is_empty() {
        println!(
            "⚠️  Warning: Found uncategorized crates (not processed): {}",
            uncategorized_crates.join(", ")
        );
        println!("   Consider adding them to NATIVE_CRATES or WASM_CRATES in check.rs");
    }

    Ok(())
}
