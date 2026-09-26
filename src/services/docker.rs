use std::fs;
use std::process::{Command, Stdio};

use crate::utils::paths::get_dar_dir;

#[derive(Debug)]
pub struct ServiceInfo {
    pub name: String,
    pub service_type: String,
    pub status: String,
    pub is_active: bool,
}

/// Checks if Docker CLI is available and if the Docker daemon is active.
pub fn verify_docker() -> Result<(), String> {
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

pub fn add(name: &str) -> Result<(), String> {
    println!("🚀 Adding service '{}'...", name);
    Ok(())
}

pub fn start(name: &str) -> Result<(), String> {
    println!("▶️ Starting service '{}'...", name);
    Ok(())
}

pub fn stop(name: &str) -> Result<(), String> {
    println!("⏹️ Stopping service '{}'...", name);
    Ok(())
}

pub fn restart(name: &str) -> Result<(), String> {
    println!("🔄 Restarting service '{}'...", name);
    Ok(())
}

pub fn remove(name: &str) -> Result<(), String> {
    println!("🗑️ Removing service '{}'...", name);
    Ok(())
}
