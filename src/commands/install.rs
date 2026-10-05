use colored::*;

use crate::config::load_or_create_config;
use crate::services::nginx::{build_and_install_nginx, create_default_nginx_config, set_nginx_capabilities, restart as restart_nginx};
use crate::services::php::{ensure_php_version, set_global_php_version};
use crate::services::ssl::ensure_mkcert;
use crate::utils::system::setup_directories;

use crate::services::dependencies::{
    ensure_nginx_source, ensure_openssl_source, ensure_pcre2_source, ensure_zlib_source,
};
use crate::services::composer::ensure_composer;
use crate::utils::paths::get_dar_dir;

/// Initializes the ~/.dar directory structure, builds isolated dependencies 
/// (Nginx, PHP-FPM, Composer), and configures initial environment settings.
pub fn run() -> Result<(), String> {
    println!("{}", "🚀 Initializing Dar environment setup...".cyan().bold());
    let dar_dir = get_dar_dir()?;

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
    let php_version = ensure_php_version(&dar_dir, &cfg.default_php)?;
    set_global_php_version(&dar_dir, &php_version)?;
    ensure_composer(&dar_dir)?;
    create_default_nginx_config(&dar_dir)?;
    restart_nginx(&dar_dir)?;

    println!(
        "\n{}",
        "✨ Dar environment successfully installed in ~/.dar!".green().bold()
    );

    Ok(())
}
