CREATE TABLE pairing_codes (
    id TEXT PRIMARY KEY,
    account TEXT NOT NULL,
    algorithm TEXT NOT NULL,
    digest BYTEA NOT NULL,
    created_at_ms BIGINT NOT NULL,
    expires_at_ms BIGINT NOT NULL,
    used_at_ms BIGINT,
    redeemed_device TEXT,
    redeemed_address TEXT,
    UNIQUE (algorithm, digest)
);

CREATE INDEX pairing_codes_account_idx ON pairing_codes (account);

CREATE TABLE pairing_redeems (
    key TEXT NOT NULL,
    address TEXT NOT NULL,
    code_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    at_ms BIGINT NOT NULL,
    PRIMARY KEY (key, address)
);
