use crate::services::php::{
    execute_isolated_php, 
    fetch_isolated_php, 
    list_installed_php_versions,
    resolve_isolation_target,
    ensure_isolated_fpm_running
};
use crate::services::nginx::ensure_and_isolate_nginx_site;
use crate::store::IsolationsStore;
use crate::utils::table::create_table;
use crate::utils::paths::get_dar_dir;

use colored::*;
use comfy_table::Cell;
use std::env;

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

    let active_version = fetch_isolated_php(&dar_dir, &version)?;

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
    let current_dir = env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;

    let resolved_version = resolve_isolation_target(target, &current_dir)?;

    println!(
        "\n{} Isolating project '{}' to PHP {}...",
        "🐘".cyan(),
        current_dir.file_name().unwrap_or_default().to_string_lossy().bold(),
        resolved_version.bold()
    );

    let socket_path = ensure_isolated_fpm_running(&dar_dir, &resolved_version)?;
    let site_name = ensure_and_isolate_nginx_site(&dar_dir, &current_dir, &socket_path)?;

    // Persist mapping into ~/.dar/isolations.json
    let mut isolations_store = IsolationsStore::load(&dar_dir)?;
    isolations_store.set_isolation(&current_dir, &resolved_version)?;

    println!(
        "✅ Site '{}' ({}.test) successfully isolated to PHP {}!\n",
        site_name.bold(),
        site_name,
        resolved_version.green().bold()
    );

    Ok(())
}

