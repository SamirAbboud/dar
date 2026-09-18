use colored::*;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::utils::archive::resolve_build_dir;
use crate::utils::system::get_available_compiler;

pub fn build_and_install_nginx(
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

    let cc = get_available_compiler();
    let num_jobs = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    println!(" Configuring Nginx build parameters (using CC={})...", cc);

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

pub fn create_default_nginx_config(dar_dir: &Path) -> Result<(), String> {
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

pub fn restart_nginx(nginx_bin: &Path, dar_dir: &Path) -> Result<(), String> {
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

