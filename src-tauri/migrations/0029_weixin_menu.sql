-- Menu authorization belongs to this local device binding and is never synced/exported.
ALTER TABLE weixin_binding ADD COLUMN menu_mode TEXT CHECK (menu_mode IN ('menu', 'record'));
ALTER TABLE weixin_binding ADD COLUMN menu_expires_at INTEGER;
ALTER TABLE weixin_binding ADD COLUMN menu_after_ms INTEGER NOT NULL DEFAULT 0;
ALTER TABLE weixin_binding ADD COLUMN menu_after_id TEXT NOT NULL DEFAULT '0';
ALTER TABLE weixin_binding ADD COLUMN last_brief_at INTEGER;

CREATE TRIGGER weixin_menu_revoke AFTER UPDATE OF enabled,bot_id,owner_id,bound_at ON weixin_binding
WHEN NEW.enabled=0 OR NEW.bot_id<>OLD.bot_id OR NEW.owner_id<>OLD.owner_id OR NEW.bound_at<>OLD.bound_at
BEGIN
    UPDATE weixin_binding SET menu_mode=NULL,menu_expires_at=NULL,menu_after_ms=0,menu_after_id='0' WHERE id=NEW.id;
END;

-- Keep every existing deduplication tombstone while extending receipt actions.
ALTER TABLE weixin_receipts RENAME TO weixin_receipts_legacy;
DROP INDEX weixin_receipts_rate;
CREATE TABLE weixin_receipts (
    bot_id TEXT NOT NULL,
    message_id TEXT NOT NULL,
    received_at INTEGER NOT NULL,
    action TEXT NOT NULL CHECK (action IN ('capture', 'global', 'brief', 'menu', 'record_next', 'job_status', 'pending_status', 'exit_menu', 'retired_command', 'rate_limited', 'queue_busy')),
    inbox_id INTEGER,
    job_id INTEGER,
    PRIMARY KEY (bot_id, message_id)
);
INSERT INTO weixin_receipts SELECT * FROM weixin_receipts_legacy;
DROP TABLE weixin_receipts_legacy;
CREATE INDEX weixin_receipts_rate ON weixin_receipts(bot_id, received_at);
