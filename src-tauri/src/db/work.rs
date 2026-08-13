//! works + resume_points repository（指南 §6.2 / §6.3）。

use rusqlite::{Connection, OptionalExtension, Row, params};

use super::{DbError, DbResult, now_unix};

// ---------- works ----------

#[derive(Debug, Clone)]
pub struct Work {
    pub id: i64,
    pub title: String,
    pub status: String, // active|paused|waiting|done|archived
    pub summary: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub archived_at: Option<i64>,
}

fn row_to_work(row: &Row) -> rusqlite::Result<Work> {
    Ok(Work {
        id: row.get(0)?,
        title: row.get(1)?,
        status: row.get(2)?,
        summary: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
        archived_at: row.get(6)?,
    })
}

pub struct WorkRepo<'a> {
    conn: &'a Connection,
}

impl<'a> WorkRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// 新增 Work。status 合法值：active|paused|waiting|done|archived。
    pub fn insert(&self, title: &str, status: &str) -> DbResult<Work> {
        let now = now_unix();
        self.conn.execute(
            "INSERT INTO works (title, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
            params![title, status, now],
        )?;
        let id = self.conn.last_insert_rowid();
        self.get(id)?
            .ok_or_else(|| DbError::NotFound("work".into()))
    }

    pub fn get(&self, id: i64) -> DbResult<Option<Work>> {
        self.conn
            .query_row(
                "SELECT id, title, status, summary, created_at, updated_at, archived_at
                 FROM works WHERE id = ?1",
                [id],
                row_to_work,
            )
            .optional()
            .map_err(DbError::from)
    }

    /// 列出全部（可按 status 过滤）。
    pub fn list(&self, status: Option<&str>) -> DbResult<Vec<Work>> {
        let (sql, params): (&str, Vec<Box<dyn rusqlite::ToSql>>) = match status {
            Some(s) => (
                "SELECT id, title, status, summary, created_at, updated_at, archived_at
                 FROM works WHERE status = ?1 ORDER BY updated_at DESC",
                vec![Box::new(s.to_string())],
            ),
            None => (
                "SELECT id, title, status, summary, created_at, updated_at, archived_at
                 FROM works ORDER BY updated_at DESC",
                vec![],
            ),
        };
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), row_to_work)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }

    /// 更新标题 / 状态 / 摘要。
    pub fn update(&self, id: i64, title: &str, status: &str, summary: Option<&str>) -> DbResult<()> {
        let affected = self.conn.execute(
            "UPDATE works SET title = ?1, status = ?2, summary = ?3, updated_at = ?4 WHERE id = ?5",
            params![title, status, summary, now_unix(), id],
        )?;
        if affected == 0 {
            return Err(DbError::NotFound("work".into()));
        }
        Ok(())
    }

    /// 归档（archived_at 置为当前时间，status 置 archived）。
    pub fn archive(&self, id: i64) -> DbResult<()> {
        let affected = self.conn.execute(
            "UPDATE works SET status = 'archived', archived_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![now_unix(), id],
        )?;
        if affected == 0 {
            return Err(DbError::NotFound("work".into()));
        }
        Ok(())
    }

    pub fn delete(&self, id: i64) -> DbResult<()> {
        let affected = self
            .conn
            .execute("DELETE FROM works WHERE id = ?1", [id])?;
        if affected == 0 {
            return Err(DbError::NotFound("work".into()));
        }
        Ok(())
    }
}

// ---------- resume_points ----------

#[derive(Debug, Clone)]
pub struct ResumePoint {
    pub id: i64,
    pub work_id: i64,
    pub current_state: String,
    pub next_step: String,
    pub remember: String,
    pub source: String, // manual|ai_draft_confirmed
    pub created_at: i64,
}

fn row_to_resume(row: &Row) -> rusqlite::Result<ResumePoint> {
    Ok(ResumePoint {
        id: row.get(0)?,
        work_id: row.get(1)?,
        current_state: row.get(2)?,
        next_step: row.get(3)?,
        remember: row.get(4)?,
        source: row.get(5)?,
        created_at: row.get(6)?,
    })
}

pub struct ResumePointRepo<'a> {
    conn: &'a Connection,
}

