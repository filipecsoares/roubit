//! OKR command handler.

use anyhow::{Context, Result};
use uuid::Uuid;

use crate::args::{OkrArgs, OkrSubcommands};
use crate::client::RoubitClient;

/// Executes OKR commands.
///
/// # Errors
/// Returns an error if client operations or JSON serialization fails.
#[allow(clippy::too_many_lines)]
pub async fn handle(args: OkrArgs, json: bool, client: &dyn RoubitClient) -> Result<()> {
    match args.action {
        OkrSubcommands::List { quarter } => {
            let objectives = if let Some(q) = quarter {
                client
                    .list_objectives_by_quarter(&q)
                    .await
                    .with_context(|| format!("failed to list objectives for quarter '{q}'"))?
            } else {
                client
                    .list_all_objectives()
                    .await
                    .context("failed to list all objectives")?
            };

            if json {
                println!("{}", serde_json::to_string_pretty(&objectives)?);
            } else if objectives.is_empty() {
                println!("No objectives found.");
            } else {
                println!("Objectives & Key Results ({}):", objectives.len());
                for item in objectives {
                    let status_badge = if item.objective.is_completed {
                        " [COMPLETED]"
                    } else {
                        ""
                    };
                    println!(
                        "\n🎯 [{}] {}{}",
                        item.objective.quarter, item.objective.title, status_badge
                    );
                    crate::ui::TerminalUi::render_okr_progress(
                        "  Objective Progress",
                        item.objective.progress_percentage,
                    );
                    for kr in item.key_results {
                        let kr_label = format!(
                            "  🔑 {} ({:.1}/{:.1} {})",
                            kr.title, kr.current_value, kr.target_value, kr.unit
                        );
                        crate::ui::TerminalUi::render_okr_progress(
                            &kr_label,
                            kr.progress_percentage,
                        );
                    }
                }
            }
        }

        OkrSubcommands::Create {
            title,
            quarter,
            description,
        } => {
            let objective = client
                .create_objective(&title, &quarter, description.as_deref())
                .await
                .context("failed to create objective")?;

            if json {
                println!("{}", serde_json::to_string_pretty(&objective)?);
            } else {
                crate::ui::TerminalUi::print_success(&format!(
                    "Objective '{}' [{}] created successfully (id: {}).",
                    objective.title, objective.quarter, objective.id
                ));
            }
        }

        OkrSubcommands::AddKr {
            objective_id,
            title,
            target,
            unit,
            habit_id,
        } => {
            let obj_uuid = Uuid::parse_str(&objective_id)
                .with_context(|| format!("Invalid UUID: '{objective_id}'"))?;

            let linked_uuid = habit_id.and_then(|h| Uuid::parse_str(&h).ok());

            let kr = client
                .create_key_result(obj_uuid, &title, target, unit.as_deref(), linked_uuid)
                .await
                .context("failed to create key result")?;

            if json {
                println!("{}", serde_json::to_string_pretty(&kr)?);
            } else {
                crate::ui::TerminalUi::print_success(&format!(
                    "Key Result '{}' added to objective {} (target: {:.1} {}).",
                    kr.title, obj_uuid, kr.target_value, kr.unit
                ));
            }
        }

        OkrSubcommands::Delete { objective_id } => {
            let id = Uuid::parse_str(&objective_id)
                .with_context(|| format!("Invalid UUID: '{objective_id}'"))?;
            client
                .delete_objective(id)
                .await
                .context("failed to delete objective")?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({ "deleted": true, "objective_id": id })
                );
            } else {
                crate::ui::TerminalUi::print_success(&format!("Objective {id} deleted."));
            }
        }

        OkrSubcommands::Summary => {
            let objectives = client
                .list_all_objectives()
                .await
                .context("failed to list objectives for summary")?;

            if json {
                let summary_json: Vec<_> = objectives
                    .iter()
                    .map(|o| {
                        serde_json::json!({
                            "id": o.objective.id,
                            "title": o.objective.title,
                            "quarter": o.objective.quarter,
                            "progress": o.objective.progress_percentage,
                            "is_completed": o.objective.is_completed,
                            "key_results_count": o.key_results.len()
                        })
                    })
                    .collect();
                println!("{}", serde_json::to_string_pretty(&summary_json)?);
            } else if objectives.is_empty() {
                println!("No objectives found to summarize.");
            } else {
                println!("OKR Progress Summary:");
                for o in objectives {
                    let title = format!("[{}] {}", o.objective.quarter, o.objective.title);
                    crate::ui::TerminalUi::render_okr_progress(
                        &title,
                        o.objective.progress_percentage,
                    );
                }
            }
        }
    }

    Ok(())
}
