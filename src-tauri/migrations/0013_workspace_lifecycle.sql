-- Keep identities and historical evidence; source files are never touched.
UPDATE workspaces SET enabled=0
WHERE NOT EXISTS (SELECT 1 FROM work_workspace_links l WHERE l.workspace_id=workspaces.id);
DELETE FROM app_settings WHERE key='main_workspace'
AND NOT EXISTS (SELECT 1 FROM workspaces w WHERE w.root_path=app_settings.value AND w.enabled=1);

CREATE TRIGGER workspace_link_added AFTER INSERT ON work_workspace_links
BEGIN
  UPDATE workspaces SET enabled=1, updated_at=unixepoch() WHERE id=NEW.workspace_id;
END;

CREATE TRIGGER workspace_last_link_removed AFTER DELETE ON work_workspace_links
WHEN NOT EXISTS (SELECT 1 FROM work_workspace_links WHERE workspace_id=OLD.workspace_id)
BEGIN
  UPDATE workspaces SET enabled=0, updated_at=unixepoch() WHERE id=OLD.workspace_id;
END;

CREATE TRIGGER workspace_disable_guard BEFORE UPDATE OF enabled ON workspaces
WHEN NEW.enabled=0 AND EXISTS (
  SELECT 1 FROM work_workspace_links l JOIN works w ON w.id=l.work_id
  WHERE l.workspace_id=OLD.id AND w.status!='archived'
)
BEGIN
  SELECT RAISE(ABORT,'该目录仍关联未归档项目，请先在项目中解除关联');
END;

CREATE TRIGGER workspace_delete_guard BEFORE DELETE ON workspaces
WHEN EXISTS (
  SELECT 1 FROM work_workspace_links l JOIN works w ON w.id=l.work_id
  WHERE l.workspace_id=OLD.id AND w.status!='archived'
)
BEGIN
  SELECT RAISE(ABORT,'该目录仍关联未归档项目，请先在项目中解除关联');
END;

CREATE TRIGGER workspace_monitor_retired AFTER UPDATE OF enabled ON workspaces
WHEN NEW.enabled=0
BEGIN
  DELETE FROM app_settings WHERE key='main_workspace' AND value=OLD.root_path;
END;
