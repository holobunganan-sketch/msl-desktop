-- v30 renumbers the menu after removing the record-next option.
-- Revoke old authorization so delayed v29 selections cannot run a new action.
-- Retain bindings, ordering watermarks, receipts, captures, work and jobs.
ALTER TABLE weixin_binding ADD COLUMN menu_protocol_after_ms INTEGER NOT NULL DEFAULT 0;

-- Include queued summons even when no old menu is currently open. The trusted
-- message clock allows +300 seconds with a 999 ms tail. Only bindings present
-- during this one-time migration need this cutoff; first-time bindings use 0.
UPDATE weixin_binding
SET menu_protocol_after_ms = (CAST(strftime('%s', 'now') AS INTEGER) + 301) * 1000 - 1;

UPDATE weixin_binding
SET menu_mode = NULL, menu_expires_at = NULL
WHERE menu_mode IN ('menu', 'record');
