//! Routine command handler.

use anyhow::{Context, Result};
use chrono::{Local, NaiveDate, Timelike};
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
#[allow(clippy::option_if_let_else)]
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
                for (block, status) in &block_statuses {
                    println!(
                        "  [{}] {:02}:{:02} - {:02}:{:02} | {} (ID: {})",
                        status.as_str(),
                        block.start_time.hour(),
                        block.start_time.minute(),
                        block.end_time.hour(),
                        block.end_time.minute(),
                        block.name,
                        block.id
                    );
                }
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
        println!(
            "Time block {} marked as [{}] for date {}.",
            block_uuid,
            status.as_str(),
            target_date
        );
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
