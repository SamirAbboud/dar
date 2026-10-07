use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use crate::services::composer::ensure_composer;

use colored::*;

/// Ensures `laravel/installer` is globally installed inside `~/.dar/composer/`
/// and writes the wrapper executable at `~/.dar/bin/laravel`.
pub fn ensure_laravel(dar_dir: &Path) -> Result<(), String> {
    // 1. Ensure isolated composer wrapper and binary exist
    ensure_composer(dar_dir)?;

    let composer_home = dar_dir.join("composer");
    let global_vendor_bin = composer_home.join("vendor").join("bin");
    let laravel_bin = global_vendor_bin.join("laravel");
    let laravel_wrapper = dar_dir.join("bin").join("laravel");

    // 2. If the vendor binary doesn't exist yet, install it via isolated Composer
    if !laravel_bin.exists() {
        println!("📦 Provisioning global Laravel installer via isolated Composer...");

        let composer_wrapper = dar_dir.join("bin").join("composer");

        let status = Command::new(&composer_wrapper)
            .env("COMPOSER_HOME", &composer_home)
            .args(["global", "require", "laravel/installer"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| format!("Failed to run composer global require: {}", e))?;

        if !status.success() {
            return Err("Failed to install laravel/installer via Composer.".to_string());
        }
    }

    // 3. Write wrapper script pointing to isolated PHP runtime + global vendor binary
    let wrapper_script = format!(
        "#!/bin/sh\nexport COMPOSER_HOME=\"{}\"\nexec \"{}\" \"{}\" \"$@\"\n",
        composer_home.display(),
        dar_dir.join("php").join("current").join("bin").join("php").display(),
        laravel_bin.display()
    );

    fs::write(&laravel_wrapper, wrapper_script)
        .map_err(|e| format!("Failed to create laravel wrapper script: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&laravel_wrapper)
            .map_err(|e| format!("Failed to read metadata for laravel wrapper: {}", e))?
            .permissions();
        perms.set_mode(0o755);

        fs::set_permissions(&laravel_wrapper, perms)
            .map_err(|e| format!("Failed to set permissions on laravel wrapper: {}", e))?;
    }

    Ok(())
}

/// Updates `laravel/installer` inside `~/.dar/composer/` using isolated Composer.
pub fn update_laravel(dar_dir: &Path) -> Result<(), String> {
    ensure_laravel(dar_dir)?;
    println!("{}", "🔄 Updating Laravel Installer...".cyan().bold());

    let composer_wrapper = dar_dir.join("bin").join("composer");
    let composer_home = dar_dir.join("composer");

    let output = Command::new(&composer_wrapper)
        .env("COMPOSER_HOME", &composer_home)
        .args(["global", "require", "laravel/installer:*", "-W"])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("Failed to execute composer update: {}", e))?;

    if output.status.success() {
        println!("{}", "✨ Laravel Installer updated successfully!".green().bold());
        Ok(())
    } else {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        Err(format!("Failed to update Laravel Installer:\n{}", err_msg))
    }
}
