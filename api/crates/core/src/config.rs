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

    #[serde(default)]
    pub redis_url: Option<String>,
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
            redis_url: None,
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

    /// Validates configuration invariants (e.g. JWT secret length and production safety).
    ///
    /// # Errors
    /// Returns error if `jwt_secret` is less than 32 characters or uses the default insecure secret
    /// in production or remote PostgreSQL environments.
    pub fn validate(&self) -> Result<(), anyhow::Error> {
        if self.jwt_secret.len() < 32 {
            anyhow::bail!("JWT_SECRET must be at least 32 characters long");
        }

        let is_prod_env =
            std::env::var("ROUBIT_ENV").is_ok_and(|v| v.trim().eq_ignore_ascii_case("production"));

        let is_remote_postgres = self.database_url.starts_with("postgres://")
            && !self.database_url.contains("127.0.0.1")
            && !self.database_url.contains("localhost");

        if (is_prod_env || is_remote_postgres) && self.jwt_secret == default_jwt_secret() {
            anyhow::bail!(
                "Default development JWT_SECRET cannot be used in production or remote PostgreSQL environments. Please set an explicit JWT_SECRET."
            );
        }

        Ok(())
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
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_short_jwt_secret_rejected() {
        let config = AppConfig {
            jwt_secret: "short_secret".to_string(),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_default_jwt_secret_rejected_on_remote_postgres() {
        let mut config = AppConfig {
            database_url: "postgres://user:pass@db.prod.internal:5432/roubit".to_string(),
            ..Default::default()
        };
        assert!(config.validate().is_err());

        // With explicit custom 32+ char secret, it passes
        config.jwt_secret = "custom_production_secure_secret_key_32_bytes!".to_string();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_from_env_fallback() -> Result<(), envy::Error> {
        let config = AppConfig::from_env()?;
        assert_eq!(config.server_port, 8080);
        assert!(config.database_url.contains("roubit.db"));
        Ok(())
    }
}
