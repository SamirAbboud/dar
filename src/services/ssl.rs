use colored::*;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};


/// Ensures the isolated `mkcert` binary exists in `~/.dar/bin/` using the specified version from config,
/// and executes `mkcert -install` to establish local Root CA trust.
pub fn ensure_mkcert(dar_dir: &Path, version: &str) -> Result<PathBuf, String> {
    let bin_dir = dar_dir.join("bin");
    if !bin_dir.exists() {
        fs::create_dir_all(&bin_dir)
            .map_err(|e| format!("Failed to create bin directory: {}", e))?;
    }

    let mkcert_bin = bin_dir.join("mkcert");

    if !mkcert_bin.exists() {
        println!("🔐 Provisioning isolated mkcert binary (v{})...", version);

        let download_url = format!(
            "https://github.com/FiloSottile/mkcert/releases/download/v{}/mkcert-v{}-linux-amd64",
            version, version
        );

        let response = ureq::get(&download_url)
            .call()
            .map_err(|e| format!("Failed to download mkcert: {}", e))?;

        {
            let mut reader = response.into_body().into_reader();
            let mut out = File::create(&mkcert_bin)
                .map_err(|e| format!("Failed to create mkcert binary file: {}", e))?;

            io::copy(&mut reader, &mut out)
                .map_err(|e| format!("Failed to save mkcert binary: {}", e))?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&mkcert_bin)
                .map_err(|e| format!("Failed to read metadata for mkcert: {}", e))?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&mkcert_bin, perms)
                .map_err(|e| format!("Failed to set permissions on mkcert binary: {}", e))?;
        }

        println!("{}", "🔑 Installing local Root CA trust store via mkcert...".cyan());
        let status = Command::new(&mkcert_bin)
            .arg("-install")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| format!("Failed to execute mkcert -install: {}", e))?;

        if !status.success() {
            return Err("Failed to install mkcert Root CA trust store.".to_string());
        }
    }

    Ok(mkcert_bin)
}
