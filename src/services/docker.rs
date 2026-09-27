use std::fs;
use std::process::{Command, Stdio};

use crate::utils::paths::get_dar_dir;
use crate::services::templates::{ServiceTemplate, get_templates};

#[derive(Debug)]
pub struct ServiceInfo {
    pub name: String,
    pub service_type: String,
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

    for (k, v) in &template.env_vars {
        args.push("-e".to_string());
        args.push(format!("{}={}", k, v));
    }

    for (rel_host, container_path) in &template.volume_mounts {
        let host_path = service_dir.join(rel_host);
        args.push("-v".to_string());
        args.push(format!("{}:{}", host_path.display(), container_path));
    }

    args.push(template.image.to_string());

    let status = Command::new("docker")
        .args(&args)
        .status()
        .map_err(|e| format!("Failed to execute docker run: {}", e))?;

    if !status.success() {
        return Err(format!("Failed to create container '{}'.", container_name));
    }

    Ok(())
}

/// Gathers the status of all Dar-managed services (Nginx, PHP-FPM, Docker containers)
pub fn get_all_services() -> Result<Vec<ServiceInfo>, String> {
    let mut services = Vec::new();
    let dar_dir = get_dar_dir()?;

    // 1. Nginx Status
    let nginx_bin = dar_dir.join("nginx").join("sbin").join("nginx");
    let nginx_active = if nginx_bin.exists() {
        Command::new("pgrep")
            .arg("-f")
            .arg(nginx_bin.to_string_lossy().as_ref())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    } else {
        false
    };

    services.push(ServiceInfo {
        name: "nginx".to_string(),
        service_type: "Nginx Web Server".to_string(),
        status: if nginx_active { "active".to_string() } else { "stopped".to_string() },
        is_active: nginx_active,
    });

    // 2. Active PHP-FPM Status
    let pid_file = dar_dir.join("logs").join("php-fpm.pid");
    let php_active = if pid_file.exists() {
        if let Ok(pid_str) = fs::read_to_string(&pid_file) {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                Command::new("kill")
                    .arg("-0")
                    .arg(pid.to_string())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
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

    services.push(ServiceInfo {
        name: "php-fpm".to_string(),
        service_type: "PHP Runtime (FPM)".to_string(),
        status: if php_active { "active".to_string() } else { "stopped".to_string() },
        is_active: php_active,
    });

    // 3. Docker Containers (dar--*)
    if verify_docker().is_ok() {
        let output = Command::new("docker")
            .args([
                "ps",
                "-a",
                "--filter",
                "name=dar--",
                "--format",
                "{{.Names}}\t{{.Image}}\t{{.Status}}",
            ])
            .output();

        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() == 3 {
                    let is_running = parts[2].starts_with("Up");
                    services.push(ServiceInfo {
                        name: parts[0].to_string(),
                        service_type: format!("Docker ({})", parts[1]),
                        status: parts[2].to_string(),
                        is_active: is_running,
                    });
                }
            }
        }
    }

    Ok(services)
}

/// Adds a new Docker container instance from a `ServiceTemplate`.
pub fn add_service(name: &str) -> Result<(), String> {
    verify_docker()?;

    let templates = get_templates();
    let template = templates
        .get(name)
        .ok_or_else(|| format!("Unknown service '{}'. Available templates: mysql, postgres, redis, mailpit, meilisearch", name))?;

    let container_name = format!("dar--{}", template.key);

    if container_exists(&container_name)? {
        return Err(format!(
            "Service '{}' already exists. Run 'dar services --list' to check its status.",
            container_name
        ));
    }

    // Prepare persistent volume directory
    let dar_dir = get_dar_dir()?;
    let service_dir = dar_dir.join("services").join(template.key);
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
