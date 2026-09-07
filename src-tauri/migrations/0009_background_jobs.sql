-- Durable execution receipts; no credentials. Formal outputs remain in their
-- own relational tables. Finished receipts are expendable after seven days.
CREATE TABLE ai_jobs (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 command TEXT NOT NULL,
 request_key TEXT NOT NULL,
 args_json TEXT NOT NULL,
 status TEXT NOT NULL CHECK(status IN ('running','completed','failed','interrupted')),
 result_json TEXT,
 error TEXT,
 created_at INTEGER NOT NULL,
 finished_at INTEGER
);
CREATE UNIQUE INDEX ai_jobs_active_request ON ai_jobs(request_key) WHERE status='running';
CREATE INDEX ai_jobs_recent ON ai_jobs(created_at DESC, id DESC);
CREATE INDEX task_work_status ON tasks(work_id, status, updated_at);
CREATE INDEX waiting_work_status ON waiting_items(work_id, status, updated_at);
CREATE INDEX inbox_unprocessed ON inbox_items(processed_at, created_at DESC);
