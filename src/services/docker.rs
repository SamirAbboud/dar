use std::fs;
use std::process::Command;

use crate::utils::paths::get_dar_dir;
use crate::services::templates::{ServiceTemplate, load_templates};
use crate::services::{nginx::is_nginx_running, php::is_fpm_running};

#[derive(Debug)]
pub struct ServiceInfo {
    pub name: String,
    pub service_type: String,
    pub port: String,
    pub status: String,
    pub is_active: bool,
}

/// Checks if Docker CLI is available and if the Docker daemon is active.
fn verify_docker() -> Result<(), String> {
    // Check if docker binary exists
    let output = Command::new("docker")
        .arg("info")
        .output()
        .map_err(|_| "❌ Docker is not installed or not in your PATH.".to_string())?;

    if !output.status.success() {
        return Err("⚠️ Docker daemon is not running. Please start Docker first.".to_string());
    }

    Ok(())
}

/// Checks if a container with the given exact name already exists in Docker (running or stopped).
fn container_exists(container_name: &str) -> Result<bool, String> {
    let check_output = Command::new("docker")
        .args([
            "ps",
            "-a",
            "--filter",
            &format!("name=^{}$", container_name),
            "--format",
            "{{.Names}}",
        ])
        .output()
        .map_err(|e| format!("Failed to query Docker daemon: {}", e))?;

    Ok(String::from_utf8_lossy(&check_output.stdout).trim() == container_name)
}

/// Provisions a new container instance from a `ServiceTemplate`.
fn create_container(template: &ServiceTemplate, service_dir: &std::path::Path) -> Result<(), String> {
    let container_name = format!("dar--{}", template.key);

    let mut args = vec![
        "run".to_string(),
        "-d".to_string(),
        "--name".to_string(),
        container_name.clone(),
        "--restart".to_string(),
        "unless-stopped".to_string(),
        "-p".to_string(),
        format!("{}:{}", template.default_port, template.container_port),
    ];

    for (host_p, container_p) in &template.additional_ports {
        args.push("-p".to_string());
        args.push(format!("{}:{}", host_p, container_p));
    }

    for (k, v) in &template.env_vars {
        args.push("-e".to_string());
        args.push(format!("{}={}", k, v));
    }

    for (rel_host, container_path) in &template.volume_mounts {
        let host_path = service_dir.join(rel_host);
        args.push("-v".to_string());
        args.push(format!("{}:{}", host_path.display(), container_path));
    }

    args.push(template.image.clone());

    if let Some(cmd) = &template.command {
        args.extend(cmd.clone());
    }

    let output = Command::new("docker")
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to execute docker run: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to create container '{}': {}",
            container_name,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

/// Builds a `ServiceInfo` summary for the local Nginx web server instance using PID check.
fn get_nginx_service_info(dar_dir: &std::path::Path) -> ServiceInfo {
    let is_active = is_nginx_running(dar_dir);

    ServiceInfo {
        name: "nginx".to_string(),
        service_type: "Nginx Web Server".to_string(),
        port: "80, 443".to_string(),
        status: if is_active { "active".to_string() } else { "stopped".to_string() },
        is_active,
    }
}

/// Builds a `ServiceInfo` summary for the local PHP-FPM runtime instance using PID check.
fn get_php_fpm_service_info(dar_dir: &std::path::Path) -> ServiceInfo {
    let is_active = is_fpm_running(dar_dir);

    ServiceInfo {
        name: "php-fpm".to_string(),
        service_type: "PHP Runtime (FPM)".to_string(),
        port: "unix socket".to_string(),
        status: if is_active { "active".to_string() } else { "stopped".to_string() },
        is_active,
    }
}

/// Formats raw Docker status string into human-readable states.
fn format_container_status(raw_status: &str, is_running: bool) -> String {
    if is_running {
        "running".to_string()
    } else if raw_status.starts_with("Exited (0)")
        || raw_status.starts_with("Exited (137)")
        || raw_status.starts_with("Exited (143)")
    {
        "stopped".to_string()
    } else if raw_status.starts_with("Exited") {
        raw_status.replace("Exited", "failed")
    } else {
        raw_status.to_string()
    }
}

/// Extracts unique "host:container" port pairs from live Docker `ps` output.
fn parse_live_ports(raw_ports: &str) -> Vec<String> {
    if raw_ports.is_empty() {
        return Vec::new();
    }

    let mut mappings = Vec::new();
    for mapping in raw_ports.split(',').map(str::trim) {
        if let Some((host_part, container_part)) = mapping.split_once("->") {
            let host_port = host_part.rsplit(':').next().unwrap_or(host_part);
            let container_port = container_part.split('/').next().unwrap_or(container_part);

            let pair = format!("{host_port}:{container_port}");
            if !mappings.contains(&pair) {
                mappings.push(pair);
            }
        }
    }
    mappings
}

/// Constructs port string from `templates.toml` when the container is inactive.
fn format_fallback_ports(tmpl: &ServiceTemplate) -> String {
    let mut ports = vec![tmpl.default_port.to_string()];
    for host_p in tmpl.additional_ports.keys() {
        ports.push(host_p.to_string());
    }
    ports.join(", ")
}

/// Parses raw tab-delimited stdout from `docker ps` into a structured `ServiceInfo` record.
fn parse_docker_service_info(
    line: &str,
    templates: &std::collections::HashMap<String, ServiceTemplate>,
) -> Option<ServiceInfo> {
    let [container_name, image, raw_ports, raw_status] = match line.split('\t').collect::<Vec<_>>()[..] {
        [c, i, p, s] => [c, i, p, s],
        _ => return None,
    };

    let service_key = container_name.trim_start_matches("dar--");
    let is_running = raw_status.starts_with("Up");
    let clean_status = format_container_status(raw_status, is_running);

    let live_mappings = parse_live_ports(raw_ports);
    let formatted_port = if !live_mappings.is_empty() {
        live_mappings.join(", ")
    } else if let Some(tmpl) = templates.get(service_key) {
        format_fallback_ports(tmpl)
    } else {
        "-".to_string()
    };

    Some(ServiceInfo {
        name: container_name.to_string(),
        service_type: format!("Docker ({image})"),
        port: formatted_port,
        status: clean_status,
        is_active: is_running,
    })
}

/// Gathers state and port configurations for all Dar environment services (Nginx, PHP-FPM, and Docker containers).
pub fn get_all_services() -> Result<Vec<ServiceInfo>, String> {
    let mut services = Vec::new();
    let dar_dir = get_dar_dir()?;
    let templates = load_templates(&dar_dir).unwrap_or_default();

    // 1. Nginx Status
    services.push(get_nginx_service_info(&dar_dir));

    // 2. Active PHP-FPM Status
    services.push(get_php_fpm_service_info(&dar_dir));

    // 3. Docker Containers (dar--*)
    if verify_docker().is_ok() {
        let output = Command::new("docker")
            .args([
                "ps",
                "-a",
                "--filter",
                "name=dar--",
                "--format",
                "{{.Names}}\t{{.Image}}\t{{.Ports}}\t{{.Status}}",
            ])
            .output();

        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                if let Some(info) = parse_docker_service_info(line, &templates) {
                    services.push(info);
                }
            }
        }
    }

    Ok(services)
}

