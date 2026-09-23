use crate::store::SitesStore;
use crate::utils::table::create_table;
use colored::*;
use comfy_table::{Attribute, Cell, Color as TableColor};

/// Lists all currently linked local development sites.
pub fn run() -> Result<(), String> {
    let home = dirs::home_dir().ok_or_else(|| "Could not determine home directory".to_string())?;
    let dar_dir = home.join(".dar");

    let store = SitesStore::load(&dar_dir)?;

    if store.sites.is_empty() {
        println!("\n{} No sites linked yet.\n", "ℹ".cyan());
        println!("Run {} inside a project directory to link a site.", "dar link".bold());
        return Ok(());
    }

    let mut table = create_table(&["Site", "URL", "Path", "Secured"]);

    let mut sorted_sites: Vec<_> = store.sites.values().collect();
    sorted_sites.sort_by(|a, b| a.name.cmp(&b.name));

    for site in sorted_sites {
        let scheme = if site.secured { "https" } else { "http" };
        let url = format!("{}://{}.test", scheme, site.name);

        let secured_badge = if site.secured {
            Cell::new("YES").fg(TableColor::Green).add_attribute(Attribute::Bold)
        } else {
            Cell::new("NO").fg(TableColor::Yellow)
        };

        table.add_row(vec![
            Cell::new(&site.name).add_attribute(Attribute::Bold),
            Cell::new(&url).fg(TableColor::Blue),
            Cell::new(site.path.display().to_string()),
            secured_badge,
        ]);
    }

    println!("\n{}", table);
    println!("\nTotal linked sites: {}\n", store.sites.len().to_string().bold());

    Ok(())
}
