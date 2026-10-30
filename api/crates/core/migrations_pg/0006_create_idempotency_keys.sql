-- 0006_create_idempotency_keys.sql
-- Transactional Idempotency Storage (RF05)

CREATE TABLE IF NOT EXISTS idempotency_keys (
    key TEXT NOT NULL,
    user_id TEXT NOT NULL,
    request_path TEXT NOT NULL,
    request_hash BYTEA NOT NULL,
    response_status INTEGER,
    response_headers TEXT,
    response_body BYTEA,
    created_at TIMESTAMPTZ NOT NULL,
    locked_until TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (user_id, key)
);

CREATE INDEX IF NOT EXISTS idx_pg_idempotency_keys_locked ON idempotency_keys(locked_until);
