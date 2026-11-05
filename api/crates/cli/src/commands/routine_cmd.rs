//! Routine command handler.

use anyhow::{Context, Result};
use chrono::{Local, NaiveDate};
use roubit_core::domain::BlockStatus;
use uuid::Uuid;

use crate::args::{RoutineArgs, RoutineSubcommands};
use crate::client::RoubitClient;

/// Executes routine commands.
///
/// # Errors
/// Returns an error if client operations or JSON serialization fails.
#[allow(clippy::too_many_lines, clippy::option_if_let_else)]
pub async fn handle(args: RoutineArgs, json: bool, client: &dyn RoubitClient) -> Result<()> {
    match args.action {
        RoutineSubcommands::List { date } => {
            let target_date = parse_date_or_today(date.as_deref())?;
            let progress_opt = client
                .get_active_routine_progress(target_date)
                .await
                .context("failed to fetch active routine progress")?;

            let Some((routine, block_statuses, progress)) = progress_opt else {
                if json {
                    println!(r#"{{"routine":null,"blocks":[],"adherence_percentage":0.0}}"#);
                } else {
                    println!("No active routine found for today.");
                }
                return Ok(());
            };

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
                client,
                &block_id,
                date.as_deref(),
                BlockStatus::InProgress,
                json,
            )
            .await?;
        }

        RoutineSubcommands::Complete { block_id, date } => {
            update_block_status(
                client,
                &block_id,
                date.as_deref(),
                BlockStatus::Completed,
                json,
            )
            .await?;
        }

        RoutineSubcommands::Skip { block_id, date } => {
            update_block_status(
                client,
                &block_id,
                date.as_deref(),
                BlockStatus::Skipped,
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
                    if let Some((start, end, block_name)) = parse_block_segment(part) {
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

            if !domain_blocks.is_empty() {
                roubit_core::TimeBlockValidator::validate_schedule(&domain_blocks)
                    .context("invalid time blocks schedule")?;
            }

            let routine = client
                .create_routine(&name, &domain_blocks, active)
                .await
                .context("failed to create routine")?;

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
            client
                .activate_routine(id)
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
            client
                .delete_routine(id)
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
    }

    Ok(())
}

async fn update_block_status(
    client: &dyn RoubitClient,
    block_id_str: &str,
    date_str: Option<&str>,
    status: BlockStatus,
    json: bool,
) -> Result<()> {
    let block_uuid = Uuid::parse_str(block_id_str)
        .with_context(|| format!("Invalid UUID for block_id: '{block_id_str}'"))?;
    let target_date = parse_date_or_today(date_str)?;

    client
        .log_block_status(block_uuid, target_date, status)
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

fn parse_block_segment(part: &str) -> Option<(chrono::NaiveTime, chrono::NaiveTime, String)> {
    let trimmed = part.trim();
    let (start_str, rest) = trimmed.split_once('-')?;
    let colon_positions: Vec<usize> = rest.match_indices(':').map(|(i, _)| i).collect();
    for &pos in &colon_positions {
        let candidate_end = &rest[..pos];
        if let Ok(end) = chrono::NaiveTime::parse_from_str(candidate_end.trim(), "%H:%M")
            .or_else(|_| chrono::NaiveTime::parse_from_str(candidate_end.trim(), "%H:%M:%S"))
        {
            if let Ok(start) = chrono::NaiveTime::parse_from_str(start_str.trim(), "%H:%M")
                .or_else(|_| chrono::NaiveTime::parse_from_str(start_str.trim(), "%H:%M:%S"))
            {
                let name = rest[pos + 1..].trim().to_string();
                return Some((start, end, name));
            }
        }
    }
    None
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_block_segment() {
        let res = parse_block_segment("08:00-10:00:Deep Focus").unwrap();
        assert_eq!(res.0, chrono::NaiveTime::from_hms_opt(8, 0, 0).unwrap());
        assert_eq!(res.1, chrono::NaiveTime::from_hms_opt(10, 0, 0).unwrap());
        assert_eq!(res.2, "Deep Focus");

        let res2 = parse_block_segment("10:00-10:30:Rest: Break").unwrap();
        assert_eq!(res2.0, chrono::NaiveTime::from_hms_opt(10, 0, 0).unwrap());
        assert_eq!(res2.1, chrono::NaiveTime::from_hms_opt(10, 30, 0).unwrap());
        assert_eq!(res2.2, "Rest: Break");
    }
}
