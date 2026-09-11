-- Pending edits share the business transaction, including cascades and rollback.
CREATE TABLE sync_pending_edits (
  seq INTEGER PRIMARY KEY,
  table_name TEXT NOT NULL,
  row_key TEXT NOT NULL,
  before_json TEXT,
  after_json TEXT,
  occurred_at_ms INTEGER NOT NULL
);
CREATE INDEX sync_pending_row ON sync_pending_edits(table_name,row_key,seq);
