CREATE TABLE manual_completion_receipts (
 id TEXT PRIMARY KEY, entity_kind TEXT NOT NULL, entity_id INTEGER NOT NULL,
 title TEXT NOT NULL, before_json TEXT NOT NULL, after_json TEXT NOT NULL,
 created_at INTEGER NOT NULL, undone_at INTEGER
);
ALTER TABLE qa_sessions ADD COLUMN expert_id INTEGER REFERENCES kol_experts(id) ON DELETE SET NULL;
ALTER TABLE qa_sessions ADD COLUMN expert_scoped INTEGER NOT NULL DEFAULT 0;
ALTER TABLE qa_sessions ADD COLUMN expert_label TEXT;
ALTER TABLE qa_turns ADD COLUMN expert_id INTEGER REFERENCES kol_experts(id) ON DELETE SET NULL;
ALTER TABLE qa_turns ADD COLUMN expert_scoped INTEGER NOT NULL DEFAULT 0;
ALTER TABLE qa_turns ADD COLUMN expert_label TEXT;
ALTER TABLE reports ADD COLUMN structured_json TEXT;
ALTER TABLE reports ADD COLUMN evidence_json TEXT;
