use std::fs;
use std::path::{Path, PathBuf};

use crate::utils::archive::find_file_recursive;

/// Generates the global `php-fpm.conf` file in `~/.dar/php/`.
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

/// Ensures a default `php.ini` file exists in `~/.dar/php/` with optimized local development settings.
pub fn ensure_php_ini(dar_dir: &Path) -> Result<PathBuf, String> {
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

/// Returns the path to the version-specific FPM socket (e.g., `~/.dar/php/php8.3-fpm.sock`).
pub fn get_isolated_fpm_socket(dar_dir: &Path, version: &str) -> PathBuf {
    dar_dir.join("php").join(format!("php{}-fpm.sock", version))
}

/// Generates a version-specific `php-fpm.conf` file for isolated project execution.
pub fn ensure_isolated_fpm_config(dar_dir: &Path, version: &str) -> Result<PathBuf, String> {
    let php_dir = dar_dir.join("php");
    let php_ini_path = ensure_php_ini(dar_dir)?;

    let conf_path = php_dir.join(format!("php{}-fpm.conf", version));
    let pid_path = dar_dir.join("logs").join(format!("php{}-fpm.pid", version));
    let log_path = dar_dir.join("logs").join(format!("php{}-fpm.log", version));
    let socket_path = get_isolated_fpm_socket(dar_dir, version);

    let conf_content = format!(
        r#"[global]
pid = {}
error_log = {}

[www]
listen = {}
listen.mode = 0666
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
        .map_err(|e| format!("Failed to write isolated php{}-fpm.conf: {}", version, e))?;

    Ok(conf_path)
}

/// Extracts and copies downloaded PHP FPM and CLI binaries into standard `sbin/` and `bin/`
/// directories inside the versioned installation folder.
pub fn setup_binary_package(
    fpm_folder: &Path,
    cli_folder: &Path,
    target_version_dir: &Path,
) -> Result<(), String> {
    let bin_dir = target_version_dir.join("bin");
    let sbin_dir = target_version_dir.join("sbin");

    fs::create_dir_all(&bin_dir).map_err(|e| format!("Failed to create bin dir: {}", e))?;
    fs::create_dir_all(&sbin_dir).map_err(|e| format!("Failed to create sbin dir: {}", e))?;

    let fpm_src = find_file_recursive(fpm_folder, "php-fpm")
        .ok_or_else(|| "php-fpm binary missing from downloaded FPM package.".to_string())?;
    let target_fpm = sbin_dir.join("php-fpm");
    fs::copy(&fpm_src, &target_fpm)
        .map_err(|e| format!("Failed to copy php-fpm executable: {}", e))?;

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
