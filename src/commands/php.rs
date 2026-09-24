use crate::services::php::{fetch_isolated_php, list_installed_php_versions};
use crate::utils::table::create_table;
use colored::*;
use comfy_table::Cell;

/// Lists all locally provisioned PHP versions.
pub fn list() -> Result<(), String> {
    let home = dirs::home_dir().ok_or_else(|| "Could not determine home directory".to_string())?;
    let dar_dir = home.join(".dar");

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
    let home = dirs::home_dir().ok_or_else(|| "Could not determine home directory".to_string())?;
    let dar_dir = home.join(".dar");

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
    let home = dirs::home_dir().ok_or_else(|| "Could not determine home directory".to_string())?;
    let dar_dir = home.join(".dar");

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
