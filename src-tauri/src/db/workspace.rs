//! workspaces + work_file_refs repository（指南 §6.1 / §6.4）。

use rusqlite::{params, Connection, OptionalExtension, Row};

use super::{now_unix, DbError, DbResult};

// ---------- workspaces ----------

#[derive(Debug, Clone, serde::Serialize)]
pub struct Workspace {
    pub id: i64,
    pub name: String,
    pub root_path: String,
    pub enabled: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DirectoryProject {
    pub id: i64,
    pub title: String,
    pub status: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProjectDirectory {
    #[serde(flatten)]
    pub workspace: Workspace,
    pub projects: Vec<DirectoryProject>,
}

fn row_to_workspace(row: &Row) -> rusqlite::Result<Workspace> {
    Ok(Workspace {
        id: row.get(0)?,
        name: row.get(1)?,
        root_path: row.get(2)?,
        enabled: row.get::<_, i64>(3)? != 0,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

pub struct WorkspaceRepo<'a> {
    conn: &'a Connection,
}

impl<'a> WorkspaceRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// 新增 workspace（enabled 默认 true）。
    pub fn insert(&self, name: &str, root_path: &str) -> DbResult<Workspace> {
        let now = now_unix();
        self.conn.execute(
            "INSERT INTO workspaces (name, root_path, enabled, created_at, updated_at)
             VALUES (?1, ?2, 1, ?3, ?3)",
            params![name, root_path, now],
        )?;
        let id = self.conn.last_insert_rowid();
        self.get(id)?
            .ok_or_else(|| DbError::NotFound("workspace".into()))
    }

    pub fn get(&self, id: i64) -> DbResult<Option<Workspace>> {
        self.conn
            .query_row(
                "SELECT id, name, root_path, enabled, created_at, updated_at
                 FROM workspaces WHERE id = ?1",
                [id],
                row_to_workspace,
            )
            .optional()
            .map_err(DbError::from)
    }

    pub fn list(&self) -> DbResult<Vec<Workspace>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, root_path, enabled, created_at, updated_at FROM workspaces ORDER BY id")?;
        let rows = stmt.query_map([], row_to_workspace)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }

    pub fn project_directories(&self) -> DbResult<Vec<ProjectDirectory>> {
        let mut stmt = self.conn.prepare(
            "SELECT w.id,w.name,w.root_path,w.enabled,w.created_at,w.updated_at,p.id,p.title,p.status
             FROM workspaces w JOIN work_workspace_links l ON l.workspace_id=w.id
             JOIN works p ON p.id=l.work_id WHERE w.enabled=1 ORDER BY w.id,p.id")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                row_to_workspace(r)?,
                DirectoryProject {
                    id: r.get(6)?,
                    title: r.get(7)?,
                    status: r.get(8)?,
                },
            ))
        })?;
        let mut directories = std::collections::BTreeMap::new();
        for row in rows {
            let (workspace, project) = row?;
            directories
                .entry(workspace.id)
                .or_insert_with(|| ProjectDirectory {
                    workspace,
                    projects: Vec::new(),
                })
                .projects
                .push(project);
        }
        Ok(directories.into_values().collect())
    }

    pub fn active(&self, id: i64) -> DbResult<Workspace> {
        self.get(id)?
            .filter(|w| w.enabled)
            .ok_or_else(|| DbError::NotFound("目录已移除，请重新选择".into()))
    }

    /// 更新名称 / 路径 / enabled。
    pub fn update(&self, id: i64, name: &str, root_path: &str, enabled: bool) -> DbResult<()> {
        let affected = self.conn.execute(
            "UPDATE workspaces SET name = ?1, root_path = ?2, enabled = ?3, updated_at = ?4 WHERE id = ?5",
            params![name, root_path, enabled as i64, now_unix(), id],
        )?;
        if affected == 0 {
            return Err(DbError::NotFound("workspace".into()));
        }
        Ok(())
    }

    /// Retire a directory from the workbench. Keep its historical identity and
    /// evidence; remove archived-project bindings only, never source files.
    pub fn delete(&self, id: i64) -> DbResult<()> {
        let tx = super::write_transaction(self.conn)?;
        WorkspaceRepo::new(&tx)
            .get(id)?
            .ok_or_else(|| DbError::NotFound("workspace".into()))?;
        let blocked: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM work_workspace_links l JOIN works w ON w.id=l.work_id WHERE l.workspace_id=?1 AND w.status!='archived')",
            [id], |r| r.get(0))?;
        if blocked {
            return Err(DbError::Migration(
                "该目录仍关联未归档项目，请先在项目中解除关联".into(),
            ));
        }
        tx.execute(
            "DELETE FROM work_workspace_links WHERE workspace_id=?1",
            [id],
        )?;
        tx.execute(
            "UPDATE workspaces SET enabled=0,updated_at=?2 WHERE id=?1",
            params![id, now_unix()],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// 按名称/路径模糊搜索（LIKE，转义由调用方处理）。
    pub fn search(&self, like: &str, limit: usize) -> DbResult<Vec<Workspace>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, root_path, enabled, created_at, updated_at
             FROM workspaces
             WHERE name LIKE ?1 ESCAPE '\\' OR root_path LIKE ?1 ESCAPE '\\'
             ORDER BY id LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![like, limit as i64], row_to_workspace)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }
}

