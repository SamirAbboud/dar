use crate::services::nginx::{create_site_config, ensure_hosts_entry, is_site_linked, restart_nginx};
use crate::commands::park::park_folder;
use colored::*;
use std::env;

pub fn run(name: Option<String>) -> Result<(), String> {
    let current_dir = env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;

    let app_name = match name {
        Some(n) if !n.trim().is_empty() => n,
        _ => current_dir
            .file_name()
            .ok_or_else(|| "Invalid directory name".to_string())?
            .to_string_lossy()
            .to_string(),
    };

    let home = dirs::home_dir()
        .ok_or_else(|| "Could not determine home directory".to_string())?;
    let dar_dir = home.join(".dar");
    let nginx_bin = dar_dir.join("nginx").join("sbin").join("nginx");

    if is_site_linked(&dar_dir, &app_name) {
        return Err(format!("Site '{}' is already linked.", app_name));
    }

    println!("\n{}", format!("🔗 Linking site: {}", app_name.bold()).cyan());

    // 1. Generate Nginx virtual host configuration
    create_site_config(&dar_dir, &app_name, &current_dir)?;

    // 2. Ensure Nginx hosts entry exists
    ensure_hosts_entry(&app_name)?;

    // 3. Auto-park parent folder if not already parked
    if let Some(parent_dir) = current_dir.parent() {
        if let Ok(was_added) = park_folder(&dar_dir, parent_dir) {
            if was_added {
                println!(
                    "{}",
                    format!("  📂 Auto-parked parent folder: {}", parent_dir.display()).dimmed()
                );
            }
        }
    }

    // 4. Restart Nginx service
    restart_nginx(&nginx_bin, &dar_dir)?;

    println!(
        "\n{} Site linked: {} -> {}\n",
        "✅".green(),
        format!("http://{}.test:8080", app_name).bold().underline(),
        current_dir.display().to_string().dimmed()
    );

    Ok(())
}