impl<'a> ResumePointRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// 新增 Resume Point。
    pub fn insert(
        &self,
        work_id: i64,
        current_state: &str,
        next_step: &str,
        remember: &str,
        source: &str,
    ) -> DbResult<ResumePoint> {
        self.conn.execute(
            "INSERT INTO resume_points (work_id, current_state, next_step, remember, source, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![work_id, current_state, next_step, remember, source, now_unix()],
        )?;
        let id = self.conn.last_insert_rowid();
        self.get(id)?
            .ok_or_else(|| DbError::NotFound("resume_point".into()))
    }

    pub fn get(&self, id: i64) -> DbResult<Option<ResumePoint>> {
        self.conn
            .query_row(
                "SELECT id, work_id, current_state, next_step, remember, source, created_at
                 FROM resume_points WHERE id = ?1",
                [id],
                row_to_resume,
            )
            .optional()
            .map_err(DbError::from)
    }

    /// 某 Work 的全部 Resume Point（新→旧）。
    pub fn list_by_work(&self, work_id: i64) -> DbResult<Vec<ResumePoint>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, work_id, current_state, next_step, remember, source, created_at
             FROM resume_points WHERE work_id = ?1 ORDER BY created_at DESC, id DESC",
        )?;
        let rows = stmt.query_map([work_id], row_to_resume)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }

    /// 某 Work 的最新一条 Resume Point。
    pub fn latest_for_work(&self, work_id: i64) -> DbResult<Option<ResumePoint>> {
        self.conn
            .query_row(
                "SELECT id, work_id, current_state, next_step, remember, source, created_at
                 FROM resume_points WHERE work_id = ?1
                 ORDER BY created_at DESC, id DESC LIMIT 1",
                [work_id],
                row_to_resume,
            )
            .optional()
            .map_err(DbError::from)
    }

    pub fn delete(&self, id: i64) -> DbResult<()> {
        let affected = self
            .conn
            .execute("DELETE FROM resume_points WHERE id = ?1", [id])?;
        if affected == 0 {
            return Err(DbError::NotFound("resume_point".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn seed_work(conn: &Connection) -> Work {
        WorkRepo::new(conn)
            .insert("老年破伤风 IIT", "active")
            .unwrap()
    }

    #[test]
    fn work_crud_and_archive() {
        let db = Database::open_in_memory().unwrap();
        let repo = WorkRepo::new(db.conn());

        let w = repo.insert("老年破伤风 IIT", "active").unwrap();
        assert_eq!(w.status, "active");
        assert!(w.archived_at.is_none());

        repo.update(w.id, "老年破伤风 IIT（第二轮）", "paused", Some("等待统计反馈"))
            .unwrap();
        let updated = repo.get(w.id).unwrap().unwrap();
        assert_eq!(updated.status, "paused");
        assert_eq!(updated.summary.as_deref(), Some("等待统计反馈"));

        repo.archive(w.id).unwrap();
        let archived = repo.get(w.id).unwrap().unwrap();
        assert_eq!(archived.status, "archived");
        assert!(archived.archived_at.is_some());

        assert_eq!(repo.list(Some("archived")).unwrap().len(), 1);
        assert_eq!(repo.list(Some("active")).unwrap().len(), 0);

        repo.delete(w.id).unwrap();
        assert!(repo.get(w.id).unwrap().is_none());
    }

    #[test]
    fn resume_point_history_and_latest() {
        let db = Database::open_in_memory().unwrap();
        let work = seed_work(db.conn());
        let repo = ResumePointRepo::new(db.conn());

        let rp1 = repo.insert(work.id, "完成方案 V2 修订", "核对统计部分", "统计由王老师负责", "manual")
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        let rp2 = repo.insert(work.id, "方案 V3 开始", "等待伦理材料", "伦理联系人：李老师", "manual")
            .unwrap();

        assert_ne!(rp1.id, rp2.id);
        let all = repo.list_by_work(work.id).unwrap();
        assert_eq!(all.len(), 2);
        // 最新在前
        assert_eq!(all[0].id, rp2.id);

        let latest = repo.latest_for_work(work.id).unwrap().unwrap();
        assert_eq!(latest.id, rp2.id);
        assert_eq!(latest.remember, "伦理联系人：李老师");

        repo.delete(rp1.id).unwrap();
        assert_eq!(repo.list_by_work(work.id).unwrap().len(), 1);
    }
}
