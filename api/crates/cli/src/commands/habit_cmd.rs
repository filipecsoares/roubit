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
                let rows: Vec<_> = habits
                    .iter()
                    .map(|h| {
                        let unit_str = h.unit.as_deref().unwrap_or("");
                        let target_desc = format!("{:.1} {unit_str}", h.target_value);
                        (h.name.clone(), target_desc, 0, !h.is_archived)
                    })
                    .collect();
                crate::ui::TerminalUi::render_habits_table(&rows);
            }
        }

        HabitSubcommands::Create {
            name,
            kind,
            target,
            unit,
            description,
        } => {
            let habit_type = match kind.as_deref().unwrap_or("binary").to_lowercase().as_str() {
                "numeric" => roubit_core::HabitType::Numeric,
                "timer" => roubit_core::HabitType::Timer,
                _ => roubit_core::HabitType::Binary,
            };

            let habit = roubit_core::Habit {
                id: Uuid::new_v4(),
                name,
                description,
                habit_type,
                target_value: target.unwrap_or(1.0),
                unit,
                frequency_days: 0b0111_1111,
                is_archived: false,
                color_hex: "#10B981".to_string(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };

            repo.create_habit(&habit)
                .await
                .context("failed to create habit")?;

            if json {
                println!("{}", serde_json::to_string_pretty(&habit)?);
            } else {
                crate::ui::TerminalUi::print_success(&format!(
                    "Habit '{}' created successfully (id: {}).",
                    habit.name, habit.id
                ));
            }
        }

        HabitSubcommands::Archive { habit_id } => {
            let id = Uuid::parse_str(&habit_id)
                .with_context(|| format!("Invalid UUID: '{habit_id}'"))?;
            repo.archive_habit(id)
                .await
                .context("failed to archive habit")?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "archived": true, "habit_id": id })
                );
            } else {
                crate::ui::TerminalUi::print_success(&format!("Habit {id} archived."));
            }
        }

        HabitSubcommands::Delete { habit_id } => {
            let id = Uuid::parse_str(&habit_id)
                .with_context(|| format!("Invalid UUID: '{habit_id}'"))?;
            repo.delete_habit(id)
                .await
                .context("failed to delete habit")?;
            if json {
                println!("{}", serde_json::json!({ "deleted": true, "habit_id": id }));
            } else {
                crate::ui::TerminalUi::print_success(&format!("Habit {id} deleted."));
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
                crate::ui::TerminalUi::print_success(&format!(
                    "Check-in recorded for habit {} on {}: value = {:.1} (completed: {})",
                    habit_uuid, log.log_date, log.actual_value, log.is_completed
                ));
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
                let rows: Vec<_> = results
                    .into_iter()
                    .map(|(h, m)| {
                        let unit_str = h.unit.as_deref().unwrap_or("");
                        let target_desc = format!("{:.1} {unit_str}", h.target_value);
                        (
                            h.name.clone(),
                            target_desc,
                            m.current_streak,
                            m.is_active_today,
                        )
                    })
                    .collect();
                crate::ui::TerminalUi::render_habits_table(&rows);
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
