use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Stores pinned upstream dependency versions for Nginx, C libraries, and PHP.
#[derive(Debug, Serialize, Deserialize)]
pub struct VersionsConfig {
    pub nginx: String,
    pub pcre2: String,
    pub openssl: String,
    pub zlib: String,
    pub default_php: String,
    pub mkcert: String,
}

impl Default for VersionsConfig {
    fn default() -> Self {
        Self {
            nginx: "1.26.2".to_string(),
            pcre2: "10.44".to_string(),
            openssl: "3.0.15".to_string(),
            zlib: "1.3.1".to_string(),
            default_php: "8.3.11".to_string(),
            mkcert: "1.4.4".to_string(),
        }
    }
}


/// Reads `~/.dar/versions.toml` if present, or creates and saves a new config 
/// initialized with default dependency versions.
pub fn load_or_create_config(dar_dir: &Path) -> Result<VersionsConfig, String> {
    let config_path = dar_dir.join("versions.toml");

    if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read versions.toml: {}", e))?;
        toml::from_str(&content).map_err(|e| format!("Failed to parse versions.toml: {}", e))
    } else {
        let default_cfg = VersionsConfig::default();
        let content = toml::to_string_pretty(&default_cfg)
            .map_err(|e| format!("Failed to serialize default config: {}", e))?;
        fs::write(&config_path, content)
            .map_err(|e| format!("Failed to write versions.toml: {}", e))?;
        Ok(default_cfg)
    }
}
