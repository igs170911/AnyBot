CREATE TABLE mailbox_letters (
    letter_id TEXT PRIMARY KEY,
    account TEXT NOT NULL,
    kind TEXT NOT NULL,
    device_id TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    delivered_at_ms BIGINT,
    state TEXT NOT NULL,
    claimed_by TEXT,
    claimed_at_ms BIGINT,
    reassigned_to TEXT,
    reassign_reason TEXT,
    closed_at_ms BIGINT
);

CREATE INDEX mailbox_letters_device_idx ON mailbox_letters (device_id, state);

CREATE INDEX mailbox_letters_state_idx ON mailbox_letters (state, kind);

CREATE TABLE card_issuers (
    stream_id TEXT NOT NULL,
    card_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    noted_at_ms BIGINT NOT NULL,
    PRIMARY KEY (stream_id, card_id)
);
