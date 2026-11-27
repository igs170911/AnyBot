CREATE TABLE computers (
    device_id TEXT PRIMARY KEY,
    account TEXT NOT NULL,
    name TEXT NOT NULL,
    key_hash TEXT NOT NULL UNIQUE,
    paired BOOLEAN NOT NULL,
    delegate_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    created_at_ms BIGINT NOT NULL,
    last_seen_ms BIGINT NOT NULL,
    password_entered_ms BIGINT NOT NULL,
    credential_used_ms BIGINT,
    online BOOLEAN NOT NULL DEFAULT TRUE,
    offline_at_ms BIGINT,
    offline_reason TEXT,
    removed_at_ms BIGINT
);

CREATE INDEX computers_account_idx ON computers (account);
