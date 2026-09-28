use crate::services::nginx::{remove_site_config, restart as restart_nginx};
use crate::services::ssl::remove_certificate;
use crate::utils::paths::{resolve_app_name, resolve_target_path, get_dar_dir};
use crate::store::SitesStore;
use colored::*;

/// Unlinks a local development site.
pub fn run(name: Option<String>, path: Option<String>) -> Result<(), String> {
    let target_path = resolve_target_path(path)?;
    let app_name = resolve_app_name(name, &target_path)?;
    let dar_dir = get_dar_dir()?;
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
    restart_nginx(&dar_dir)?;

    println!(
        "\n{} Unlinked site: {}\n",
        "✅".green(),
        format!("{}.test", app_name).bold()
    );

    Ok(())
}
