use colored::*;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use crate::services::composer::ensure_composer;
use crate::services::php::config::{
    ensure_isolated_fpm_config, get_isolated_fpm_socket, setup_binary_package,
};
use crate::services::php::lifecycle::{is_fpm_running, start, stop};
use crate::services::php::version::resolve_php_version_alias;
use crate::utils::archive::fetch_and_extract;

/// Ensures a specific PHP version is downloaded and provisioned in `versions_dir`.
pub fn ensure_php_version(dar_dir: &Path, raw_version: &str) -> Result<String, String> {
    let php_version = resolve_php_version_alias(raw_version);
    let versions_dir = dar_dir.join("php").join("versions");
    let target_version_dir = versions_dir.join(&php_version);

    if !target_version_dir.exists() {
        println!("📦 Provisioning PHP {} runtime...", php_version);

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

    Ok(php_version)
}

/// Sets a provisioned PHP version as the global active version.
pub fn set_global_php_version(dar_dir: &Path, php_version: &str) -> Result<(), String> {
    let target_version_dir = dar_dir.join("php").join("versions").join(php_version);
    let current_symlink = dar_dir.join("php").join("current");

    if !target_version_dir.exists() {
        return Err(format!("PHP version {} is not provisioned.", php_version));
    }

    stop(dar_dir)?;

    if current_symlink.exists() || fs::symlink_metadata(&current_symlink).is_ok() {
        let _ = fs::remove_file(&current_symlink);
    }

    symlink(&target_version_dir, &current_symlink)
        .map_err(|e| format!("Failed to symlink current PHP version: {}", e))?;

    ensure_composer(dar_dir)?;
    start(dar_dir)?;

    Ok(())
}

/// Ensures the isolated PHP runtime is provisioned and its dedicated FPM process is active.
pub fn ensure_isolated_fpm_running(dar_dir: &Path, version: &str) -> Result<PathBuf, String> {
    let resolved_version = resolve_php_version_alias(version);
    let target_version_dir = dar_dir.join("php").join("versions").join(&resolved_version);

    if !target_version_dir.exists() {
        println!("📦 Provisioning isolated PHP {} runtime...", resolved_version);
        ensure_php_version(dar_dir, &resolved_version)?;
    }

    let socket_path = get_isolated_fpm_socket(dar_dir, version);

    if is_fpm_running(dar_dir, Some(version)) && socket_path.exists() {
        return Ok(socket_path);
    }

    let conf_path = ensure_isolated_fpm_config(dar_dir, version)?;
    let fpm_bin = target_version_dir.join("sbin").join("php-fpm");

    if !fpm_bin.exists() {
        return Err(format!("PHP-FPM binary missing at {}", fpm_bin.display()));
    }

    println!("🚀 Starting isolated PHP {} FPM daemon...", version);

    let status = Command::new(&fpm_bin)
        .arg("-y")
        .arg(&conf_path)
        .arg("-D")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("Failed to start php{}-fpm: {}", version, e))?;

    if !status.success() {
        return Err(format!("PHP-FPM {} daemon failed to start.", version));
    }

    for _ in 0..10 {
        if socket_path.exists() {
            println!("✨ PHP {} FPM listening at {}", version.bold(), socket_path.display());
            return Ok(socket_path);
        }
        thread::sleep(Duration::from_millis(500));
    }

    Err(format!("Timed out waiting for socket {}", socket_path.display()))
}
