use colored::*;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::utils::archive::resolve_build_dir;
use crate::utils::system::get_available_compiler;

/// Builds and installs Nginx from source.
pub fn build_and_install_nginx(
    src_folder_raw: &Path,
    pcre_folder_raw: &Path,
    openssl_folder_raw: &Path,
    zlib_folder_raw: &Path,
    install_dir: &Path,
    dar_dir: &Path,
) -> Result<(), String> {
    let src_folder = resolve_build_dir(src_folder_raw);
    let pcre_folder = resolve_build_dir(pcre_folder_raw);
    let openssl_folder = resolve_build_dir(openssl_folder_raw);
    let zlib_folder = resolve_build_dir(zlib_folder_raw);

    let cc = get_available_compiler();
    let num_jobs = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);

    println!(" Configuring Nginx build parameters (using CC={})...", cc);

    let linux_config_path = src_folder.join("src").join("os").join("unix").join("ngx_linux_config.h");
    if linux_config_path.exists() {
        if let Ok(content) = fs::read_to_string(&linux_config_path) {
            if content.contains("#include <crypt.h>") {
                let patched = content.replace("#include <crypt.h>", "// #include <crypt.h>");
                let _ = fs::write(&linux_config_path, patched);
            }
        }
    }

    let configure_bin = src_folder.join("configure");

    let configure_status = Command::new(configure_bin)
        .current_dir(&src_folder)
        .env("CC", &cc)
        .arg(format!("--with-cc={}", cc))
        .arg(format!("--prefix={}", install_dir.display()))
        .arg(format!(
            "--error-log-path={}",
            dar_dir.join("logs").join("error.log").display()
        ))
        .arg(format!(
            "--http-log-path={}",
            dar_dir.join("logs").join("access.log").display()
        ))
        .arg(format!("--with-pcre={}", pcre_folder.display()))
        .arg("--with-pcre-jit")
        .arg(format!("--with-openssl={}", openssl_folder.display()))
        .arg(format!("--with-openssl-opt=CC={}", cc))
        .arg(format!("--with-zlib={}", zlib_folder.display()))
        .arg("--with-http_ssl_module")
        .arg("--with-http_v2_module")
        .arg("--without-http_auth_basic_module")
        .arg("--with-cc-opt=-Wno-error")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("Failed to execute ./configure: {}", e))?;

    if !configure_status.success() {
        return Err("Nginx configuration failed.".to_string());
    }

    println!(" Compiling Nginx along with static PCRE2, OpenSSL & zlib...");
    let make_status = Command::new("make")
        .arg(format!("-j{}", num_jobs))
        .current_dir(&src_folder)
        .env("CC", &cc)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("Failed to run make: {}", e))?;

    if !make_status.success() {
        return Err("Nginx compilation (make) failed.".to_string());
    }

    println!(" Installing Nginx (make install)...");
    let install_status = Command::new("make")
        .arg("install")
        .current_dir(&src_folder)
        .env("CC", &cc)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("Failed to run make install: {}", e))?;

    if !install_status.success() {
        return Err("Nginx installation (make install) failed.".to_string());
    }

    println!(" {}", "Nginx successfully built and installed!".green());
    Ok(())
}

/// Grants the Nginx binary capability to bind to low-numbered privileged ports (port 80)
pub fn set_nginx_capabilities(nginx_bin: &Path) -> Result<(), String> {
    let status = Command::new("sudo")
        .arg("setcap")
        .arg("cap_net_bind_service=+ep")
        .arg(nginx_bin)
        .status()
        .map_err(|e| format!("Failed to execute setcap: {}", e))?;

    if !status.success() {
        return Err("Failed to grant port 80 binding privileges to Nginx binary.".to_string());
    }

    Ok(())
}
