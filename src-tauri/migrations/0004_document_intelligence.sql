-- 0004_document_intelligence.sql
-- 工作目录正文索引与可重建缓存清单；正文不进入 SQLite。

CREATE TABLE IF NOT EXISTS work_workspace_links (
  work_id      INTEGER NOT NULL REFERENCES works(id) ON DELETE CASCADE,
  workspace_id INTEGER NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  is_primary   INTEGER NOT NULL DEFAULT 0,
  created_at   INTEGER NOT NULL,
  PRIMARY KEY (work_id, workspace_id)
);
CREATE INDEX IF NOT EXISTS idx_work_workspace_links_workspace
  ON work_workspace_links(workspace_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_work_workspace_links_primary
  ON work_workspace_links(work_id) WHERE is_primary = 1;

CREATE TABLE IF NOT EXISTS document_index (
  id                 INTEGER PRIMARY KEY AUTOINCREMENT,
  workspace_id       INTEGER NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  path               TEXT NOT NULL,
  relative_path      TEXT NOT NULL,
  extension          TEXT NOT NULL DEFAULT '',
  size               INTEGER NOT NULL,
  modified_at        INTEGER NOT NULL,
  content_hash       TEXT,
  extract_status     TEXT NOT NULL DEFAULT 'pending',
  char_count         INTEGER NOT NULL DEFAULT 0,
  cache_rel_path     TEXT,
  summary            TEXT,
  summary_hash       TEXT,
  summary_model_id   INTEGER REFERENCES provider_models(id) ON DELETE SET NULL,
  last_extracted_at  INTEGER,
  last_analyzed_at   INTEGER,
  last_accessed_at   INTEGER,
  error_code         TEXT,
  error_message      TEXT,
  UNIQUE(workspace_id, path)
);
CREATE INDEX IF NOT EXISTS idx_document_index_workspace
  ON document_index(workspace_id, extract_status);
CREATE INDEX IF NOT EXISTS idx_document_index_hash
  ON document_index(content_hash);

CREATE TABLE IF NOT EXISTS cache_entries (
  id             INTEGER PRIMARY KEY AUTOINCREMENT,
  category       TEXT NOT NULL,
  relative_path  TEXT NOT NULL UNIQUE,
  content_hash   TEXT,
  size_bytes     INTEGER NOT NULL DEFAULT 0,
  rebuildable    INTEGER NOT NULL DEFAULT 1,
  owner_type     TEXT,
  owner_id       INTEGER,
  created_at     INTEGER NOT NULL,
  last_accessed_at INTEGER NOT NULL,
  expires_at     INTEGER
);
CREATE INDEX IF NOT EXISTS idx_cache_entries_category
  ON cache_entries(category, last_accessed_at);
CREATE INDEX IF NOT EXISTS idx_cache_entries_owner
  ON cache_entries(owner_type, owner_id);
