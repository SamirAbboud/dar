use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::Command;

use crate::services::docker::verify_docker;
use crate::services::nginx::is_nginx_running;
use crate::services::php::is_fpm_running;
use crate::services::templates::{load_templates, ServiceTemplate};
use crate::store::IsolationsStore;
use crate::utils::paths::get_dar_dir;

#[derive(Debug)]
pub struct ServiceInfo {
    pub name: String,
    pub service_type: String,
    pub port: String,
    pub status: String,
    pub is_active: bool,
}

/// Builds a `ServiceInfo` summary for the local Nginx web server instance using PID check.
pub fn get_nginx_service_info(dar_dir: &Path) -> ServiceInfo {
    let is_active = is_nginx_running(dar_dir);

    ServiceInfo {
        name: "nginx".to_string(),
        service_type: "Nginx Web Server".to_string(),
        port: "80, 443".to_string(),
        status: if is_active { "active".to_string() } else { "stopped".to_string() },
        is_active,
    }
}

/// Builds `ServiceInfo` summaries for the global PHP-FPM and all running/configured isolated instances.
pub fn get_php_fpm_services_info(dar_dir: &Path) -> Vec<ServiceInfo> {
    let mut services = Vec::new();

    let is_global_active = is_fpm_running(dar_dir, None);
    services.push(ServiceInfo {
        name: "php-fpm".to_string(),
        service_type: "PHP Runtime (FPM - Global)".to_string(),
        port: "unix socket".to_string(),
        status: if is_global_active {
            "active".to_string()
        } else {
            "stopped".to_string()
        },
        is_active: is_global_active,
    });

    if let Ok(store) = IsolationsStore::load(dar_dir) {
        let unique_versions: HashSet<_> = store.mappings.values().cloned().collect();

        for ver in unique_versions {
            let is_active = is_fpm_running(dar_dir, Some(&ver));

            services.push(ServiceInfo {
                name: format!("php-fpm@{}", ver),
                service_type: format!("PHP Runtime (FPM v{})", ver),
                port: "unix socket".to_string(),
                status: if is_active {
                    "active".to_string()
                } else {
                    "stopped".to_string()
                },
                is_active,
            });
        }
    }

    services
}

/// Formats raw Docker status string into human-readable states.
pub fn format_container_status(raw_status: &str, is_running: bool) -> String {
    if is_running {
        "running".to_string()
    } else if raw_status.starts_with("Exited (0)")
        || raw_status.starts_with("Exited (137)")
        || raw_status.starts_with("Exited (143)")
    {
        "stopped".to_string()
    } else if raw_status.starts_with("Exited") {
        raw_status.replace("Exited", "failed")
    } else {
        raw_status.to_string()
    }
}

/// Extracts unique "host:container" port pairs from live Docker `ps` output.
pub fn parse_live_ports(raw_ports: &str) -> Vec<String> {
    if raw_ports.is_empty() {
        return Vec::new();
    }

    let mut mappings = Vec::new();
    for mapping in raw_ports.split(',').map(str::trim) {
        if let Some((host_part, container_part)) = mapping.split_once("->") {
            let host_port = host_part.rsplit(':').next().unwrap_or(host_part);
            let container_port = container_part.split('/').next().unwrap_or(container_part);

            let pair = format!("{host_port}:{container_port}");
            if !mappings.contains(&pair) {
                mappings.push(pair);
            }
        }
    }
    mappings
}

/// Constructs port string from `templates.toml` when the container is inactive.
pub fn format_fallback_ports(tmpl: &ServiceTemplate) -> String {
    let mut ports = vec![tmpl.default_port.to_string()];
    for host_p in tmpl.additional_ports.keys() {
        ports.push(host_p.to_string());
    }
    ports.join(", ")
}

/// Parses raw tab-delimited stdout from `docker ps` into a structured `ServiceInfo` record.
pub fn parse_docker_service_info(
    line: &str,
    templates: &HashMap<String, ServiceTemplate>,
) -> Option<ServiceInfo> {
    let [container_name, image, raw_ports, raw_status] = match line.split('\t').collect::<Vec<_>>()[..] {
        [c, i, p, s] => [c, i, p, s],
        _ => return None,
    };

    let service_key = container_name.trim_start_matches("dar--");
    let is_running = raw_status.starts_with("Up");
    let clean_status = format_container_status(raw_status, is_running);

    let live_mappings = parse_live_ports(raw_ports);
    let formatted_port = if !live_mappings.is_empty() {
        live_mappings.join(", ")
    } else if let Some(tmpl) = templates.get(service_key) {
        format_fallback_ports(tmpl)
    } else {
        "-".to_string()
    };

    Some(ServiceInfo {
        name: container_name.to_string(),
        service_type: format!("Docker ({image})"),
        port: formatted_port,
        status: clean_status,
        is_active: is_running,
    })
}

/// Gathers state and port configurations for all Dar environment services (Nginx, PHP-FPM, and Docker containers).
pub fn get_all_services() -> Result<Vec<ServiceInfo>, String> {
    let mut services = Vec::new();
    let dar_dir = get_dar_dir()?;
    let templates = load_templates(&dar_dir).unwrap_or_default();

    services.push(get_nginx_service_info(&dar_dir));
    services.extend(get_php_fpm_services_info(&dar_dir));

    if verify_docker().is_ok() {
        let output = Command::new("docker")
            .args([
                "ps",
                "-a",
                "--filter",
                "name=dar--",
                "--format",
                "{{.Names}}\t{{.Image}}\t{{.Ports}}\t{{.Status}}",
            ])
            .output();

        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                if let Some(info) = parse_docker_service_info(line, &templates) {
                    services.push(info);
                }
            }
        }
    }

    Ok(services)
}
