use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct AppConfig {
    pub base_url: Option<String>,
    pub cookies: HashMap<String, String>,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let path = get_config_path()?;
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            let config: AppConfig = serde_json::from_str(&content)?;
            Ok(config)
        } else {
            Ok(AppConfig::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = get_config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn get_cookie_header(&self) -> String {
        self.cookies
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("; ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_cookie_header() {
        let mut config = AppConfig::default();
        config
            .cookies
            .insert("session".to_string(), "123".to_string());
        config.cookies.insert("foo".to_string(), "bar".to_string());

        let header = config.get_cookie_header();
        assert!(header.contains("session=123"));
        assert!(header.contains("foo=bar"));
        // Only one separator if two items
        if config.cookies.len() > 1 {
            assert!(header.contains("; "));
        }
    }
}

fn get_config_path() -> Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("com", "owojudge", "cli")
        .context("Could not determine config directory")?;
    Ok(proj_dirs.config_dir().join("config.json"))
}
