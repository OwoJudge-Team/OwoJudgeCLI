use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct AppConfig {
    pub base_url: Option<String>,
    pub cookies: BTreeMap<String, String>,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        match fs::read_to_string(&path) {
            Ok(content) => Ok(serde_json::from_str(&content)?),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(AppConfig::default()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
        }

        Ok(())
    }

    pub fn get_cookie_header(&self) -> String {
        self.cookies
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("; ")
    }

    fn config_path() -> Result<PathBuf> {
        let proj_dirs = ProjectDirs::from("com", "owojudge", "cli")
            .context("Could not determine config directory")?;
        Ok(proj_dirs.config_dir().join("config.json"))
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
