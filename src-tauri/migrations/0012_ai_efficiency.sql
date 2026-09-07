-- Fixed-size reuse metadata; no prompt, source text or credential contents.
CREATE TABLE ai_efficiency_state (
  id INTEGER PRIMARY KEY CHECK(id=1),
  fingerprint TEXT,
  run_id INTEGER REFERENCES analysis_runs(id) ON DELETE SET NULL,
  reused_checks INTEGER NOT NULL DEFAULT 0,
  input_chars_saved INTEGER NOT NULL DEFAULT 0,
  last_checked_at INTEGER
);
INSERT INTO ai_efficiency_state(id) VALUES(1);
