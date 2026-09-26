use colored::*;
use std::fs;
use std::process::Command;
use crate::utils::paths::get_dar_dir;

/// Stops active Dar background services (Nginx and PHP-FPM).
pub fn run() -> Result<(), String> {
    println!("{}", "🛑 Stopping Dar background services...".cyan().bold());

    let dar_dir = get_dar_dir()?;
    let logs_dir = dar_dir.join("logs");

    // 1. Stop Nginx
    let nginx_pid_file = logs_dir.join("nginx.pid");
    if nginx_pid_file.exists() {
        if let Ok(pid_str) = fs::read_to_string(&nginx_pid_file) {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                let _ = Command::new("kill").arg(pid.to_string()).status();
                println!("🌐 Nginx service stopped.");
            }
        }
        let _ = fs::remove_file(&nginx_pid_file);
    } else {
        println!("🌐 Nginx is not running.");
    }

    // 2. Stop PHP-FPM
    let php_pid_file = logs_dir.join("php-fpm.pid");
    if php_pid_file.exists() {
        if let Ok(pid_str) = fs::read_to_string(&php_pid_file) {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                let _ = Command::new("kill").arg(pid.to_string()).status();
                println!("🐘 PHP-FPM service stopped.");
            }
        }
        let _ = fs::remove_file(&php_pid_file);
    } else {
        println!("🐘 PHP-FPM is not running.");
    }

    println!(
        "\n{}",
        "✨ All Dar services have been stopped.".green().bold()
    );

    Ok(())
}
