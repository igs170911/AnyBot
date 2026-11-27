CREATE TABLE two_step (
    account TEXT PRIMARY KEY,
    secret_sealed BYTEA,
    enabled_at_ms BIGINT,
    pending_sealed BYTEA,
    pending_at_ms BIGINT
);

CREATE TABLE two_step_recovery (
    account TEXT NOT NULL,
    pending BOOLEAN NOT NULL,
    slot SMALLINT NOT NULL,
    algorithm TEXT NOT NULL,
    digest BYTEA NOT NULL,
    used_at_ms BIGINT,
    PRIMARY KEY (account, pending, slot)
);

CREATE TABLE two_step_used_steps (
    account TEXT NOT NULL,
    step BIGINT NOT NULL,
    used_at_ms BIGINT NOT NULL,
    PRIMARY KEY (account, step)
);

CREATE TABLE two_step_challenges (
    id_hash BYTEA PRIMARY KEY,
    account TEXT NOT NULL,
    credential TEXT NOT NULL,
    label TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    tries INTEGER NOT NULL DEFAULT 0,
    state TEXT NOT NULL DEFAULT 'open',
    passed_at_ms BIGINT
);

CREATE INDEX two_step_challenges_account ON two_step_challenges (account);
