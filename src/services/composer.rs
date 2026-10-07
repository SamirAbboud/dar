use std::fs::{self, File};
use std::io;
use std::path::Path;
use std::process::Command;

use colored::*;

/// Downloads the latest stable `composer.phar` into `~/.dar/bin/` if missing 
/// and generates an executable shell wrapper script pointing to the isolated PHP runtim
pub fn ensure_composer(dar_dir: &Path) -> Result<(), String> {
    let bin_dir = dar_dir.join("bin");
    if !bin_dir.exists() {
        fs::create_dir_all(&bin_dir)
            .map_err(|e| format!("Failed to create bin directory: {}", e))?;
    }

    let composer_phar = bin_dir.join("composer.phar");
    let composer_wrapper = bin_dir.join("composer");

    if !composer_phar.exists() {
        println!("📦 Provisioning isolated Composer binary...");

        let download_url = "https://getcomposer.org/download/latest-stable/composer.phar";

        let response = ureq::get(download_url)
            .call()
            .map_err(|e| format!("Failed to download Composer: {}", e))?;

        let mut reader = response.into_body().into_reader();
        let mut out = File::create(&composer_phar)
            .map_err(|e| format!("Failed to create composer.phar: {}", e))?;

        io::copy(&mut reader, &mut out)
            .map_err(|e| format!("Failed to save composer binary: {}", e))?;
    }

    // Always ensure the wrapper points to ~/.dar/php/current/bin/php
    let wrapper_script = format!(
        "#!/bin/sh\nexec \"{}\" \"{}\" \"$@\"\n",
        dar_dir.join("php").join("current").join("bin").join("php").display(),
        composer_phar.display()
    );

    fs::write(&composer_wrapper, wrapper_script)
        .map_err(|e| format!("Failed to create composer wrapper: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&composer_wrapper)
            .map_err(|e| format!("Failed to read metadata for composer wrapper: {}", e))?
            .permissions();
        perms.set_mode(0o755);

        fs::set_permissions(&composer_wrapper, perms)
            .map_err(|e| format!("Failed to set permissions on composer wrapper: {}", e))?;
    }

    Ok(())
}

/// Updates Composer using its native `self-update` command.
pub fn update_composer(dar_dir: &Path) -> Result<(), String> {
    ensure_composer(dar_dir)?;
    println!("{}", "🔄 Updating Composer...".cyan().bold());

    let composer_wrapper = dar_dir.join("bin").join("composer");

    let status = Command::new(&composer_wrapper)
        .arg("self-update")
        .status()
        .map_err(|e| format!("Failed to run composer self-update: {}", e))?;

    if status.success() {
        println!("{}", "✨ Composer updated successfully!".green().bold());
        Ok(())
    } else {
        Err("Failed to update Composer.".to_string())
    }
}
