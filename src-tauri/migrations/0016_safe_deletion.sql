ALTER TABLE works ADD COLUMN revision INTEGER NOT NULL DEFAULT 1;
CREATE TRIGGER works_revision AFTER UPDATE ON works WHEN NEW.revision=OLD.revision
BEGIN UPDATE works SET revision=OLD.revision+1 WHERE id=NEW.id; END;
CREATE TABLE knowledge_deleted_sources(source_id TEXT PRIMARY KEY,deleted_at INTEGER NOT NULL);
