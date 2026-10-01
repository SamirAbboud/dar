use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::collections::HashMap;
use colored::*;

use crate::services::composer::ensure_composer;
use crate::utils::archive::{fetch_and_extract, find_file_recursive};

/// Normalizes inputs like "php@8.2", "php-8.2", "v8.2" down to "8.2".
pub fn normalize_php_version(raw_version: &str) -> String {
    let trimmed = raw_version.trim().to_lowercase();
    let cleaned = trimmed
        .strip_prefix("php@")
        .or_else(|| trimmed.strip_prefix("php-"))
        .or_else(|| trimmed.strip_prefix("php"))
        .or_else(|| trimmed.strip_prefix('v'))
        .unwrap_or(&trimmed);

    cleaned.to_string()
}

/// Dynamically resolves shorthand ("8.2") to the latest available static-php-cli build on dl.static-php.dev.
pub fn resolve_php_version_alias(raw_version: &str) -> String {
    let clean = normalize_php_version(raw_version);

    if clean.split('.').count() >= 3 {
        return clean;
    }

    // Query dl.static-php.dev CDN index directly
    if let Ok(res) = ureq::get("https://dl.static-php.dev/static-php-cli/common/")
        .header("User-Agent", "dar-cli")
        .call()
    {
        if let Ok(html) = res.into_body().read_to_string() {
            let prefix = format!("php-{}.", clean);
            // Store tuples of (patch_number as u32, full_version_string)
            let mut matches: Vec<(u32, String)> = Vec::new();

            for line in html.lines() {
                if let Some(start) = line.find(&prefix) {
                    let rest = &line[start + 4..]; // Skip "php-"
                    if let Some(end) = rest.find("-cli-linux-x86_64.tar.gz") {
                        let full_version = &rest[..end]; // e.g. "8.3.11"

                        // Extract the patch version (e.g. "11" from "8.3.11")
                        if let Some(patch_str) = full_version.split('.').nth(2) {
                            if let Ok(patch_num) = patch_str.parse::<u32>() {
                                matches.push((patch_num, full_version.to_string()));
                            }
                        }
                    }
                }
            }

            // Max by the integer patch number!
            if let Some((_, latest_version)) = matches.iter().max_by_key(|(patch, _)| patch) {
                println!("🌐 Resolved PHP {} -> {} via static-php CDN", clean, latest_version);
                return latest_version.clone();
            }
        }
    }

    // Fallback static map if offline
    let fallbacks: HashMap<&str, &str> = HashMap::from([
        ("8.0", "8.0.30"),
        ("8.1", "8.1.30"),
        ("8.2", "8.2.32"),
        ("8.3", "8.3.11"),
        ("8.4", "8.4.8"),
        ("8.5", "8.5.8"),
    ]);

    fallbacks.get(clean.as_str()).unwrap_or(&clean.as_str()).to_string()
}

/// Scans `~/.dar/php/versions` and returns (installed_versions, active_version_option).
pub fn list_installed_php_versions(dar_dir: &Path) -> (Vec<String>, Option<String>) {
    let versions_dir = dar_dir.join("php").join("versions");
    let current_symlink = dar_dir.join("php").join("current");

    let mut installed = Vec::new();
    if let Ok(entries) = fs::read_dir(&versions_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    installed.push(name.to_string());
                }
            }
        }
    }
    installed.sort();

    // Canonicalize or inspect target filename cleanly
    let active = if let Ok(target) = fs::read_link(&current_symlink) {
        target
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
    } else {
        None
    };

    (installed, active)
}

