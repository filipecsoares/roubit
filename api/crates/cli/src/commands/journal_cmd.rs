//! Journal command handler.

use anyhow::{Context, Result};
use chrono::{Local, NaiveDate};
use roubit_core::{
    crypto::{kdf::KdfService, memory::SecretString},
    domain::{JournalRepository, JournalService, SqliteJournalRepository},
};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::args::{JournalArgs, JournalSubcommands};

/// Executes encrypted journal commands.
///
/// # Errors
/// Returns an error if database access, cryptographic operations, or parsing fails.
#[allow(clippy::too_many_lines, clippy::option_if_let_else)]
pub async fn handle(args: JournalArgs, json: bool, pool: &SqlitePool) -> Result<()> {
    let repo = SqliteJournalRepository::new(pool.clone());
    let service = JournalService::new(repo.clone(), KdfService::default_recommended());

    match args.action {
        JournalSubcommands::Write {
            date,
            mood,
            tags,
            content,
        } => {
            let target_date = parse_date_or_today(date.as_deref())?;
            let pass = resolve_passphrase();

            let text = content.unwrap_or_else(|| {
                "Daily personal reflection recorded via Roubit CLI.".to_string()
            });

            let meta = service
                .create_or_update_entry(target_date, mood, tags, SecretString::new(text), &pass)
                .await
                .context("failed to encrypt and persist journal entry")?;

            if json {
                println!("{}", serde_json::to_string_pretty(&meta)?);
            } else {
                println!(
                    "Encrypted journal entry saved for date {} (ID: {}, Mood: {:?}, Payload: {} bytes)",
                    meta.entry_date, meta.id, meta.mood_score, meta.ciphertext_bytes_len
                );
            }
        }

        JournalSubcommands::Read { date, id } => {
            let pass = resolve_passphrase();

            let decrypted = if let Some(id_str) = id {
                let uuid = Uuid::parse_str(&id_str)
                    .with_context(|| format!("Invalid UUID for journal entry id: '{id_str}'"))?;
                service
                    .read_and_decrypt_entry(uuid, &pass)
                    .await
                    .context("failed to decrypt journal entry with provided passphrase")?
            } else {
                let target_date = parse_date_or_today(date.as_deref())?;
                service
                    .read_and_decrypt_entry_by_date(target_date, &pass)
                    .await
                    .context("failed to decrypt journal entry with provided passphrase")?
                    .context("No journal entry found for target date")?
            };

            if json {
                let json_repr = serde_json::json!({
                    "id": decrypted.metadata.id,
                    "date": decrypted.metadata.entry_date.to_string(),
                    "mood": decrypted.metadata.mood_score,
                    "tags": decrypted.metadata.tags,
                    "content": decrypted.plaintext_content
                });
                println!("{json_repr}");
            } else {
                println!(
                    "--- Journal Entry: {} (Mood: {:?}) ---",
                    decrypted.metadata.entry_date, decrypted.metadata.mood_score
                );
                if !decrypted.metadata.tags.is_empty() {
                    println!("Tags: {}", decrypted.metadata.tags.join(", "));
                }
                println!("\n{}", decrypted.plaintext_content);
                println!("---------------------------------------");
            }
        }

        JournalSubcommands::List { from, to } => {
            let today = Local::now().date_naive();
            let start = if let Some(f) = from {
                NaiveDate::parse_from_str(&f, "%Y-%m-%d")
                    .with_context(|| format!("Invalid start date '{f}'. Expected YYYY-MM-DD"))?
            } else {
                today - chrono::Duration::days(30)
            };

            let end = if let Some(t) = to {
                NaiveDate::parse_from_str(&t, "%Y-%m-%d")
                    .with_context(|| format!("Invalid end date '{t}'. Expected YYYY-MM-DD"))?
            } else {
                today
            };

            let metadata_list = repo
                .list_metadata_in_range(start, end)
                .await
                .context("failed to list journal metadata")?;

            if json {
                println!("{}", serde_json::to_string_pretty(&metadata_list)?);
            } else if metadata_list.is_empty() {
                println!("No journal entries found between {start} and {end}.");
            } else {
                println!("Journal Entries ({start} to {end}):");
                for meta in metadata_list {
                    let mood_display = meta
                        .mood_score
                        .map_or_else(|| "-".to_string(), |m| format!("{m}/5"));
                    println!(
                        "  {} | Mood: {:<3} | Tags: {:<20} | Encrypted Size: {:>4} B | ID: {}",
                        meta.entry_date,
                        mood_display,
                        meta.tags.join(", "),
                        meta.ciphertext_bytes_len,
                        meta.id
                    );
                }
            }
        }
    }

    Ok(())
}

fn resolve_passphrase() -> SecretString {
    if let Ok(pass) = std::env::var("ROUBIT_PASSPHRASE") {
        if !pass.trim().is_empty() {
            return SecretString::new(pass);
        }
    }

    // Default fallback vault passphrase if not set in environment
    SecretString::new("roubit-default-cli-vault-passphrase".to_string())
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
