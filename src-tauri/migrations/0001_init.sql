-- 0001_init.sql — MSL Desktop 初始 schema（开发指南 §6 数据模型）
-- 所有时间字段使用 INTEGER：Unix epoch 秒（UTC）。
-- 所有 status/kind 等枚举字段使用 TEXT，合法值由应用层校验。

-- 6.1 workspaces
CREATE TABLE IF NOT EXISTS workspaces (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  name       TEXT NOT NULL,
  root_path  TEXT NOT NULL,
  enabled    INTEGER NOT NULL DEFAULT 1,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

-- 6.2 works（status: active|paused|waiting|done|archived）
CREATE TABLE IF NOT EXISTS works (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  title       TEXT NOT NULL,
  status      TEXT NOT NULL DEFAULT 'active',
  summary     TEXT,
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL,
  archived_at INTEGER
);

-- 6.3 resume_points（source: manual|ai_draft_confirmed）
CREATE TABLE IF NOT EXISTS resume_points (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  work_id       INTEGER NOT NULL REFERENCES works(id),
  current_state TEXT NOT NULL DEFAULT '',
  next_step     TEXT NOT NULL DEFAULT '',
  remember      TEXT NOT NULL DEFAULT '',
  source        TEXT NOT NULL DEFAULT 'manual',
  created_at    INTEGER NOT NULL
);

-- 6.4 work_file_refs（只存路径引用，不复制真实文件）
CREATE TABLE IF NOT EXISTS work_file_refs (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  work_id      INTEGER NOT NULL REFERENCES works(id),
  workspace_id INTEGER REFERENCES workspaces(id),
  path         TEXT NOT NULL,
  label        TEXT,
  pinned       INTEGER NOT NULL DEFAULT 0,
  created_at   INTEGER NOT NULL
);

-- 6.5 tasks（status: next|scheduled|waiting|paused|done）
CREATE TABLE IF NOT EXISTS tasks (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  work_id         INTEGER REFERENCES works(id),
  title           TEXT NOT NULL,
  status          TEXT NOT NULL DEFAULT 'next',
  priority        TEXT NOT NULL DEFAULT 'normal',
  due_at          INTEGER,
  scheduled_start INTEGER,
  scheduled_end   INTEGER,
  notes           TEXT,
  created_at      INTEGER NOT NULL,
  updated_at      INTEGER NOT NULL,
  completed_at    INTEGER
);

-- 6.6 waiting_items（status: open|resolved）
CREATE TABLE IF NOT EXISTS waiting_items (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  work_id      INTEGER REFERENCES works(id),
  title        TEXT NOT NULL,
  waiting_for  TEXT NOT NULL DEFAULT '',
  started_at   INTEGER NOT NULL,
  follow_up_at INTEGER,
  status       TEXT NOT NULL DEFAULT 'open',
  notes        TEXT,
  created_at   INTEGER NOT NULL,
  updated_at   INTEGER NOT NULL,
  resolved_at  INTEGER
);

-- 6.7 inbox_items
CREATE TABLE IF NOT EXISTS inbox_items (
  id                INTEGER PRIMARY KEY AUTOINCREMENT,
  content           TEXT NOT NULL,
  created_at        INTEGER NOT NULL,
  processed_at      INTEGER,
  converted_to_type TEXT,
  converted_to_id   INTEGER
);

-- 6.8 calendar_events（kind: meeting|kol_visit|deadline|travel|work_block|other）
CREATE TABLE IF NOT EXISTS calendar_events (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  work_id    INTEGER REFERENCES works(id),
  title      TEXT NOT NULL,
  start_at   INTEGER NOT NULL,
  end_at     INTEGER,
  all_day    INTEGER NOT NULL DEFAULT 0,
  location   TEXT,
  notes      TEXT,
  kind       TEXT NOT NULL DEFAULT 'other',
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

-- 6.9 activity_events（event_type 示例见指南 §6.9）
CREATE TABLE IF NOT EXISTS activity_events (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  timestamp     INTEGER NOT NULL,
  event_type    TEXT NOT NULL,
  workspace_id  INTEGER REFERENCES workspaces(id),
  work_id       INTEGER REFERENCES works(id),
  entity_type   TEXT,
  entity_id     INTEGER,
  path          TEXT,
  display_text  TEXT NOT NULL DEFAULT '',
  metadata_json TEXT,
  dedupe_key    TEXT
);

-- 6.10 daily_briefs（brief_date 使用 YYYY-MM-DD）
CREATE TABLE IF NOT EXISTS daily_briefs (
  id                   INTEGER PRIMARY KEY AUTOINCREMENT,
  brief_date           TEXT NOT NULL,
  generated_at         INTEGER NOT NULL,
  provider_id          TEXT,
  content              TEXT NOT NULL,
  source_snapshot_hash TEXT
);

-- 6.11 provider_settings（API Key 只存 keyring reference，不落库明文）
CREATE TABLE IF NOT EXISTS provider_settings (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  display_name TEXT NOT NULL,
  provider_type TEXT NOT NULL,
  base_url     TEXT NOT NULL DEFAULT '',
  model        TEXT NOT NULL DEFAULT '',
  enabled      INTEGER NOT NULL DEFAULT 0,
  created_at   INTEGER NOT NULL,
  updated_at   INTEGER NOT NULL
);

-- app_settings（通用 key-value）
CREATE TABLE IF NOT EXISTS app_settings (
  key        TEXT PRIMARY KEY,
  value      TEXT,
  updated_at INTEGER NOT NULL
);

-- 常用索引
CREATE INDEX IF NOT EXISTS idx_works_status          ON works(status);
CREATE INDEX IF NOT EXISTS idx_resume_points_work    ON resume_points(work_id, created_at);
CREATE INDEX IF NOT EXISTS idx_work_file_refs_work   ON work_file_refs(work_id);
CREATE INDEX IF NOT EXISTS idx_tasks_work            ON tasks(work_id);
CREATE INDEX IF NOT EXISTS idx_tasks_status          ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_due             ON tasks(due_at);
CREATE INDEX IF NOT EXISTS idx_waiting_status        ON waiting_items(status);
CREATE INDEX IF NOT EXISTS idx_waiting_followup      ON waiting_items(follow_up_at);
CREATE INDEX IF NOT EXISTS idx_calendar_start        ON calendar_events(start_at);
CREATE INDEX IF NOT EXISTS idx_activity_timestamp    ON activity_events(timestamp);
CREATE INDEX IF NOT EXISTS idx_activity_event_type   ON activity_events(event_type);
CREATE INDEX IF NOT EXISTS idx_activity_work         ON activity_events(work_id);
CREATE INDEX IF NOT EXISTS idx_activity_path         ON activity_events(path);
CREATE INDEX IF NOT EXISTS idx_daily_briefs_date     ON daily_briefs(brief_date);