/// Downloads and sets up an isolated PHP runtime (FPM and CLI) for the specified version,
/// updating the `~/.dar/php/current` symlink to point to the active version.
pub fn fetch_isolated_php(dar_dir: &Path, raw_version: &str) -> Result<String, String> {
    let php_version = resolve_php_version_alias(raw_version);
    let versions_dir = dar_dir.join("php").join("versions");
    let target_version_dir = versions_dir.join(&php_version);
    let current_symlink = dar_dir.join("php").join("current");

    if !target_version_dir.exists() {
        println!("📦 Provisioning isolated PHP {} runtime...", php_version);

        let src_dir = dar_dir.join("src");
        let base_url = "https://dl.static-php.dev/static-php-cli/common";

        let fpm_url = format!("{}/php-{}-fpm-linux-x86_64.tar.gz", base_url, php_version);
        let cli_url = format!("{}/php-{}-cli-linux-x86_64.tar.gz", base_url, php_version);

        let fpm_dir = fetch_and_extract(
            &src_dir,
            &format!("PHP {} FPM", php_version),
            &format!("php-fpm-pkg-{}", php_version),
            &format!("php-{}-fpm.tar.gz", php_version),
            &fpm_url,
        )
        .map_err(|e| format!("FPM download failed: {}", e))?;

        let cli_dir = fetch_and_extract(
            &src_dir,
            &format!("PHP {} CLI", php_version),
            &format!("php-cli-pkg-{}", php_version),
            &format!("php-{}-cli.tar.gz", php_version),
            &cli_url,
        )
        .map_err(|e| format!("CLI download failed: {}", e))?;

        setup_binary_package(&fpm_dir, &cli_dir, &target_version_dir)?;
    }

    // Stop current FPM service before updating symlink
    stop(dar_dir)?;

    // Set current symlink
    if current_symlink.exists() || fs::symlink_metadata(&current_symlink).is_ok() {
        let _ = fs::remove_file(&current_symlink);
    }

    symlink(&target_version_dir, &current_symlink)
        .map_err(|e| format!("Failed to symlink current PHP version: {}", e))?;

    // Ensure Composer wrapper points to the new current PHP binary
    ensure_composer(dar_dir)?;

    // Start PHP-FPM with the new version
    start(dar_dir)?;

    Ok(php_version)
}

/// Generates the global `php-fpm.conf` file in `~/.dar/php/`, configuring socket listening,
/// process management settings, and logging locations.
pub fn create_default_php_fpm_config(dar_dir: &Path) -> Result<PathBuf, String> {
    let php_dir = dar_dir.join("php");
    if !php_dir.exists() {
        fs::create_dir_all(&php_dir)
            .map_err(|e| format!("Failed to create php directory: {}", e))?;
    }

    let php_ini_path = ensure_php_ini(dar_dir)?;

    let conf_path = php_dir.join("php-fpm.conf");
    let pid_path = dar_dir.join("logs").join("php-fpm.pid");
    let log_path = dar_dir.join("logs").join("php-fpm.log");
    let socket_path = php_dir.join("php-fpm.sock");

    let conf_content = format!(
        r#"[global]
pid = {}
error_log = {}

[www]
listen = {}
pm = dynamic
pm.max_children = 5
pm.start_servers = 2
pm.min_spare_servers = 1
pm.max_spare_servers = 3
php_admin_value[php_ini] = {}
"#,
        pid_path.display(),
        log_path.display(),
        socket_path.display(),
        php_ini_path.display()
    );

    fs::write(&conf_path, conf_content)
        .map_err(|e| format!("Failed to write php-fpm.conf: {}", e))?;

    Ok(conf_path)
}

