//! tasks + waiting_items repository（指南 §6.5 / §6.6）。

use rusqlite::{Connection, OptionalExtension, Row, params};

use super::{DbError, DbResult, now_unix};

// ---------- tasks ----------

#[derive(Debug, Clone, serde::Serialize)]
pub struct Task {
    pub id: i64,
    pub work_id: Option<i64>,
    pub title: String,
    pub status: String, // next|scheduled|waiting|paused|done
    pub priority: String,
    pub due_at: Option<i64>,
    pub scheduled_start: Option<i64>,
    pub scheduled_end: Option<i64>,
    pub notes: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub completed_at: Option<i64>,
}

fn row_to_task(row: &Row) -> rusqlite::Result<Task> {
    Ok(Task {
        id: row.get(0)?,
        work_id: row.get(1)?,
        title: row.get(2)?,
        status: row.get(3)?,
        priority: row.get(4)?,
        due_at: row.get(5)?,
        scheduled_start: row.get(6)?,
        scheduled_end: row.get(7)?,
        notes: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        completed_at: row.get(11)?,
    })
}

pub struct TaskRepo<'a> {
    conn: &'a Connection,
}

impl<'a> TaskRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// 新增 Task。
    pub fn insert(
        &self,
        work_id: Option<i64>,
        title: &str,
        priority: &str,
        due_at: Option<i64>,
        notes: Option<&str>,
    ) -> DbResult<Task> {
        let now = now_unix();
        self.conn.execute(
            "INSERT INTO tasks (work_id, title, status, priority, due_at, notes, created_at, updated_at)
             VALUES (?1, ?2, 'next', ?3, ?4, ?5, ?6, ?6)",
            params![work_id, title, priority, due_at, notes, now],
        )?;
        let id = self.conn.last_insert_rowid();
        self.get(id)?
            .ok_or_else(|| DbError::NotFound("task".into()))
    }

    pub fn get(&self, id: i64) -> DbResult<Option<Task>> {
        self.conn
            .query_row(
                "SELECT id, work_id, title, status, priority, due_at, scheduled_start,
                        scheduled_end, notes, created_at, updated_at, completed_at
                 FROM tasks WHERE id = ?1",
                [id],
                row_to_task,
            )
            .optional()
            .map_err(DbError::from)
    }

    /// 列出（可按 status / work 过滤）。
    pub fn list(&self, status: Option<&str>, work_id: Option<i64>) -> DbResult<Vec<Task>> {
        let mut sql = String::from(
            "SELECT id, work_id, title, status, priority, due_at, scheduled_start,
                    scheduled_end, notes, created_at, updated_at, completed_at
             FROM tasks WHERE 1=1",
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(s) = status {
            sql.push_str(" AND status = ?");
            params.push(Box::new(s.to_string()));
        }
        if let Some(w) = work_id {
            sql.push_str(" AND work_id = ?");
            params.push(Box::new(w));
        }
        sql.push_str(" ORDER BY due_at IS NULL, due_at, id");
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), row_to_task)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }

    /// 更新标题 / 优先级 / 截止 / 备注。
    pub fn update(
        &self,
        id: i64,
        title: &str,
        priority: &str,
        due_at: Option<i64>,
        notes: Option<&str>,
    ) -> DbResult<()> {
        let affected = self.conn.execute(
            "UPDATE tasks SET title = ?1, priority = ?2, due_at = ?3, notes = ?4, updated_at = ?5 WHERE id = ?6",
            params![title, priority, due_at, notes, now_unix(), id],
        )?;
        if affected == 0 {
            return Err(DbError::NotFound("task".into()));
        }
        Ok(())
    }

    /// 完成（status=done, completed_at=now）；再次完成幂等。
    pub fn complete(&self, id: i64) -> DbResult<()> {
        let affected = self.conn.execute(
            "UPDATE tasks SET status = 'done', completed_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![now_unix(), id],
        )?;
        if affected == 0 {
            return Err(DbError::NotFound("task".into()));
        }
        Ok(())
    }

    pub fn delete(&self, id: i64) -> DbResult<()> {
        let affected = self
            .conn
            .execute("DELETE FROM tasks WHERE id = ?1", [id])?;
        if affected == 0 {
            return Err(DbError::NotFound("task".into()));
        }
        Ok(())
    }
}

// ---------- waiting_items ----------

#[derive(Debug, Clone, serde::Serialize)]
pub struct WaitingItem {
    pub id: i64,
    pub work_id: Option<i64>,
    pub title: String,
    pub waiting_for: String,
    pub started_at: i64,
    pub follow_up_at: Option<i64>,
    pub status: String, // open|resolved
    pub notes: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub resolved_at: Option<i64>,
}

fn row_to_waiting(row: &Row) -> rusqlite::Result<WaitingItem> {
    Ok(WaitingItem {
        id: row.get(0)?,
        work_id: row.get(1)?,
        title: row.get(2)?,
        waiting_for: row.get(3)?,
        started_at: row.get(4)?,
        follow_up_at: row.get(5)?,
        status: row.get(6)?,
        notes: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
        resolved_at: row.get(10)?,
    })
}

