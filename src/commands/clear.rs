use colored::*;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

use crate::utils::paths::get_dar_dir;
use crate::services::{
    php::stop as stop_php,
    nginx::stop as stop_nginx,
    docker::stop_all as stop_docker,
};

pub fn run(
    logs: bool,
    cache: bool,
    runtimes: bool,
    all: bool,
    force: bool,
) -> Result<(), String> {
    let dar_dir = get_dar_dir()?;

    if !dar_dir.exists() {
        println!("{}", "ℹ️  No ~/.dar directory found. Nothing to clean.".yellow());
        return Ok(());
    }

    // 1. Full Wipe (--all)
    if all {
        return wipe_all(&dar_dir, force);
    }

    // 2. Default logic: If no flags passed, default to safe cleanup (logs + cache)
    let clean_logs = logs || (!cache && !runtimes);
    let clean_cache = cache || (!logs && !runtimes);

    let mut cleaned_something = false;

    if clean_logs {
        clear_logs(&dar_dir)?;
        cleaned_something = true;
    }

    if clean_cache {
        clear_cache(&dar_dir)?;
        cleaned_something = true;
    }

    if runtimes {
        if !force && !confirm("⚠️  Wipe installed PHP versions and managed binaries in ~/.dar?")? {
            println!("{}", "Skipped runtimes cleanup.".yellow());
        } else {
            clear_runtimes(&dar_dir)?;
            cleaned_something = true;
        }
    }

    if cleaned_something {
        println!("{}", "✨ Cleanup complete!".green().bold());
    }

    Ok(())
}

pub fn wipe_all(dar_dir: &Path, force: bool) -> Result<(), String> {
    if !force && !confirm("⚠️  This will stop all running daemons/containers and completely wipe ~/.dar. Proceed?")? {
        println!("Operation cancelled.");
        return Ok(());
    }

    println!("🛑 Stopping all active services...");

    // 1. Docker containers
    let _ = stop_docker();

    // 2. Nginx
    let _ = stop_nginx(dar_dir);

    // 3. Global & Isolated PHP-FPM daemons
    let _ = stop_php(dar_dir);

    // 4. Delete ~/.dar
    println!("🧹 Removing ~/.dar directory...");
    if let Err(e) = fs::remove_dir_all(dar_dir) {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            println!("⚠️  Root-owned Docker data detected. Requesting sudo to clean up...");
            let status = Command::new("sudo")
                .args(["rm", "-rf", dar_dir.to_str().unwrap()])
                .status()
                .map_err(|err| format!("Failed to execute sudo rm: {}", err))?;

            if !status.success() {
                return Err("Failed to remove ~/.dar using sudo.".to_string());
            }
        } else {
            return Err(format!("Failed to remove ~/.dar: {}", e));
        }
    }

    println!("✨ Environment and all running services successfully wiped!");
    Ok(())
}

fn clear_logs(dar_dir: &Path) -> Result<(), String> {
    println!("{}", "🧹 Cleaning temporary logs and runtime buffers...".cyan());

    // Clean ~/.dar/logs
    let logs_dir = dar_dir.join("logs");
    if logs_dir.exists() {
        if let Ok(entries) = fs::read_dir(&logs_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let _ = fs::remove_file(path);
                }
            }
        }
    }

    // Clean Nginx temp directories
    let nginx_dir = dar_dir.join("nginx");
    let temp_dirs = [
        "client_body_temp",
        "fastcgi_temp",
        "proxy_temp",
        "scgi_temp",
        "uwsgi_temp",
    ];

    for temp in temp_dirs {
        let path = nginx_dir.join(temp);
        if path.exists() {
            let _ = fs::remove_dir_all(&path);
            let _ = fs::create_dir_all(&path);
        }
    }

    Ok(())
}

fn clear_cache(dar_dir: &Path) -> Result<(), String> {
    println!("{}", "🧹 Removing downloaded build sources and Composer cache...".cyan());

    // Clean ~/.dar/src
    let src_dir = dar_dir.join("src");
    if src_dir.exists() {
        let _ = fs::remove_dir_all(&src_dir);
        let _ = fs::create_dir_all(&src_dir);
    }

    // Clean ~/.dar/composer/cache
    let composer_cache = dar_dir.join("composer").join("cache");
    if composer_cache.exists() {
        let _ = fs::remove_dir_all(&composer_cache);
    }

    Ok(())
}

fn clear_runtimes(dar_dir: &Path) -> Result<(), String> {
    println!("🧹 Cleaning unused PHP runtimes, secondary versions, and stale FPM sockets...");

    let php_dir = dar_dir.join("php");
    if !php_dir.exists() {
        return Ok(());
    }

    // 1. Determine default version name from `current` symlink
    let current_symlink = php_dir.join("current");
    let active_version = if let Ok(target) = fs::read_link(&current_symlink) {
        target
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
    } else {
        None
    };

    // 2. Remove unused secondary PHP versions (keep default version)
    let versions_dir = php_dir.join("versions");
    if versions_dir.exists() {
        if let Ok(entries) = fs::read_dir(&versions_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(version_name) = path.file_name().and_then(|n| n.to_str()) {
                    if Some(version_name) != active_version.as_deref() {
                        let _ = fs::remove_dir_all(&path);
                    }
                }
            }
        }
    }

    // 3. Clean up non-default FPM configs and active socket files
    if let Ok(entries) = fs::read_dir(&php_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Protect base global files, versions directory, and current symlink
            if file_name == "php.ini"
                || file_name == "php-fpm.conf"
                || file_name == "versions"
                || file_name == "current"
            {
                continue;
            }

            // Remove leftover version FPM configs and socket files
            if file_name.ends_with(".sock") || (file_name.ends_with("-fpm.conf") && file_name != "php-fpm.conf") {
                let _ = fs::remove_file(&path);
            }
        }
    }

    // Notice: ~/.dar/bin is deliberately left untouched so wrappers remain active!

    Ok(())
}

fn confirm(prompt: &str) -> Result<bool, String> {
    print!("{} [y/N]: ", prompt.bold());
    io::stdout().flush().map_err(|e| e.to_string())?;

    let mut input = String::new();
    io::stdin().read_line(&mut input).map_err(|e| e.to_string())?;

    Ok(matches!(input.trim().to_lowercase().as_str(), "y" | "yes"))
}