// ---------- workspace_file_state ----------

/// 文件事实快照：只保存 metadata，不保存正文。
#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkspaceFileState {
    pub workspace_id: i64,
    pub path: String,
    pub modified_at: i64,
    pub size: u64,
    pub seen_at: i64,
}

pub struct WorkspaceFileStateRepo<'a> {
    conn: &'a Connection,
}

impl<'a> WorkspaceFileStateRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn list(&self, workspace_id: i64) -> DbResult<Vec<WorkspaceFileState>> {
        let mut stmt = self.conn.prepare(
            "SELECT workspace_id, path, modified_at, size, seen_at
             FROM workspace_file_state WHERE workspace_id = ?1 ORDER BY path",
        )?;
        let rows = stmt.query_map([workspace_id], |row| {
            Ok(WorkspaceFileState {
                workspace_id: row.get(0)?,
                path: row.get(1)?,
                modified_at: row.get(2)?,
                size: row.get::<_, i64>(3)?.max(0) as u64,
                seen_at: row.get(4)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }

    pub fn count(&self, workspace_id: i64) -> DbResult<u64> {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM workspace_file_state WHERE workspace_id = ?1",
                [workspace_id],
                |row| row.get::<_, i64>(0),
            )
            .map(|n| n.max(0) as u64)
            .map_err(DbError::from)
    }

    /// 事件成功写入后更新实时快照；不存在的文件由调用方删除。
    pub fn upsert(
        &self,
        workspace_id: i64,
        path: &str,
        modified_at: i64,
        size: u64,
    ) -> DbResult<()> {
        self.conn.execute(
            "INSERT INTO workspace_file_state (workspace_id, path, modified_at, size, seen_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(workspace_id, path) DO UPDATE SET
               modified_at = excluded.modified_at,
               size = excluded.size,
               seen_at = excluded.seen_at",
            params![workspace_id, path, modified_at, size as i64, now_unix()],
        )?;
        Ok(())
    }

    pub fn delete(&self, workspace_id: i64, path: &str) -> DbResult<()> {
        self.conn.execute(
            "DELETE FROM workspace_file_state WHERE workspace_id = ?1 AND path = ?2",
            params![workspace_id, path],
        )?;
        Ok(())
    }

    pub fn workspace_for_path(&self, path: &str) -> DbResult<Option<Workspace>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, root_path, enabled, created_at, updated_at
             FROM workspaces WHERE enabled = 1 ORDER BY length(root_path) DESC",
        )?;
        let candidates = stmt.query_map([], row_to_workspace)?;
        for item in candidates {
            let ws = item?;
            if std::path::Path::new(path).starts_with(std::path::Path::new(&ws.root_path)) {
                return Ok(Some(ws));
            }
        }
        Ok(None)
    }
}

// ---------- work_file_refs ----------

