CREATE TABLE runner_service_device (
    singleton BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (singleton),
    device_id TEXT NOT NULL,
    key_hash TEXT NOT NULL UNIQUE,
    key_sealed BYTEA NOT NULL,
    created_at_ms BIGINT NOT NULL
);
