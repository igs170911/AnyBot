ALTER TABLE outbox ADD COLUMN local_detail TEXT;
ALTER TABLE outbox ADD COLUMN turn INTEGER;
ALTER TABLE outbox ADD COLUMN effect TEXT;

CREATE INDEX outbox_by_task_turn ON outbox (task_id, turn, local_id);
