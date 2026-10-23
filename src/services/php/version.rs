use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Normalizes inputs like "php@8.2", "php-8.2", "v8.2" down to "8.2".
pub fn normalize_php_version(raw_version: &str) -> String {
    let trimmed = raw_version.trim().to_lowercase();
    let cleaned = trimmed
        .strip_prefix("php@")
        .or_else(|| trimmed.strip_prefix("php-"))
        .or_else(|| trimmed.strip_prefix("php"))
        .or_else(|| trimmed.strip_prefix('v'))
        .unwrap_or(&trimmed);

    cleaned.to_string()
}

/// Dynamically resolves shorthand ("8.2") to the latest available static-php-cli build on dl.static-php.dev.
pub fn resolve_php_version_alias(raw_version: &str) -> String {
    let clean = normalize_php_version(raw_version);

    if clean.split('.').count() >= 3 {
        return clean;
    }

    if let Ok(res) = ureq::get("https://dl.static-php.dev/static-php-cli/common/")
        .header("User-Agent", "dar-cli")
        .call()
    {
        if let Ok(html) = res.into_body().read_to_string() {
            let prefix = format!("php-{}.", clean);
            let mut matches: Vec<(u32, String)> = Vec::new();

            for line in html.lines() {
                if let Some(start) = line.find(&prefix) {
                    let rest = &line[start + 4..];
                    if let Some(end) = rest.find("-cli-linux-x86_64.tar.gz") {
                        let full_version = &rest[..end];

                        if let Some(patch_str) = full_version.split('.').nth(2) {
                            if let Ok(patch_num) = patch_str.parse::<u32>() {
                                matches.push((patch_num, full_version.to_string()));
                            }
                        }
                    }
                }
            }

            if let Some((_, latest_version)) = matches.iter().max_by_key(|(patch, _)| patch) {
                println!("🌐 Resolved PHP {} -> {} via static-php CDN", clean, latest_version);
                return latest_version.clone();
            }
        }
    }

    let fallbacks: HashMap<&str, &str> = HashMap::from([
        ("8.0", "8.0.30"),
        ("8.1", "8.1.30"),
        ("8.2", "8.2.32"),
        ("8.3", "8.3.11"),
        ("8.4", "8.4.8"),
        ("8.5", "8.5.8"),
    ]);

    fallbacks.get(clean.as_str()).unwrap_or(&clean.as_str()).to_string()
}

/// Scans `~/.dar/php/versions` and returns (installed_versions, active_version_option).
pub fn list_installed_php_versions(dar_dir: &Path) -> (Vec<String>, Option<String>) {
    let versions_dir = dar_dir.join("php").join("versions");
    let current_symlink = dar_dir.join("php").join("current");

    let mut installed = Vec::new();
    if let Ok(entries) = fs::read_dir(&versions_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    installed.push(name.to_string());
                }
            }
        }
    }
    installed.sort();

    let active = if let Ok(target) = fs::read_link(&current_symlink) {
        target
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
    } else {
        None
    };

    (installed, active)
}

/// Resolves the raw target version string (from CLI or `.darrc`) down to a clean version.
pub fn resolve_isolation_target(raw_target: Option<String>, project_dir: &Path) -> Result<String, String> {
    let version_str = match raw_target {
        Some(v) if !v.trim().is_empty() => v,
        _ => match parse_darrc(project_dir)? {
            Some(v) => v,
            None => {
                return Err(
                    "No version specified and no .darrc file found in the current directory.\n\
                     Usage: dar php -i <version> or create a .darrc with 'php=8.2'".to_string(),
                );
            }
        },
    };

    Ok(normalize_php_version(&version_str))
}

/// Attempts to parse `php = ...` from a `.darrc` file in the target directory.
pub fn parse_darrc(dir: &Path) -> Result<Option<String>, String> {
    let darrc_path = dir.join(".darrc");
    if !darrc_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&darrc_path)
        .map_err(|e| format!("Failed to read .darrc at {}: {}", darrc_path.display(), e))?;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        if let Some((key, val)) = line.split_once('=') {
            if key.trim().eq_ignore_ascii_case("php") {
                let clean_val = val.trim().trim_matches('"').trim_matches('\'').trim();
                if !clean_val.is_empty() {
                    return Ok(Some(clean_val.to_string()));
                }
            }
        }
    }

    Ok(None)
}
