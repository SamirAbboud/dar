use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::utils::archive::{fetch_and_extract, find_file_recursive};

enum PhpProvider {
    StaticPhpCli,
    ShivamMathur,
}

impl PhpProvider {
    fn name(&self) -> &'static str {
        match self {
            Self::StaticPhpCli => "static-php-cli",
            Self::ShivamMathur => "shivammathur/php-builder",
        }
    }

    fn fpm_url(&self, version: &str) -> String {
        match self {
            Self::StaticPhpCli => format!(
                "https://dl.static-php.dev/static-php-cli/common/php-{}-fpm-linux-x86_64.tar.gz",
                version
            ),
            Self::ShivamMathur => format!(
                "https://github.com/shivammathur/php-builder/releases/download/v{}/php-{}-linux-x86_64.tar.gz",
                version, version
            ),
        }
    }

    fn cli_url(&self, version: &str) -> String {
        match self {
            Self::StaticPhpCli => format!(
                "https://dl.static-php.dev/static-php-cli/common/php-{}-cli-linux-x86_64.tar.gz",
                version
            ),
            Self::ShivamMathur => format!(
                "https://github.com/shivammathur/php-builder/releases/download/v{}/php-{}-linux-x86_64.tar.gz",
                version, version
            ),
        }
    }
}

pub fn fetch_isolated_php(dar_dir: &Path, php_version: &str) -> Result<(), String> {
    let versions_dir = dar_dir.join("php").join("versions");
    let target_version_dir = versions_dir.join(php_version);
    let current_symlink = dar_dir.join("php").join("current");

    if !target_version_dir.exists() {
        println!(" 📦 Provisioning isolated PHP {} runtime...", php_version);

        let src_dir = dar_dir.join("src");
        let providers = [PhpProvider::StaticPhpCli, PhpProvider::ShivamMathur];
        let mut fpm_folder = None;
        let mut cli_folder = None;

        for provider in &providers {
            println!(" 🔍 Attempting download from {}...", provider.name());

            let fpm_res = fetch_and_extract(
                &src_dir,
                &format!("PHP {} FPM", php_version),
                &format!("php-fpm-pkg-{}", php_version),
                &format!("php-{}-fpm.tar.gz", php_version),
                &provider.fpm_url(php_version),
            );

            let cli_res = fetch_and_extract(
                &src_dir,
                &format!("PHP {} CLI", php_version),
                &format!("php-cli-pkg-{}", php_version),
                &format!("php-{}-cli.tar.gz", php_version),
                &provider.cli_url(php_version),
            );

            if let (Ok(fpm), Ok(cli)) = (fpm_res, cli_res) {
                fpm_folder = Some(fpm);
                cli_folder = Some(cli);
                break;
            } else {
                println!(" ⚠️ Provider {} failed. Trying next...", provider.name());
            }
        }

        let fpm_dir = fpm_folder.ok_or_else(|| {
            format!("Could not download PHP {} FPM binary.", php_version)
        })?;
        let cli_dir = cli_folder.ok_or_else(|| {
            format!("Could not download PHP {} CLI binary.", php_version)
        })?;

        setup_binary_package(&fpm_dir, &cli_dir, &target_version_dir)?;
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

    let php_ini_path = ensure_php_ini(dar_dir)?;

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
php_admin_value[php_ini] = {}
"#,
        pid_path.display(),
        log_path.display(),
        php_ini_path.display()
    );

    fs::write(&conf_path, conf_content)
        .map_err(|e| format!("Failed to write php-fpm.conf: {}", e))?;

    Ok(conf_path)
}

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
