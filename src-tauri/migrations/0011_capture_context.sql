CREATE TABLE capture_context (
    inbox_id INTEGER PRIMARY KEY REFERENCES inbox_items(id) ON DELETE CASCADE,
    work_id INTEGER REFERENCES works(id) ON DELETE SET NULL,
    entity_kind TEXT,
    entity_id INTEGER,
    CHECK ((entity_kind IS NULL AND entity_id IS NULL) OR (entity_kind IN ('work','task','waiting','calendar') AND entity_id IS NOT NULL))
);
CREATE INDEX idx_capture_context_work ON capture_context(work_id);
