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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicesConfig {
    pub services: HashMap<String, ServiceTemplate>,
}

impl Default for ServicesConfig {
    fn default() -> Self {
        let mut services = HashMap::new();

        services.insert(
            "mysql".to_string(),
            ServiceTemplate {
                key: "mysql".to_string(),
                name: "MySQL Database".to_string(),
                image: "mysql:8.0".to_string(),
                default_port: 3306,
                container_port: 3306,
                additional_ports: HashMap::new(),
                env_vars: HashMap::from([
                    ("MYSQL_ROOT_PASSWORD".to_string(), "root".to_string()),
                    ("MYSQL_ALLOW_EMPTY_PASSWORD".to_string(), "yes".to_string()),
                ]),
                volume_mounts: HashMap::from([("data".to_string(), "/var/lib/mysql".to_string())]),
            },
        );

        services.insert(
            "postgres".to_string(),
            ServiceTemplate {
                key: "postgres".to_string(),
                name: "PostgreSQL Database".to_string(),
                image: "postgres:16-alpine".to_string(),
                default_port: 5432,
                container_port: 5432,
                additional_ports: HashMap::new(),
                env_vars: HashMap::from([("POSTGRES_PASSWORD".to_string(), "postgres".to_string())]),
                volume_mounts: HashMap::from([("data".to_string(), "/var/lib/postgresql/data".to_string())]),
            },
        );

        services.insert(
            "redis".to_string(),
            ServiceTemplate {
                key: "redis".to_string(),
                name: "Redis In-Memory Data Store".to_string(),
                image: "redis:7-alpine".to_string(),
                default_port: 6379,
                container_port: 6379,
                additional_ports: HashMap::new(),
                env_vars: HashMap::new(),
                volume_mounts: HashMap::from([("data".to_string(), "/data".to_string())]),
            },
        );

        services.insert(
            "mailpit".to_string(),
            ServiceTemplate {
                key: "mailpit".to_string(),
                name: "Mailpit SMTP & Web Mail Inspector".to_string(),
                image: "axllent/mailpit:latest".to_string(),
                default_port: 8025,
                container_port: 8025,
                additional_ports: HashMap::from([(1025, 1025)]),
                env_vars: HashMap::new(),
                volume_mounts: HashMap::from([("data".to_string(), "/data".to_string())]),
            },
        );

        services.insert(
            "meilisearch".to_string(),
            ServiceTemplate {
                key: "meilisearch".to_string(),
                name: "Meilisearch Search Engine".to_string(),
                image: "getmeili/meilisearch:v1.6".to_string(),
                default_port: 7700,
                container_port: 7700,
                additional_ports: HashMap::new(),
                env_vars: HashMap::from([("MEILI_MASTER_KEY".to_string(), "masterKey".to_string())]),
                volume_mounts: HashMap::from([("data".to_string(), "/meili_data".to_string())]),
            },
        );

        Self { services }
    }
}

/// Loads templates from `~/.dar/templates.toml`. Creates default file if missing.
pub fn load_templates(dar_dir: &Path) -> Result<HashMap<String, ServiceTemplate>, String> {
    let config_path = dar_dir.join("templates.toml");

    if !config_path.exists() {
        let default_config = ServicesConfig::default();
        let serialized = toml::to_string_pretty(&default_config)
            .map_err(|e| format!("Failed to serialize default templates config: {}", e))?;
        fs::write(&config_path, serialized)
            .map_err(|e| format!("Failed to write templates.toml: {}", e))?;
        return Ok(default_config.services);
    }

    let content = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read templates.toml: {}", e))?;

    let config: ServicesConfig = toml::from_str(&content)
        .map_err(|e| format!("Failed to parse templates.toml: {}", e))?;

    Ok(config.services)
}
