pub mod site;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use site::Site;

pub struct SitesStore {
    file_path: PathBuf,
    pub sites: HashMap<String, Site>,
}

impl SitesStore {
    /// Loads the sites store from `~/.dar/sites.json`.
    pub fn load(dar_dir: &Path) -> Result<Self, String> {
        let file_path = dar_dir.join("sites.json");

        if !file_path.exists() {
            return Ok(Self {
                file_path,
                sites: HashMap::new(),
            });
        }

        let content = fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read sites store: {}", e))?;

        let sites: HashMap<String, Site> = serde_json::from_str(&content)
            .unwrap_or_default();

        Ok(Self { file_path, sites })
    }

    /// Saves the current store to `~/.dar/sites.json`.
    pub fn save(&self) -> Result<(), String> {
        let content = serde_json::to_string_pretty(&self.sites)
            .map_err(|e| format!("Failed to serialize sites store: {}", e))?;

        fs::write(&self.file_path, content)
            .map_err(|e| format!("Failed to write sites store: {}", e))
    }

    /// Adds or updates a site in the store.
    pub fn add(&mut self, site: Site) -> Result<(), String> {
        self.sites.insert(site.name.clone(), site);
        self.save()
    }

    /// Gets a site by name.
    pub fn get(&self, name: &str) -> Option<&Site> {
        self.sites.get(name)
    }

    /// Removes a site by name.
    pub fn remove(&mut self, name: &str) -> Result<Option<Site>, String> {
        let removed = self.sites.remove(name);
        if removed.is_some() {
            self.save()?;
        }
        Ok(removed)
    }

    /// Updates the SSL secured status of a site and persists changes to disk.
    pub fn set_secured(&mut self, name: &str, secured: bool) -> Result<bool, String> {
        if let Some(site) = self.sites.get_mut(name) {
            site.set_secured(secured);
            self.save()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
