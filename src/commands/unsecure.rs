use crate::services::nginx::{create_site_config, restart_nginx};
use crate::services::ssl::remove_certificate;
use crate::utils::paths::{resolve_app_name, resolve_target_path};
use crate::store::SitesStore;
use colored::*;

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
