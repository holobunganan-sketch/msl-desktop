-- Canonical versioned documents for readable AI output. Legacy text columns remain
-- available and are populated from the canonical document by application code.
CREATE TABLE ai_readable_documents (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  owner_kind TEXT NOT NULL,
  owner_id TEXT NOT NULL,
  schema_version TEXT NOT NULL,
  schema_hash TEXT NOT NULL,
  document_json TEXT NOT NULL,
  input_fingerprint TEXT NOT NULL DEFAULT '',
  prompt_version TEXT NOT NULL DEFAULT '',
  model_description TEXT NOT NULL DEFAULT '',
  revision INTEGER NOT NULL DEFAULT 1,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  UNIQUE(owner_kind, owner_id)
);

CREATE INDEX idx_ai_readable_documents_updated
  ON ai_readable_documents(updated_at DESC);

