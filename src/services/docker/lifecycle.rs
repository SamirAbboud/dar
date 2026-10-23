use std::process::Command;

/// Checks if Docker CLI is available and if the Docker daemon is active.
pub fn verify_docker() -> Result<(), String> {
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
pub fn container_exists(container_name: &str) -> Result<bool, String> {
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
