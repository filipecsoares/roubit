//! Configuration command handler.

use anyhow::{Context, Result};
use roubit_core::AppConfig;

use crate::args::{ConfigArgs, ConfigSubcommands};
use crate::client::config::CliConfigManager;

/// Executes configuration commands.
///
/// # Errors
/// Returns an error if writing to the configuration file fails.
pub fn handle(args: ConfigArgs, json: bool, config: &AppConfig) -> Result<()> {
    match args.action {
        ConfigSubcommands::Show => {
            let persisted = CliConfigManager::load_config().unwrap_or_default();
            if json {
                let serialized = serde_json::json!({
                    "database_url": config.database_url,
                    "server_host": config.server_host,
                    "server_port": config.server_port,
                    "log_level": config.log_level,
                    "persisted": {
                        "database_url": persisted.database_url,
                        "api_url": persisted.api_url,
                        "output_format": persisted.output_format
                    }
                });
                println!("{serialized}");
            } else {
                println!("Roubit Configuration:");
                println!("  Database URL : {}", config.database_url);
                println!("  Server Host  : {}", config.server_host);
                println!("  Server Port  : {}", config.server_port);
                println!("  Log Level    : {}", config.log_level);
                println!("\nPersisted Settings (~/.roubit/config.json):");
                println!(
                    "  Database Path: {}",
                    persisted.database_url.as_deref().unwrap_or("(not set)")
                );
                println!(
                    "  API URL      : {}",
                    persisted.api_url.as_deref().unwrap_or("(not set)")
                );
                println!(
                    "  Output Format: {}",
                    persisted.output_format.as_deref().unwrap_or("(not set)")
                );
            }
        }

        ConfigSubcommands::Set {
            db,
            api_url,
            output_format,
        } => {
            let mut persisted = CliConfigManager::load_config().unwrap_or_default();
            if let Some(db_path) = db {
                persisted.database_url = Some(db_path);
            }
            if let Some(url) = api_url {
                persisted.api_url = Some(url);
            }
            if let Some(fmt) = output_format {
                persisted.output_format = Some(fmt);
            }

            CliConfigManager::save_config(&persisted)
                .context("failed to save configuration to ~/.roubit/config.json")?;

            if json {
                let res = serde_json::json!({
                    "status": "updated",
                    "database_url": persisted.database_url,
                    "api_url": persisted.api_url,
                    "output_format": persisted.output_format
                });
                println!("{res}");
            } else {
                println!("Configuration updated in ~/.roubit/config.json:");
                if let Some(ref db_path) = persisted.database_url {
                    println!("  Database path: {db_path}");
                }
                if let Some(ref url) = persisted.api_url {
                    println!("  API URL      : {url}");
                }
                if let Some(ref fmt) = persisted.output_format {
                    println!("  Output format: {fmt}");
                }
            }
        }
    }

    Ok(())
}
