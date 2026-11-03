//! Habit command handler.

use anyhow::{Context, Result};
use chrono::{Local, NaiveDate};
use roubit_core::domain::{HabitRepository, SqliteHabitRepository, StreakCalculator};
use uuid::Uuid;

use crate::args::{HabitArgs, HabitSubcommands};
use crate::client::RoubitClient;

/// Executes habit commands.
///
/// # Errors
/// Returns an error if client operation or serialization fails.
#[allow(clippy::too_many_lines, clippy::option_if_let_else)]
pub async fn handle(args: HabitArgs, json: bool, client: &dyn RoubitClient) -> Result<()> {
    match args.action {
        HabitSubcommands::List { all } => {
            let habits = if all {
                client
                    .list_habits()
                    .await
                    .context("failed to list habits")?
            } else {
                client
                    .list_active_habits()
                    .await
                    .context("failed to list habits")?
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
                        let streak_u32 = u32::try_from(h.current_streak.max(0)).unwrap_or(0);
                        (h.name.clone(), target_desc, streak_u32, !h.is_archived)
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

            let habit = client
                .create_habit(
                    &name,
                    habit_type,
                    target.unwrap_or(1.0),
                    unit.as_deref(),
                    description.as_deref(),
                )
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
            client
                .archive_habit(id)
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
            client
                .delete_habit(id)
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

            let log = client
                .habit_checkin(habit_uuid, target_date, checkin_val)
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
            let habits = client
                .list_active_habits()
                .await
                .context("failed to list active habits for streaks")?;

            let today = Local::now().date_naive();
            let mut results = Vec::new();

            for h in &habits {
                let metrics = if let Some(pool) = client.local_pool() {
                    let repo = SqliteHabitRepository::new(pool.clone());
                    let start_date = today - chrono::Duration::days(365);
                    let logs = repo
                        .get_logs_for_habit(h.id, roubit_core::LOCAL_USER_ID, start_date, today)
                        .await
                        .unwrap_or_default();

                    StreakCalculator::calculate_for_habit(h, &logs, today)
                } else {
                    roubit_core::StreakMetrics {
                        current_streak: u32::try_from(h.current_streak.max(0)).unwrap_or(0),
                        longest_streak: u32::try_from(h.longest_streak.max(0)).unwrap_or(0),
                        is_active_today: false,
                    }
                };
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
