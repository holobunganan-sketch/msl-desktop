-- Homepage decision deferral and model-generated periodic reports.

ALTER TABLE ai_proposals ADD COLUMN deferred_at INTEGER;
CREATE INDEX IF NOT EXISTS idx_ai_proposals_recent
  ON ai_proposals(created_at DESC, status, analysis_run_id);

-- Extend the task router while preserving every configured route.
DROP INDEX IF EXISTS idx_ai_task_routes_model;
ALTER TABLE ai_task_routes RENAME TO ai_task_routes_v6;
CREATE TABLE ai_task_routes (
  task_kind TEXT PRIMARY KEY CHECK (task_kind IN (
    'workspace_analysis', 'work_draft', 'global_analysis', 'daily_brief',
    'weekly_report', 'monthly_report', 'translation', 'general'
  )),
  provider_model_id INTEGER REFERENCES provider_models(id) ON DELETE SET NULL,
  updated_at INTEGER NOT NULL
);
INSERT INTO ai_task_routes(task_kind,provider_model_id,updated_at)
  SELECT task_kind,provider_model_id,updated_at FROM ai_task_routes_v6;
DROP TABLE ai_task_routes_v6;
CREATE INDEX idx_ai_task_routes_model ON ai_task_routes(provider_model_id);

CREATE TABLE IF NOT EXISTS reports (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  kind TEXT NOT NULL CHECK (kind IN ('weekly','monthly')),
  period_start INTEGER NOT NULL,
  period_end INTEGER NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('running','completed','failed')),
  provider_model_id INTEGER REFERENCES provider_models(id) ON DELETE SET NULL,
  content TEXT,
  snapshot_hash TEXT,
  source_counts_json TEXT NOT NULL DEFAULT '{}',
  source_report_ids_json TEXT NOT NULL DEFAULT '[]',
  error_code TEXT,
  error_message TEXT,
  retention_state TEXT NOT NULL DEFAULT 'kept'
    CHECK (retention_state IN ('kept','superseded')),
  generated_at INTEGER,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  CHECK (period_end > period_start)
);
CREATE INDEX IF NOT EXISTS idx_reports_kind_period
  ON reports(kind, period_start DESC, period_end DESC);
CREATE INDEX IF NOT EXISTS idx_reports_status_time
  ON reports(status, updated_at DESC);

CREATE TABLE IF NOT EXISTS report_schedule_state (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  weekly_enabled INTEGER NOT NULL DEFAULT 1,
  weekly_weekday INTEGER NOT NULL DEFAULT 6 CHECK (weekly_weekday BETWEEN 0 AND 6),
  weekly_hour INTEGER NOT NULL DEFAULT 17 CHECK (weekly_hour BETWEEN 0 AND 23),
  weekly_minute INTEGER NOT NULL DEFAULT 0 CHECK (weekly_minute BETWEEN 0 AND 59),
  last_weekly_period_key TEXT,
  monthly_enabled INTEGER NOT NULL DEFAULT 1,
  monthly_day INTEGER NOT NULL DEFAULT 1 CHECK (monthly_day BETWEEN 1 AND 28),
  monthly_hour INTEGER NOT NULL DEFAULT 9 CHECK (monthly_hour BETWEEN 0 AND 23),
  monthly_minute INTEGER NOT NULL DEFAULT 0 CHECK (monthly_minute BETWEEN 0 AND 59),
  last_monthly_period_key TEXT,
  updated_at INTEGER NOT NULL
);

INSERT OR IGNORE INTO report_schedule_state (
  id, weekly_enabled, weekly_weekday, weekly_hour, weekly_minute,
  monthly_enabled, monthly_day, monthly_hour, monthly_minute, updated_at
) VALUES (1, 1, 6, 17, 0, 1, 1, 9, 0, strftime('%s','now'));
