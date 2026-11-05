//! Roubit Command Line Interface (CLI).
//!
//! Provides an ergonomic terminal interface for routines, habits, OKRs, and encrypted journaling.

pub mod args;
pub mod client;
pub mod commands;
pub mod interactive;
pub mod ui;

use clap::Parser;
use roubit_core::{init_telemetry, AppConfig};

use crate::args::{CliArgs, Commands};
use crate::ui::TerminalUi;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    TerminalUi::init();
    let args = CliArgs::parse();
    let config = AppConfig::from_env().unwrap_or_default();

    // Setup logging level: increase if -v / -vv is specified
    let log_level = match args.verbose {
        0 => config.log_level.clone(),
        1 => "roubit=debug,info".to_string(),
        _ => "roubit=trace,debug".to_string(),
    };
    init_telemetry(&log_level);

    let Some(command) = args.command else {
        if args.json {
            let json_version = serde_json::json!({
                "app": "roubit",
                "version": roubit_core::VERSION,
                "status": "ready"
            });
            println!("{json_version}");
        } else {
            println!(
                "Roubit CLI v{} — High-performance routines, habits, OKRs & encrypted journaling.",
                roubit_core::VERSION
            );
            println!("Run 'roubit --help' to inspect available commands.");
        }
        return Ok(());
    };

    // If config command, we don't necessarily require database initialization
    if let Commands::Config(config_args) = command {
        commands::config_cmd::handle(config_args, args.json, &config)?;
        return Ok(());
    }

    // Admin commands handle their own connection endpoints
    if let Commands::Admin(admin_args) = command {
        commands::admin_cmd::execute(&admin_args.action, args.json).await?;
        return Ok(());
    }

    // Auth commands connect directly to remote endpoints or manage credentials
    if let Commands::Auth(auth_args) = command {
        commands::auth_cmd::handle(auth_args, args.json, args.remote.as_deref()).await?;
        return Ok(());
    }

    // Determine database URL override or default from args, persistent config, or env
    let persisted_cfg = client::CliConfigManager::load_config().unwrap_or_default();
    let is_json = args.json || persisted_cfg.output_format.as_deref() == Some("json");
    let db_url = args
        .db
        .as_deref()
        .or(persisted_cfg.database_url.as_deref())
        .unwrap_or(config.database_url.as_str());

    let db_flag = args.db.as_deref().or(if persisted_cfg.api_url.is_none() {
        Some(db_url)
    } else {
        None
    });

    let client = client::ClientFactory::create_client(args.remote.as_deref(), db_flag, None)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    match command {
        Commands::Routine(routine_args) => {
            commands::routine_cmd::handle(routine_args, is_json, client.as_ref()).await?;
        }
        Commands::Habit(habit_args) => {
            commands::habit_cmd::handle(habit_args, is_json, client.as_ref()).await?;
        }
        Commands::Okr(okr_args) => {
            commands::okr_cmd::handle(okr_args, is_json, client.as_ref()).await?;
        }
        Commands::Journal(journal_args) => {
            commands::journal_cmd::handle(journal_args, is_json, client.as_ref()).await?;
        }
        Commands::Config(_) | Commands::Admin(_) | Commands::Auth(_) => unreachable!(),
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::match_wildcard_for_single_variants)]
mod tests {
    use super::*;
    use crate::args::{
        ConfigSubcommands, HabitSubcommands, JournalSubcommands, OkrSubcommands, RoutineSubcommands,
    };

    #[test]
    fn test_cli_version_is_accessible() {
        assert_eq!(roubit_core::VERSION, "0.1.0");
    }

    #[test]
    fn test_clap_parse_global_flags() {
        let args = CliArgs::try_parse_from([
            "roubit",
            "--json",
            "-v",
            "--db",
            "sqlite://test.db",
            "config",
            "show",
        ])
        .unwrap();

        assert!(args.json);
        assert_eq!(args.verbose, 1);
        assert_eq!(args.db.as_deref(), Some("sqlite://test.db"));
        assert!(matches!(
            args.command,
            Some(Commands::Config(args::ConfigArgs {
                action: ConfigSubcommands::Show
            }))
        ));
    }

    #[test]
    fn test_clap_parse_habit_checkin() {
        let args =
            CliArgs::try_parse_from(["roubit", "habit", "checkin", "run-10k", "--value", "10"])
                .unwrap();

        match args.command {
            Some(Commands::Habit(h)) => match h.action {
                HabitSubcommands::Checkin {
                    habit_id,
                    value,
                    date,
                } => {
                    assert_eq!(habit_id, "run-10k");
                    assert_eq!(value, Some(10.0));
                    assert_eq!(date, None);
                }
                _ => panic!("Expected Checkin variant"),
            },
            _ => panic!("Expected Habit command"),
        }
    }

