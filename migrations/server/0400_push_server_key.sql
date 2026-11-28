CREATE TABLE push_server_key (
    singleton BOOLEAN PRIMARY KEY DEFAULT TRUE CHECK (singleton),
    public_key TEXT NOT NULL,
    private_sealed BYTEA NOT NULL,
    created_at_ms BIGINT NOT NULL
);
