use colored::*;
use std::fs::{self, File};
use std::path::Path;
use std::process::Command;

use crate::services::templates::load_templates;

/// Creates the essential `~/.dar` directory hierarchy (bin, nginx, sites, logs, certs, src, php) 
/// and initializes `parked.txt` if it does not already exist.
pub fn setup_directories(dar_dir: &Path) -> Result<(), String> {
    let dirs = [
        dar_dir.to_path_buf(),
        dar_dir.join("bin"),
        dar_dir.join("nginx"),
        dar_dir.join("sites"),
        dar_dir.join("logs"),
        dar_dir.join("certs"),
        dar_dir.join("src"),
        dar_dir.join("composer"),
        dar_dir.join("php"),
        dar_dir.join("php").join("versions"),
        dar_dir.join("services"),
    ];

    for dir in &dirs {
        if !dir.exists() {
            fs::create_dir_all(dir)
                .map_err(|e| format!("Failed to create directory {}: {}", dir.display(), e))?;
            println!(" Created directory: {}", dir.display().to_string().dimmed());
        }
    }

    let parked_store = dar_dir.join("parked.txt");
    if !parked_store.exists() {
        File::create(&parked_store)
            .map_err(|e| format!("Failed to create parked store: {}", e))?;
    }

    let sites_store = dar_dir.join("sites.json");
    if !sites_store.exists() {
        fs::write(&sites_store, "{}")
            .map_err(|e| format!("Failed to create sites store: {}", e))?;
    }

    let isolations_store = dar_dir.join("isolations.json");
    if !isolations_store.exists() {
        fs::write(&isolations_store, "{}")
            .map_err(|e| format!("Failed to create isolations store: {}", e))?;
    }

    // Ensure templates.toml is populated with default service configurations
    load_templates(dar_dir)?;

    Ok(())
}

/// Detects an available C compiler on the host system, checking the `CC` environment variable 
/// first, then probing for `gcc`, `clang`, or `cc`, falling back to `clang`.
pub fn get_available_compiler() -> String {
    if let Ok(cc) = std::env::var("CC") {
        return cc;
    }

    let candidates = ["gcc", "clang", "cc"];
    for compiler in candidates {
        if Command::new(compiler)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok()
        {
            return compiler.to_string();
        }
    }

    "clang".to_string()
}

/// Checks if a process is running on the host system using `pgrep`.
pub fn is_process_running(process_name: &str) -> bool {
    Command::new("pgrep")
        .arg("-x")
        .arg(process_name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}
