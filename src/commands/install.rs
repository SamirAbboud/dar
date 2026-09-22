use colored::*;

use crate::config::load_or_create_config;
use crate::services::nginx::{build_and_install_nginx, create_default_nginx_config, restart_nginx, set_nginx_capabilities};
use crate::services::php::{ensure_php_fpm_running, fetch_isolated_php};
use crate::services::ssl::ensure_mkcert;
use crate::utils::system::setup_directories;

use crate::services::dependencies::{
    ensure_nginx_source, ensure_openssl_source, ensure_pcre2_source, ensure_zlib_source,
};
use crate::services::composer::ensure_composer;

/// Initializes the ~/.dar directory structure, builds isolated dependencies 
/// (Nginx, PHP-FPM, Composer), and configures initial environment settings.
pub fn run() -> Result<(), String> {
    println!("{}", "🚀 Initializing Dar environment setup...".cyan().bold());

    let home_dir = dirs::home_dir().ok_or_else(|| "Could not locate home directory".to_string())?;
    let dar_dir = home_dir.join(".dar");

    setup_directories(&dar_dir)?;

    let cfg = load_or_create_config(&dar_dir)?;

    let nginx_install_dir = dar_dir.join("nginx");
    let nginx_bin = nginx_install_dir.join("sbin").join("nginx");

    if !nginx_bin.exists() {
        println!("\n{}", "📦 Building isolated Nginx binary...".yellow().bold());
        let src_dir = dar_dir.join("src");

        let pcre_folder = ensure_pcre2_source(&src_dir, &cfg.pcre2)?;
        let openssl_folder = ensure_openssl_source(&src_dir, &cfg.openssl)?;
        let zlib_folder = ensure_zlib_source(&src_dir, &cfg.zlib)?;
        let nginx_src_folder = ensure_nginx_source(&src_dir, &cfg.nginx)?;

        build_and_install_nginx(
            &nginx_src_folder,
            &pcre_folder,
            &openssl_folder,
            &zlib_folder,
            &nginx_install_dir,
            &dar_dir,
        )?;
    }

    set_nginx_capabilities(&nginx_bin)?;
    ensure_mkcert(&dar_dir, &cfg.mkcert)?;
    fetch_isolated_php(&dar_dir, &cfg.default_php)?;
    ensure_composer(&dar_dir)?;
    create_default_nginx_config(&dar_dir)?;
    ensure_php_fpm_running(&dar_dir)?;
    restart_nginx(&nginx_bin, &dar_dir)?;

    println!(
        "\n{}",
        "✨ Dar environment successfully installed in ~/.dar!".green().bold()
    );

    Ok(())
}