/// Adds a new Docker container instance from a `ServiceTemplate`.
pub fn add_service(name: &str) -> Result<(), String> {
    verify_docker()?;

    let dar_dir = get_dar_dir()?;
    let templates = load_templates(&dar_dir)?;

    let template = templates
        .get(name)
        .ok_or_else(|| {
            let available: Vec<&str> = templates.keys().map(|s| s.as_str()).collect();
            format!("Unknown service template '{}'. Available templates: {}", name, available.join(", "))
        })?;

    let container_name = format!("dar--{}", template.key);

    if container_exists(&container_name)? {
        return Err(format!(
            "Service '{}' already exists. Run 'dar services --list' to check its status.",
            container_name
        ));
    }

    // Prepare persistent volume directory
    let service_dir = dar_dir.join("services").join(&template.key);
    fs::create_dir_all(service_dir.join("data"))
        .map_err(|e| format!("Failed to create service volume directory: {}", e))?;

    println!(
        "🚀 Provisioning {} ({}) on port {}...",
        template.name, template.image, template.default_port
    );

    create_container(template, &service_dir)?;

    println!("✨ Service '{}' added successfully and running!", container_name);
    Ok(())
}

/// Starts an existing Docker container by name.
pub fn start(name: &str) -> Result<(), String> {
    verify_docker()?;

    let container_name = format!("dar--{}", name);
    if !container_exists(&container_name)? {
        return Err(format!("Service container '{}' does not exist.", container_name));
    }

    println!("▶️ Starting service '{}'...", container_name);

    let output = Command::new("docker")
        .args(["start", &container_name])
        .output()
        .map_err(|e| format!("Failed to execute docker start: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to start service '{}': {}",
            container_name,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    println!("✨ Service '{}' started successfully!", container_name);
    Ok(())
}

/// Stops a running Docker container by name.
pub fn stop(name: &str) -> Result<(), String> {
    verify_docker()?;

    let container_name = format!("dar--{}", name);
    if !container_exists(&container_name)? {
        return Err(format!("Service container '{}' does not exist.", container_name));
    }

    println!("⏹️ Stopping service '{}'...", container_name);

    let output = Command::new("docker")
        .args(["stop", &container_name])
        .output()
        .map_err(|e| format!("Failed to execute docker stop: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to stop service '{}': {}",
            container_name,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    println!("🛑 Service '{}' stopped.", container_name);
    Ok(())
}

/// Restarts a running or stopped Docker container by name.
pub fn restart(name: &str) -> Result<(), String> {
    verify_docker()?;

    let container_name = format!("dar--{}", name);
    if !container_exists(&container_name)? {
        return Err(format!("Service container '{}' does not exist.", container_name));
    }

    println!("🔄 Restarting service '{}'...", container_name);

    let output = Command::new("docker")
        .args(["restart", &container_name])
        .output()
        .map_err(|e| format!("Failed to execute docker restart: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to restart service '{}': {}",
            container_name,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    println!("✨ Service '{}' restarted successfully!", container_name);
    Ok(())
}

/// Removes a Docker container and optionally prompts/purges persistent data.
pub fn remove(name: &str) -> Result<(), String> {
    verify_docker()?;

    let container_name = format!("dar--{}", name);
    if !container_exists(&container_name)? {
        return Err(format!("Service container '{}' does not exist.", container_name));
    }

    println!("🗑️ Removing service container '{}'...", container_name);

    // Force remove container (stops and removes in one command)
    let output = Command::new("docker")
        .args(["rm", "-f", &container_name])
        .output()
        .map_err(|e| format!("Failed to execute docker rm: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to remove service '{}': {}",
            container_name,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    // Optionally clean up or notify about persistent data path (~/.dar/services/<name>)
    let dar_dir = get_dar_dir()?;
    let service_dir = dar_dir.join("services").join(name);
    if service_dir.exists() {
        let _ = fs::remove_dir_all(&service_dir);
    }

    println!("✨ Service '{}' and its data removed successfully!", container_name);
    Ok(())
}
