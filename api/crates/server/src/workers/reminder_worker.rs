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
    let entity_uuid = uuid::Uuid::parse_str(entity_id).ok();
    let user_uuid = uuid::Uuid::parse_str(user_id).ok();

    match entity_type {
        "time_block" => {
            let row = if let (Some(e_id), Some(u_id)) = (entity_uuid, user_uuid) {
                sqlx::query(
                    r"
                    SELECT tb.name
                    FROM time_blocks tb
                    JOIN routines r ON tb.routine_id = r.id
                    WHERE tb.id = ? AND r.user_id = ?
                    ",
                )
                .bind(e_id)
                .bind(u_id)
                .fetch_optional(db)
                .await
                .ok()
                .flatten()
            } else {
                None
            };

            let title_str = row
                .and_then(|r| r.try_get::<String, _>("name").ok())
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
            let row = if let (Some(e_id), Some(u_id)) = (entity_uuid, user_uuid) {
                sqlx::query("SELECT name FROM habits WHERE id = ? AND user_id = ?")
                    .bind(e_id)
                    .bind(u_id)
                    .fetch_optional(db)
                    .await
                    .ok()
                    .flatten()
            } else {
                None
            };

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
    let current_time_str = now.format("%H:%M:%S").to_string();

    let rows = sqlx::query(
        r"
        SELECT tb.id, tb.routine_id, tb.name, tb.start_time, r.user_id
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
            let entity_id = record
                .try_get::<uuid::Uuid, _>("id")
                .map(|u| u.to_string())
                .or_else(|_| record.try_get::<String, _>("id"))
                .unwrap_or_default();
            let user_id = record
                .try_get::<uuid::Uuid, _>("user_id")
                .map(|u| u.to_string())
                .or_else(|_| record.try_get::<String, _>("user_id"))
                .unwrap_or_default();
            let title: String = record.try_get("name").unwrap_or_default();

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

    #[tokio::test]
    async fn test_hydrate_and_catch_up_reminders_with_real_entities() {
        use chrono::{NaiveTime, Utc};
        use roubit_core::{
            create_sqlite_pool, run_migrations, Habit, HabitRepository, Routine, RoutineRepository,
            SqliteHabitRepository, SqliteRoutineRepository, TimeBlock,
        };
        use uuid::Uuid;

        let pool = create_sqlite_pool("sqlite::memory:").await.unwrap();
        run_migrations(&pool).await.unwrap();

        let user_id = Uuid::new_v4();
        let other_user_id = Uuid::new_v4();
        roubit_core::domain::seed_test_user(&pool, user_id)
            .await
            .unwrap();
        roubit_core::domain::seed_test_user(&pool, other_user_id)
            .await
            .unwrap();

        let routine_id = Uuid::new_v4();
        let block_id = Uuid::new_v4();
        let routine = Routine {
            id: routine_id,
            user_id: Some(user_id),
            name: "Rotina Matinal".to_string(),
            description: None,
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let block = TimeBlock {
            id: block_id,
            routine_id,
            name: "Deep Work Arquitetura".to_string(),
            start_time: NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            end_time: NaiveTime::from_hms_opt(1, 0, 0).unwrap(),
            category: "Work".to_string(),
            color_hex: "#6366F1".to_string(),
            order_index: 0,
            created_at: Utc::now(),
        };
        let routine_repo = SqliteRoutineRepository::new(pool.clone());
        routine_repo
            .create_routine_with_blocks(user_id, &routine, &[block])
            .await
            .unwrap();

        let habit = Habit::new_binary("Meditação Mindfulness", None);
        let habit_id = habit.id;
        let habit_repo = SqliteHabitRepository::new(pool.clone());
        habit_repo.create_habit(user_id, &habit).await.unwrap();

        // 1. Hydrate time_block for owner -> returns actual block name
        let block_notif = hydrate_reminder(
            &pool,
            &user_id.to_string(),
            "time_block",
            &block_id.to_string(),
        )
        .await;
        assert_eq!(block_notif.title, "Rotina: Deep Work Arquitetura");
        assert!(block_notif.message.contains("Deep Work Arquitetura"));

        // 2. Hydrate time_block for different user -> tenant isolation prevents leaking block name
        let foreign_block_notif = hydrate_reminder(
            &pool,
            &other_user_id.to_string(),
            "time_block",
            &block_id.to_string(),
        )
        .await;
        assert_eq!(foreign_block_notif.title, "Rotina: Bloco de Rotina");

        // 3. Hydrate habit for owner -> returns actual habit name
        let habit_notif =
            hydrate_reminder(&pool, &user_id.to_string(), "habit", &habit_id.to_string()).await;
        assert_eq!(
            habit_notif.title,
            "Lembrete de Hábito: Meditação Mindfulness"
        );

        // 4. Catch-up missed reminders -> decodes BLOB UUIDs and tb.name without error
        let (tx, mut rx) = broadcast::channel(16);
        catch_up_missed_reminders(&pool, &tx).await;
        let caught_up = rx.try_recv().unwrap();
        assert_eq!(caught_up.user_id, user_id.to_string());
        assert_eq!(caught_up.entity_id, block_id.to_string());
        assert_eq!(caught_up.title, "Rotina Ativa: Deep Work Arquitetura");
    }
}
