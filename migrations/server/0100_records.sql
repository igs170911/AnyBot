CREATE TABLE bots (
    id TEXT PRIMARY KEY,
    account TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    deleted BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX bots_account_idx ON bots (account);

CREATE TABLE record_streams (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    owner TEXT NOT NULL,
    bot_id TEXT,
    next_seq BIGINT NOT NULL DEFAULT 1,
    created_at_ms BIGINT NOT NULL,
    UNIQUE (kind, owner),
    UNIQUE (bot_id)
);

CREATE TABLE records (
    stream_id TEXT NOT NULL,
    seq BIGINT NOT NULL,
    device_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    offline_sent BOOLEAN NOT NULL DEFAULT FALSE,
    task_id TEXT,
    task_seq BIGINT,
    purged BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (stream_id, seq),
    UNIQUE (stream_id, device_id)
);

CREATE INDEX records_stream_idx ON records (stream_id, seq);

CREATE INDEX records_task_idx ON records (task_id, task_seq);

CREATE TABLE task_counters (
    task_id TEXT PRIMARY KEY,
    next_seq BIGINT NOT NULL DEFAULT 1
);
