use colored::*;
use comfy_table::{Attribute, Cell, Color as TableColor};

use crate::services::{docker, nginx, php};
use crate::utils::{table::create_table, paths::get_dar_dir};

pub fn list() -> Result<(), String> {
    let services = docker::get_all_services()?;

    if services.is_empty() {
        println!("\n{} No Dar services found.\n", "ℹ".cyan());
        return Ok(());
    }

    let mut table = create_table(&["Service", "Type", "Port", "Status"]);

    for svc in &services {
        let status_badge = if svc.is_active {
            Cell::new(format!("✓ {}", svc.status))
                .fg(TableColor::Green)
                .add_attribute(Attribute::Bold)
        } else {
            Cell::new(&svc.status).fg(TableColor::Red)
        };

        table.add_row(vec![
            Cell::new(&svc.name).add_attribute(Attribute::Bold),
            Cell::new(&svc.service_type).fg(TableColor::Cyan),
            Cell::new(&svc.port).fg(TableColor::Yellow),
            status_badge,
        ]);
    }

    println!("\n{}", table);
    println!("\nTotal services: {}\n", services.len().to_string().bold());

    Ok(())
}

pub fn add(name: &str) -> Result<(), String> {
    docker::add_service(name)
}

pub fn start(name: Option<&str>) -> Result<(), String> {
    let dar_dir = get_dar_dir()?;

    match name {
        Some(svc) => {
            let target = svc.trim().to_lowercase();

            if target == "nginx" {
                nginx::start(&dar_dir)
            } else if target == "php-fpm" || target == "php" {
                // Starts global PHP-FPM + ALL isolated PHP-FPM daemons
                php::start(&dar_dir)
            } else if target == "php-fpm@global" || target == "php@global" {
                // Starts ONLY the global PHP-FPM daemon
                php::start_global_fpm(&dar_dir)
            } else if let Some(version) = target.strip_prefix("php-fpm@") {
                // Starts ONLY a specific isolated version (e.g. php-fpm@8.3)
                php::start_version(&dar_dir, version)
            } else {
                docker::start(svc)
            }
        }
        None => {
            println!("{}", "🚀 Starting all core Dar services...".cyan().bold());
            php::start(&dar_dir)?;
            nginx::start(&dar_dir)?;
            println!("\n{}", "✨ All core services started!".green().bold());
            Ok(())
        }
    }
}

pub fn stop(name: Option<&str>) -> Result<(), String> {
    let dar_dir = get_dar_dir()?;

    match name {
        Some(svc) => {
            let target = svc.trim().to_lowercase();

            if target == "nginx" {
                nginx::stop(&dar_dir)
            } else if target == "php-fpm" || target == "php" {
                // Stops global PHP-FPM + ALL isolated PHP-FPM daemons
                php::stop(&dar_dir)
            } else if target == "php-fpm@global" || target == "php@global" {
                // Stops ONLY the global PHP-FPM daemon
                php::stop_global_fpm(&dar_dir)
            } else if let Some(version) = target.strip_prefix("php-fpm@") {
                // Stops ONLY a specific isolated version (e.g. php-fpm@8.3)
                php::stop_version(&dar_dir, version)
            } else {
                docker::stop(svc)
            }
        }
        None => {
            println!("{}", "🛑 Stopping all core Dar services...".cyan().bold());
            nginx::stop(&dar_dir)?;
            php::stop(&dar_dir)?;
            println!("\n{}", "✨ All core services stopped.".green().bold());
            Ok(())
        }
    }
}

pub fn restart(name: Option<&str>) -> Result<(), String> {
    let dar_dir = get_dar_dir()?;

    match name {
        Some(svc) => {
            let target = svc.trim().to_lowercase();

            if target == "nginx" {
                nginx::restart(&dar_dir)
            } else if target == "php-fpm" || target == "php" {
                // Restarts global PHP-FPM + ALL isolated PHP-FPM daemons
                php::restart(&dar_dir)
            } else if target == "php-fpm@global" || target == "php@global" {
                // Restarts ONLY the global PHP-FPM daemon
                php::restart_global_fpm(&dar_dir)
            } else if let Some(version) = target.strip_prefix("php-fpm@") {
                // Restarts ONLY a specific isolated version (e.g. php-fpm@8.3)
                php::restart_version(&dar_dir, version)
            } else {
                docker::restart(svc)
            }
        }
        None => {
            println!("{}", "🔄 Restarting all core Dar services...".cyan().bold());
            php::restart(&dar_dir)?;
            nginx::restart(&dar_dir)?;
            println!("\n{}", "✨ All core services restarted!".green().bold());
            Ok(())
        }
    }
}

pub fn remove(name: &str) -> Result<(), String> {
    docker::remove(name)
}
