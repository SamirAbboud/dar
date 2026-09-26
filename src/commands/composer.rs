use std::process::{Command, Stdio};
use crate::services::composer::ensure_composer;

/// Executes the local Composer wrapper script with all supplied arguments.
pub fn run(args: &[String]) -> Result<(), String> {
    let home = dirs::home_dir().ok_or_else(|| "Could not determine home directory".to_string())?;
    let dar_dir = home.join(".dar");

    // Make sure composer.phar and wrapper script are present and up to date
    ensure_composer(&dar_dir)?;

    let composer_wrapper = dar_dir.join("bin").join("composer");

    let status = Command::new(&composer_wrapper)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| format!("Failed to execute composer wrapper: {}", e))?;

    if !status.success() {
        return Err(format!(
            "Composer process exited with status code: {:?}",
            status.code()
        ));
    }

    Ok(())
}
