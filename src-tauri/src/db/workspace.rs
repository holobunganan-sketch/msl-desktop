//! workspaces + work_file_refs repository（指南 §6.1 / §6.4）。

use rusqlite::{Connection, OptionalExtension, Row, params};

use super::{DbError, DbResult, now_unix};

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

    /// 删除不存在的记录应报 NotFound
    pub fn delete(&self, id: i64) -> DbResult<()> {
        let affected = self
            .conn
            .execute("DELETE FROM workspaces WHERE id = ?1", [id])?;
        if affected == 0 {
            return Err(DbError::NotFound("workspace".into()));
        }
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
        assert!(repo.get(ws.id).unwrap().is_none());
        // 删除不存在的记录应报 NotFound
        assert!(matches!(
            repo.delete(999),
            Err(DbError::NotFound(_))
        ));
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
            .insert(1, Some(ws.id), "C:\\Work\\IIT\\方案V3.docx", Some("研究方案"))
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
}
