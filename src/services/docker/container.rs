use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::services::docker::{container_exists, stop, verify_docker};
use crate::services::templates::{load_templates, ServiceTemplate};
use crate::utils::paths::get_dar_dir;

/// Ensures that the dedicated `dar-net` Docker bridge network exists.
pub fn ensure_docker_network() -> Result<(), String> {
    let check = Command::new("docker")
        .args(["network", "inspect", "dar-net"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if let Ok(status) = check {
        if !status.success() {
            let output = Command::new("docker")
                .args(["network", "create", "dar-net"])
                .output()
                .map_err(|e| format!("Failed to execute docker network create: {}", e))?;

            if !output.status.success() {
                return Err(format!(
                    "Failed to create docker network 'dar-net': {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }
    }

    Ok(())
}

/// Provisions a new container instance from a `ServiceTemplate`.
pub fn create_container(template: &ServiceTemplate, service_dir: &Path) -> Result<(), String> {
    ensure_docker_network()?;

    let container_name = format!("dar--{}", template.key);

    let mut args = vec![
        "run".to_string(),
        "-d".to_string(),
        "--name".to_string(),
        container_name.clone(),
        "--network".to_string(),
        "dar-net".to_string(),
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

/// Removes a Docker container and optionally prompts/purges persistent data.
pub fn remove(name: &str) -> Result<(), String> {
    verify_docker()?;

    let container_name = format!("dar--{}", name);
    if !container_exists(&container_name)? {
        return Err(format!("Service container '{}' does not exist.", container_name));
    }

    println!("🗑️ Removing service container '{}'...", container_name);

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

    let dar_dir = get_dar_dir()?;
    let service_dir = dar_dir.join("services").join(name);
    if service_dir.exists() {
        let _ = fs::remove_dir_all(&service_dir);
    }

    println!("✨ Service '{}' and its data removed successfully!", container_name);
    Ok(())
}

/// Stops all active Dar-managed Docker containers (containers starting with `dar--`).
pub fn stop_all() -> Result<(), String> {
    verify_docker()?;

    let output = Command::new("docker")
        .args([
            "ps",
            "-a",
            "--filter",
            "name=dar--",
            "--format",
            "{{.Names}}",
        ])
        .output()
        .map_err(|e| format!("Failed to list Docker containers: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "Failed to query Docker containers: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let containers: Vec<&str> = stdout
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    if containers.is_empty() {
        println!("ℹ️  No active Dar Docker containers found.");
        return Ok(());
    }

    println!("🛑 Stopping all Dar-managed Docker containers...");

    for container in containers {
        if let Some(service_name) = container.strip_prefix("dar--") {
            let _ = stop(service_name);
        }
    }

    Ok(())
}
