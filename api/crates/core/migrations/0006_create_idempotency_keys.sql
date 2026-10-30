-- 0006_create_idempotency_keys.sql
-- Transactional Idempotency Storage (RF05)

CREATE TABLE IF NOT EXISTS idempotency_keys (
    key TEXT NOT NULL,
    user_id TEXT NOT NULL,
    request_path TEXT NOT NULL,
    request_hash BLOB NOT NULL, -- SHA-256 of request body
    response_status INTEGER,    -- NULL while in-flight
    response_headers TEXT,      -- JSON-encoded headers
    response_body BLOB,
    created_at TEXT NOT NULL,
    locked_until TEXT NOT NULL,
    PRIMARY KEY (user_id, key)
);

CREATE INDEX IF NOT EXISTS idx_idempotency_keys_locked ON idempotency_keys(locked_until);
