//! Habit command handler.

use anyhow::{Context, Result};
use chrono::{Local, NaiveDate};
use roubit_core::domain::{HabitRepository, SqliteHabitRepository, StreakCalculator};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::args::{HabitArgs, HabitSubcommands};

/// Executes habit commands.
///
/// # Errors
/// Returns an error if database access or serialization fails.
#[allow(clippy::too_many_lines, clippy::option_if_let_else)]
pub async fn handle(args: HabitArgs, json: bool, pool: &SqlitePool) -> Result<()> {
    let repo = SqliteHabitRepository::new(pool.clone());

    match args.action {
        HabitSubcommands::List { all } => {
            let habits = if all {
                repo.list_all_habits()
                    .await
                    .context("failed to list habits from database")?
            } else {
                repo.list_active_habits()
                    .await
                    .context("failed to list habits from database")?
            };

            if json {
                let serialized = serde_json::to_string_pretty(&habits)?;
                println!("{serialized}");
            } else if habits.is_empty() {
                println!("No habits found.");
            } else {
                println!("Found {} habit(s):", habits.len());
                for h in habits {
                    let unit_str = h.unit.as_deref().unwrap_or("");
                    println!(
                        "  [{}] {} ({}) - Target: {:.1} {} (ID: {})",
                        if h.is_archived { "ARCHIVED" } else { "ACTIVE" },
                        h.name,
                        h.habit_type.as_str(),
                        h.target_value,
                        unit_str,
                        h.id
                    );
                }
            }
        }

        HabitSubcommands::Checkin {
            habit_id,
            value,
            date,
        } => {
            let habit_uuid = Uuid::parse_str(&habit_id)
                .with_context(|| format!("Invalid UUID for habit_id: '{habit_id}'"))?;
            let target_date = parse_date_or_today(date.as_deref())?;
            let checkin_val = value.unwrap_or(1.0);

            let log = repo
                .record_check_in(habit_uuid, target_date, checkin_val, false)
                .await
                .context("failed to record habit check-in")?;

            if json {
                let serialized = serde_json::to_string_pretty(&log)?;
                println!("{serialized}");
            } else {
                println!(
                    "Check-in recorded for habit {} on {}: value = {:.1} (completed: {})",
                    habit_uuid, log.log_date, log.actual_value, log.is_completed
                );
            }
        }

        HabitSubcommands::Streaks => {
            let habits = repo
                .list_active_habits()
                .await
                .context("failed to list active habits for streaks")?;

            let today = Local::now().date_naive();
            let mut results = Vec::new();

            for h in &habits {
                let start_date = today - chrono::Duration::days(365);
                let logs = repo
                    .get_logs_for_habit(h.id, start_date, today)
                    .await
                    .unwrap_or_default();

                let metrics = StreakCalculator::calculate_for_habit(h, &logs, today);
                results.push((h, metrics));
            }

            if json {
                let json_list: Vec<_> = results
                    .iter()
                    .map(|(h, m)| {
                        serde_json::json!({
                            "habit_id": h.id,
                            "name": h.name,
                            "current_streak": m.current_streak,
                            "longest_streak": m.longest_streak,
                            "is_active_today": m.is_active_today,
                        })
                    })
                    .collect();
                println!("{}", serde_json::to_string_pretty(&json_list)?);
            } else if results.is_empty() {
                println!("No active habits found to compute streaks.");
            } else {
                println!("Habit Streak Metrics (as of {today}):");
                for (h, m) in results {
                    println!(
                        "  {} | Current: {}d | Longest: {}d | Active Today: {}",
                        h.name,
                        m.current_streak,
                        m.longest_streak,
                        if m.is_active_today { "Yes" } else { "No" }
                    );
                }
            }
        }
    }

    Ok(())
}

fn parse_date_or_today(date_str: Option<&str>) -> Result<NaiveDate> {
    date_str.map_or_else(
        || Ok(Local::now().date_naive()),
        |s| {
            NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .with_context(|| format!("Invalid date format: '{s}'. Expected YYYY-MM-DD"))
        },
    )
}
