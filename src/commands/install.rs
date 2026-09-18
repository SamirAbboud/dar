use colored::*;
use flate2::read::GzDecoder;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tar::Archive;
use std::os::unix::fs::symlink;

use crate::config::load_or_create_config;

pub fn run() -> Result<(), String> {
    println!("{}", "🚀 Initializing Dar environment setup...".cyan().bold());

    let home_dir = dirs::home_dir().ok_or_else(|| "Could not locate home directory".to_string())?;
    let dar_dir = home_dir.join(".dar");

    setup_directories(&dar_dir)?;

    let cfg = load_or_create_config(&dar_dir)?;

    let nginx_install_dir = dar_dir.join("nginx");
    let nginx_bin = nginx_install_dir.join("sbin").join("nginx");

    if !nginx_bin.exists() {
        println!("\n{}", "📦 Building isolated Nginx binary...".yellow().bold());
        let src_dir = dar_dir.join("src");

        let pcre_folder = fetch_and_extract(
            &src_dir,
            "PCRE2",
            &format!("pcre2-{}", cfg.pcre2),
            &format!("pcre2-{}.tar.gz", cfg.pcre2),
            &format!(
                "https://github.com/PCRE2Project/pcre2/releases/download/pcre2-{}/pcre2-{}.tar.gz",
                cfg.pcre2, cfg.pcre2
            ),
        )?;

        let openssl_folder = fetch_and_extract(
            &src_dir,
            "OpenSSL",
            &format!("openssl-{}", cfg.openssl),
            &format!("openssl-{}.tar.gz", cfg.openssl),
            &format!(
                "https://github.com/openssl/openssl/releases/download/openssl-{}/openssl-{}.tar.gz",
                cfg.openssl, cfg.openssl
            ),
        )?;

        let zlib_folder = fetch_and_extract(
            &src_dir,
            "zlib",
            &format!("zlib-{}", cfg.zlib),
            &format!("zlib-{}.tar.gz", cfg.zlib),
            &format!(
                "https://github.com/madler/zlib/releases/download/v{}/zlib-{}.tar.gz",
                cfg.zlib, cfg.zlib
            ),
        )?;

        let nginx_src_folder = fetch_and_extract(
            &src_dir,
            "Nginx",
            &format!("nginx-{}", cfg.nginx),
            &format!("nginx-{}.tar.gz", cfg.nginx),
            &format!("https://nginx.org/download/nginx-{}.tar.gz", cfg.nginx),
        )?;

        build_and_install_nginx(
            &nginx_src_folder,
            &pcre_folder, 
            &openssl_folder,
            &zlib_folder,
            &nginx_install_dir, 
            &dar_dir
        )?;
    }

    fetch_isolated_php(&dar_dir, &cfg.default_php)?;
    create_default_nginx_config(&dar_dir)?;
    ensure_php_fpm_running(&dar_dir)?;
    restart_nginx(&nginx_bin, &dar_dir)?;

    println!(
        "\n{}",
        "✨ Dar environment successfully installed in ~/.dar!".green().bold()
    );

    Ok(())
}

fn setup_directories(dar_dir: &Path) -> Result<(), String> {
    let dirs = [
        dar_dir.to_path_buf(),
        dar_dir.join("bin"),
        dar_dir.join("nginx"),
        dar_dir.join("sites"),
        dar_dir.join("logs"),
        dar_dir.join("src"),
        dar_dir.join("php"),
        dar_dir.join("php").join("versions"),
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

    Ok(())
}

fn fetch_and_extract(
    src_dir: &Path,
    name: &str,
    folder_name: &str,
    tarball_name: &str,
    download_url: &str,
) -> Result<PathBuf, String> {
    let target_folder = src_dir.join(folder_name);
    if target_folder.exists() {
        return Ok(target_folder);
    }

    let tarball_path = src_dir.join(tarball_name);

    if !tarball_path.exists() {
        println!(" Downloading {} static source...", name);
        let response = ureq::get(download_url)
            .call()
            .map_err(|e| format!("Failed to download {}: {}", name, e))?;

        let mut reader = response.into_body().into_reader();
        let mut out = File::create(&tarball_path)
            .map_err(|e| format!("Failed to create file {}: {}", tarball_path.display(), e))?;

        io::copy(&mut reader, &mut out)
            .map_err(|e| format!("Error saving {} tarball: {}", name, e))?;
    }

    println!(" Extracting {} archive...", name);
    let tar_gz = File::open(&tarball_path)
        .map_err(|e| format!("Failed to open {} tarball: {}", name, e))?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);

    fs::create_dir_all(&target_folder)
        .map_err(|e| format!("Failed to create folder {}: {}", target_folder.display(), e))?;

    archive
        .unpack(&target_folder)
        .map_err(|e| format!("Failed to extract {} archive: {}", name, e))?;

    Ok(target_folder)
}

