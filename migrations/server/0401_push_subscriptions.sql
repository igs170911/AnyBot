CREATE TABLE push_subscriptions (
    id TEXT PRIMARY KEY,
    account TEXT NOT NULL,
    device_id TEXT NOT NULL,
    endpoint TEXT NOT NULL UNIQUE,
    p256dh TEXT NOT NULL,
    auth_sealed BYTEA NOT NULL,
    created_at_ms BIGINT NOT NULL
);

CREATE INDEX push_subscriptions_account_idx ON push_subscriptions (account, created_at_ms);
