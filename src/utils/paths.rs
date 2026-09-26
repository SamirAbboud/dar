use std::env;
use std::path::{Path, PathBuf};

/// Returns the central `~/.dar` configuration directory.
pub fn get_dar_dir() -> Result<PathBuf, String> {
    let home_dir = dirs::home_dir().ok_or_else(|| "Could not locate home directory".to_string())?;
    Ok(home_dir.join(".dar"))
}

/// Resolves target directory from an optional CLI path argument or current working directory.
pub fn resolve_target_path(path_arg: Option<String>) -> Result<PathBuf, String> {
    match path_arg {
        Some(p) => {
            let custom_path = PathBuf::from(p);
            if !custom_path.exists() {
                return Err(format!("The path '{}' does not exist.", custom_path.display()));
            }
            custom_path
                .canonicalize()
                .map_err(|e| format!("Path resolution failed: {}", e))
        }
        None => env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e)),
    }
}

/// Resolves app name from an optional CLI name argument or target directory name.
pub fn resolve_app_name(name_arg: Option<String>, target_path: &Path) -> Result<String, String> {
    match name_arg {
        Some(n) if !n.trim().is_empty() => Ok(n.trim().to_string()),
        _ => target_path
            .file_name()
            .ok_or_else(|| "Invalid target directory name".to_string())
            .map(|s| s.to_string_lossy().to_string()),
    }
}
