use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ServiceTemplate {
    pub key: &'static str,
    pub name: &'static str,
    pub image: &'static str,
    pub default_port: u16,
    pub container_port: u16,
    pub env_vars: Vec<(&'static str, &'static str)>,
    pub volume_mounts: Vec<(&'static str, &'static str)>, // (relative host dir, container path)
}

pub fn get_templates() -> HashMap<&'static str, ServiceTemplate> {
    let mut m = HashMap::new();

    m.insert(
        "mysql",
        ServiceTemplate {
            key: "mysql",
            name: "MySQL Database",
            image: "mysql:8.0",
            default_port: 3306,
            container_port: 3306,
            env_vars: vec![
                ("MYSQL_ROOT_PASSWORD", "root"),
                ("MYSQL_ALLOW_EMPTY_PASSWORD", "yes"),
            ],
            volume_mounts: vec![("data", "/var/lib/mysql")],
        },
    );

    m.insert(
        "postgres",
        ServiceTemplate {
            key: "postgres",
            name: "PostgreSQL Database",
            image: "postgres:16-alpine",
            default_port: 5432,
            container_port: 5432,
            env_vars: vec![("POSTGRES_PASSWORD", "postgres")],
            volume_mounts: vec![("data", "/var/lib/postgresql/data")],
        },
    );

    m.insert(
        "redis",
        ServiceTemplate {
            key: "redis",
            name: "Redis In-Memory Data Store",
            image: "redis:7-alpine",
            default_port: 6379,
            container_port: 6379,
            env_vars: vec![],
            volume_mounts: vec![("data", "/data")],
        },
    );

    m.insert(
        "mailpit",
        ServiceTemplate {
            key: "mailpit",
            name: "Mailpit SMTP & Web Mail Inspector",
            image: "axllent/mailpit:latest",
            default_port: 8025,
            container_port: 8025,
            env_vars: vec![],
            volume_mounts: vec![("data", "/data")],
        },
    );

    m.insert(
        "meilisearch",
        ServiceTemplate {
            key: "meilisearch",
            name: "Meilisearch Search Engine",
            image: "getmeili/meilisearch:v1.6",
            default_port: 7700,
            container_port: 7700,
            env_vars: vec![("MEILI_MASTER_KEY", "masterKey")],
            volume_mounts: vec![("data", "/meili_data")],
        },
    );

    m
}
