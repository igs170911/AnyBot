CREATE TABLE streams (
    stream TEXT PRIMARY KEY NOT NULL,
    through_seq INTEGER NOT NULL DEFAULT 0,
    through_record_id TEXT NOT NULL DEFAULT ''
);

CREATE TABLE records (
    stream TEXT NOT NULL,
    seq INTEGER NOT NULL,
    record_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    offline_sent INTEGER NOT NULL,
    task_id TEXT,
    task_seq INTEGER,
    purged INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (stream, seq)
);

CREATE INDEX records_by_record_id ON records (stream, record_id);

CREATE TABLE outbox (
    local_id INTEGER PRIMARY KEY AUTOINCREMENT,
    stream TEXT NOT NULL,
    record_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    task_id TEXT,
    offline_at_create INTEGER NOT NULL,
    attempted_at_ms INTEGER,
    offline_sent INTEGER,
    state TEXT NOT NULL,
    reject_code TEXT,
    rejected_at_ms INTEGER,
    UNIQUE (stream, record_id)
);

CREATE INDEX outbox_by_state ON outbox (state, stream, local_id);
