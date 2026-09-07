CREATE TABLE IF NOT EXISTS daily_activity_rollups (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  rollup_date TEXT NOT NULL,
  workspace_id INTEGER REFERENCES workspaces(id) ON DELETE SET NULL,
  work_id INTEGER REFERENCES works(id) ON DELETE SET NULL,
  event_type TEXT NOT NULL,
  event_count INTEGER NOT NULL DEFAULT 0,
  first_at INTEGER NOT NULL,
  last_at INTEGER NOT NULL,
  sample_text TEXT NOT NULL DEFAULT '',
  created_at INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_daily_activity_rollups_unique
  ON daily_activity_rollups(rollup_date, COALESCE(workspace_id,0), COALESCE(work_id,0), event_type);
CREATE INDEX IF NOT EXISTS idx_daily_activity_rollups_date ON daily_activity_rollups(rollup_date);

CREATE TABLE IF NOT EXISTS storage_cleanup_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  trigger TEXT NOT NULL,
  status TEXT NOT NULL,
  started_at INTEGER NOT NULL,
  finished_at INTEGER,
  bytes_before INTEGER NOT NULL DEFAULT 0,
  bytes_after INTEGER NOT NULL DEFAULT 0,
  deleted_counts_json TEXT NOT NULL DEFAULT '{}',
  error_code TEXT,
  error_message TEXT
);
CREATE INDEX IF NOT EXISTS idx_storage_cleanup_runs_time ON storage_cleanup_runs(started_at);

INSERT OR IGNORE INTO app_settings(key,value,updated_at) VALUES
 ('cache_auto_enabled','true',strftime('%s','now')),
 ('cache_limit_bytes','2147483648',strftime('%s','now')),
 ('cache_high_water_percent','80',strftime('%s','now')),
 ('cache_target_percent','60',strftime('%s','now')),
 ('cache_last_auto_cleanup_at',NULL,strftime('%s','now'));
