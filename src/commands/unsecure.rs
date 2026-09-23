use crate::services::nginx::{create_site_config, restart_nginx};
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

/// Downgrades an HTTPS site to plain HTTP.
pub fn run(name: Option<String>, path: Option<String>) -> Result<(), String> {
    let target_path = resolve_target_path(path)?;
    let app_name = resolve_app_name(name, &target_path)?;

    let home = dirs::home_dir().ok_or_else(|| "Could not determine home directory".to_string())?;
    let dar_dir = home.join(".dar");
    let nginx_bin = dar_dir.join("nginx").join("sbin").join("nginx");

    let mut store = SitesStore::load(&dar_dir)?;

    let site = match store.get(&app_name) {
        Some(s) => s.clone(),
        None => {
            println!(
                "\n{} Site '{}' is not linked.\n",
                "ℹ".cyan(),
                app_name.bold()
            );
            return Ok(());
        }
    };

    if !site.secured {
        println!(
            "\n{} Site '{}' is already unsecured ({})\n",
            "ℹ".cyan(),
            app_name.bold(),
            format!("http://{}.test", app_name).underline()
        );
        return Ok(());
    }

    // 1. Delete certificates
    remove_certificate(&dar_dir, &app_name)?;

    // 2. Regenerate HTTP-only Nginx configuration
    create_site_config(&dar_dir, &app_name, &site.path, false)?;

    // 3. Update store
    store.set_secured(&app_name, false)?;

    // 4. Reload Nginx
    restart_nginx(&nginx_bin, &dar_dir)?;

    println!(
        "\n{} Unsecured site: {}\n",
        "✅".green(),
        format!("http://{}.test", app_name).bold().underline()
    );

    Ok(())
}
