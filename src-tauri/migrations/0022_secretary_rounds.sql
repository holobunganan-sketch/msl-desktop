-- Local admission checkpoints; existing work records and opinions are untouched.
CREATE TABLE secretary_rounds (
  scope TEXT PRIMARY KEY,
  baseline TEXT NOT NULL DEFAULT '',
  epoch INTEGER NOT NULL DEFAULT 0,
  last_run INTEGER,
  active_run INTEGER,
  completed_at INTEGER
);
CREATE TABLE secretary_proposal_scopes (
  proposal_id INTEGER NOT NULL REFERENCES ai_proposals(id) ON DELETE CASCADE,
  scope TEXT NOT NULL,
  PRIMARY KEY(proposal_id, scope)
);
CREATE INDEX idx_secretary_proposal_scope ON secretary_proposal_scopes(scope);
