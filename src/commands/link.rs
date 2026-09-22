use crate::services::nginx::{create_site_config, ensure_hosts_entry, is_site_linked, restart_nginx};
use crate::commands::park::park_folder;
use crate::services::ssl::generate_certificate;
use colored::*;
use std::env;
use std::fs;
use std::path::PathBuf;

/// Links a directory to a local `.test` domain.
/// 
/// Accepts optional `--name`, `--path`, and `--secure` overrides.
pub fn run(name: Option<String>, path: Option<String>, secure: bool) -> Result<(), String> {
    let current_dir = env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;

    let target_path = match path {
        Some(p) => {
            let custom_path = PathBuf::from(p);
            if !custom_path.exists() {
                return Err(format!("The path '{}' does not exist.", custom_path.display()));
            }
            custom_path.canonicalize().map_err(|e| format!("Path resolution failed: {}", e))?
        }
        None => current_dir,
    };

    let app_name = match name {
        Some(n) if !n.trim().is_empty() => n.trim().to_string(),
        _ => target_path
            .file_name()
            .ok_or_else(|| "Invalid target directory name".to_string())?
            .to_string_lossy()
            .to_string(),
    };

    let home = dirs::home_dir()
        .ok_or_else(|| "Could not determine home directory".to_string())?;
    let dar_dir = home.join(".dar");
    let nginx_bin = dar_dir.join("nginx").join("sbin").join("nginx");
    let mkcert_bin = dar_dir.join("bin").join("mkcert");

    // Correct path: ~/.dar/sites/<app_name>.conf
    let config_path = dar_dir.join("sites").join(format!("{}.conf", app_name));
    let already_linked = is_site_linked(&dar_dir, &app_name);

    if already_linked {
        let is_already_secured = match fs::read_to_string(&config_path) {
            Ok(content) => content.contains("ssl_certificate") || content.contains("listen 443"),
            Err(_) => false,
        };

        // 1. If site is already HTTPS
        if is_already_secured {
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

    // 3. Ensure local DNS entry exists
    ensure_hosts_entry(&app_name)?;

    // 4. Auto-park parent folder if not already parked
    if let Some(parent_dir) = target_path.parent() {
        if let Ok(was_added) = park_folder(&dar_dir, parent_dir) {
            if was_added {
                println!(
                    "{}",
                    format!("  📂 Auto-parked parent folder: {}", parent_dir.display()).dimmed()
                );
            }
        }
    }

    // 5. Reload Nginx
    restart_nginx(&nginx_bin, &dar_dir)?;

    let protocol = if secure { "https" } else { "http" };
    let action_msg = if already_linked && secure {
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
