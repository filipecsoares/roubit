//! Background worker consuming Redis keyspace notifications for scheduled reminders.

use std::{convert::Infallible, time::Duration};

use axum::{
    extract::State,
    response::sse::{Event, KeepAlive, Sse},
};
use roubit_core::ReminderNotification;
use sqlx::{Row, SqlitePool};
use tokio::sync::broadcast;
use tokio_stream::{wrappers::BroadcastStream, Stream, StreamExt};

use crate::{auth::AuthenticatedUser, state::AppState};

/// Parses a Redis reminder key into `(user_id, entity_type, entity_id)`.
///
/// Expected format: `reminder:<user_id>:<entity_type>:<entity_id>`
#[must_use]
pub fn parse_reminder_key(key: &str) -> Option<(String, String, String)> {
    let stripped = key.strip_prefix("reminder:")?;
    let mut parts = stripped.split(':');
    let user_id = parts.next()?.to_string();
    let entity_type = parts.next()?.to_string();
    let entity_id = parts.next()?.to_string();
    if user_id.is_empty() || entity_type.is_empty() || entity_id.is_empty() {
        return None;
    }
    Some((user_id, entity_type, entity_id))
}

/// Hydrates a reminder notification with domain details from the database.
pub async fn hydrate_reminder(
    db: &SqlitePool,
    user_id: &str,
    entity_type: &str,
    entity_id: &str,
) -> ReminderNotification {
    let now = chrono::Utc::now();

    match entity_type {
        "time_block" => {
            let row = sqlx::query("SELECT title FROM time_blocks WHERE id = ?")
                .bind(entity_id)
                .fetch_optional(db)
                .await
                .ok()
                .flatten();

            let title_str = row
                .and_then(|r| r.try_get::<String, _>("title").ok())
                .unwrap_or_else(|| "Bloco de Rotina".to_string());

            ReminderNotification {
                user_id: user_id.to_string(),
                entity_type: entity_type.to_string(),
                entity_id: entity_id.to_string(),
                title: format!("Rotina: {title_str}"),
                message: format!("O bloco '{title_str}' está começando agora."),
                scheduled_at: now,
            }
        }
        "habit" => {
            let row = sqlx::query("SELECT name FROM habits WHERE id = ?")
                .bind(entity_id)
                .fetch_optional(db)
                .await
                .ok()
                .flatten();

            let name_str = row
                .and_then(|r| r.try_get::<String, _>("name").ok())
                .unwrap_or_else(|| "Hábito".to_string());

            ReminderNotification {
                user_id: user_id.to_string(),
                entity_type: entity_type.to_string(),
                entity_id: entity_id.to_string(),
                title: format!("Lembrete de Hábito: {name_str}"),
                message: format!("Hora de cumprir seu hábito diário: {name_str}!"),
                scheduled_at: now,
            }
        }
        _ => ReminderNotification {
            user_id: user_id.to_string(),
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
            title: format!("Lembrete: {entity_type}"),
            message: "Seu lembrete agendado foi acionado.".to_string(),
            scheduled_at: now,
        },
    }
}

/// Runs a fast catch-up check on startup for any routines whose start times passed during downtime.
pub async fn catch_up_missed_reminders(
    db: &SqlitePool,
    tx_notification: &broadcast::Sender<ReminderNotification>,
) {
    let now = chrono::Utc::now();
    let current_time_str = now.format("%H:%M").to_string();

    let rows = sqlx::query(
        r"
        SELECT tb.id, tb.routine_id, tb.title, tb.start_time, r.user_id
        FROM time_blocks tb
        JOIN routines r ON r.id = tb.routine_id
        WHERE r.is_active = 1 AND tb.start_time <= ?
        ORDER BY tb.start_time DESC
        LIMIT 10
        ",
    )
    .bind(&current_time_str)
    .fetch_all(db)
    .await;

    if let Ok(records) = rows {
        for record in records {
            let entity_id: String = record.get("id");
            let user_id: String = record.get("user_id");
            let title: String = record.get("title");

            let notif = ReminderNotification {
                user_id,
                entity_type: "time_block".to_string(),
                entity_id,
                title: format!("Rotina Ativa: {title}"),
                message: format!("Bloco '{title}' programado para hoje."),
                scheduled_at: now,
            };
            let _ = tx_notification.send(notif);
        }
    }
}

