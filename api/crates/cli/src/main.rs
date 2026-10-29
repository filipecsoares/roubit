//! Roubit Command Line Interface

use roubit_core::{init_telemetry, AppConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::from_env().unwrap_or_default();
    init_telemetry(&config.log_level);
    tracing::info!("Starting Roubit CLI v{}", roubit_core::VERSION);
    println!("Roubit CLI v{}", roubit_core::VERSION);
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_cli_version_is_accessible() {
        assert!(!roubit_core::VERSION.is_empty());
    }
}