    #[test]
    fn test_clap_parse_routine_complete() {
        let args = CliArgs::try_parse_from([
            "roubit",
            "routine",
            "complete",
            "block-uuid-123",
            "--date",
            "2026-10-30",
        ])
        .unwrap();

        match args.command {
            Some(Commands::Routine(r)) => match r.action {
                RoutineSubcommands::Complete { block_id, date } => {
                    assert_eq!(block_id, "block-uuid-123");
                    assert_eq!(date.as_deref(), Some("2026-10-30"));
                }
                _ => panic!("Expected Complete variant"),
            },
            _ => panic!("Expected Routine command"),
        }
    }

    #[test]
    fn test_clap_parse_okr_list_and_summary() {
        let args_list =
            CliArgs::try_parse_from(["roubit", "okr", "list", "--quarter", "2026-Q4"]).unwrap();
        match args_list.command {
            Some(Commands::Okr(o)) => match o.action {
                OkrSubcommands::List { quarter } => {
                    assert_eq!(quarter.as_deref(), Some("2026-Q4"));
                }
                _ => panic!("Expected List variant"),
            },
            _ => panic!("Expected Okr command"),
        }

        let args_summary = CliArgs::try_parse_from(["roubit", "okr", "summary"]).unwrap();
        match args_summary.command {
            Some(Commands::Okr(o)) => match o.action {
                OkrSubcommands::Summary => (),
                _ => panic!("Expected Summary variant"),
            },
            _ => panic!("Expected Okr command"),
        }
    }

    #[test]
    fn test_clap_parse_journal_write_and_read() {
        let args_write = CliArgs::try_parse_from([
            "roubit",
            "journal",
            "write",
            "--mood",
            "5",
            "--tags",
            "deepwork,rust",
            "--content",
            "Built modular CLI tree with Clap",
        ])
        .unwrap();

        match args_write.command {
            Some(Commands::Journal(j)) => match j.action {
                JournalSubcommands::Write {
                    mood,
                    tags,
                    content,
                    ..
                } => {
                    assert_eq!(mood, Some(5));
                    assert_eq!(tags, vec!["deepwork".to_string(), "rust".to_string()]);
                    assert_eq!(content.as_deref(), Some("Built modular CLI tree with Clap"));
                }
                _ => panic!("Expected Write variant"),
            },
            _ => panic!("Expected Journal command"),
        }
    }

    #[test]
    fn test_clap_parse_admin_migrate_to_postgres() {
        let args = CliArgs::try_parse_from([
            "roubit",
            "admin",
            "migrate-to-postgres",
            "--from",
            "sqlite://roubit.db",
            "--to",
            "postgres://user:pass@localhost:5432/roubit",
        ])
        .unwrap();

        match args.command {
            Some(Commands::Admin(admin_args)) => match admin_args.action {
                args::AdminAction::MigrateToPostgres { from, to } => {
                    assert_eq!(from, "sqlite://roubit.db");
                    assert_eq!(to, "postgres://user:pass@localhost:5432/roubit");
                }
            },
            _ => panic!("Expected Admin command"),
        }
    }

    #[test]
    fn test_clap_parse_auth_commands() {
        let args_login = CliArgs::try_parse_from([
            "roubit",
            "auth",
            "login",
            "--url",
            "http://127.0.0.1:8080",
            "--email",
            "test@example.com",
            "--password",
            "secret123",
        ])
        .unwrap();

        match args_login.command {
            Some(Commands::Auth(auth_args)) => match auth_args.action {
                args::AuthSubcommands::Login {
                    url,
                    email,
                    password,
                } => {
                    assert_eq!(url.as_deref(), Some("http://127.0.0.1:8080"));
                    assert_eq!(email.as_deref(), Some("test@example.com"));
                    assert_eq!(password.as_deref(), Some("secret123"));
                }
                _ => panic!("Expected Login subcommand"),
            },
            _ => panic!("Expected Auth command"),
        }

        let args_logout = CliArgs::try_parse_from(["roubit", "auth", "logout"]).unwrap();
        assert!(matches!(
            args_logout.command,
            Some(Commands::Auth(args::AuthArgs {
                action: args::AuthSubcommands::Logout
            }))
        ));

        let args_status = CliArgs::try_parse_from(["roubit", "auth", "status"]).unwrap();
        assert!(matches!(
            args_status.command,
            Some(Commands::Auth(args::AuthArgs {
                action: args::AuthSubcommands::Status
            }))
        ));
    }
}
