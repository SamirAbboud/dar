use colored::*;
use comfy_table::{Attribute, Cell, Color as TableColor};

use crate::services::docker;
use crate::utils::table::create_table;

pub fn list() -> Result<(), String> {
    let services = docker::get_all_services()?;

    if services.is_empty() {
        println!("\n{} No Dar services found.\n", "ℹ".cyan());
        return Ok(());
    }

    let mut table = create_table(&["Service", "Type", "Status"]);

    for svc in &services {
        let status_badge = if svc.is_active {
            Cell::new(&format!("✓ {}", svc.status))
                .fg(TableColor::Green)
                .add_attribute(Attribute::Bold)
        } else {
            Cell::new(&svc.status).fg(TableColor::Red)
        };

        table.add_row(vec![
            Cell::new(&svc.name).add_attribute(Attribute::Bold),
            Cell::new(&svc.service_type).fg(TableColor::Cyan),
            status_badge,
        ]);
    }

    println!("\n{}", table);
    println!("\nTotal services: {}\n", services.len().to_string().bold());

    Ok(())
}

pub fn add(name: &str) -> Result<(), String> {
    docker::add(name)
}

pub fn start(name: &str) -> Result<(), String> {
    docker::start(name)
}

pub fn stop(name: &str) -> Result<(), String> {
    docker::stop(name)
}

pub fn restart(name: &str) -> Result<(), String> {
    docker::restart(name)
}

pub fn remove(name: &str) -> Result<(), String> {
    docker::remove(name)
}
