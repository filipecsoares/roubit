//! Session credentials management with restricted file permissions (0600 / 0700).

use std::fs;
use std::io;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Stored authentication credentials for remote HTTP access.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredCredentials {
    /// Remote API base URL.
    pub api_url: String,
    /// JWT bearer session token.
    pub token: String,
    /// Email of the authenticated user.
    pub email: String,
    /// ISO-8601 timestamp of when credentials were saved.
    pub saved_at: String,
}

/// Helper for reading, persisting, and wiping session credentials to disk.
#[derive(Debug, Default)]
pub struct CredentialsManager;

impl CredentialsManager {
    /// Resolves standard path for the credentials file (~/.roubit/credentials.json).
    /// Can be overridden via the `ROUBIT_CREDENTIALS_PATH` environment variable.
    #[must_use]
    pub fn credentials_path() -> PathBuf {
        if let Ok(override_path) = std::env::var("ROUBIT_CREDENTIALS_PATH") {
            let trimmed = override_path.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed);
            }
        }

        let base_dir = std::env::var("HOME").map_or_else(|_| PathBuf::from("."), PathBuf::from);
        base_dir.join(".roubit").join("credentials.json")
    }

    /// Loads saved credentials from disk if present and non-empty.
    #[must_use]
    pub fn load_credentials() -> Option<StoredCredentials> {
        let path = Self::credentials_path();
        let bytes = fs::read(path).ok()?;
        let creds: StoredCredentials = serde_json::from_slice(&bytes).ok()?;
        if creds.token.trim().is_empty() {
            None
        } else {
            Some(creds)
        }
    }

    /// Loads saved token from credentials file if present.
    #[must_use]
    pub fn load_token() -> Option<String> {
        Self::load_credentials().map(|c| c.token)
    }

    /// Saves credentials to disk ensuring restricted 0700 directory and 0600 file permissions.
    ///
    /// # Errors
    /// Returns `io::Error` if directory creation or file writing fails.
    pub fn save_credentials(creds: &StoredCredentials) -> io::Result<()> {
        let path = Self::credentials_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
            #[cfg(unix)]
            {
                let dir_perms = fs::Permissions::from_mode(0o700);
                let _ = fs::set_permissions(parent, dir_perms);
            }
        }

        let serialized = serde_json::to_vec_pretty(creds)
            .map_err(|e| io::Error::other(format!("JSON serialization failed: {e}")))?;

        fs::write(&path, serialized)?;

        #[cfg(unix)]
        {
            let perms = fs::Permissions::from_mode(0o600);
            fs::set_permissions(&path, perms)?;
        }

        Ok(())
    }

    /// Removes the credentials file from disk.
    ///
    /// # Errors
    /// Returns `io::Error` if the file exists and cannot be removed.
    pub fn delete_credentials() -> io::Result<()> {
        let path = Self::credentials_path();
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_credentials_path_resolution() {
        let path = CredentialsManager::credentials_path();
        assert!(path.to_string_lossy().contains("credentials.json"));
    }

    #[test]
    fn test_save_load_and_delete_credentials() {
        let temp_dir = tempfile::tempdir().unwrap();
        let creds_file = temp_dir.path().join("sub").join("credentials.json");

        std::env::set_var("ROUBIT_CREDENTIALS_PATH", creds_file.to_str().unwrap());

        let creds = StoredCredentials {
            api_url: "http://127.0.0.1:8080".to_string(),
            token: "test.jwt.token".to_string(),
            email: "alice@example.com".to_string(),
            saved_at: "2026-11-02T12:00:00Z".to_string(),
        };

        CredentialsManager::save_credentials(&creds).unwrap();

        #[cfg(unix)]
        {
            let meta = fs::metadata(&creds_file).unwrap();
            let mode = meta.permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "File must have 0600 permissions");

            let parent_meta = fs::metadata(creds_file.parent().unwrap()).unwrap();
            let parent_mode = parent_meta.permissions().mode() & 0o777;
            assert_eq!(parent_mode, 0o700, "Directory must have 0700 permissions");
        }

        let loaded = CredentialsManager::load_credentials().unwrap();
        assert_eq!(loaded, creds);
        assert_eq!(
            CredentialsManager::load_token().as_deref(),
            Some("test.jwt.token")
        );

        CredentialsManager::delete_credentials().unwrap();
        assert!(CredentialsManager::load_credentials().is_none());
        assert!(!creds_file.exists());

        std::env::remove_var("ROUBIT_CREDENTIALS_PATH");
    }
}
