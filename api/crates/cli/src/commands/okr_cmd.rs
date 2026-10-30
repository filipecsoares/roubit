//! OKR command handler.

use anyhow::{Context, Result};
use roubit_core::domain::{OkrRepository, SqliteOkrRepository};
use sqlx::SqlitePool;

use crate::args::{OkrArgs, OkrSubcommands};

/// Executes OKR commands.
///
/// # Errors
/// Returns an error if database access or JSON serialization fails.
pub async fn handle(args: OkrArgs, json: bool, pool: &SqlitePool) -> Result<()> {
    let repo = SqliteOkrRepository::new(pool.clone());

    match args.action {
        OkrSubcommands::List { quarter } => {
            let objectives = if let Some(q) = quarter {
                repo.list_objectives_by_quarter(&q)
                    .await
                    .with_context(|| format!("failed to list objectives for quarter '{q}'"))?
            } else {
                repo.list_all_objectives()
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

        OkrSubcommands::Summary => {
            let objectives = repo
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
