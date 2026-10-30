//! Roubit REST API Server

use roubit_core::{create_sqlite_pool, init_telemetry, run_migrations, AppConfig};
use roubit_server::run_server;

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

    let pool = create_sqlite_pool(&config.database_url).await?;
    run_migrations(&pool).await?;
    tracing::info!("Database migrations applied successfully");

    run_server(config, pool).await?;
    Ok(())
}
