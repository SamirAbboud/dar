use colored::*;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::utils::archive::resolve_build_dir;
use crate::utils::system::get_available_compiler;

/// Builds and installs Nginx from source.
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

/// Creates a default Nginx configuration file in `~/.dar/nginx/conf/nginx.conf`.
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
        listen 80 default_server;
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

/// Restarts the Nginx service.
pub fn restart_nginx(nginx_bin: &Path, dar_dir: &Path) -> Result<(), String> {
    println!("{}", "🔄 Reloading Nginx service...".cyan());
    let pid_file = dar_dir.join("logs").join("nginx.pid");

    if pid_file.exists() {
        if let Ok(pid_str) = fs::read_to_string(&pid_file) {
            let pid = pid_str.trim();

            let reload_status = Command::new("kill")
                .arg("-HUP")
                .arg(pid)
                .stderr(Stdio::null())
                .status();

            if let Ok(status) = reload_status {
                if status.success() {
                    return Ok(());
                }
            }
        }
        // Remove stale PID file if process was not running
        let _ = fs::remove_file(&pid_file);
    }

    // Start Nginx cleanly if reload failed or PID wasn't active
    let conf_file = dar_dir.join("nginx").join("conf").join("nginx.conf");
    let start_status = Command::new(nginx_bin)
        .arg("-c")
        .arg(conf_file)
        .status()
        .map_err(|e| format!("Failed to execute Nginx binary: {}", e))?;

    if !start_status.success() {
        return Err("Nginx failed to start.".to_string());
    }

    Ok(())
}

/// Generates the Nginx server block `.conf` file inside `~/.dar/sites/`.
pub fn create_site_config(
    dar_dir: &Path,
    app_name: &str,
    app_path: &Path,
    is_secured: bool,
) -> Result<(), String> {
    let sites_dir = dar_dir.join("sites");
    let logs_dir = dar_dir.join("logs");
    let certs_dir = dar_dir.join("certs");
    let config_file = sites_dir.join(format!("{}.conf", app_name));

    let public_dir = app_path.join("public");
    let document_root = if public_dir.exists() {
        public_dir
    } else {
        app_path.to_path_buf()
    };

    let access_log = logs_dir.join(format!("{}.access.log", app_name));
    let error_log = logs_dir.join(format!("{}.error.log", app_name));
    let socket_path = dar_dir.join("php").join("php-fpm.sock");

    let nginx_config = if is_secured {
        let cert_file = certs_dir.join(format!("{}.test.pem", app_name));
        let key_file = certs_dir.join(format!("{}.test-key.pem", app_name));

        build_https_config(
            app_name,
            &document_root,
            &cert_file,
            &key_file,
            &access_log,
            &error_log,
            &socket_path,
        )
    } else {
        build_http_config(
            app_name,
            &document_root,
            &access_log,
            &error_log,
            &socket_path,
        )
    };

    fs::write(&config_file, nginx_config)
        .map_err(|e| format!("Failed to write site config: {}", e))?;

    Ok(())
}

/// Ensures `127.0.0.1 <app_name>.test` exists in `/etc/hosts`.
pub fn ensure_hosts_entry(app_name: &str) -> Result<(), String> {
    let domain = format!("{}.test", app_name);
    let hosts_path = Path::new("/etc/hosts");

    // Read existing hosts content
    let hosts_content = fs::read_to_string(hosts_path).unwrap_or_default();

    if hosts_content.lines().any(|l| l.contains(&domain)) {
        return Ok(());
    }

    // Check if /etc/hosts is a read-only symlink (common on NixOS)
    let is_readonly_system = hosts_path
        .symlink_metadata()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);

    if is_readonly_system {
        println!(
            "  {}",
            format!("ℹ System /etc/hosts is read-only. Using local DNS for {}.", domain).dimmed()
        );
        return Ok(());
    }

    // Attempt sudo append only on writable Linux distributions
    println!("  {}", format!("Adding {} to /etc/hosts...", domain).cyan());

    let host_entry = format!("127.0.0.1 {}\n", domain);
    let status = Command::new("sudo")
        .arg("sh")
        .arg("-c")
        .arg(format!("echo '{}' >> /etc/hosts", host_entry.trim()))
        .stderr(Stdio::null())
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("  {}", format!("✔ Added {} to /etc/hosts", domain).dimmed());
        }
        _ => {
            println!(
                "  {}",
                format!("ℹ Could not write to /etc/hosts. Using local DNS for {}.", domain).dimmed()
            );
        }
    }

    Ok(())
}

/// Grants the Nginx binary capability to bind to low-numbered privileged ports (port 80)
pub fn set_nginx_capabilities(nginx_bin: &Path) -> Result<(), String> {
    // Grant privilege to bind to port 80 without root
    let status = Command::new("sudo")
        .arg("setcap")
        .arg("cap_net_bind_service=+ep")
        .arg(nginx_bin)
        .status()
        .map_err(|e| format!("Failed to execute setcap: {}", e))?;

    if !status.success() {
        return Err("Failed to grant port 80 binding privileges to Nginx binary.".to_string());
    }

    Ok(())
}

/// Builds the standard HTTP Nginx server block.
fn build_http_config(
    app_name: &str,
    root: &Path,
    access_log: &Path,
    error_log: &Path,
    fpm_sock: &Path,
) -> String {
    format!(
        r#"server {{
    listen 80;
    server_name {app_name}.test;
    root {root};

    index index.php index.html index.htm;

    access_log {access_log};
    error_log  {error_log};

    location / {{
        try_files $uri $uri/ /index.php?$query_string;
    }}

    location ~ \.php$ {{
        include fastcgi_params;
        fastcgi_pass unix:{fpm_sock};
        fastcgi_index index.php;
        fastcgi_param SCRIPT_FILENAME $document_root$fastcgi_script_name;
    }}

    location ~ /\.ht {{
        deny all;
    }}
}}
"#,
        app_name = app_name,
        root = root.display(),
        access_log = access_log.display(),
        error_log = error_log.display(),
        fpm_sock = fpm_sock.display(),
    )
}

/// Builds the HTTPS Nginx server block with HTTP -> HTTPS redirection.
fn build_https_config(
    app_name: &str,
    root: &Path,
    cert: &Path,
    key: &Path,
    access_log: &Path,
    error_log: &Path,
    fpm_sock: &Path,
) -> String {

    format!(
        r#"server {{
    listen 80;
    server_name {app_name}.test;
    return 301 https://$host$request_uri;
}}

server {{
    listen 443 ssl;
    server_name {app_name}.test;
    root {root};

    ssl_certificate     {cert};
    ssl_certificate_key {key};

    index index.php index.html index.htm;

    access_log {access_log};
    error_log  {error_log};

    location / {{
        try_files $uri $uri/ /index.php?$query_string;
    }}

    location ~ \.php$ {{
        include fastcgi_params;
        fastcgi_pass unix:{fpm_sock};
        fastcgi_index index.php;
        fastcgi_param SCRIPT_FILENAME $document_root$fastcgi_script_name;
    }}

    location ~ /\.ht {{
        deny all;
    }}
}}
"#,
        app_name = app_name,
        root = root.display(),
        cert = cert.display(),
        key = key.display(),
        access_log = access_log.display(),
        error_log = error_log.display(),
        fpm_sock = fpm_sock.display(),
    )
}

