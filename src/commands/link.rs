use crate::services::nginx::{create_site_config, ensure_hosts_entry, restart_nginx};
use crate::commands::park::park_folder;
use crate::services::ssl::generate_certificate;
use crate::utils::paths::{resolve_app_name, resolve_target_path};
use crate::store::SitesStore;
use crate::store::site::Site;
use colored::*;

/// Links a directory to a local `.test` domain.
pub fn run(name: Option<String>, path: Option<String>, secure: bool) -> Result<(), String> {
    let target_path = resolve_target_path(path)?;
    let app_name = resolve_app_name(name, &target_path)?;

    let home = dirs::home_dir().ok_or_else(|| "Could not determine home directory".to_string())?;
    let dar_dir = home.join(".dar");
    let nginx_bin = dar_dir.join("nginx").join("sbin").join("nginx");
    let mkcert_bin = dar_dir.join("bin").join("mkcert");

    // Load store
    let mut store = SitesStore::load(&dar_dir)?;
    let existing_site = store.get(&app_name).cloned();

    if let Some(ref site) = existing_site {
        // 1. If site is already HTTPS
        if site.secured {
            println!(
                "\n{} Site '{}' is already linked and secured ({})\n",
                "ℹ".cyan(),
                app_name.bold(),
                format!("https://{}.test", app_name).underline()
            );
            return Ok(());
        }

        // 2. If site is HTTP and user ran `link` without `--secure`
        if !secure {
            println!(
                "\n{} Site '{}' is already linked ({})\n",
                "ℹ".cyan(),
                app_name.bold(),
                format!("http://{}.test", app_name).underline()
            );
            return Ok(());
        }

        // 3. If site is HTTP and user ran `secure` / `link -s` -> Upgrade to HTTPS
        println!(
            "\n{}",
            format!("🔒 Securing existing site: {}", app_name.bold()).cyan()
        );
    } else {
        println!("\n{}", format!("🔗 Linking site: {}", app_name.bold()).cyan());
    }

    // 1. Generate SSL certificate if securing
    if secure {
        println!("{}", "🔑 Generating SSL certificates via mkcert...".dimmed());
        generate_certificate(&dar_dir, &app_name, &mkcert_bin)?;
    }

    // 2. Generate/overwrite Nginx virtual host configuration
    create_site_config(&dar_dir, &app_name, &target_path, secure)?;

    // 3. Save / Update site entry in store
    let site_record = Site::new(&app_name, target_path.clone(), secure);
    store.add(site_record)?;

    // 4. Ensure local DNS entry exists
    ensure_hosts_entry(&app_name)?;

    // 5. Auto-park parent folder if not already parked
    if let Some(parent_dir) = target_path.parent() {
        if let Ok(was_added) = park_folder(&dar_dir, parent_dir) {
            if was_added {
                println!(
                    "{}",
                    format!("📂 Auto-parked parent folder: {}", parent_dir.display()).dimmed()
                );
            }
        }
    }

    // 6. Reload Nginx
    restart_nginx(&nginx_bin, &dar_dir)?;

    let protocol = if secure { "https" } else { "http" };
    let action_msg = if existing_site.is_some() && secure {
        "Site secured"
    } else {
        "Site linked"
    };

    println!(
        "\n{} {}: {} -> {}\n",
        "✅".green(),
        action_msg,
        format!("{}://{}.test", protocol, app_name).bold().underline(),
        target_path.display().to_string().dimmed()
    );

    Ok(())
}
