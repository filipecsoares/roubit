//! Routine command handler.

use anyhow::{Context, Result};
use chrono::{Local, NaiveDate};
use roubit_core::domain::{
    BlockStatus, RoutineProgressTracker, RoutineRepository, SqliteRoutineRepository,
};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::args::{RoutineArgs, RoutineSubcommands};

/// Executes routine commands.
///
/// # Errors
/// Returns an error if database access or JSON serialization fails.
#[allow(clippy::too_many_lines, clippy::option_if_let_else)]
pub async fn handle(args: RoutineArgs, json: bool, pool: &SqlitePool) -> Result<()> {
    let repo = SqliteRoutineRepository::new(pool.clone());

    match args.action {
        RoutineSubcommands::List { date } => {
            let target_date = parse_date_or_today(date.as_deref())?;
            let routine_opt = repo
                .get_active_routine()
                .await
                .context("failed to fetch active routine")?;

            let Some(routine) = routine_opt else {
                if json {
                    println!(r#"{{"routine":null,"blocks":[],"adherence_percentage":0.0}}"#);
                } else {
                    println!("No active routine found for today.");
                }
                return Ok(());
            };

            let blocks = repo
                .list_blocks_for_routine(routine.id)
                .await
                .context("failed to list blocks for routine")?;

            let mut block_statuses = Vec::new();
            for block in &blocks {
                let log_opt = repo
                    .get_daily_log(block.id, target_date)
                    .await
                    .context("failed to fetch block log")?;
                let status = log_opt.map_or(BlockStatus::Pending, |l| l.status);
                block_statuses.push((block.clone(), status));
            }

            let now_time = Local::now().time();
            let progress =
                RoutineProgressTracker::compute_progress(&blocks, &block_statuses, now_time);

            if json {
                let serialized = serde_json::to_string_pretty(&progress)?;
                println!("{serialized}");
            } else {
                println!(
                    "Routine: {} | Date: {} | Adherence: {:.1}%",
                    routine.name, target_date, progress.adherence_percentage
                );
                crate::ui::TerminalUi::render_routine_table_with_statuses(&block_statuses);
            }
        }

        RoutineSubcommands::Start { block_id, date } => {
            update_block_status(
                &repo,
                &block_id,
                date.as_deref(),
                BlockStatus::InProgress,
                json,
            )
            .await?;
        }

        RoutineSubcommands::Complete { block_id, date } => {
            update_block_status(
                &repo,
                &block_id,
                date.as_deref(),
                BlockStatus::Completed,
                json,
            )
            .await?;
        }

        RoutineSubcommands::Create {
            name,
            blocks,
            active,
        } => {
            let routine_id = Uuid::new_v4();
            let mut domain_blocks = Vec::new();

            if let Some(blocks_str) = blocks {
                for (idx, part) in blocks_str.split(',').enumerate() {
                    let segments: Vec<&str> = part.split(':').collect();
                    if segments.len() >= 2 {
                        let times: Vec<&str> = segments[0].split('-').collect();
                        if times.len() == 2 {
                            let start = chrono::NaiveTime::parse_from_str(times[0].trim(), "%H:%M")
                                .with_context(|| format!("invalid start time {}", times[0]))?;
                            let end = chrono::NaiveTime::parse_from_str(times[1].trim(), "%H:%M")
                                .with_context(|| format!("invalid end time {}", times[1]))?;
                            let block_name = segments[1..].join(":").trim().to_string();

                            domain_blocks.push(roubit_core::TimeBlock {
                                id: Uuid::new_v4(),
                                routine_id,
                                name: block_name,
                                start_time: start,
                                end_time: end,
                                category: "General".to_string(),
                                color_hex: "#6366F1".to_string(),
                                order_index: i32::try_from(idx).unwrap_or(0),
                                created_at: chrono::Utc::now(),
                            });
                        }
                    }
                }
            }

            if !domain_blocks.is_empty() {
                roubit_core::TimeBlockValidator::validate_schedule(&domain_blocks)
                    .context("invalid time blocks schedule")?;
            }

            let routine = roubit_core::Routine {
                id: routine_id,
                name,
                description: None,
                is_active: active,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };

            repo.create_routine_with_blocks(&routine, &domain_blocks)
                .await
                .context("failed to create routine")?;

            if active {
                let _ = repo.set_active_routine(routine_id).await;
            }

            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "routine": routine,
                        "blocks_count": domain_blocks.len()
                    })
                );
            } else {
                crate::ui::TerminalUi::print_success(&format!(
                    "Routine '{}' created with {} block(s) (id: {}).",
                    routine.name,
                    domain_blocks.len(),
                    routine.id
                ));
            }
        }

        RoutineSubcommands::Activate { routine_id } => {
            let id = Uuid::parse_str(&routine_id)
                .with_context(|| format!("Invalid UUID: '{routine_id}'"))?;
            repo.set_active_routine(id)
                .await
                .context("failed to activate routine")?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "activated": true, "routine_id": id })
                );
            } else {
                crate::ui::TerminalUi::print_success(&format!(
                    "Routine {id} activated successfully."
                ));
            }
        }

        RoutineSubcommands::Delete { routine_id } => {
            let id = Uuid::parse_str(&routine_id)
                .with_context(|| format!("Invalid UUID: '{routine_id}'"))?;
            repo.delete_routine(id)
                .await
                .context("failed to delete routine")?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "deleted": true, "routine_id": id })
                );
            } else {
                crate::ui::TerminalUi::print_success(&format!(
                    "Routine {id} deleted successfully."
                ));
            }
        }

        RoutineSubcommands::Skip { block_id, date } => {
            update_block_status(
                &repo,
                &block_id,
                date.as_deref(),
                BlockStatus::Skipped,
                json,
            )
            .await?;
        }
    }

    Ok(())
}

async fn update_block_status(
    repo: &SqliteRoutineRepository,
    block_id_str: &str,
    date_str: Option<&str>,
    status: BlockStatus,
    json: bool,
) -> Result<()> {
    let block_uuid = Uuid::parse_str(block_id_str)
        .with_context(|| format!("Invalid UUID for block_id: '{block_id_str}'"))?;
    let target_date = parse_date_or_today(date_str)?;

    repo.log_block_status(block_uuid, target_date, status, None)
        .await
        .context("failed to update time block status")?;

    if json {
        let response = serde_json::json!({
            "status": "success",
            "block_id": block_uuid,
            "date": target_date.to_string(),
            "new_status": status.as_str()
        });
        println!("{response}");
    } else {
        crate::ui::TerminalUi::print_success(&format!(
            "Time block {} marked as [{}] for date {}.",
            block_uuid,
            status.as_str(),
            target_date
        ));
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
