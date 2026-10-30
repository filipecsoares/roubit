//! Session credentials management with restricted file permissions (0600).

use std::fs;
use std::io;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Stored authentication tokens for remote HTTP access.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredCredentials {
    /// JWT bearer token.
    pub token: String,
    /// Base URL associated with the token.
    pub api_url: Option<String>,
}

/// Helper for reading and persisting session credentials to disk.
#[derive(Debug, Default)]
pub struct CredentialsManager;

impl CredentialsManager {
    /// Resolves standard path for the credentials file (~/.config/roubit/credentials.json).
    #[must_use]
    pub fn credentials_path() -> PathBuf {
        let base_dir = std::env::var("HOME").map_or_else(|_| PathBuf::from("."), PathBuf::from);
        base_dir
            .join(".config")
            .join("roubit")
            .join("credentials.json")
    }

    /// Loads saved token from credentials file if present.
    #[must_use]
    pub fn load_token() -> Option<String> {
        let path = Self::credentials_path();
        let bytes = fs::read(path).ok()?;
        let creds: StoredCredentials = serde_json::from_slice(&bytes).ok()?;
        if creds.token.trim().is_empty() {
            None
        } else {
            Some(creds.token)
        }
    }

    /// Saves token to disk ensuring restricted 0600 file permissions.
    ///
    /// # Errors
    /// Returns `io::Error` if directory creation or file writing fails.
    pub fn save_token(token: &str, api_url: Option<&str>) -> io::Result<()> {
        let path = Self::credentials_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let creds = StoredCredentials {
            token: token.to_string(),
            api_url: api_url.map(ToString::to_string),
        };
        let serialized = serde_json::to_vec_pretty(&creds)
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
    fn test_credentials_path_resolution() {
        let path = CredentialsManager::credentials_path();
        assert!(path.to_string_lossy().contains("credentials.json"));
    }
}
