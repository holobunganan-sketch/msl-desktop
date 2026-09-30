-- Retry instructions are local execution metadata. They contain entity IDs only,
-- never credentials, a frozen prompt, document bodies or original file paths.
CREATE TABLE analysis_run_requests (
    run_id INTEGER PRIMARY KEY REFERENCES analysis_runs(id) ON DELETE CASCADE,
    task_kind TEXT NOT NULL CHECK (task_kind IN ('global_analysis', 'work_draft')),
    scope_json TEXT NOT NULL CHECK (json_valid(scope_json)),
    created_at INTEGER NOT NULL
);
