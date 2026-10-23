use colored::*;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::services::php::create_default_php_fpm_config;
use crate::services::php::ensure_isolated_fpm_running;
use crate::store::IsolationsStore;

/// Checks if a process is running by sending signal 0 to the PID stored in a file.
pub fn is_pid_file_running(pid_file: &Path) -> bool {
    if !pid_file.exists() {
        return false;
    }

    if let Ok(pid_str) = fs::read_to_string(pid_file) {
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            return Command::new("kill")
                .arg("-0")
                .arg(pid.to_string())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
        }
    }
    false
}

/// Returns the PID file path for global or version-isolated PHP-FPM.
pub fn get_fpm_pid_file(dar_dir: &Path, version: Option<&str>) -> PathBuf {
    let filename = match version {
        Some(v) => format!("php{}-fpm.pid", v),
        None => "php-fpm.pid".to_string(),
    };
    dar_dir.join("logs").join(filename)
}

/// Checks if PHP-FPM (global or isolated) process is running via sending signal 0.
pub fn is_fpm_running(dar_dir: &Path, version: Option<&str>) -> bool {
    let pid_file = get_fpm_pid_file(dar_dir, version);
    is_pid_file_running(&pid_file)
}

/// Starts ONLY the global/default PHP-FPM daemon.
pub fn start_global_fpm(dar_dir: &Path) -> Result<(), String> {
    if is_fpm_running(dar_dir, None) {
        println!("{}", "🐘 Global PHP-FPM is already running.".yellow());
        return Ok(());
    }

    let local_php_fpm = dar_dir
        .join("php")
        .join("current")
        .join("sbin")
        .join("php-fpm");

    if !local_php_fpm.exists() {
        return Err(format!(
            "Local PHP-FPM binary not found at {}. Please run 'dar install' first.",
            local_php_fpm.display()
        ));
    }

    let conf_path = create_default_php_fpm_config(dar_dir)?;

    println!("{}", "🚀 Starting global PHP-FPM service...".cyan());

    let status = Command::new(&local_php_fpm)
        .arg("-y")
        .arg(&conf_path)
        .arg("-D")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("Failed to start global php-fpm: {}", e))?;

    if !status.success() {
        return Err("Global PHP-FPM failed to start.".to_string());
    }

    println!("{}", "✨ Global PHP-FPM started successfully!".green());
    Ok(())
}

/// Starts ONLY a specific isolated PHP version daemon (e.g., "8.3").
pub fn start_version(dar_dir: &Path, version: &str) -> Result<(), String> {
    ensure_isolated_fpm_running(dar_dir, version)?;
    println!("{}", format!("✨ Isolated PHP {} FPM started successfully!", version).green());
    Ok(())
}

/// Starts the global PHP-FPM process AND all active isolated PHP-FPM daemons.
pub fn start(dar_dir: &Path) -> Result<(), String> {
    start_global_fpm(dar_dir)?;

    if let Ok(store) = IsolationsStore::load(dar_dir) {
        let unique_versions: HashSet<_> = store.mappings.values().cloned().collect();
        for ver in unique_versions {
            if let Err(e) = ensure_isolated_fpm_running(dar_dir, &ver) {
                println!(
                    "{}",
                    format!("⚠️ Failed to start isolated PHP {} FPM: {}", ver, e).yellow()
                );
            }
        }
    }

    Ok(())
}

/// Helper to kill a process given its PID file path and remove the PID file.
pub fn stop_pid_file(pid_file: &Path, label: &str) -> bool {
    if pid_file.exists() {
        if let Ok(pid_str) = fs::read_to_string(pid_file) {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                println!("{}", format!("🛑 Stopping {}...", label).cyan());
                let _ = Command::new("kill")
                    .arg(pid.to_string())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
        }
        let _ = fs::remove_file(pid_file);
        println!("{}", format!("✨ {} stopped successfully.", label).green());
        true
    } else {
        false
    }
}

/// Stops ONLY the global/default PHP-FPM daemon.
pub fn stop_global_fpm(dar_dir: &Path) -> Result<(), String> {
    let pid_file = dar_dir.join("logs").join("php-fpm.pid");
    if !stop_pid_file(&pid_file, "global PHP-FPM service") {
        println!("{}", "🐘 Global PHP-FPM is not running.".yellow());
    }
    Ok(())
}

/// Stops ONLY a specific isolated PHP version daemon (e.g., "8.3").
pub fn stop_version(dar_dir: &Path, version: &str) -> Result<(), String> {
    let pid_file = dar_dir.join("logs").join(format!("php{}-fpm.pid", version));
    let label = format!("isolated PHP {} FPM service", version);
    if !stop_pid_file(&pid_file, &label) {
        println!("{}", format!("🐘 Isolated PHP {} FPM is not running.", version).yellow());
    }
    Ok(())
}

/// Stops the global PHP-FPM daemon AND all active isolated PHP-FPM daemons.
pub fn stop(dar_dir: &Path) -> Result<(), String> {
    stop_global_fpm(dar_dir)?;

    if let Ok(store) = IsolationsStore::load(dar_dir) {
        let unique_versions: HashSet<_> = store.mappings.values().cloned().collect();
        for ver in unique_versions {
            let _ = stop_version(dar_dir, &ver);
        }
    }

    Ok(())
}

/// Restarts ONLY the global PHP-FPM daemon.
pub fn restart_global_fpm(dar_dir: &Path) -> Result<(), String> {
    stop_global_fpm(dar_dir)?;
    start_global_fpm(dar_dir)
}

/// Restarts ONLY a specific isolated PHP version daemon (e.g., "8.3").
pub fn restart_version(dar_dir: &Path, version: &str) -> Result<(), String> {
    stop_version(dar_dir, version)?;
    start_version(dar_dir, version)
}

/// Restarts global PHP-FPM AND all isolated PHP-FPM daemons.
pub fn restart(dar_dir: &Path) -> Result<(), String> {
    stop(dar_dir)?;
    start(dar_dir)
}

/// Executes raw CLI arguments with the currently active isolated PHP binary.
pub fn execute_isolated_php(dar_dir: &Path, args: &[String]) -> Result<(), String> {
    let php_bin = dar_dir.join("php").join("current").join("bin").join("php");

    if !php_bin.exists() {
        return Err(format!(
            "No active PHP binary found at `{}`. Run `dar php -u <version>` first.",
            php_bin.display()
        ));
    }

    let status = Command::new(&php_bin)
        .args(args)
        .status()
        .map_err(|e| format!("Failed to execute php: {}", e))?;

    if !status.success() {
        if let Some(code) = status.code() {
            std::process::exit(code);
        }
    }

    Ok(())
}
