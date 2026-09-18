use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::utils::archive::{fetch_and_extract, find_file_recursive};

pub fn fetch_isolated_php(dar_dir: &Path, php_version: &str) -> Result<(), String> {
    let versions_dir = dar_dir.join("php").join("versions");
    let target_version_dir = versions_dir.join(php_version);
    let current_symlink = dar_dir.join("php").join("current");

    if !target_version_dir.exists() {
        println!(" 📦 Provisioning isolated PHP {} runtime...", php_version);

        let src_dir = dar_dir.join("src");
        let tarball_name = format!("php-{}-fpm-linux-x86_64.tar.gz", php_version);

        let download_url = format!(
            "https://dl.static-php.dev/static-php-cli/common/php-{}-fpm-linux-x86_64.tar.gz",
            php_version
        );

        let extracted_folder = fetch_and_extract(
            &src_dir,
            &format!("PHP {}", php_version),
            &format!("php-pkg-{}", php_version),
            &tarball_name,
            &download_url,
        )?;

        // Organize into standard bin/sbin layout under target version directory
        let bin_dir = target_version_dir.join("bin");
        let sbin_dir = target_version_dir.join("sbin");
        fs::create_dir_all(&bin_dir)
            .map_err(|e| format!("Failed to create bin dir: {}", e))?;
        fs::create_dir_all(&sbin_dir)
            .map_err(|e| format!("Failed to create sbin dir: {}", e))?;

        // Locate php-fpm inside extracted files
        let fpm_source = find_file_recursive(&extracted_folder, "php-fpm")
            .ok_or_else(|| "php-fpm binary missing from downloaded package.".to_string())?;

        let target_fpm = sbin_dir.join("php-fpm");
        fs::copy(&fpm_source, &target_fpm)
            .map_err(|e| format!("Failed to copy php-fpm executable: {}", e))?;

        // Set executable permissions (0755)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = fs::metadata(&target_fpm) {
                let mut perms = meta.permissions();
                perms.set_mode(0o755);
                let _ = fs::set_permissions(&target_fpm, perms);
            }
        }

        // Link CLI binary to the same executable
        let target_cli = bin_dir.join("php");
        let cli_source = find_file_recursive(&extracted_folder, "php");

        if let Some(cli_path) = cli_source {
            let _ = fs::copy(&cli_path, &target_cli);
        } else {
            let _ = symlink(&target_fpm, &target_cli);
        }

        #[cfg(unix)]
        if target_cli.exists() {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = fs::metadata(&target_cli) {
                let mut perms = meta.permissions();
                perms.set_mode(0o755);
                let _ = fs::set_permissions(&target_cli, perms);
            }
        }
    }

    // Set current symlink
    if current_symlink.exists() || fs::symlink_metadata(&current_symlink).is_ok() {
        let _ = fs::remove_file(&current_symlink);
    }

    symlink(&target_version_dir, &current_symlink)
        .map_err(|e| format!("Failed to symlink current PHP version: {}", e))?;

    Ok(())
}

pub fn ensure_php_fpm_running(dar_dir: &Path) -> Result<(), String> {
    let conf_path = create_default_php_fpm_config(dar_dir)?;
    let local_php_fpm = dar_dir
        .join("php")
        .join("current")
        .join("sbin")
        .join("php-fpm");

    let pid_file = dar_dir.join("logs").join("php-fpm.pid");

    // Check if PID file exists and process is active
    let is_running = if pid_file.exists() {
        if let Ok(pid_str) = fs::read_to_string(&pid_file) {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                Command::new("kill")
                    .arg("-0")
                    .arg(pid.to_string())
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false)
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    if !is_running {
        println!(" 🐘 Starting local PHP-FPM service...");

        if !local_php_fpm.exists() {
            return Err(format!(
                "Local PHP-FPM binary not found at {}",
                local_php_fpm.display()
            ));
        }

        let status = Command::new(&local_php_fpm)
            .arg("-y")
            .arg(&conf_path)
            .arg("-D")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| format!("Failed to start isolated php-fpm: {}", e))?;

        if !status.success() {
            return Err("PHP-FPM failed to start.".to_string());
        }
    } else {
        println!(" 🐘 Local PHP-FPM is already running.");
    }

    Ok(())
}

pub fn create_default_php_fpm_config(dar_dir: &Path) -> Result<PathBuf, String> {
    let php_dir = dar_dir.join("php");
    if !php_dir.exists() {
        fs::create_dir_all(&php_dir)
            .map_err(|e| format!("Failed to create php directory: {}", e))?;
    }

    let conf_path = php_dir.join("php-fpm.conf");
    let pid_path = dar_dir.join("logs").join("php-fpm.pid");
    let log_path = dar_dir.join("logs").join("php-fpm.log");

    let conf_content = format!(
        r#"[global]
pid = {}
error_log = {}

[www]
listen = 127.0.0.1:9000
pm = dynamic
pm.max_children = 5
pm.start_servers = 2
pm.min_spare_servers = 1
pm.max_spare_servers = 3
"#,
        pid_path.display(),
        log_path.display()
    );

    fs::write(&conf_path, conf_content)
        .map_err(|e| format!("Failed to write php-fpm.conf: {}", e))?;

    Ok(conf_path)
}

