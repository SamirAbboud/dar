use crate::store::{IsolationsStore, SitesStore};
use crate::utils::table::create_table;
use crate::utils::paths::get_dar_dir;
use colored::*;
use comfy_table::{Attribute, Cell, Color as TableColor};

/// Lists all currently linked local development sites.
pub fn run() -> Result<(), String> {
    let dar_dir = get_dar_dir()?;

    let sites_store = SitesStore::load(&dar_dir)?;

    if sites_store.sites.is_empty() {
        println!("\n{} No sites linked yet.\n", "ℹ".cyan());
        println!("Run {} inside a project directory to link a site.", "dar link".bold());
        return Ok(());
    }

    let isolations_store = IsolationsStore::load(&dar_dir).ok();

    let mut table = create_table(&["Site", "URL", "Path", "PHP Version", "Secured"]);

    let mut sorted_sites: Vec<_> = sites_store.sites.values().collect();
    sorted_sites.sort_by(|a, b| a.name.cmp(&b.name));

    for site in sorted_sites {
        let scheme = if site.secured { "https" } else { "http" };
        let url = format!("{}://{}.test", scheme, site.name);

        let secured_badge = if site.secured {
            Cell::new("YES").fg(TableColor::Green).add_attribute(Attribute::Bold)
        } else {
            Cell::new("NO").fg(TableColor::Yellow)
        };

        let php_badge = match isolations_store.as_ref().and_then(|store| store.get_isolation(&site.path)) {
            Some(version) => Cell::new(format!("PHP {}", version))
                .fg(TableColor::Cyan)
                .add_attribute(Attribute::Bold),
            None => Cell::new("Global").fg(TableColor::DarkGrey),
        };

        table.add_row(vec![
            Cell::new(&site.name).add_attribute(Attribute::Bold),
            Cell::new(&url).fg(TableColor::Blue),
            Cell::new(site.path.display().to_string()),
            php_badge,
            secured_badge,
        ]);
    }

    println!("\n{}", table);
    println!("\nTotal linked sites: {}\n", sites_store.sites.len().to_string().bold());

    Ok(())
}

