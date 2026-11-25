CREATE TABLE schema_marker (
    app TEXT PRIMARY KEY
);

INSERT INTO schema_marker (app)
VALUES
    ('anybot.server');

CREATE TABLE server_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE accounts (
    account TEXT PRIMARY KEY,
    password_hash TEXT NOT NULL,
    admin BOOLEAN NOT NULL DEFAULT FALSE,
    disabled BOOLEAN NOT NULL DEFAULT FALSE,
    created_at_ms BIGINT NOT NULL,
    last_password_at_ms BIGINT NOT NULL
);

CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    account TEXT NOT NULL,
    key_hash TEXT NOT NULL,
    label TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    last_active_ms BIGINT NOT NULL,
    last_password_at_ms BIGINT NOT NULL
);

CREATE INDEX sessions_account_idx ON sessions (account);

CREATE INDEX sessions_last_active_idx ON sessions (account, last_active_ms);

CREATE TABLE auth_events (
    kind TEXT NOT NULL,
    address TEXT NOT NULL,
    account TEXT NOT NULL,
    at_ms BIGINT NOT NULL
);

CREATE INDEX auth_events_window_idx ON auth_events (kind, account, at_ms);

CREATE INDEX auth_events_address_idx ON auth_events (kind, address, at_ms);

CREATE TABLE idempotency (
    key TEXT PRIMARY KEY,
    request_hash TEXT NOT NULL,
    status_code INT NOT NULL,
    body TEXT NOT NULL,
    at_ms BIGINT NOT NULL
);
