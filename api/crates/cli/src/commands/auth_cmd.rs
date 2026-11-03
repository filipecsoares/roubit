//! Authentication and remote credentials session handler.

use std::io::{self, Write};

use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::Deserialize;
use uuid::Uuid;
use zeroize::Zeroize;

use crate::args::{AuthArgs, AuthSubcommands};
use crate::client::{CredentialsManager, StoredCredentials};

#[derive(Debug, Deserialize)]
struct AuthApiResponse {
    token: String,
    user: UserInfoDto,
}

#[derive(Debug, Deserialize)]
struct UserInfoDto {
    #[allow(dead_code)]
    id: Uuid,
    email: String,
}

/// Executes authentication commands (`login`, `logout`, `status`).
///
/// # Errors
/// Returns an error if network communication, terminal I/O, or credential storage fails.
#[allow(clippy::too_many_lines)]
pub async fn handle(args: AuthArgs, json: bool, remote_override: Option<&str>) -> Result<()> {
    match args.action {
        AuthSubcommands::Login {
            url,
            email,
            password,
        } => {
            let base_url = url
                .or_else(|| remote_override.map(ToString::to_string))
                .or_else(|| std::env::var("ROUBIT_API_URL").ok())
                .or_else(|| std::env::var("ROUBIT_REMOTE_URL").ok())
                .unwrap_or_else(|| "http://127.0.0.1:8080".to_string());

            let email_str = if let Some(e) = email {
                e.trim().to_string()
            } else {
                print!("Email: ");
                io::stdout().flush().context("failed to flush stdout")?;
                let mut input = String::new();
                io::stdin()
                    .read_line(&mut input)
                    .context("failed to read email")?;
                input.trim().to_string()
            };

            if email_str.is_empty() {
                bail!("Email cannot be empty.");
            }

            let mut password_str = if let Some(p) = password {
                p
            } else {
                rpassword::prompt_password("Password: ").context("failed to read password")?
            };

            let client = Client::builder().build()?;
            let login_url = format!("{base_url}/api/v1/auth/login");

            let response = client
                .post(&login_url)
                .json(&serde_json::json!({
                    "email": email_str,
                    "password": password_str,
                }))
                .send()
                .await
                .with_context(|| {
                    format!("Failed to connect to authentication server at {base_url}")
                })?;

            // Securely wipe sensitive password buffer from memory
            password_str.zeroize();

            if !response.status().is_success() {
                let status = response.status().as_u16();
                let err_text = response.text().await.unwrap_or_default();
                bail!("Authentication failed (HTTP {status}): {err_text}");
            }

            let auth_res: AuthApiResponse = response
                .json()
                .await
                .context("Failed to deserialize server authentication response")?;

            let creds = StoredCredentials {
                api_url: base_url.clone(),
                token: auth_res.token,
                email: auth_res.user.email,
                saved_at: chrono::Utc::now().to_rfc3339(),
            };

            CredentialsManager::save_credentials(&creds)
                .context("Failed to securely save credentials to disk")?;

            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "status": "authenticated",
                        "email": creds.email,
                        "api_url": creds.api_url,
                        "saved_at": creds.saved_at,
                    }))?
                );
            } else {
                crate::ui::TerminalUi::print_success(&format!(
                    "Successfully logged in as {} ({})",
                    creds.email, creds.api_url
                ));
            }
        }

        AuthSubcommands::Logout => {
            CredentialsManager::delete_credentials()
                .context("Failed to delete stored credentials")?;

            if json {
                println!("{}", serde_json::json!({ "status": "logged_out" }));
            } else {
                crate::ui::TerminalUi::print_success(
                    "Successfully logged out. Session credentials removed.",
                );
            }
        }

        AuthSubcommands::Status => match CredentialsManager::load_credentials() {
            Some(creds) => {
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "authenticated": true,
                            "email": creds.email,
                            "api_url": creds.api_url,
                            "saved_at": creds.saved_at,
                        }))?
                    );
                } else {
                    println!("Authentication Status: Logged in");
                    println!("Email:                 {}", creds.email);
                    println!("API Base URL:          {}", creds.api_url);
                    println!("Session Saved At:      {}", creds.saved_at);
                }
            }
            None => {
                if json {
                    println!("{}", serde_json::json!({ "authenticated": false }));
                } else {
                    println!("Not authenticated. Run 'roubit auth login' to authenticate.");
                }
            }
        },
    }

    Ok(())
}
