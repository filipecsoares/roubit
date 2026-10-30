//! Roubit Command Line Interface (CLI).
//!
//! Provides an ergonomic terminal interface for routines, habits, OKRs, and encrypted journaling.

pub mod args;
pub mod commands;

use clap::Parser;
use roubit_core::{create_sqlite_pool, init_telemetry, run_migrations, AppConfig};

use crate::args::{CliArgs, Commands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
        commands::config_cmd::handle(config_args, args.json, &config);
        return Ok(());
    }

    // Determine database URL override or default
    let db_url = args.db.as_deref().unwrap_or(config.database_url.as_str());

    let pool = create_sqlite_pool(db_url).await?;
    run_migrations(&pool).await?;

    match command {
        Commands::Routine(routine_args) => {
            commands::routine_cmd::handle(routine_args, args.json, &pool).await?;
        }
        Commands::Habit(habit_args) => {
            commands::habit_cmd::handle(habit_args, args.json, &pool).await?;
        }
        Commands::Okr(okr_args) => {
            commands::okr_cmd::handle(okr_args, args.json, &pool).await?;
        }
        Commands::Journal(journal_args) => {
            commands::journal_cmd::handle(journal_args, args.json, &pool).await?;
        }
        Commands::Config(_) => unreachable!(),
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
}
