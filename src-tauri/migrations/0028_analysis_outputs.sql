-- Model output is durable user-visible data, independent of executable proposals.
CREATE TABLE analysis_outputs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id INTEGER NOT NULL UNIQUE REFERENCES analysis_runs(id) ON DELETE CASCADE,
    raw_output TEXT NOT NULL,
    warnings_json TEXT NOT NULL DEFAULT '[]',
    created_at INTEGER NOT NULL
);
