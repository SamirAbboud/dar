use colored::*;

use crate::config::load_or_create_config;
use crate::services::nginx::{build_and_install_nginx, create_default_nginx_config, restart_nginx};
use crate::services::php::{ensure_php_fpm_running, fetch_isolated_php};
use crate::utils::archive::fetch_and_extract;
use crate::utils::system::setup_directories;

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

        let pcre_folder = fetch_and_extract(
            &src_dir,
            "PCRE2",
            &format!("pcre2-{}", cfg.pcre2),
            &format!("pcre2-{}.tar.gz", cfg.pcre2),
            &format!(
                "https://github.com/PCRE2Project/pcre2/releases/download/pcre2-{}/pcre2-{}.tar.gz",
                cfg.pcre2, cfg.pcre2
            ),
        )?;

        let openssl_folder = fetch_and_extract(
            &src_dir,
            "OpenSSL",
            &format!("openssl-{}", cfg.openssl),
            &format!("openssl-{}.tar.gz", cfg.openssl),
            &format!(
                "https://github.com/openssl/openssl/releases/download/openssl-{}/openssl-{}.tar.gz",
                cfg.openssl, cfg.openssl
            ),
        )?;

        let zlib_folder = fetch_and_extract(
            &src_dir,
            "zlib",
            &format!("zlib-{}", cfg.zlib),
            &format!("zlib-{}.tar.gz", cfg.zlib),
            &format!(
                "https://github.com/madler/zlib/releases/download/v{}/zlib-{}.tar.gz",
                cfg.zlib, cfg.zlib
            ),
        )?;

        let nginx_src_folder = fetch_and_extract(
            &src_dir,
            "Nginx",
            &format!("nginx-{}", cfg.nginx),
            &format!("nginx-{}.tar.gz", cfg.nginx),
            &format!("https://nginx.org/download/nginx-{}.tar.gz", cfg.nginx),
        )?;

        build_and_install_nginx(
            &nginx_src_folder,
            &pcre_folder,
            &openssl_folder,
            &zlib_folder,
            &nginx_install_dir,
            &dar_dir,
        )?;
    }

    fetch_isolated_php(&dar_dir, &cfg.default_php)?;
    create_default_nginx_config(&dar_dir)?;
    ensure_php_fpm_running(&dar_dir)?;
    restart_nginx(&nginx_bin, &dar_dir)?;

    println!(
        "\n{}",
        "✨ Dar environment successfully installed in ~/.dar!".green().bold()
    );

    Ok(())
}
