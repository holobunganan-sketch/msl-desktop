-- Durable, local classification feedback memory for AI proposal review.

ALTER TABLE ai_proposals ADD COLUMN suggested_kind TEXT NOT NULL DEFAULT '';
ALTER TABLE ai_proposals ADD COLUMN suggested_work_id INTEGER REFERENCES works(id) ON DELETE SET NULL;

UPDATE ai_proposals
SET suggested_kind = kind,
    suggested_work_id = work_id
WHERE suggested_kind = '';

CREATE TABLE IF NOT EXISTS classification_memories (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  fingerprint TEXT NOT NULL UNIQUE,
  cue_text TEXT NOT NULL,
  suggested_kind TEXT NOT NULL,
  preferred_kind TEXT,
  suggested_work_id INTEGER REFERENCES works(id) ON DELETE SET NULL,
  preferred_work_id INTEGER REFERENCES works(id) ON DELETE SET NULL,
  source_types_json TEXT NOT NULL DEFAULT '[]',
  payload_keys_json TEXT NOT NULL DEFAULT '[]',
  positive_count INTEGER NOT NULL DEFAULT 0,
  negative_count INTEGER NOT NULL DEFAULT 0,
  correction_count INTEGER NOT NULL DEFAULT 0,
  last_feedback TEXT NOT NULL CHECK (last_feedback IN ('accepted','corrected','rejected')),
  last_proposal_id INTEGER,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_classification_memories_rank
  ON classification_memories(updated_at DESC, correction_count DESC, positive_count DESC);
