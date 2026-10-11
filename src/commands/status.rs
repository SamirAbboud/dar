use colored::*;
use comfy_table::{Attribute, Cell, Color as TableColor};
use std::path::Path;

use crate::services::docker::{
    get_all_services,
    ServiceInfo,
};
use crate::utils::{
    table::create_table,
    paths::get_dar_dir,
    system::is_process_running,
};
use crate::services::php::list_installed_php_versions;
use crate::store::{SitesStore, IsolationsStore};

pub fn run() -> Result<(), String> {
    let dar_dir = get_dar_dir()?;
    let services = get_all_services()?;

    let dns_active = is_process_running("dnsmasq");
    let dns_status = if dns_active {
        "DNS: dnsmasq active".green()
    } else {
        "DNS: inactive".yellow()
    };

    println!("\n  {} ({})", "Dar Environment Dashboard".cyan().bold(), dns_status);

    render_system_table(&services);
    render_php_runtimes_table(&dar_dir)?;
    render_sites_table(&dar_dir, &services)?;
    render_docker_table(&services);

    Ok(())
}

/// 1. System Services (Nginx & PHP-FPM daemons)
fn render_system_table(services: &[ServiceInfo]) {
    let mut table = create_table(&["Service", "Type", "Port / Socket", "Status"]);

    let dnsmasq_running = is_process_running("dnsmasq");
    let dnsmasq_status = if dnsmasq_running {
        Cell::new("✓ active")
            .fg(TableColor::Green)
            .add_attribute(Attribute::Bold)
    } else {
        Cell::new("stopped").fg(TableColor::Red)
    };

    table.add_row(vec![
        Cell::new("dnsmasq").add_attribute(Attribute::Bold),
        Cell::new("Local DNS Resolver").fg(TableColor::Cyan),
        Cell::new("53").fg(TableColor::Yellow),
        dnsmasq_status,
    ]);

    let system_services: Vec<&ServiceInfo> = services
        .iter()
        .filter(|svc| !svc.name.starts_with("dar--"))
        .collect();

    for svc in &system_services {
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

    println!("\n  {}", format!("System Services ({})", system_services.len()).bold());
    println!("{}", table);
}

/// 2. Provisioned PHP Runtimes (from list_installed_php_versions)
fn render_php_runtimes_table(dar_dir: &Path) -> Result<(), String> {
    let mut table = create_table(&["Status", "Version", "Path"]);
    let (installed, active) = list_installed_php_versions(dar_dir);

    if installed.is_empty() {
        table.add_row(vec![
            Cell::new("-"),
            Cell::new("No PHP versions provisioned").fg(TableColor::DarkGrey),
            Cell::new("-"),
        ]);
    } else {
        for ver in &installed {
            let is_active = active.as_deref() == Some(ver.as_str());
            let path = dar_dir.join("php").join("versions").join(ver);

            let status_cell = if is_active {
                Cell::new("✓ active")
                    .fg(TableColor::Green)
                    .add_attribute(Attribute::Bold)
            } else {
                Cell::new("")
            };

            table.add_row(vec![
                status_cell,
                Cell::new(ver).add_attribute(Attribute::Bold),
                Cell::new(path.display().to_string()).fg(TableColor::DarkGrey),
            ]);
        }
    }

    println!("\n  {}", format!("PHP Runtimes ({})", installed.len()).bold());
    println!("{}", table);
    Ok(())
}

/// 3. Active Sites (using SitesStore & IsolationsStore)
fn render_sites_table(dar_dir: &Path, services: &[ServiceInfo]) -> Result<(), String> {
    let sites_store = SitesStore::load(dar_dir)?;
    let mut table = create_table(&["Site", "URL", "Path", "PHP Version", "Secured"]);

    if sites_store.sites.is_empty() {
        table.add_row(vec![
            Cell::new("No sites linked yet").fg(TableColor::DarkGrey),
            Cell::new("-"),
            Cell::new("-"),
            Cell::new("-"),
            Cell::new("-"),
        ]);
    } else {
        let isolations_store = IsolationsStore::load(dar_dir).ok();

        let mut sorted_sites: Vec<_> = sites_store.sites.values().collect();
        sorted_sites.sort_by(|a, b| a.name.cmp(&b.name));

        let is_global_running = services
            .iter()
            .any(|s| s.name == "php-fpm" && s.is_active);

        let active_isolated_versions: std::collections::HashSet<&str> = services
            .iter()
            .filter(|s| s.is_active && s.name.starts_with("php-fpm@"))
            .filter_map(|s| s.name.strip_prefix("php-fpm@"))
            .collect();

        for site in sorted_sites {
            let scheme = if site.secured { "https" } else { "http" };
            let url = format!("{}://{}.test", scheme, site.name);

            let secured_badge = if site.secured {
                Cell::new("● YES")
                    .fg(TableColor::Green)
                    .add_attribute(Attribute::Bold)
            } else {
                Cell::new("○ NO").fg(TableColor::Yellow)
            };

            let php_badge = match isolations_store.as_ref().and_then(|s| s.get_isolation(&site.path)) {
                Some(ver) => {
                    if active_isolated_versions.contains(ver.as_str()) {
                        Cell::new(format!("PHP {}", ver))
                            .fg(TableColor::Cyan)
                            .add_attribute(Attribute::Bold)
                    } else {
                        Cell::new(format!("PHP {} (fpm stopped)", ver))
                            .fg(TableColor::Yellow)
                            .add_attribute(Attribute::Bold)
                    }
                }
                None => {
                    if is_global_running {
                        Cell::new("Global").fg(TableColor::DarkGrey)
                    } else {
                        Cell::new("Global (fpm stopped)")
                            .fg(TableColor::Yellow)
                            .add_attribute(Attribute::Bold)
                    }
                }
            };

            table.add_row(vec![
                Cell::new(&site.name).add_attribute(Attribute::Bold),
                Cell::new(&url).fg(TableColor::Blue),
                Cell::new(site.path.display().to_string()),
                php_badge,
                secured_badge,
            ]);
        }
    }

    println!("\n  {}", format!("Active Sites ({})", sites_store.sites.len()).bold());
    println!("{}", table);
    Ok(())
}

/// 4. Service Containers (Docker dar--* containers matching dar services -l)
fn render_docker_table(services: &[ServiceInfo]) {
    let mut table = create_table(&["Container", "Type", "Port", "Status"]);

    let docker_services: Vec<&ServiceInfo> = services
        .iter()
        .filter(|svc| svc.name.starts_with("dar--"))
        .collect();

    if docker_services.is_empty() {
        table.add_row(vec![
            Cell::new("No service containers active").fg(TableColor::DarkGrey),
            Cell::new("-"),
            Cell::new("-"),
            Cell::new("-"),
        ]);
    } else {
        for svc in &docker_services {
            let container_name = svc.name.trim_start_matches("dar--");
            let status_badge = if svc.is_active {
                Cell::new(format!("✓ {}", svc.status))
                    .fg(TableColor::Green)
                    .add_attribute(Attribute::Bold)
            } else {
                Cell::new(&svc.status).fg(TableColor::Red)
            };

            let port_display = if svc.port.is_empty() {
                "-".to_string()
            } else {
                svc.port.clone()
            };

            table.add_row(vec![
                Cell::new(container_name).add_attribute(Attribute::Bold),
                Cell::new(&svc.service_type).fg(TableColor::Cyan),
                Cell::new(port_display).fg(TableColor::Yellow),
                status_badge,
            ]);
        }
    }

    println!("\n  {}", format!("Service Containers ({})", docker_services.len()).bold());
    println!("{}\n", table);
}

