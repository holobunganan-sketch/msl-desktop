-- Rebuildable cognition and durable decision/audit records; no source file bodies.
CREATE TABLE cognition_entries (
 scope_key TEXT PRIMARY KEY,
 fingerprint TEXT NOT NULL,
 version INTEGER NOT NULL,
 markdown TEXT NOT NULL,
 document_count INTEGER NOT NULL DEFAULT 0,
 ready_count INTEGER NOT NULL DEFAULT 0,
 generated_at INTEGER NOT NULL
);
CREATE TABLE review_decisions (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 proposal_id INTEGER NOT NULL REFERENCES ai_proposals(id),
 reason_code TEXT NOT NULL,
 note TEXT NOT NULL DEFAULT '',
 created_at INTEGER NOT NULL
);
CREATE INDEX idx_review_decision_proposal ON review_decisions(proposal_id);
CREATE TABLE proposal_receipts (
 id TEXT PRIMARY KEY,
 proposal_ids_json TEXT NOT NULL,
 changes_json TEXT NOT NULL,
 created_at INTEGER NOT NULL,
 undone_at INTEGER
);
CREATE INDEX idx_proposal_receipts_created ON proposal_receipts(created_at DESC);
ALTER TABLE ai_proposals ADD COLUMN input_signature TEXT;
CREATE INDEX idx_proposal_history_key ON ai_proposals(dedupe_key, status);
