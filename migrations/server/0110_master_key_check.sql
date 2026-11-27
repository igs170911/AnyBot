CREATE TABLE master_key_check (
    id SMALLINT PRIMARY KEY CHECK (id = 1),
    key_id TEXT NOT NULL,
    algorithm TEXT NOT NULL,
    check_value BYTEA NOT NULL,
    recorded_at_ms BIGINT NOT NULL
);
