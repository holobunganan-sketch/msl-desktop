-- Durable local metadata for portable, record-aware folder synchronization.
-- Paths and credentials deliberately stay outside these portable tables.
CREATE TABLE sync_local_state (
  id INTEGER PRIMARY KEY CHECK(id = 1),
  device_id TEXT NOT NULL,
  dataset_id TEXT NOT NULL DEFAULT '',
  generation TEXT NOT NULL DEFAULT '',
  next_seq INTEGER NOT NULL DEFAULT 1,
  suppress_capture INTEGER NOT NULL DEFAULT 0,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);
INSERT INTO sync_local_state(id,device_id,created_at,updated_at)
VALUES(1,lower(hex(randomblob(16))),strftime('%s','now'),strftime('%s','now'));

CREATE TABLE sync_entities (
  table_name TEXT NOT NULL,
  local_key TEXT NOT NULL,
  entity_uid TEXT NOT NULL,
  deleted INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY(table_name,local_key),
  UNIQUE(table_name,entity_uid)
);

CREATE TABLE sync_row_versions (
  table_name TEXT NOT NULL,
  row_key TEXT NOT NULL,
  row_json TEXT NOT NULL,
  field_versions_json TEXT NOT NULL DEFAULT '{}',
  deleted INTEGER NOT NULL DEFAULT 0,
  edited_at_ms INTEGER NOT NULL,
  device_id TEXT NOT NULL,
  seq INTEGER NOT NULL,
  PRIMARY KEY(table_name,row_key)
);

CREATE TABLE sync_outbox (
  change_id TEXT PRIMARY KEY,
  dataset_id TEXT NOT NULL,
  generation TEXT NOT NULL,
  device_id TEXT NOT NULL,
  seq INTEGER NOT NULL,
  payload_json TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  published_at INTEGER,
  UNIQUE(device_id,seq)
);

CREATE TABLE sync_applied (
  change_id TEXT PRIMARY KEY,
  device_id TEXT NOT NULL,
  seq INTEGER NOT NULL,
  applied_at INTEGER NOT NULL,
  UNIQUE(device_id,seq)
);

CREATE TABLE sync_conflicts (
  id TEXT PRIMARY KEY,
  table_name TEXT NOT NULL,
  row_key TEXT NOT NULL,
  field TEXT NOT NULL,
  local_value TEXT NOT NULL,
  remote_value TEXT NOT NULL,
  base_value TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  resolved_at INTEGER,
  resolution TEXT
);

CREATE TABLE sync_peer_rows (
  table_name TEXT NOT NULL,
  row_key TEXT NOT NULL,
  device_id TEXT NOT NULL,
  row_json TEXT NOT NULL,
  field_versions_json TEXT NOT NULL DEFAULT '{}',
  deleted INTEGER NOT NULL,
  seen_at INTEGER NOT NULL,
  PRIMARY KEY(table_name,row_key,device_id)
);

CREATE INDEX sync_outbox_unpublished ON sync_outbox(published_at,seq);
CREATE INDEX sync_conflicts_open ON sync_conflicts(resolved_at,created_at);
