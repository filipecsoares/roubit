//! Persistent CLI configuration file management (~/.roubit/config.json).

use std::fs;
use std::io;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Persisted CLI settings stored on disk (~/.roubit/config.json).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct CliConfigFile {
    /// Custom SQLite database path or connection URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub database_url: Option<String>,

    /// Remote API base URL (e.g., `<http://localhost:8080>`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_url: Option<String>,

    /// Default output format ("text" or "json").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
}

/// Helper for reading, persisting, and inspecting CLI configuration on disk.
#[derive(Debug, Default)]
pub struct CliConfigManager;

impl CliConfigManager {
    /// Resolves standard path for the configuration file (~/.roubit/config.json).
    /// Can be overridden via the `ROUBIT_CONFIG_PATH` environment variable.
    #[must_use]
    pub fn config_path() -> PathBuf {
        if let Ok(override_path) = std::env::var("ROUBIT_CONFIG_PATH") {
            let trimmed = override_path.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed);
            }
        }

        let base_dir = std::env::var("HOME").map_or_else(|_| PathBuf::from("."), PathBuf::from);
        base_dir.join(".roubit").join("config.json")
    }

    /// Loads saved configuration from disk if present and valid.
    #[must_use]
    pub fn load_config() -> Option<CliConfigFile> {
        let path = Self::config_path();
        let bytes = fs::read(path).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    /// Saves configuration to disk ensuring restricted 0700 directory and 0600 file permissions.
    ///
    /// # Errors
    /// Returns `io::Error` if directory creation or file writing fails.
    pub fn save_config(config: &CliConfigFile) -> io::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
            #[cfg(unix)]
            {
                let dir_perms = fs::Permissions::from_mode(0o700);
                let _ = fs::set_permissions(parent, dir_perms);
            }
        }

        let serialized = serde_json::to_vec_pretty(config)
            .map_err(|e| io::Error::other(format!("JSON serialization failed: {e}")))?;

        fs::write(&path, serialized)?;

        #[cfg(unix)]
        {
            let perms = fs::Permissions::from_mode(0o600);
            fs::set_permissions(&path, perms)?;
        }

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_config_path_resolution() {
        let path = CliConfigManager::config_path();
        assert!(path.to_string_lossy().contains("config.json"));
    }

    #[test]
    fn test_save_and_load_config() {
        let temp_dir = tempfile::tempdir().unwrap();
        let cfg_file = temp_dir.path().join("sub").join("config.json");
        std::env::set_var("ROUBIT_CONFIG_PATH", cfg_file.to_str().unwrap());

        let cfg = CliConfigFile {
            database_url: Some("sqlite:///tmp/test.db".to_string()),
            api_url: Some("http://127.0.0.1:8080".to_string()),
            output_format: Some("json".to_string()),
        };

        CliConfigManager::save_config(&cfg).unwrap();
        let loaded = CliConfigManager::load_config().unwrap();
        assert_eq!(loaded, cfg);

        std::env::remove_var("ROUBIT_CONFIG_PATH");
    }
}
