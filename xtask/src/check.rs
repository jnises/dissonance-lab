use anyhow::{Context, Result};
use std::collections::HashSet;
use std::env;

use crate::utils::{find_project_root, get_workspace_crates, run_cargo_command};

/// Define which crates should use which target
const NATIVE_CRATES: &[&str] = &["xtask", "dev-log-server"];
const WASM_CRATES: &[&str] = &["dissonance-lab", "audio-worklet", "shared-types"];
const WASM_TARGET: &str = "wasm32-unknown-unknown";

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
    for crate_name in &crates {
        if NATIVE_CRATES.contains(&crate_name.as_str()) {
            run_cargo_command("check", crate_name, None, "Checking")?;
        }
    }

    // Check WASM crates
    println!("🌐 Checking WASM crates...");
    for crate_name in &crates {
        if WASM_CRATES.contains(&crate_name.as_str()) {
            run_cargo_command("check", crate_name, Some(WASM_TARGET), "Checking")?;
        }
    }

    verify_crate_coverage(&crates)?;

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
    for crate_name in &crates {
        if NATIVE_CRATES.contains(&crate_name.as_str()) {
            run_cargo_command("clippy", crate_name, None, "Running clippy on")?;
        }
    }

    // Clippy WASM crates
    println!("🌐 Running clippy on WASM crates...");
    for crate_name in &crates {
        if WASM_CRATES.contains(&crate_name.as_str()) {
            run_cargo_command("clippy", crate_name, Some(WASM_TARGET), "Running clippy on")?;
        }
    }

    verify_crate_coverage(&crates)?;

    println!("✅ All crates linted successfully!");
    println!("   📦 Native crates linted: {}", NATIVE_CRATES.len());
    println!("   🌐 WASM crates linted: {}", WASM_CRATES.len());

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
