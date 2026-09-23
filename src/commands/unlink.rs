use crate::services::nginx::{remove_site_config, restart_nginx};
use crate::services::ssl::remove_certificate;
use crate::store::SitesStore;
use colored::*;
use std::env;
use std::path::{Path, PathBuf};

fn resolve_target_path(path_arg: Option<String>) -> Result<PathBuf, String> {
    match path_arg {
        Some(p) => {
            let custom_path = PathBuf::from(p);
            if !custom_path.exists() {
                return Err(format!("The path '{}' does not exist.", custom_path.display()));
            }
            custom_path.canonicalize().map_err(|e| format!("Path resolution failed: {}", e))
        }
        None => env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e)),
    }
}

fn resolve_app_name(name_arg: Option<String>, target_path: &Path) -> Result<String, String> {
    match name_arg {
        Some(n) if !n.trim().is_empty() => Ok(n.trim().to_string()),
        _ => target_path
            .file_name()
            .ok_or_else(|| "Invalid target directory name".to_string())
            .map(|s| s.to_string_lossy().to_string()),
    }
}

/// Unlinks a local development site.
pub fn run(name: Option<String>, path: Option<String>) -> Result<(), String> {
    let target_path = resolve_target_path(path)?;
    let app_name = resolve_app_name(name, &target_path)?;

    let home = dirs::home_dir().ok_or_else(|| "Could not determine home directory".to_string())?;
    let dar_dir = home.join(".dar");
    let nginx_bin = dar_dir.join("nginx").join("sbin").join("nginx");

    let mut store = SitesStore::load(&dar_dir)?;

    if !store.sites.contains_key(&app_name) {
        println!(
            "\n{} Site '{}' is not linked.\n",
            "ℹ".cyan(),
            app_name.bold()
        );
        return Ok(());
    }

    // 1. Remove site config
    remove_site_config(&dar_dir, &app_name)?;

    // 2. Clean up SSL certs if they exist
    remove_certificate(&dar_dir, &app_name)?;

    // 3. Remove from store
    store.remove(&app_name)?;

    // 4. Reload Nginx
    restart_nginx(&nginx_bin, &dar_dir)?;

    println!(
        "\n{} Unlinked site: {}\n",
        "✅".green(),
        format!("{}.test", app_name).bold()
    );

    Ok(())
}
