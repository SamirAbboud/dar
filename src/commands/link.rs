use crate::services::nginx::{create_site_config, ensure_hosts_entry, is_site_linked, restart_nginx};
use crate::commands::park::park_folder;
use colored::*;
use std::env;
use std::path::PathBuf;

/// Links a directory to a local `.test` domain.
/// 
/// Accepts optional `--name` and `--path` overrides. If omitted, defaults to 
/// the current working directory path and its folder name.
pub fn run(name: Option<String>, path: Option<String>) -> Result<(), String> {
    let current_dir = env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;

    let target_path = match path {
        Some(p) => {
            let custom_path = PathBuf::from(p);
            if !custom_path.exists() {
                return Err(format!("The path '{}' does not exist.", custom_path.display()));
            }
            custom_path.canonicalize().map_err(|e| format!("Path resolution failed: {}", e))?
        }
        None => current_dir,
    };

    let app_name = match name {
        Some(n) if !n.trim().is_empty() => n.trim().to_string(),
        _ => target_path
            .file_name()
            .ok_or_else(|| "Invalid target directory name".to_string())?
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
    create_site_config(&dar_dir, &app_name, &target_path)?;

    // 2. Ensure Nginx hosts entry exists
    ensure_hosts_entry(&app_name)?;

    // 3. Auto-park parent folder if not already parked
    if let Some(parent_dir) = target_path.parent() {
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
        format!("http://{}.test", app_name).bold().underline(),
        target_path.display().to_string().dimmed()
    );

    Ok(())
}
