-- AI secretary scheduling, analysis runs and confirmation queue.
CREATE TABLE IF NOT EXISTS analysis_schedule_state (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  enabled INTEGER NOT NULL DEFAULT 1,
  interval_minutes INTEGER NOT NULL DEFAULT 180 CHECK (interval_minutes BETWEEN 30 AND 1440),
  daily_enabled INTEGER NOT NULL DEFAULT 1,
  daily_hour INTEGER NOT NULL DEFAULT 6 CHECK (daily_hour BETWEEN 0 AND 23),
  daily_minute INTEGER NOT NULL DEFAULT 0 CHECK (daily_minute BETWEEN 0 AND 59),
  last_interval_run_at INTEGER,
  last_daily_local_date TEXT,
  updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS analysis_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  trigger TEXT NOT NULL,
  status TEXT NOT NULL,
  period_start INTEGER,
  period_end INTEGER,
  provider_model_id INTEGER REFERENCES provider_models(id) ON DELETE SET NULL,
  started_at INTEGER NOT NULL,
  finished_at INTEGER,
  source_counts_json TEXT,
  snapshot_hash TEXT,
  summary TEXT,
  brief_id INTEGER REFERENCES daily_briefs(id) ON DELETE SET NULL,
  error_code TEXT,
  error_message TEXT,
  created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_analysis_runs_status_time ON analysis_runs(status, started_at);

CREATE TABLE IF NOT EXISTS ai_proposals (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  analysis_run_id INTEGER REFERENCES analysis_runs(id) ON DELETE SET NULL,
  kind TEXT NOT NULL,
  operation TEXT NOT NULL,
  target_id INTEGER,
  work_id INTEGER REFERENCES works(id) ON DELETE SET NULL,
  workspace_id INTEGER REFERENCES workspaces(id) ON DELETE SET NULL,
  dedupe_key TEXT NOT NULL,
  title TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  reason TEXT NOT NULL DEFAULT '',
  source_refs_json TEXT NOT NULL DEFAULT '[]',
  confidence REAL,
  user_edited INTEGER NOT NULL DEFAULT 0,
  status TEXT NOT NULL DEFAULT 'pending',
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  decided_at INTEGER
);
CREATE INDEX IF NOT EXISTS idx_ai_proposals_status ON ai_proposals(status, updated_at);
CREATE INDEX IF NOT EXISTS idx_ai_proposals_scope ON ai_proposals(workspace_id, work_id);
CREATE INDEX IF NOT EXISTS idx_ai_proposals_run ON ai_proposals(analysis_run_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_ai_proposals_pending_dedupe
  ON ai_proposals(dedupe_key) WHERE status = 'pending';

ALTER TABLE daily_briefs ADD COLUMN retention_state TEXT NOT NULL DEFAULT 'kept';
ALTER TABLE daily_briefs ADD COLUMN analysis_run_id INTEGER REFERENCES analysis_runs(id) ON DELETE SET NULL;
ALTER TABLE daily_briefs ADD COLUMN superseded_at INTEGER;
INSERT OR IGNORE INTO analysis_schedule_state
  (id, enabled, interval_minutes, daily_enabled, daily_hour, daily_minute, updated_at)
VALUES (1, 1, 180, 1, 6, 0, strftime('%s','now'));
