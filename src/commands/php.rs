use crate::services::php::{
    execute_isolated_php,
    ensure_php_version,
    set_global_php_version,
    list_installed_php_versions,
    resolve_isolation_target,
    ensure_isolated_fpm_running
};
use crate::services::nginx::{ensure_and_isolate_nginx_site, create_site_config, restart as restart_nginx};
use crate::store::{IsolationsStore, SitesStore};
use crate::utils::table::create_table;
use crate::utils::paths::{get_dar_dir, resolve_target_path};

use colored::*;
use comfy_table::Cell;

/// Lists all locally provisioned PHP versions.
pub fn list() -> Result<(), String> {
    let dar_dir = get_dar_dir()?;

    let (installed, active) = list_installed_php_versions(&dar_dir);

    if installed.is_empty() {
        println!(
            "\n{} No PHP versions provisioned yet. Run '{}' to install one.\n",
            "ℹ".cyan(),
            "dar php --use 8.3".bold()
        );
        return Ok(());
    }

    let mut table = create_table(&["Status", "Version", "Path"]);

    for ver in installed {
        let is_active = active.as_deref() == Some(ver.as_str());
        let status_text = if is_active { "✓ active" } else { "" };
        let path = dar_dir.join("php").join("versions").join(&ver);

        // Render cell with explicit text and apply colors through string formatting cleanly
        let status_cell = if is_active {
            Cell::new(status_text).fg(comfy_table::Color::Green)
        } else {
            Cell::new("")
        };

        table.add_row(vec![
            status_cell,
            Cell::new(&ver),
            Cell::new(path.display().to_string()).fg(comfy_table::Color::DarkGrey),
        ]);
    }

    println!("\n{}", table);
    Ok(())
}

/// Switches or provisions the active isolated PHP version.
pub fn use_version(version: String) -> Result<(), String> {
    let dar_dir = get_dar_dir()?;

    println!("\n{}", format!("🐘 Switching to PHP {}...", version.bold()).cyan());

    let active_version = ensure_php_version(&dar_dir, &version)?;
    set_global_php_version(&dar_dir, &active_version)?;

    println!(
        "\n{} Now using PHP {}\n",
        "✅".green(),
        active_version.bold()
    );

    Ok(())
}

/// Displays the currently active PHP version.
pub fn current() -> Result<(), String> {
    let dar_dir = get_dar_dir()?;

    let (_, active) = list_installed_php_versions(&dar_dir);

    match active {
        Some(ver) => println!(
            "\n{} Active PHP version: {}\n",
            "🐘".cyan(),
            ver.bold()
        ),
        None => println!(
            "\n{} No active PHP version set. Run '{}' to activate one.\n",
            "ℹ".cyan(),
            "dar php --use 8.3".bold()
        ),
    }

    Ok(())
}

/// Executes raw CLI arguments against the currently active isolated PHP binary.
pub fn passthrough(args: &[String]) -> Result<(), String> {
    let dar_dir = get_dar_dir()?;
    execute_isolated_php(&dar_dir, args)
}

/// Isolates the current project site to a specific PHP version.
pub fn isolate(target: Option<String>) -> Result<(), String> {
    let dar_dir = get_dar_dir()?;
    let target_path = resolve_target_path(None)?;

    let resolved_version = resolve_isolation_target(target, &target_path)?;

    println!(
        "\n{} Isolating project '{}' to PHP {}...",
        "🐘".cyan(),
        target_path.file_name().unwrap_or_default().to_string_lossy().bold(),
        resolved_version.bold()
    );

    let socket_path = ensure_isolated_fpm_running(&dar_dir, &resolved_version)?;
    let site_name = ensure_and_isolate_nginx_site(&dar_dir, &target_path, &socket_path)?;

    // Persist mapping into ~/.dar/isolations.json
    let mut isolations_store = IsolationsStore::load(&dar_dir)?;
    isolations_store.set_isolation(&target_path, &resolved_version)?;

    println!(
        "✅ Site '{}' ({}.test) successfully isolated to PHP {}!\n",
        site_name.bold(),
        site_name,
        resolved_version.green().bold()
    );

    Ok(())
}

/// Unisolates the current project site.
pub fn unisolate() -> Result<(), String> {
    let target_path = resolve_target_path(None)?;
    let dar_dir = get_dar_dir()?;

    let mut isolations_store = IsolationsStore::load(&dar_dir)?;

    match isolations_store.remove_isolation(&target_path)? {
        Some(previous_version) => {
            let sites_store = SitesStore::load(&dar_dir)?;

            if let Some(site) = sites_store.find_by_path(&target_path) {
                create_site_config(&dar_dir, &site.name, &target_path, site.secured)?;
                restart_nginx(&dar_dir)?;

                println!(
                    "\n{} Removed PHP {} isolation from site '{}'. Restored to global PHP-FPM.\n",
                    "✅".green(),
                    previous_version.bold(),
                    site.name.bold()
                );
            } else {
                println!(
                    "\n{} Removed PHP {} isolation mapping for path: {}\n",
                    "✅".green(),
                    previous_version.bold(),
                    target_path.display().to_string().dimmed()
                );
            }
        }
        None => {
            println!(
                "\n{} Directory '{}' is not isolated.\n",
                "ℹ".cyan(),
                target_path.display().to_string().dimmed()
            );
        }
    }

    Ok(())
}
