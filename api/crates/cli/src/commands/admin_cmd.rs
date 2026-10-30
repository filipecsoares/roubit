//! Administrative commands execution handler (e.g., database migrations).

use colored::Colorize;
use roubit_core::{create_sqlite_pool, migrate_sqlite_to_postgres};
use sqlx::PgPool;

use crate::args::AdminAction;

/// Executes administrative actions.
///
/// # Errors
/// Returns error if database connection or migration execution fails.
pub async fn execute(action: &AdminAction, json_output: bool) -> anyhow::Result<()> {
    match action {
        AdminAction::MigrateToPostgres { from, to } => {
            if !json_output {
                println!(
                    "{} Connecting to source SQLite database: {}",
                    "->".cyan().bold(),
                    from.yellow()
                );
            }
            let sqlite_pool = create_sqlite_pool(from).await?;

            if !json_output {
                println!(
                    "{} Connecting to target PostgreSQL database: {}",
                    "->".cyan().bold(),
                    to.yellow()
                );
            }
            let pg_pool = PgPool::connect(to).await?;

            if !json_output {
                println!(
                    "{} Running PostgreSQL migrations & streaming data...",
                    "->".cyan().bold()
                );
            }

            let report = migrate_sqlite_to_postgres(&sqlite_pool, &pg_pool).await?;

            if json_output {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("{}", "Migration completed successfully!".green().bold());
                println!("  {} Users: {}", "-".dimmed(), report.users);
                println!("  {} Routines: {}", "-".dimmed(), report.routines);
                println!("  {} Time blocks: {}", "-".dimmed(), report.time_blocks);
                println!(
                    "  {} Daily routine logs: {}",
                    "-".dimmed(),
                    report.daily_routine_logs
                );
                println!("  {} Habits: {}", "-".dimmed(), report.habits);
                println!("  {} Habit logs: {}", "-".dimmed(), report.habit_logs);
                println!("  {} Objectives: {}", "-".dimmed(), report.objectives);
                println!("  {} Key results: {}", "-".dimmed(), report.key_results);
                println!(
                    "  {} Journal entries: {}",
                    "-".dimmed(),
                    report.journal_entries
                );
                println!(
                    "  {} Idempotency keys: {}",
                    "-".dimmed(),
                    report.idempotency_keys
                );
                println!(
                    "  {} Total records transferred: {}",
                    "=".bold(),
                    report.total_records().to_string().cyan().bold()
                );
            }
            Ok(())
        }
    }
}