#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkFileRef {
    pub id: i64,
    pub work_id: i64,
    pub workspace_id: Option<i64>,
    pub path: String,
    pub label: Option<String>,
    pub pinned: bool,
    pub created_at: i64,
}

fn row_to_file_ref(row: &Row) -> rusqlite::Result<WorkFileRef> {
    Ok(WorkFileRef {
        id: row.get(0)?,
        work_id: row.get(1)?,
        workspace_id: row.get(2)?,
        path: row.get(3)?,
        label: row.get(4)?,
        pinned: row.get::<_, i64>(5)? != 0,
        created_at: row.get(6)?,
    })
}

pub struct WorkFileRefRepo<'a> {
    conn: &'a Connection,
}

impl<'a> WorkFileRefRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// 新增文件引用（只存路径，不复制文件）。
    pub fn insert(
        &self,
        work_id: i64,
        workspace_id: Option<i64>,
        path: &str,
        label: Option<&str>,
    ) -> DbResult<WorkFileRef> {
        self.conn.execute(
            "INSERT INTO work_file_refs (work_id, workspace_id, path, label, pinned, created_at)
             VALUES (?1, ?2, ?3, ?4, 0, ?5)",
            params![work_id, workspace_id, path, label, now_unix()],
        )?;
        let id = self.conn.last_insert_rowid();
        self.get(id)?
            .ok_or_else(|| DbError::NotFound("work_file_ref".into()))
    }

    pub fn get(&self, id: i64) -> DbResult<Option<WorkFileRef>> {
        self.conn
            .query_row(
                "SELECT id, work_id, workspace_id, path, label, pinned, created_at
                 FROM work_file_refs WHERE id = ?1",
                [id],
                row_to_file_ref,
            )
            .optional()
            .map_err(DbError::from)
    }

    pub fn list_by_work(&self, work_id: i64) -> DbResult<Vec<WorkFileRef>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, work_id, workspace_id, path, label, pinned, created_at
             FROM work_file_refs WHERE work_id = ?1 ORDER BY pinned DESC, id",
        )?;
        let rows = stmt.query_map([work_id], row_to_file_ref)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }

    /// 更新 label 与 pinned。
    pub fn update(&self, id: i64, label: Option<&str>, pinned: bool) -> DbResult<()> {
        let affected = self.conn.execute(
            "UPDATE work_file_refs SET label = ?1, pinned = ?2 WHERE id = ?3",
            params![label, pinned as i64, id],
        )?;
        if affected == 0 {
            return Err(DbError::NotFound("work_file_ref".into()));
        }
        Ok(())
    }

    pub fn delete(&self, id: i64) -> DbResult<()> {
        let affected = self
            .conn
            .execute("DELETE FROM work_file_refs WHERE id = ?1", [id])?;
        if affected == 0 {
            return Err(DbError::NotFound("work_file_ref".into()));
        }
        Ok(())
    }

    /// 删除某 Work 的文件引用（供归档清理）。
    pub fn delete_by_work(&self, work_id: i64) -> DbResult<()> {
        self.conn
            .execute("DELETE FROM work_file_refs WHERE work_id = ?1", [work_id])?;
        Ok(())
    }

    /// 按路径/标签模糊搜索。
    pub fn search(&self, like: &str, limit: usize) -> DbResult<Vec<WorkFileRef>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, work_id, workspace_id, path, label, pinned, created_at
             FROM work_file_refs
             WHERE path LIKE ?1 ESCAPE '\\' OR (label IS NOT NULL AND label LIKE ?1 ESCAPE '\\')
             ORDER BY pinned DESC, id LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![like, limit as i64], row_to_file_ref)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn workspace_crud() {
        let db = Database::open_in_memory().unwrap();
        let repo = WorkspaceRepo::new(db.conn());

        let ws = repo.insert("主工作目录", "C:\\Work").unwrap();
        assert!(ws.id > 0);
        assert!(ws.enabled);

        let got = repo.get(ws.id).unwrap().unwrap();
        assert_eq!(got.name, "主工作目录");
        assert_eq!(got.root_path, "C:\\Work");

        repo.update(ws.id, "医学工作", "D:\\Medical", false)
            .unwrap();
        let updated = repo.get(ws.id).unwrap().unwrap();
        assert_eq!(updated.name, "医学工作");
        assert!(!updated.enabled);

        let list = repo.list().unwrap();
        assert_eq!(list.len(), 1);

        repo.delete(ws.id).unwrap();
        assert!(!repo.get(ws.id).unwrap().unwrap().enabled);
        // 删除不存在的记录应报 NotFound
        assert!(matches!(repo.delete(999), Err(DbError::NotFound(_))));
    }

    #[test]
    fn work_file_ref_crud() {
        let db = Database::open_in_memory().unwrap();
        let ws_repo = WorkspaceRepo::new(db.conn());
        let ref_repo = WorkFileRefRepo::new(db.conn());

        let ws = ws_repo.insert("workspace", "C:\\Work").unwrap();
        // 先建一个 work（works 表引用）
        crate::db::work::WorkRepo::new(db.conn())
            .insert("老年破伤风 IIT", "active")
            .unwrap();

        let r = ref_repo
            .insert(
                1,
                Some(ws.id),
                "C:\\Work\\IIT\\方案V3.docx",
                Some("研究方案"),
            )
            .unwrap();
        assert!(r.pinned == false);

        ref_repo.update(r.id, Some("方案V3"), true).unwrap();
        let updated = ref_repo.get(r.id).unwrap().unwrap();
        assert_eq!(updated.label.as_deref(), Some("方案V3"));
        assert!(updated.pinned);

        let by_work = ref_repo.list_by_work(1).unwrap();
        assert_eq!(by_work.len(), 1);
        assert!(by_work[0].pinned);

        ref_repo.delete(r.id).unwrap();
        assert!(ref_repo.get(r.id).unwrap().is_none());
    }

    #[test]
    fn directory_removal_blocks_every_unarchived_project_status() {
        for status in ["active", "paused", "waiting", "done"] {
            let db = Database::open_in_memory().unwrap();
            let ws = WorkspaceRepo::new(db.conn())
                .insert("shared", "C:/synthetic")
                .unwrap();
            let work = crate::db::work::WorkRepo::new(db.conn())
                .insert("Protected project", status)
                .unwrap();
            crate::db::documents::WorkWorkspaceLinkRepo::new(db.conn())
                .link(work.id, ws.id, false)
                .unwrap();
            assert!(
                WorkspaceRepo::new(db.conn()).delete(ws.id).is_err(),
                "{status} must block removal"
            );
            assert!(
                WorkspaceRepo::new(db.conn())
                    .get(ws.id)
                    .unwrap()
                    .unwrap()
                    .enabled
            );
            assert_eq!(
                crate::db::documents::WorkWorkspaceLinkRepo::new(db.conn())
                    .list_by_work(work.id)
                    .unwrap(),
                vec![ws.id]
            );
        }
    }

    #[test]
    fn archived_directory_removal_preserves_evidence_and_can_be_rebound() {
        let db = Database::open_in_memory().unwrap();
        let repo = WorkspaceRepo::new(db.conn());
        let ws = repo
            .insert("Archived folder", "C:/synthetic-archive")
            .unwrap();
        let work = crate::db::work::WorkRepo::new(db.conn())
            .insert("Archived project", "archived")
            .unwrap();
        let links = crate::db::documents::WorkWorkspaceLinkRepo::new(db.conn());
        links.link(work.id, ws.id, false).unwrap();
        WorkspaceFileStateRepo::new(db.conn())
            .upsert(ws.id, "C:/synthetic-archive/note.txt", 1, 7)
            .unwrap();
        let file = WorkFileRefRepo::new(db.conn())
            .insert(work.id, Some(ws.id), "C:/synthetic-archive/note.txt", None)
            .unwrap();
        repo.delete(ws.id).unwrap();
        assert!(
            repo.get(ws.id).unwrap().is_some(),
            "Historical workspace identity must survive removal"
        );
        assert!(!repo.get(ws.id).unwrap().unwrap().enabled);
        assert!(links.list_by_workspace(ws.id).unwrap().is_empty());
        assert_eq!(
            WorkspaceFileStateRepo::new(db.conn()).count(ws.id).unwrap(),
            1
        );
        assert!(WorkFileRefRepo::new(db.conn())
            .get(file.id)
            .unwrap()
            .is_some());
        let active = crate::db::work::WorkRepo::new(db.conn())
            .insert("New project", "active")
            .unwrap();
        links.link(active.id, ws.id, false).unwrap();
        assert!(repo.get(ws.id).unwrap().unwrap().enabled);
    }

    #[test]
    fn last_project_unlink_retires_directory_but_shared_link_keeps_it() {
        let db = Database::open_in_memory().unwrap();
        let repo = WorkspaceRepo::new(db.conn());
        let ws = repo.insert("shared", "C:/synthetic").unwrap();
        let work_repo = crate::db::work::WorkRepo::new(db.conn());
        let a = work_repo.insert("A", "active").unwrap();
        let b = work_repo.insert("B", "archived").unwrap();
        let links = crate::db::documents::WorkWorkspaceLinkRepo::new(db.conn());
        links.link(a.id, ws.id, true).unwrap();
        links.link(b.id, ws.id, true).unwrap();
        crate::db::provider::AppSettingsRepo::new(db.conn())
            .set("main_workspace", &ws.root_path)
            .unwrap();
        links.unlink(a.id, ws.id).unwrap();
        assert!(repo.get(ws.id).unwrap().unwrap().enabled);
        links.unlink(b.id, ws.id).unwrap();
        assert!(!repo.get(ws.id).unwrap().unwrap().enabled);
        assert!(crate::db::provider::AppSettingsRepo::new(db.conn())
            .get("main_workspace")
            .unwrap()
            .is_none());
        links.link(a.id, ws.id, false).unwrap();
        assert!(repo.get(ws.id).unwrap().unwrap().enabled);
        work_repo.delete(a.id).unwrap();
        assert!(!repo.get(ws.id).unwrap().unwrap().enabled);
    }

    #[test]
    fn migration_retires_only_unlinked_directories_without_dropping_history() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY,name TEXT,applied_at INTEGER);").unwrap();
        for migration in crate::db::migrations::MIGRATIONS
            .iter()
            .filter(|m| m.version <= 12)
        {
            conn.execute_batch(migration.sql).unwrap();
            conn.execute(
                "INSERT INTO schema_migrations VALUES (?1,?2,0)",
                params![migration.version, migration.name],
            )
            .unwrap();
        }
        let orphan = WorkspaceRepo::new(&conn).insert("old", "C:/old").unwrap();
        let linked = WorkspaceRepo::new(&conn).insert("kept", "C:/kept").unwrap();
        let work = crate::db::work::WorkRepo::new(&conn)
            .insert("Archived project", "archived")
            .unwrap();
        crate::db::documents::WorkWorkspaceLinkRepo::new(&conn)
            .link(work.id, linked.id, false)
            .unwrap();
        crate::db::provider::AppSettingsRepo::new(&conn)
            .set("main_workspace", &orphan.root_path)
            .unwrap();
        WorkspaceFileStateRepo::new(&conn)
            .upsert(orphan.id, "C:/old/file.txt", 1, 3)
            .unwrap();
        crate::db::migrations::run(&mut conn).unwrap();
        crate::db::migrations::run(&mut conn).unwrap();
        assert!(
            !WorkspaceRepo::new(&conn)
                .get(orphan.id)
                .unwrap()
                .unwrap()
                .enabled
        );
        assert!(
            WorkspaceRepo::new(&conn)
                .get(linked.id)
                .unwrap()
                .unwrap()
                .enabled
        );
        assert_eq!(
            WorkspaceFileStateRepo::new(&conn).count(orphan.id).unwrap(),
            1
        );
        assert!(crate::db::provider::AppSettingsRepo::new(&conn)
            .get("main_workspace")
            .unwrap()
            .is_none());
    }
}
