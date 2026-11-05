//! Journal command handler.

use anyhow::{Context, Result};
use base64::prelude::*;
use chrono::{Local, NaiveDate};
use roubit_core::{
    crypto::{
        cipher::{AesGcmCipher, EncryptedEnvelope},
        kdf::KdfService,
        memory::SecretString,
    },
    LOCAL_USER_ID,
};
use uuid::Uuid;

use crate::args::{JournalArgs, JournalSubcommands};
use crate::client::RoubitClient;

/// Executes encrypted journal commands.
///
/// # Errors
/// Returns an error if client access, cryptographic operations, or parsing fails.
#[allow(clippy::too_many_lines, clippy::option_if_let_else)]
pub async fn handle(args: JournalArgs, json: bool, client: &dyn RoubitClient) -> Result<()> {
    match args.action {
        JournalSubcommands::Write {
            date,
            mood,
            tags,
            content,
        } => {
            let target_date = parse_date_or_today(date.as_deref())?;
            let pass = resolve_passphrase()?;

            let reflection_secret = if let Some(c) = content {
                SecretString::new(c)
            } else if crate::ui::TerminalUi::is_stdout_terminal() {
                let initial = format!("# Reflection for {target_date}\n\n");
                crate::interactive::TerminalPrompt::compose_in_editor(Some(&initial))
                    .context("failed to write reflection in external editor")?
            } else {
                SecretString::new("Daily personal reflection recorded via Roubit CLI.".to_string())
            };

            let (ciphertext_b64, salt_b64, nonce_b64) =
                encrypt_reflection(target_date, &reflection_secret, &pass)?;

            client
                .save_journal(
                    target_date,
                    &ciphertext_b64,
                    &salt_b64,
                    &nonce_b64,
                    mood.map(i32::from),
                    &tags,
                )
                .await
                .context("failed to persist encrypted journal entry")?;

            if json {
                let res = serde_json::json!({
                    "entry_date": target_date.to_string(),
                    "mood_score": mood,
                    "tags": tags,
                    "ciphertext_bytes_len": ciphertext_b64.len()
                });
                println!("{res}");
            } else {
                crate::ui::TerminalUi::print_success(&format!(
                    "Encrypted journal entry saved for date {} (Mood: {:?}, Tags: {})",
                    target_date,
                    mood,
                    tags.join(", ")
                ));
            }
        }

        JournalSubcommands::Read { date, id } => {
            let pass = resolve_passphrase()?;

            let entry_opt = if let Some(id_str) = id {
                let uuid = Uuid::parse_str(&id_str)
                    .with_context(|| format!("Invalid UUID for journal entry id: '{id_str}'"))?;
                client
                    .get_journal_entry(uuid)
                    .await
                    .context("failed to fetch journal entry by id")?
            } else {
                let target_date = parse_date_or_today(date.as_deref())?;
                client
                    .get_journal_entry_by_date(target_date)
                    .await
                    .context("failed to fetch journal entry by date")?
            };

            let Some(entry) = entry_opt else {
                anyhow::bail!("No journal entry found for the specified target.");
            };

            let plaintext = decrypt_reflection(
                entry.metadata.entry_date,
                entry.metadata.user_id,
                &entry.salt,
                &entry.nonce,
                &entry.encrypted_payload,
                &pass,
            )?;

            if json {
                let json_repr = serde_json::json!({
                    "id": entry.metadata.id,
                    "date": entry.metadata.entry_date.to_string(),
                    "mood": entry.metadata.mood_score,
                    "tags": entry.metadata.tags,
                    "content": plaintext
                });
                println!("{json_repr}");
            } else {
                crate::ui::TerminalUi::print_success(&format!(
                    "Decrypted journal entry for {}:",
                    entry.metadata.entry_date
                ));
                println!(
                    "--- Journal Entry: {} (Mood: {:?}) ---",
                    entry.metadata.entry_date, entry.metadata.mood_score
                );
                if !entry.metadata.tags.is_empty() {
                    println!("Tags: {}", entry.metadata.tags.join(", "));
                }
                println!("\n{plaintext}");
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

            let metadata_list = client
                .list_journal_metadata(start, end)
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

        JournalSubcommands::Delete { date } => {
            let target_date = NaiveDate::parse_from_str(&date, "%Y-%m-%d")
                .with_context(|| format!("Invalid date format '{date}'. Expected YYYY-MM-DD"))?;

            let deleted = client
                .delete_journal_entry_by_date(target_date)
                .await
                .context("failed to delete journal entry")?;

            if deleted {
                if json {
                    println!("{}", serde_json::json!({ "deleted": true, "date": date }));
                } else {
                    crate::ui::TerminalUi::print_success(&format!(
                        "Journal entry for {date} deleted."
                    ));
                }
            } else if json {
                println!(
                    "{}",
                    serde_json::json!({ "deleted": false, "message": "not found" })
                );
            } else {
                println!("No journal entry found for {date}.");
            }
        }
    }

    Ok(())
}

fn encrypt_reflection(
    date: NaiveDate,
    plaintext: &SecretString,
    passphrase: &SecretString,
) -> Result<(String, String, String)> {
    let kdf = KdfService::default_recommended();
    let salt = KdfService::generate_salt();
    let derived_key = kdf
        .derive_key(passphrase.as_bytes(), &salt)
        .context("Argon2id key derivation failed")?;

    let aad = format!("{LOCAL_USER_ID}:{date}");
    let envelope = AesGcmCipher::encrypt(
        derived_key.as_bytes(),
        plaintext.as_bytes(),
        Some(aad.as_bytes()),
    )
    .context("AES-256-GCM authenticated encryption failed")?;

    Ok((
        BASE64_STANDARD.encode(envelope.ciphertext),
        BASE64_STANDARD.encode(salt),
        BASE64_STANDARD.encode(envelope.nonce),
    ))
}

fn decrypt_reflection(
    date: NaiveDate,
    user_id: Option<Uuid>,
    salt: &[u8; 16],
    nonce: &[u8; 12],
    ciphertext: &[u8],
    passphrase: &SecretString,
) -> Result<String> {
    let kdf = KdfService::default_recommended();
    let derived_key = kdf
        .derive_key(passphrase.as_bytes(), salt)
        .context("Argon2id key derivation failed")?;

    let envelope = EncryptedEnvelope {
        nonce: *nonce,
        ciphertext: ciphertext.to_vec(),
    };

    let target_uid = user_id.unwrap_or(LOCAL_USER_ID);
    let aad = format!("{target_uid}:{date}");

    // Attempt decryption with AAD first (Rust service convention),
    // then fallback without AAD (compatible with web frontend).
    let decrypted_bytes =
        AesGcmCipher::decrypt(derived_key.as_bytes(), &envelope, Some(aad.as_bytes()))
            .or_else(|_| AesGcmCipher::decrypt(derived_key.as_bytes(), &envelope, None))
            .context("Decryption failed: wrong passphrase or corrupted payload")?;

    String::from_utf8(decrypted_bytes).context("Decrypted journal content is not valid UTF-8")
}

fn resolve_passphrase() -> Result<SecretString> {
    if let Ok(pass) = std::env::var("ROUBIT_PASSPHRASE") {
        if !pass.trim().is_empty() {
            return Ok(SecretString::new(pass));
        }
    }

    if crate::ui::TerminalUi::is_stdout_terminal() {
        crate::interactive::TerminalPrompt::prompt_passphrase("Enter journal master passphrase: ")
            .context("failed to prompt for passphrase")
    } else {
        Ok(SecretString::new(
            "roubit-default-cli-vault-passphrase".to_string(),
        ))
    }
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
