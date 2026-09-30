ALTER TABLE works ADD COLUMN category TEXT CHECK(category IN ('clinical','non_clinical'));

CREATE TABLE project_relations (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 entity_kind TEXT NOT NULL CHECK(entity_kind IN ('task','waiting','calendar','inbox')),
 entity_id INTEGER NOT NULL,
 clinical_work_id INTEGER REFERENCES works(id) ON DELETE SET NULL,
 revision INTEGER NOT NULL DEFAULT 1,
 created_at INTEGER NOT NULL,
 updated_at INTEGER NOT NULL,
 UNIQUE(entity_kind,entity_id)
);
CREATE INDEX project_relations_clinical ON project_relations(clinical_work_id,entity_kind);
CREATE TRIGGER project_relations_revision AFTER UPDATE ON project_relations
WHEN NEW.revision=OLD.revision BEGIN
 UPDATE project_relations SET revision=OLD.revision+1 WHERE id=NEW.id;
END;
CREATE TRIGGER project_relations_task_delete AFTER DELETE ON tasks BEGIN
 DELETE FROM project_relations WHERE entity_kind='task' AND entity_id=OLD.id;
END;
CREATE TRIGGER project_relations_waiting_delete AFTER DELETE ON waiting_items BEGIN
 DELETE FROM project_relations WHERE entity_kind='waiting' AND entity_id=OLD.id;
END;
CREATE TRIGGER project_relations_calendar_delete AFTER DELETE ON calendar_events BEGIN
 DELETE FROM project_relations WHERE entity_kind='calendar' AND entity_id=OLD.id;
END;
CREATE TRIGGER project_relations_inbox_delete AFTER DELETE ON inbox_items BEGIN
 DELETE FROM project_relations WHERE entity_kind='inbox' AND entity_id=OLD.id;
END;

-- A clinical owner is the single direct project relationship. Moving an item
-- to that owner clears any additional study link, including on synced edits.
CREATE VIEW invalid_project_relations AS
SELECT r.id FROM project_relations r WHERE r.clinical_work_id IS NOT NULL AND (
 EXISTS(SELECT 1 FROM works WHERE id=r.clinical_work_id AND category IS NOT 'clinical') OR
 (r.entity_kind='task' AND EXISTS(SELECT 1 FROM tasks t JOIN works w ON w.id=t.work_id WHERE t.id=r.entity_id AND w.category='clinical')) OR
 (r.entity_kind='waiting' AND EXISTS(SELECT 1 FROM waiting_items t JOIN works w ON w.id=t.work_id WHERE t.id=r.entity_id AND w.category='clinical')) OR
 (r.entity_kind='calendar' AND EXISTS(SELECT 1 FROM calendar_events t JOIN works w ON w.id=t.work_id WHERE t.id=r.entity_id AND w.category='clinical')) OR
 (r.entity_kind='inbox' AND EXISTS(SELECT 1 FROM capture_context t JOIN works w ON w.id=t.work_id WHERE t.inbox_id=r.entity_id AND w.category='clinical'))
);
CREATE TRIGGER project_relations_validate_insert AFTER INSERT ON project_relations BEGIN
 UPDATE project_relations SET clinical_work_id=NULL,updated_at=MAX(updated_at+1,unixepoch()) WHERE id=NEW.id AND id IN (SELECT id FROM invalid_project_relations);
END;
CREATE TRIGGER project_relations_validate_update AFTER UPDATE OF clinical_work_id ON project_relations BEGIN
 UPDATE project_relations SET clinical_work_id=NULL,updated_at=MAX(updated_at+1,unixepoch()) WHERE id=NEW.id AND id IN (SELECT id FROM invalid_project_relations);
END;
CREATE TRIGGER project_relations_task_move AFTER UPDATE OF work_id ON tasks BEGIN
 UPDATE project_relations SET clinical_work_id=NULL,updated_at=MAX(updated_at+1,unixepoch()) WHERE entity_kind='task' AND entity_id=NEW.id AND id IN (SELECT id FROM invalid_project_relations);
END;
CREATE TRIGGER project_relations_waiting_move AFTER UPDATE OF work_id ON waiting_items BEGIN
 UPDATE project_relations SET clinical_work_id=NULL,updated_at=MAX(updated_at+1,unixepoch()) WHERE entity_kind='waiting' AND entity_id=NEW.id AND id IN (SELECT id FROM invalid_project_relations);
END;
CREATE TRIGGER project_relations_calendar_move AFTER UPDATE OF work_id ON calendar_events BEGIN
 UPDATE project_relations SET clinical_work_id=NULL,updated_at=MAX(updated_at+1,unixepoch()) WHERE entity_kind='calendar' AND entity_id=NEW.id AND id IN (SELECT id FROM invalid_project_relations);
END;
CREATE TRIGGER project_relations_capture_move AFTER UPDATE OF work_id ON capture_context BEGIN
 UPDATE project_relations SET clinical_work_id=NULL,updated_at=MAX(updated_at+1,unixepoch()) WHERE entity_kind='inbox' AND entity_id=NEW.inbox_id AND id IN (SELECT id FROM invalid_project_relations);
END;
CREATE TRIGGER project_relations_category_change AFTER UPDATE OF category ON works BEGIN
 UPDATE project_relations SET clinical_work_id=NULL,updated_at=MAX(updated_at+1,unixepoch()) WHERE id IN (SELECT id FROM invalid_project_relations);
END;

-- Existing capture triggers embed their column list. Refresh the work triggers
-- after this additive migration so edits to category have field timestamps.
DROP TRIGGER IF EXISTS sync_capture_works_INSERT;
DROP TRIGGER IF EXISTS sync_capture_works_UPDATE;
DROP TRIGGER IF EXISTS sync_capture_works_DELETE;
