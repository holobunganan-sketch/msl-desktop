-- Device lifecycle, durable conflict decisions and per-device workspace paths.
CREATE TABLE sync_devices (
  device_id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL DEFAULT '',
  active INTEGER NOT NULL DEFAULT 1,
  first_seen_at INTEGER NOT NULL,
  last_seen_at INTEGER NOT NULL,
  acknowledged_at INTEGER
);

CREATE TABLE sync_checkpoints (
  id TEXT PRIMARY KEY,
  dataset_id TEXT NOT NULL,
  generation TEXT NOT NULL,
  created_by TEXT NOT NULL,
  sequence INTEGER NOT NULL,
  state_hash TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  UNIQUE(dataset_id, generation, created_by, sequence)
);

CREATE TABLE sync_workspace_bindings (
  workspace_uid TEXT PRIMARY KEY,
  local_root TEXT NOT NULL,
  available INTEGER NOT NULL DEFAULT 1,
  updated_at INTEGER NOT NULL
);

CREATE TABLE sync_feedback_events (
  event_uid TEXT PRIMARY KEY,
  proposal_uid TEXT NOT NULL,
  decision TEXT NOT NULL,
  preferred_kind TEXT,
  reason_code TEXT,
  device_id TEXT NOT NULL,
  occurred_at INTEGER NOT NULL
);

CREATE INDEX idx_sync_conflicts_open
  ON sync_conflicts(resolved_at, created_at DESC);
CREATE INDEX idx_sync_devices_active
  ON sync_devices(active, last_seen_at DESC);

