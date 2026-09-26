use colored::*;

use crate::services::nginx::restart_nginx;
use crate::services::php::ensure_php_fpm_running;
use crate::utils::paths::get_dar_dir;

/// Starts the local development services (PHP-FPM and Nginx) if they are not already running.
pub fn run() -> Result<(), String> {
    println!("{}", "🚀 Starting Dar background services...".cyan().bold());
    let dar_dir = get_dar_dir()?;

    if !dar_dir.exists() {
        return Err("Dar environment not found. Please run 'dar install' first.".to_string());
    }

    let nginx_bin = dar_dir.join("nginx").join("sbin").join("nginx");
    if !nginx_bin.exists() {
        return Err("Nginx binary missing. Please run 'dar install' first.".to_string());
    }

    // 1. Start PHP-FPM
    ensure_php_fpm_running(&dar_dir)?;

    // 2. Start/Reload Nginx
    restart_nginx(&nginx_bin, &dar_dir)?;

    println!(
        "\n{}",
        "✨ All Dar services are up and running!".green().bold()
    );

    Ok(())
}
