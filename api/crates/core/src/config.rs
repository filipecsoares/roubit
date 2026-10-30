//! Application configuration module with environment parsing.

use serde::Deserialize;

/// Global application configuration
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_database_url")]
    pub database_url: String,

    #[serde(default = "default_server_port")]
    pub server_port: u16,

    #[serde(default = "default_server_host")]
    pub server_host: String,

    #[serde(default = "default_log_level")]
    pub log_level: String,

    #[serde(default = "default_jwt_secret")]
    pub jwt_secret: String,

    #[serde(default = "default_allowed_origins")]
    pub allowed_origins: String,
}

fn default_database_url() -> String {
    "sqlite://roubit.db?mode=rwc".to_string()
}

const fn default_server_port() -> u16 {
    8080
}

fn default_server_host() -> String {
    "127.0.0.1".to_string()
}

fn default_log_level() -> String {
    "roubit=info,tower_http=info".to_string()
}

fn default_jwt_secret() -> String {
    "roubit-development-secret-key-32-bytes-minimum!".to_string()
}

fn default_allowed_origins() -> String {
    "http://localhost:5173,http://127.0.0.1:5173".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            database_url: default_database_url(),
            server_port: default_server_port(),
            server_host: default_server_host(),
            log_level: default_log_level(),
            jwt_secret: default_jwt_secret(),
            allowed_origins: default_allowed_origins(),
        }
    }
}

impl AppConfig {
    /// Loads configuration from the environment, reading `.env` if available.
    ///
    /// # Errors
    /// Returns `envy::Error` if any defined environment variable fails deserialization.
    pub fn from_env() -> Result<Self, envy::Error> {
        let _ = dotenvy::dotenv(); // Load optional .env file; silently ignore if not present
        envy::from_env::<Self>()
    }

    /// Returns the parsed allowed origins as a vector of origin strings.
    pub fn parsed_allowed_origins(&self) -> Vec<String> {
        self.allowed_origins
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_trait() {
        let config = AppConfig::default();
        assert_eq!(config.server_port, 8080);
        assert_eq!(config.server_host, "127.0.0.1");
        assert_eq!(config.database_url, "sqlite://roubit.db?mode=rwc");
        assert_eq!(config.log_level, "roubit=info,tower_http=info");
    }

    #[test]
    fn test_from_env_fallback() -> Result<(), envy::Error> {
        let config = AppConfig::from_env()?;
        assert_eq!(config.server_port, 8080);
        assert!(config.database_url.contains("roubit.db"));
        Ok(())
    }
}
