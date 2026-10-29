//! Roubit REST API Server

use roubit_core::{init_telemetry, AppConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::from_env().unwrap_or_default();
    init_telemetry(&config.log_level);
    tracing::info!(
        host = %config.server_host,
        port = config.server_port,
        "Starting Roubit REST API Server v{}",
        roubit_core::VERSION
    );
    println!("Roubit REST API Server v{}", roubit_core::VERSION);
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_server_version_is_accessible() {
        assert_eq!(roubit_core::VERSION, "0.1.0");
    }
}
