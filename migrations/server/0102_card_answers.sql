CREATE TABLE card_answers (
    stream_id TEXT NOT NULL,
    card_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    answer TEXT NOT NULL,
    seq BIGINT NOT NULL,
    PRIMARY KEY (stream_id, card_id)
);