/// Checks if PHP-FPM process is running via sending signal 0 to its PID.
pub fn is_fpm_running(dar_dir: &Path) -> bool {
    let pid_file = dar_dir.join("logs").join("php-fpm.pid");
    if !pid_file.exists() {
        return false;
    }

    if let Ok(pid_str) = fs::read_to_string(&pid_file) {
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

/// Starts PHP-FPM if it's not already running.
pub fn start(dar_dir: &Path) -> Result<(), String> {
    if is_fpm_running(dar_dir) {
        println!("{}", "🐘 PHP-FPM is already running.".yellow());
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

    println!("{}", "🚀 Starting PHP-FPM service...".cyan());

    let status = Command::new(&local_php_fpm)
        .arg("-y")
        .arg(&conf_path)
        .arg("-D")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("Failed to start php-fpm: {}", e))?;

    if !status.success() {
        return Err("PHP-FPM failed to start.".to_string());
    }

    println!("{}", "✨ PHP-FPM started successfully!".green());
    Ok(())
}

/// Stops active PHP-FPM service using its PID file.
pub fn stop(dar_dir: &Path) -> Result<(), String> {
    let pid_file = dar_dir.join("logs").join("php-fpm.pid");

    if pid_file.exists() {
        if let Ok(pid_str) = fs::read_to_string(&pid_file) {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                println!("{}", "🛑 Stopping PHP-FPM service...".cyan());
                let _ = Command::new("kill")
                    .arg(pid.to_string())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
        }
        let _ = fs::remove_file(&pid_file);
        println!("{}", "✨ PHP-FPM stopped successfully.".green());
    } else {
        println!("{}", "🐘 PHP-FPM is not running.".yellow());
    }

    Ok(())
}

/// Restarts PHP-FPM service by stopping any existing instance and starting a fresh process.
pub fn restart(dar_dir: &Path) -> Result<(), String> {
    println!("{}", "🔄 Restarting PHP-FPM service...".cyan());
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

/// Extracts and copies downloaded PHP FPM and CLI binaries into standard `sbin/` and `bin/` 
/// directories inside the versioned installation folder, ensuring `0755` execution per
fn setup_binary_package(
    fpm_folder: &Path,
    cli_folder: &Path,
    target_version_dir: &Path,
) -> Result<(), String> {
    let bin_dir = target_version_dir.join("bin");
    let sbin_dir = target_version_dir.join("sbin");

    fs::create_dir_all(&bin_dir).map_err(|e| format!("Failed to create bin dir: {}", e))?;
    fs::create_dir_all(&sbin_dir).map_err(|e| format!("Failed to create sbin dir: {}", e))?;

    // 1. Install php-fpm -> sbin/php-fpm
    let fpm_src = find_file_recursive(fpm_folder, "php-fpm")
        .ok_or_else(|| "php-fpm binary missing from downloaded FPM package.".to_string())?;
    let target_fpm = sbin_dir.join("php-fpm");
    fs::copy(&fpm_src, &target_fpm)
        .map_err(|e| format!("Failed to copy php-fpm executable: {}", e))?;

    // 2. Install php CLI -> bin/php
    let cli_src = find_file_recursive(cli_folder, "php")
        .ok_or_else(|| "php binary missing from downloaded CLI package.".to_string())?;
    let target_cli = bin_dir.join("php");
    fs::copy(&cli_src, &target_cli)
        .map_err(|e| format!("Failed to copy php CLI executable: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&target_fpm, fs::Permissions::from_mode(0o755));
        let _ = fs::set_permissions(&target_cli, fs::Permissions::from_mode(0o755));
    }

    Ok(())
}

/// Ensures a default `php.ini` file exists in `~/.dar/php/` with optimized local development settings.
fn ensure_php_ini(dar_dir: &Path) -> Result<PathBuf, String> {
    let php_dir = dar_dir.join("php");
    let ini_path = php_dir.join("php.ini");

    if !ini_path.exists() {
        let default_ini = r#"[PHP]
memory_limit = 512M
upload_max_filesize = 64M
post_max_size = 64M
max_execution_time = 300
date.timezone = UTC
display_errors = On
error_reporting = E_ALL & ~E_DEPRECATED & ~E_STRICT

[Date]
date.timezone = UTC
"#;

        fs::write(&ini_path, default_ini)
            .map_err(|e| format!("Failed to create php.ini: {}", e))?;
    }

    Ok(ini_path)
}
