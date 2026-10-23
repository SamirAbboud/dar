use colored::*;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

/// Checks if Nginx is actively running by sending signal 0 to its PID.
pub fn is_nginx_running(dar_dir: &Path) -> bool {
    let pid_file = dar_dir.join("logs").join("nginx.pid");
    if !pid_file.exists() {
        return false;
    }

    if let Ok(pid_str) = fs::read_to_string(&pid_file) {
        if let Ok(pid) = pid_str.trim().parse::<i32>() {
            return Command::new("kill")
                .arg("-0")
                .arg(pid.to_string())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
        }
    }
    false
}

/// Starts Nginx if it's not already running.
pub fn start(dar_dir: &Path) -> Result<(), String> {
    let nginx_bin = dar_dir.join("nginx").join("sbin").join("nginx");
    if !nginx_bin.exists() {
        return Err("Nginx binary missing. Please run 'dar install' first.".to_string());
    }

    if is_nginx_running(dar_dir) {
        println!("{}", "🌐 Nginx is already running.".yellow());
        return Ok(());
    }

    println!("{}", "🚀 Starting Nginx Web Server...".cyan());
    let conf_file = dar_dir.join("nginx").join("conf").join("nginx.conf");

    let status = Command::new(&nginx_bin)
        .arg("-c")
        .arg(&conf_file)
        .status()
        .map_err(|e| format!("Failed to execute Nginx binary: {}", e))?;

    if !status.success() {
        return Err("Nginx failed to start.".to_string());
    }

    println!("{}", "✨ Nginx started successfully!".green());
    Ok(())
}

/// Stops active Nginx process using its PID file.
pub fn stop(dar_dir: &Path) -> Result<(), String> {
    let pid_file = dar_dir.join("logs").join("nginx.pid");

    if pid_file.exists() {
        if let Ok(pid_str) = fs::read_to_string(&pid_file) {
            let pid = pid_str.trim();
            println!("{}", "🛑 Stopping Nginx service...".cyan());

            let _ = Command::new("kill")
                .arg(pid)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
        let _ = fs::remove_file(&pid_file);
        println!("{}", "✨ Nginx stopped successfully.".green());
    } else {
        println!("{}", "🌐 Nginx is not running.".yellow());
    }

    Ok(())
}

/// Restarts Nginx via SIGHUP reload if running, or performs a full start.
pub fn restart(dar_dir: &Path) -> Result<(), String> {
    let nginx_bin = dar_dir.join("nginx").join("sbin").join("nginx");
    if !nginx_bin.exists() {
        return Err("Nginx binary missing. Please run 'dar install' first.".to_string());
    }

    let pid_file = dar_dir.join("logs").join("nginx.pid");

    if pid_file.exists() && is_nginx_running(dar_dir) {
        if let Ok(pid_str) = fs::read_to_string(&pid_file) {
            let pid = pid_str.trim();
            println!("{}", "🔄 Reloading Nginx configuration...".cyan());

            let status = Command::new("kill")
                .arg("-HUP")
                .arg(pid)
                .stderr(Stdio::null())
                .status();

            if let Ok(s) = status {
                if s.success() {
                    println!("{}", "✨ Nginx reloaded successfully!".green());
                    return Ok(());
                }
            }
        }
        let _ = fs::remove_file(&pid_file);
    }

    start(dar_dir)
}
