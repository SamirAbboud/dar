use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use chrono::Local;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Site {
    pub name: String,
    pub path: PathBuf,
    pub secured: bool,
    pub created_at: String,
}

impl Site {
    pub fn new(name: impl Into<String>, path: PathBuf, secured: bool) -> Self {
        Self {
            name: name.into(),
            path,
            secured,
            created_at: Local::now().to_rfc3339(),
        }
    }

    pub fn set_secured(&mut self, secured: bool) {
        self.secured = secured;
    }
}
