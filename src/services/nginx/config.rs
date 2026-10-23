use colored::*;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::services::nginx::lifecycle::restart;
use crate::services::php::get_isolated_fpm_socket;
use crate::store::{IsolationsStore, Site, SitesStore};

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
    include        mime.types;
    default_type   application/octet-stream;

    sendfile        on;
    keepalive_timeout  65;

    # Fallback server block for unlinked HTTP sites
    server {{
        listen 80 default_server;
        listen [::]:80 default_server;
        server_name _;

        location / {{
            default_type text/html;
            return 404 '<!DOCTYPE html>
<html>
<head>
    <title>Site Not Linked - Dar</title>
    <style>
        body {{ font-family: system-ui, -apple-system, sans-serif; background: #1a1b26; color: #c0caf5; display: flex; justify-content: center; height: 100vh; margin: 0; }}
        .card {{ background: #24283b; padding: 2.5rem; border-radius: 12px; border: 1px solid #414868; max-width: 480px; text-align: center; box-shadow: 0 10px 30px rgba(0,0,0,0.4); margin-top: 15vh; height: fit-content; }}
        h1 {{ color: #f7768e; margin-top: 0; font-size: 1.5rem; }}
        p {{ color: #9aa5ce; line-height: 1.5; font-size: 0.95rem; }}
        code {{ background: #292e42; color: #7aa2f7; padding: 0.2rem 0.5rem; border-radius: 4px; font-family: monospace; font-size: 0.9rem; }}
        .cmd {{ background: #1d202f; padding: 0.75rem; border-radius: 6px; border: 1px solid #414868; display: block; margin: 1.5rem 0; text-align: left; }}
    </style>
</head>
<body>
    <div class="card">
        <h1>Site Not Linked</h1>
        <p>Local DNS is working, but this <code>.test</code> domain has not been registered in <strong>dar</strong> yet.</p>
        <div class="cmd">
            <code>&#36; cd /path/to/project</code><br>
            <code>&#36; dar link</code>
        </div>
        <p style="font-size: 0.85rem; color: #6c7086;">Run <code>dar status</code> or <code>dar sites</code> to view active linked applications.</p>
    </div>
</body>
</html>';
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

    let socket_path = match IsolationsStore::load(dar_dir) {
        Ok(store) => match store.get_isolation(app_path) {
            Some(version) => get_isolated_fpm_socket(dar_dir, version),
            None => dar_dir.join("php").join("php-fpm.sock"),
        },
        Err(_) => dar_dir.join("php").join("php-fpm.sock"),
    };

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

    let hosts_content = fs::read_to_string(hosts_path).unwrap_or_default();

    if hosts_content.lines().any(|l| l.contains(&domain)) {
        return Ok(());
    }

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

/// Deletes the site configuration file if it exists.
pub fn remove_site_config(dar_dir: &Path, app_name: &str) -> Result<(), String> {
    let config_file = dar_dir.join("sites").join(format!("{}.conf", app_name));
    if config_file.exists() {
        fs::remove_file(&config_file)
            .map_err(|e| format!("Failed to delete site config: {}", e))?;
    }
    Ok(())
}

/// Ensures the site is linked and its Nginx vhost exists, then updates `fastcgi_pass` 
/// to point to the isolated socket path and reloads Nginx.
pub fn ensure_and_isolate_nginx_site(
    dar_dir: &Path,
    project_path: &Path,
    socket_path: &Path,
) -> Result<String, String> {
    let mut store = SitesStore::load(dar_dir)?;

    let site_name = match store.find_by_path(project_path) {
        Some(s) => s.name.clone(),
        None => {
            let default_name = project_path
                .file_name()
                .ok_or_else(|| "Failed to derive site name from directory path".to_string())?
                .to_string_lossy()
                .to_string();

            println!("🔗 Auto-linking site as '{}.test'...", default_name);

            create_site_config(dar_dir, &default_name, project_path, false)?;

            let new_site = Site::new(default_name.clone(), project_path.to_path_buf(), false);
            store.add(new_site)?;

            default_name
        }
    };

    let conf_path = dar_dir.join("sites").join(format!("{}.conf", site_name));

    if !conf_path.exists() {
        return Err(format!(
            "Nginx site configuration file missing at `{}`.",
            conf_path.display()
        ));
    }

    let content = fs::read_to_string(&conf_path)
        .map_err(|e| format!("Failed to read site config {}: {}", conf_path.display(), e))?;

    if !content.contains("fastcgi_pass") {
        return Err(format!(
            "No `fastcgi_pass` directive found in `{}` to update.",
            conf_path.display()
        ));
    }

    let new_content = content
        .lines()
        .map(|line| {
            if line.trim_start().starts_with("fastcgi_pass") {
                let indent = &line[..line.len() - line.trim_start().len()];
                format!("{}fastcgi_pass unix:{};", indent, socket_path.display())
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n") + "\n";

    fs::write(&conf_path, new_content)
        .map_err(|e| format!("Failed to write updated site config: {}", e))?;

    restart(dar_dir)?;

    Ok(site_name)
}

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