/// Starts the background Tokio worker listening to Redis Keyspace Notifications (`__keyevent@0__:expired`).
pub fn start_reminder_worker(
    redis_url: Option<String>,
    db: SqlitePool,
    tx_notification: broadcast::Sender<ReminderNotification>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        // Fast catch-up on startup
        catch_up_missed_reminders(&db, &tx_notification).await;

        let Some(url) = redis_url else {
            tracing::info!("Redis not configured; reminder background keyspace listener idle");
            return;
        };

        let mut backoff = Duration::from_secs(1);
        loop {
            tracing::info!("Connecting to Redis keyspace notifications stream at {url}...");

            let client = match redis::Client::open(url.as_str()) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(error = %e, "Invalid Redis URL for pubsub; retrying in {backoff:?}");
                    tokio::time::sleep(backoff).await;
                    backoff = (backoff * 2).min(Duration::from_secs(30));
                    continue;
                }
            };

            let mut pubsub = match client.get_async_pubsub().await {
                Ok(ps) => ps,
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to open async pubsub; retrying in {backoff:?}");
                    tokio::time::sleep(backoff).await;
                    backoff = (backoff * 2).min(Duration::from_secs(30));
                    continue;
                }
            };

            // Subscribe to keyspace expired events
            if let Err(e) = pubsub.subscribe("__keyevent@0__:expired").await {
                tracing::warn!(error = %e, "Failed to subscribe to keyspace expired channel; retrying in {backoff:?}");
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(Duration::from_secs(30));
                continue;
            }

            tracing::info!("Subscribed to Redis keyspace channel `__keyevent@0__:expired`");
            backoff = Duration::from_secs(1);

            let mut msg_stream = pubsub.on_message();
            while let Some(msg) = msg_stream.next().await {
                let payload_res: Result<String, _> = msg.get_payload();
                if let Ok(key) = payload_res {
                    if let Some((user_id, entity_type, entity_id)) = parse_reminder_key(&key) {
                        tracing::info!(user_id = %user_id, entity_type = %entity_type, entity_id = %entity_id, "Redis reminder expired event received");
                        let notification =
                            hydrate_reminder(&db, &user_id, &entity_type, &entity_id).await;
                        let _ = tx_notification.send(notification);
                    }
                }
            }

            tracing::warn!("Redis pubsub stream ended, reconnecting in {backoff:?}...");
            tokio::time::sleep(backoff).await;
            backoff = (backoff * 2).min(Duration::from_secs(30));
        }
    })
}

/// Server-Sent Events (SSE) endpoint: `GET /api/v1/notifications/stream`
pub async fn notification_stream_handler(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.tx_notification.subscribe();
    let user_id = user.id.to_string();

    let stream = BroadcastStream::new(rx).filter_map(move |item| match item {
        Ok(notif) if notif.user_id == user_id => {
            let json = serde_json::to_string(&notif).unwrap_or_default();
            Some(Ok(Event::default().event("reminder").data(json)))
        }
        _ => None,
    });

    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_reminder_key_valid() {
        let key = "reminder:usr_123:time_block:blk_456";
        let parsed = parse_reminder_key(key);
        assert_eq!(
            parsed,
            Some((
                "usr_123".to_string(),
                "time_block".to_string(),
                "blk_456".to_string()
            ))
        );
    }

    #[test]
    fn test_parse_reminder_key_habit() {
        let key = "reminder:u1:habit:h2";
        let parsed = parse_reminder_key(key);
        assert_eq!(
            parsed,
            Some(("u1".to_string(), "habit".to_string(), "h2".to_string()))
        );
    }

    #[test]
    fn test_parse_reminder_key_invalid() {
        assert_eq!(parse_reminder_key("not_a_reminder:key"), None);
        assert_eq!(parse_reminder_key("reminder:usr"), None);
        assert_eq!(parse_reminder_key("reminder:usr:time_block"), None);
        assert_eq!(parse_reminder_key("reminder:::"), None);
    }
}
