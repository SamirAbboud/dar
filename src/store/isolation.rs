use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub struct IsolationsStore {
    file_path: PathBuf,
    /// Maps project path string -> PHP version string (e.g. "/home/samir/Code/Sites/larademo" -> "8.3")
    pub mappings: HashMap<String, String>,
}

impl IsolationsStore {
    /// Loads the isolations store from `~/.dar/isolations.json`.
    pub fn load(dar_dir: &Path) -> Result<Self, String> {
        let file_path = dar_dir.join("isolations.json");

        if !file_path.exists() {
            return Ok(Self {
                file_path,
                mappings: HashMap::new(),
            });
        }

        let content = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read isolations store: {}", e))?;

        let mappings: HashMap<String, String> = serde_json::from_str(&content)
            .unwrap_or_default();

        Ok(Self { file_path, mappings })
    }

    /// Saves the mappings to `~/.dar/isolations.json`.
    pub fn save(&self) -> Result<(), String> {
        let content = serde_json::to_string_pretty(&self.mappings)
            .map_err(|e| format!("Failed to serialize isolations store: {}", e))?;

        fs::write(&self.file_path, content)
            .map_err(|e| format!("Failed to write isolations store: {}", e))
    }

    /// Records or updates an isolation entry for a project directory path.
    pub fn set_isolation(&mut self, project_path: &Path, version: &str) -> Result<(), String> {
        let path_str = project_path
            .canonicalize()
            .unwrap_or_else(|_| project_path.to_path_buf())
            .display()
            .to_string();

        self.mappings.insert(path_str, version.to_string());
        self.save()
    }

    /// Gets the isolated version for a given project path.
    pub fn get_isolation(&self, project_path: &Path) -> Option<&String> {
        let path_str = project_path
            .canonicalize()
            .unwrap_or_else(|_| project_path.to_path_buf())
            .display()
            .to_string();

        self.mappings.get(&path_str)
    }

    #[allow(dead_code)]
    /// Removes an isolation mapping for a project path.
    pub fn remove_isolation(&mut self, project_path: &Path) -> Result<Option<String>, String> {
        let path_str = project_path
            .canonicalize()
            .unwrap_or_else(|_| project_path.to_path_buf())
            .display()
            .to_string();

        let removed = self.mappings.remove(&path_str);
        if removed.is_some() {
            self.save()?;
        }
        Ok(removed)
    }
}
