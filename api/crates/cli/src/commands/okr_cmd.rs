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
                    println!(
                        "\n🎯 [{}] {} ({}) - {:.1}% {}",
                        item.objective.quarter,
                        item.objective.title,
                        item.objective.id,
                        item.objective.progress_percentage,
                        if item.objective.is_completed {
                            "[COMPLETED]"
                        } else {
                            ""
                        }
                    );
                    for kr in item.key_results {
                        println!(
                            "   └─ 🔑 {} | {:.1} / {:.1} {} (Weight: {:.1}, Progress: {:.1}%)",
                            kr.title,
                            kr.current_value,
                            kr.target_value,
                            kr.unit,
                            kr.weight,
                            kr.progress_percentage
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
                    println!(
                        "  [{}] {:<35} | Progress: {:>5.1}% | KRs: {}",
                        o.objective.quarter,
                        o.objective.title,
                        o.objective.progress_percentage,
                        o.key_results.len()
                    );
                }
            }
        }
    }

    Ok(())
}
