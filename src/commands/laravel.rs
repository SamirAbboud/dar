use std::process::{Command, Stdio};
use crate::services::laravel::ensure_laravel;
use crate::utils::paths::get_dar_dir;

/// Executes the local Laravel installer wrapper script with all supplied arguments.
pub fn run(args: &[String]) -> Result<(), String> {
    let dar_dir = get_dar_dir()?;

    // Make sure laravel.phar and wrapper script are present and up to date
    ensure_laravel(&dar_dir)?;

    let laravel_wrapper = dar_dir.join("bin").join("laravel");

    let status = Command::new(&laravel_wrapper)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| format!("Failed to execute laravel wrapper: {}", e))?;

    if !status.success() {
        return Err(format!(
            "Laravel process exited with status code: {:?}",
            status.code()
        ));
    }

    Ok(())
}
