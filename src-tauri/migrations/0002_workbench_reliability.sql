-- 0002_workbench_reliability.sql
-- 追加可靠性字段；不修改或删除 0001 中的表和数据。

ALTER TABLE provider_settings ADD COLUMN credential_ref TEXT;
UPDATE provider_settings
SET credential_ref = 'provider-' || id
WHERE credential_ref IS NULL OR trim(credential_ref) = '';
CREATE UNIQUE INDEX IF NOT EXISTS idx_provider_credential_ref
  ON provider_settings(credential_ref);

CREATE TABLE IF NOT EXISTS workspace_file_state (
  workspace_id INTEGER NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  path         TEXT NOT NULL,
  modified_at  INTEGER NOT NULL,
  size         INTEGER NOT NULL,
  seen_at      INTEGER NOT NULL,
  PRIMARY KEY (workspace_id, path)
);
CREATE INDEX IF NOT EXISTS idx_workspace_file_state_workspace
  ON workspace_file_state(workspace_id);
CREATE INDEX IF NOT EXISTS idx_workspace_file_state_modified
  ON workspace_file_state(modified_at);

ALTER TABLE daily_briefs ADD COLUMN period_start INTEGER;
ALTER TABLE daily_briefs ADD COLUMN period_end INTEGER;
ALTER TABLE daily_briefs ADD COLUMN locale TEXT;
ALTER TABLE daily_briefs ADD COLUMN source_snapshot_json TEXT;
ALTER TABLE daily_briefs ADD COLUMN ai_used INTEGER NOT NULL DEFAULT 0;
ALTER TABLE daily_briefs ADD COLUMN warning TEXT;