fn build_and_install_nginx(
    src_folder_raw: &Path,
    pcre_folder_raw: &Path,
    openssl_folder_raw: &Path,
    zlib_folder_raw: &Path,
    install_dir: &Path,
    dar_dir: &Path,
) -> Result<(), String> {
    let src_folder = resolve_build_dir(src_folder_raw);
    let pcre_folder = resolve_build_dir(pcre_folder_raw);
    let openssl_folder = resolve_build_dir(openssl_folder_raw);
    let zlib_folder = resolve_build_dir(zlib_folder_raw);
    // Detect compiler or default to clang
    let cc = get_available_compiler();
    let num_jobs = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    println!(" Configuring Nginx build parameters (using CC={})...", cc);

    // Patch ngx_linux_config.h cleanly without nesting comment blocks
    let linux_config_path = src_folder.join("src").join("os").join("unix").join("ngx_linux_config.h");
    if linux_config_path.exists() {
        if let Ok(content) = fs::read_to_string(&linux_config_path) {
            if content.contains("#include <crypt.h>") {
                let patched = content.replace("#include <crypt.h>", "// #include <crypt.h>");
                let _ = fs::write(&linux_config_path, patched);
            }
        }
    }

    let configure_bin = src_folder.join("configure");

    let configure_status = Command::new(configure_bin)
        .current_dir(&src_folder)
        .env("CC", &cc)
        .arg(format!("--with-cc={}", cc))
        .arg(format!("--prefix={}", install_dir.display()))
        .arg(format!(
            "--error-log-path={}",
            dar_dir.join("logs").join("error.log").display()
        ))
        .arg(format!(
            "--http-log-path={}",
            dar_dir.join("logs").join("access.log").display()
        ))
        .arg(format!("--with-pcre={}", pcre_folder.display()))
        .arg("--with-pcre-jit")
        .arg(format!("--with-openssl={}", openssl_folder.display()))
        .arg(format!("--with-openssl-opt=CC={}", cc))
        .arg(format!("--with-zlib={}", zlib_folder.display()))
        .arg("--with-http_ssl_module")
        .arg("--with-http_v2_module")
        .arg("--without-http_auth_basic_module")
        .arg("--with-cc-opt=-Wno-error")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("Failed to execute ./configure: {}", e))?;

    if !configure_status.success() {
        return Err("Nginx configuration failed.".to_string());
    }

    println!(" Compiling Nginx along with static PCRE2, OpenSSL & zlib...");
    let make_status = Command::new("make")
        .arg(format!("-j{}", num_jobs))
        .current_dir(&src_folder)
        .env("CC", &cc)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("Failed to run make: {}", e))?;

    if !make_status.success() {
        return Err("Nginx compilation (make) failed.".to_string());
    }

    println!(" Installing Nginx (make install)...");
    let install_status = Command::new("make")
        .arg("install")
        .current_dir(&src_folder)
        .env("CC", &cc)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("Failed to run make install: {}", e))?;

    if !install_status.success() {
        return Err("Nginx installation (make install) failed.".to_string());
    }

    println!(" {}", "Nginx successfully built and installed!".green());
    Ok(())
}

fn get_available_compiler() -> String {
    if let Ok(cc) = std::env::var("CC") {
        return cc;
    }

    let candidates = ["gcc", "clang", "cc"];
    for compiler in candidates {
        if Command::new(compiler)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
        {
            return compiler.to_string();
        }
    }

    "clang".to_string()
}

fn create_default_nginx_config(dar_dir: &Path) -> Result<(), String> {
    let conf_dir = dar_dir.join("nginx").join("conf");
    let conf_file = conf_dir.join("nginx.conf");

    let sites_path = dar_dir.join("sites").join("*.conf");
    let nginx_conf_content = format!(
        r#"worker_processes 1;
pid {};

events {{
    worker_connections 1024;
}}

http {{
    include       mime.types;
    default_type  application/octet-stream;

    sendfile        on;
    keepalive_timeout  65;

    server {{
        listen 8080 default_server;
        server_name localhost;

        location / {{
            return 200 'Dar environment is active!';
            add_header Content-Type text/plain;
        }}
    }}

    include {};
}}
"#,
        dar_dir.join("logs").join("nginx.pid").display(),
        sites_path.display()
    );

    fs::write(&conf_file, nginx_conf_content)
        .map_err(|e| format!("Failed to write nginx.conf: {}", e))?;

    Ok(())
}

fn ensure_php_fpm_running(dar_dir: &Path) -> Result<(), String> {
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

fn create_default_php_fpm_config(dar_dir: &Path) -> Result<PathBuf, String> {
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

fn restart_nginx(nginx_bin: &Path, dar_dir: &Path) -> Result<(), String> {
    println!(" 🔄 Starting/Reloading Nginx service...");
    let conf_path = dar_dir.join("nginx").join("conf").join("nginx.conf");
    let pid_file = dar_dir.join("logs").join("nginx.pid");

    // Stop existing process if PID exists
    let _ = Command::new(nginx_bin)
        .arg("-s")
        .arg("stop")
        .arg("-c")
        .arg(&conf_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    // Kill process using the stored PID if port is still bound
    if pid_file.exists() {
        if let Ok(pid_str) = fs::read_to_string(&pid_file) {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                let _ = Command::new("kill").arg("-9").arg(pid.to_string()).status();
            }
        }
        let _ = fs::remove_file(&pid_file);
    }

    // Give the kernel socket a split second to release port 8080
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Start fresh instance
    let start_status = Command::new(nginx_bin)
        .arg("-c")
        .arg(&conf_path)
        .status()
        .map_err(|e| format!("Failed to start Nginx: {}", e))?;

    if !start_status.success() {
        return Err("Nginx failed to start.".to_string());
    }

    Ok(())
}

fn fetch_isolated_php(dar_dir: &Path, php_version: &str) -> Result<(), String> {
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

fn find_file_recursive(dir: &Path, filename: &str) -> Option<PathBuf> {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.file_name() == Some(std::ffi::OsStr::new(filename)) {
                return Some(path);
            } else if path.is_dir() {
                if let Some(found) = find_file_recursive(&path, filename) {
                    return Some(found);
                }
            }
        }
    }
    None
}

fn resolve_build_dir(path: &Path) -> PathBuf {
    // If path contains a single nested directory, use that instead
    if let Ok(entries) = fs::read_dir(path) {
        let items: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
        if items.len() == 1 && items[0].is_dir() {
            return items[0].clone();
        }
    }
    path.to_path_buf()
}