pub struct WaitingRepo<'a> {
    conn: &'a Connection,
}

impl<'a> WaitingRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// 新增 waiting item。
    pub fn insert(
        &self,
        work_id: Option<i64>,
        title: &str,
        waiting_for: &str,
        follow_up_at: Option<i64>,
        notes: Option<&str>,
    ) -> DbResult<WaitingItem> {
        let now = now_unix();
        self.conn.execute(
            "INSERT INTO waiting_items
               (work_id, title, waiting_for, started_at, follow_up_at, status, notes, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'open', ?6, ?4, ?4)",
            params![work_id, title, waiting_for, now, follow_up_at, notes],
        )?;
        let id = self.conn.last_insert_rowid();
        self.get(id)?
            .ok_or_else(|| DbError::NotFound("waiting_item".into()))
    }

    pub fn get(&self, id: i64) -> DbResult<Option<WaitingItem>> {
        self.conn
            .query_row(
                "SELECT id, work_id, title, waiting_for, started_at, follow_up_at,
                        status, notes, created_at, updated_at, resolved_at
                 FROM waiting_items WHERE id = ?1",
                [id],
                row_to_waiting,
            )
            .optional()
            .map_err(DbError::from)
    }

    /// 列出（可按 status / work 过滤；默认 open 优先 + follow_up 排序）。
    pub fn list(&self, status: Option<&str>, work_id: Option<i64>) -> DbResult<Vec<WaitingItem>> {
        let mut sql = String::from(
            "SELECT id, work_id, title, waiting_for, started_at, follow_up_at,
                    status, notes, created_at, updated_at, resolved_at
             FROM waiting_items WHERE 1=1",
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(s) = status {
            sql.push_str(" AND status = ?");
            params.push(Box::new(s.to_string()));
        }
        if let Some(w) = work_id {
            sql.push_str(" AND work_id = ?");
            params.push(Box::new(w));
        }
        sql.push_str(" ORDER BY (status = 'resolved'), follow_up_at IS NULL, follow_up_at, id");
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), row_to_waiting)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }

    /// 解决（status=resolved, resolved_at=now）。
    pub fn resolve(&self, id: i64) -> DbResult<()> {
        let affected = self.conn.execute(
            "UPDATE waiting_items SET status = 'resolved', resolved_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![now_unix(), id],
        )?;
        if affected == 0 {
            return Err(DbError::NotFound("waiting_item".into()));
        }
        Ok(())
    }

    pub fn delete(&self, id: i64) -> DbResult<()> {
        let affected = self
            .conn
            .execute("DELETE FROM waiting_items WHERE id = ?1", [id])?;
        if affected == 0 {
            return Err(DbError::NotFound("waiting_item".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn task_crud_and_complete() {
        let db = Database::open_in_memory().unwrap();
        let repo = TaskRepo::new(db.conn());

        let due = now_unix() + 86400;
        let t = repo.insert(None, "核对入排标准", "high", Some(due), Some("对照方案 V3")).unwrap();
        assert_eq!(t.status, "next");

        repo.complete(t.id).unwrap();
        let done = repo.get(t.id).unwrap().unwrap();
        assert_eq!(done.status, "done");
        assert!(done.completed_at.is_some());

        let open = repo.list(Some("next"), None).unwrap();
        assert_eq!(open.len(), 0);
        let done_list = repo.list(Some("done"), None).unwrap();
        assert_eq!(done_list.len(), 1);
    }

    #[test]
    fn task_filter_by_work() {
        let db = Database::open_in_memory().unwrap();
        let repo = TaskRepo::new(db.conn());
        crate::db::work::WorkRepo::new(db.conn())
            .insert("Work A", "active")
            .unwrap();
        repo.insert(Some(1), "A 的任务", "normal", None, None).unwrap();
        repo.insert(None, "独立任务", "normal", None, None).unwrap();

        assert_eq!(repo.list(None, Some(1)).unwrap().len(), 1);
        assert_eq!(repo.list(None, None).unwrap().len(), 2);
    }

    #[test]
    fn waiting_crud_and_resolve() {
        let db = Database::open_in_memory().unwrap();
        let repo = WaitingRepo::new(db.conn());

        let follow = now_unix() + 172800;
        let w = repo.insert(None, "统计方案反馈", "王老师", Some(follow), None).unwrap();
        assert_eq!(w.status, "open");
        assert_eq!(w.waiting_for, "王老师");

        repo.resolve(w.id).unwrap();
        let resolved = repo.get(w.id).unwrap().unwrap();
        assert_eq!(resolved.status, "resolved");
        assert!(resolved.resolved_at.is_some());

        let open = repo.list(Some("open"), None).unwrap();
        assert_eq!(open.len(), 0);
    }
}
