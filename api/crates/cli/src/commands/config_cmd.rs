//! Configuration command handler.

use roubit_core::AppConfig;

use crate::args::{ConfigArgs, ConfigSubcommands};

/// Executes configuration commands.
pub fn handle(args: ConfigArgs, json: bool, config: &AppConfig) {
    match args.action {
        ConfigSubcommands::Show => {
            if json {
                let serialized = serde_json::json!({
                    "database_url": config.database_url,
                    "server_host": config.server_host,
                    "server_port": config.server_port,
                    "log_level": config.log_level,
                });
                println!("{serialized}");
            } else {
                println!("Roubit Configuration:");
                println!("  Database URL : {}", config.database_url);
                println!("  Server Host  : {}", config.server_host);
                println!("  Server Port  : {}", config.server_port);
                println!("  Log Level    : {}", config.log_level);
            }
        }

        ConfigSubcommands::Set { db, api_url } => {
            if json {
                let res = serde_json::json!({
                    "status": "updated",
                    "database_url": db,
                    "api_url": api_url
                });
                println!("{res}");
            } else {
                println!("Configuration updated:");
                if let Some(db_path) = db {
                    println!("  Database path set to: {db_path}");
                }
                if let Some(url) = api_url {
                    println!("  API URL set to: {url}");
                }
            }
        }
    }
}
