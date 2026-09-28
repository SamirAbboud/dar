use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceTemplate {
    pub key: String,
    pub name: String,
    pub image: String,
    pub default_port: u16,
    pub container_port: u16,
    #[serde(default)]
    pub additional_ports: HashMap<u16, u16>,
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
    #[serde(default)]
    pub volume_mounts: HashMap<String, String>,
    #[serde(default)]
    pub command: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicesConfig {
    pub services: HashMap<String, ServiceTemplate>,
}

const DEFAULT_TEMPLATES_TOML: &str = r#"[services.mysql]
key = "mysql"
name = "MySQL Database"
image = "mysql:8.0"
default_port = 3306
container_port = 3306
additional_ports = {}
env_vars = { MYSQL_ROOT_PASSWORD = "root", MYSQL_ALLOW_EMPTY_PASSWORD = "yes" }
volume_mounts = { data = "/var/lib/mysql" }

[services.postgres]
key = "postgres"
name = "PostgreSQL Database"
image = "postgres:16-alpine"
default_port = 5432
container_port = 5432
additional_ports = {}
env_vars = { POSTGRES_PASSWORD = "postgres" }
volume_mounts = { data = "/var/lib/postgresql/data" }

[services.redis]
key = "redis"
name = "Redis In-Memory Data Store"
image = "redis:7-alpine"
default_port = 6379
container_port = 6379
additional_ports = {}
env_vars = {}
volume_mounts = { data = "/data" }

[services.mailpit]
key = "mailpit"
name = "Mailpit SMTP & Web Mail Inspector"
image = "axllent/mailpit:latest"
default_port = 8025
container_port = 8025
additional_ports = { 1025 = 1025 }
env_vars = {}
volume_mounts = { data = "/data" }

[services.meilisearch]
key = "meilisearch"
name = "Meilisearch Search Engine"
image = "getmeili/meilisearch:v1.6"
default_port = 7700
container_port = 7700
additional_ports = {}
env_vars = { MEILI_MASTER_KEY = "masterKey" }
volume_mounts = { data = "/meili_data" }
"#;

/// Loads templates from `~/.dar/templates.toml`. Creates default file if missing.
pub fn load_templates(dar_dir: &Path) -> Result<HashMap<String, ServiceTemplate>, String> {
    let config_path = dar_dir.join("templates.toml");

    if !config_path.exists() {
        fs::write(&config_path, DEFAULT_TEMPLATES_TOML)
            .map_err(|e| format!("Failed to write templates.toml: {}", e))?;
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read templates.toml: {}", e))?;

    let config: ServicesConfig = toml::from_str(&content)
        .map_err(|e| format!("Failed to parse templates.toml: {}", e))?;

    Ok(config.services)
}
