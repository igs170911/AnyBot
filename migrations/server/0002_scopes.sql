ALTER TABLE auth_events ADD COLUMN id BIGSERIAL PRIMARY KEY;

ALTER TABLE idempotency DROP CONSTRAINT idempotency_pkey;

ALTER TABLE idempotency ADD COLUMN scope TEXT NOT NULL;

ALTER TABLE idempotency ADD PRIMARY KEY (key, scope);

CREATE INDEX auth_events_account_order_idx ON auth_events (kind, account, id);
