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
        Some(svc) => match svc.trim().to_lowercase().as_str() {
            "nginx" => nginx::start(&dar_dir),
            "php-fpm" | "php" => php::start(&dar_dir),
            _ => docker::start(svc),
        },
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
        Some(svc) => match svc.trim().to_lowercase().as_str() {
            "nginx" => nginx::stop(&dar_dir),
            "php-fpm" | "php" => php::stop(&dar_dir),
            _ => docker::stop(svc),
        },
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
        Some(svc) => match svc.trim().to_lowercase().as_str() {
            "nginx" => nginx::restart(&dar_dir),
            "php-fpm" | "php" => php::restart(&dar_dir),
            _ => docker::restart(svc),
        },
        None => {
            println!("{}", "🔄 Restarting all core Dar services...".cyan().bold());
            nginx::restart(&dar_dir)?;
            php::restart(&dar_dir)?;
            println!("\n{}", "✨ All core services restarted!".green().bold());
            Ok(())
        }
    }
}

pub fn remove(name: &str) -> Result<(), String> {
    docker::remove(name)
}
