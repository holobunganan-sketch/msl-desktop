-- 0003_ai_provider_catalog.sql
-- 把旧的单模型 Provider 扩展为 connection / model / task route 三层，保持旧数据与 credential_ref。

ALTER TABLE provider_settings ADD COLUMN template_kind TEXT NOT NULL DEFAULT 'custom';
ALTER TABLE provider_settings ADD COLUMN auth_mode TEXT NOT NULL DEFAULT 'bearer';
ALTER TABLE provider_settings ADD COLUMN models_endpoint TEXT;
ALTER TABLE provider_settings ADD COLUMN last_models_refresh_at INTEGER;

CREATE TABLE IF NOT EXISTS provider_models (
  id               INTEGER PRIMARY KEY AUTOINCREMENT,
  provider_id      INTEGER NOT NULL REFERENCES provider_settings(id) ON DELETE CASCADE,
  model_id         TEXT NOT NULL,
  display_name     TEXT NOT NULL,
  protocol         TEXT NOT NULL CHECK (protocol IN ('chat_completions', 'responses', 'anthropic_messages')),
  endpoint_path    TEXT NOT NULL,
  capabilities_json TEXT NOT NULL DEFAULT '{}',
  source           TEXT NOT NULL CHECK (source IN ('legacy', 'template', 'remote', 'manual')),
  enabled          INTEGER NOT NULL DEFAULT 1,
  available        INTEGER NOT NULL DEFAULT 1,
  created_at       INTEGER NOT NULL,
  updated_at       INTEGER NOT NULL,
  UNIQUE(provider_id, model_id)
);

CREATE INDEX IF NOT EXISTS idx_provider_models_provider
  ON provider_models(provider_id);
CREATE INDEX IF NOT EXISTS idx_provider_models_enabled
  ON provider_models(provider_id, enabled, available);

CREATE TABLE IF NOT EXISTS ai_task_routes (
  task_kind        TEXT PRIMARY KEY CHECK (task_kind IN (
    'workspace_analysis', 'work_draft', 'global_analysis',
    'daily_brief', 'translation', 'general'
  )),
  provider_model_id INTEGER REFERENCES provider_models(id) ON DELETE SET NULL,
  updated_at       INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_ai_task_routes_model
  ON ai_task_routes(provider_model_id);

-- 旧 Provider 的非空 model 迁移为 legacy Chat Completions 模型。
INSERT INTO provider_models (
  provider_id, model_id, display_name, protocol, endpoint_path,
  capabilities_json, source, enabled, available, created_at, updated_at
)
SELECT
  p.id,
  p.model,
  p.model,
  'chat_completions',
  '/chat/completions',
  '{}',
  'legacy',
  p.enabled,
  1,
  p.created_at,
  p.updated_at
FROM provider_settings p
WHERE trim(p.model) <> ''
  AND NOT EXISTS (
    SELECT 1 FROM provider_models m
    WHERE m.provider_id = p.id AND m.model_id = p.model
  );

UPDATE provider_settings
SET template_kind = CASE
  WHEN rtrim(lower(trim(base_url)), '/') = 'https://api.deepseek.com' THEN 'deepseek'
  ELSE 'custom'
END
WHERE template_kind = 'custom';
