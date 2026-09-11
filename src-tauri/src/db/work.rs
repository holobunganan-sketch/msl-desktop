//! works + resume_points repository（指南 §6.2 / §6.3）。

use rusqlite::{params, Connection, OptionalExtension, Row};

use super::{now_unix, DbError, DbResult};

// ---------- works ----------

#[derive(Debug, Clone, serde::Serialize)]
pub struct Work {
    pub id: i64,
    pub title: String,
    pub revision: i64,
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
        revision: row.get(7)?,
        status: row.get(2)?,
        summary: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
        archived_at: row.get(6)?,
    })
}

pub fn validate_project_scope(conn: &Connection, kind: &str, work_id: Option<i64>) -> DbResult<()> {
    if !matches!(kind, "task" | "waiting" | "calendar" | "resume_point") {
        return Ok(());
    }
    let Some(id) = work_id else {
        return if kind == "resume_point" {
            Err(DbError::Migration("请先选择要归入的长期项目".into()))
        } else {
            Ok(())
        };
    };
    let work = WorkRepo::new(conn).get(id)?.ok_or_else(|| {
        DbError::Migration("所选项目已不存在，请重新选择项目或设为独立事项".into())
    })?;
    if work.status == "archived" {
        return Err(DbError::Migration(
            "所选项目已归档，请选择其他项目或先恢复项目".into(),
        ));
    }
    Ok(())
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
                "SELECT id, title, status, summary, created_at, updated_at, archived_at, revision
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
                "SELECT id, title, status, summary, created_at, updated_at, archived_at, revision
                 FROM works WHERE status = ?1 ORDER BY updated_at DESC",
                vec![Box::new(s.to_string())],
            ),
            None => (
                "SELECT id, title, status, summary, created_at, updated_at, archived_at, revision
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
    pub fn update(
        &self,
        id: i64,
        title: &str,
        status: &str,
        summary: Option<&str>,
    ) -> DbResult<()> {
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

    #[cfg(test)]
    pub fn delete(&self, id: i64) -> DbResult<()> {
        let work = self
            .get(id)?
            .ok_or_else(|| DbError::NotFound("work".into()))?;
        self.delete_confirmed(id, &work.title, work.revision)
    }
    pub fn delete_confirmed(
        &self,
        id: i64,
        confirmation_name: &str,
        revision: i64,
    ) -> DbResult<()> {
        let tx = super::write_transaction(self.conn)?;
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM works WHERE id = ?1)",
            [id],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(DbError::NotFound("work".into()));
        }

        if !tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM works WHERE id=?1 AND title=?2 AND revision=?3)",
            params![id, confirmation_name.trim(), revision],
            |r| r.get::<_, bool>(0),
        )? {
            return Err(DbError::Migration(
                "项目名称不一致或已更新，请刷新后重新确认".into(),
            ));
        }
        let mut retired = vec![format!("work:{id}")];
        for row in
            super::knowledge::rows(&tx, "SELECT id FROM resume_points WHERE work_id=?1", &[&id])?
        {
            retired.push(format!("resume:{}", row["id"]));
        }
        super::source_lifecycle::retire(&tx, &retired)?;
        // Actionable records remain useful after a long-term project is removed.
        // Detach them so they continue as temporary tasks, waits, and events.
        for table in [
            "tasks",
            "waiting_items",
            "calendar_events",
            "activity_events",
        ] {
            tx.execute(
                &format!("UPDATE {table} SET work_id = NULL WHERE work_id = ?1"),
                [id],
            )?;
        }
        // Progress snapshots and file links describe this work itself. Removing
        // these database references never touches files in the linked workspace.
        tx.execute("DELETE FROM resume_points WHERE work_id = ?1", [id])?;
        tx.execute("DELETE FROM work_file_refs WHERE work_id = ?1", [id])?;

        let affected = tx.execute("DELETE FROM works WHERE id = ?1", [id])?;
        if affected == 0 {
            return Err(DbError::NotFound("work".into()));
        }
        tx.commit()?;
        Ok(())
    }

    /// 按标题/摘要模糊搜索。
    pub fn search(&self, like: &str, limit: usize) -> DbResult<Vec<Work>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, status, summary, created_at, updated_at, archived_at, revision
             FROM works
             WHERE title LIKE ?1 ESCAPE '\\' OR (summary IS NOT NULL AND summary LIKE ?1 ESCAPE '\\')
             ORDER BY updated_at DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![like, limit as i64], row_to_work)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }
}

// ---------- resume_points ----------

#[derive(Debug, Clone, serde::Serialize)]
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

    /// 按 current_state/next_step/remember 模糊搜索。
    pub fn search(&self, like: &str, limit: usize) -> DbResult<Vec<ResumePoint>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, work_id, current_state, next_step, remember, source, created_at
             FROM resume_points
             WHERE current_state LIKE ?1 ESCAPE '\\'
                OR next_step LIKE ?1 ESCAPE '\\'
                OR remember LIKE ?1 ESCAPE '\\'
             ORDER BY created_at DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![like, limit as i64], row_to_resume)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn typed_project_delete_rejects_wrong_name_and_stale_version() {
        let db = super::super::Database::open_in_memory().unwrap();
        let repo = super::WorkRepo::new(db.conn());
        let work = repo.insert("测试项目", "active").unwrap();
        assert!(repo.delete_confirmed(work.id, "错误项目", 1).is_err());
        repo.update(work.id, "测试项目", "active", Some("变更摘要"))
            .unwrap();
        assert!(repo.delete_confirmed(work.id, "测试项目", 1).is_err());
        repo.delete_confirmed(work.id, " 测试项目 ", 2).unwrap();
        assert!(repo.get(work.id).unwrap().is_none());
    }
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

        repo.update(
            w.id,
            "老年破伤风 IIT（第二轮）",
            "paused",
            Some("等待统计反馈"),
        )
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
    fn deleting_work_preserves_actionable_items_as_temporary_and_removes_owned_context() {
        let db = Database::open_in_memory().unwrap();
        let repo = WorkRepo::new(db.conn());
        let work = repo.insert("需要删除的长期项目", "active").unwrap();
        let now = now_unix();

        db.conn()
            .execute(
                "INSERT INTO resume_points (work_id,current_state,next_step,remember,source,created_at) VALUES (?1,'当前状态','下一步','','manual',?2)",
                params![work.id, now],
            )
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO work_file_refs (work_id,path,label,pinned,created_at) VALUES (?1,'C:/project/plan.docx','方案',0,?2)",
                params![work.id, now],
            )
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO tasks (work_id,title,status,priority,created_at,updated_at) VALUES (?1,'仍需处理的任务','next','normal',?2,?2)",
                params![work.id, now],
            )
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO waiting_items (work_id,title,waiting_for,started_at,status,created_at,updated_at) VALUES (?1,'仍需跟进的等待事项','同事',?2,'open',?2,?2)",
                params![work.id, now],
            )
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO calendar_events (work_id,title,start_at,all_day,kind,created_at,updated_at) VALUES (?1,'仍需保留的日程',?2,1,'other',?2,?2)",
                params![work.id, now],
            )
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO activity_events (timestamp,event_type,work_id,display_text) VALUES (?1,'work.updated',?2,'历史活动')",
                params![now, work.id],
            )
            .unwrap();

        repo.delete(work.id).unwrap();

        assert!(repo.get(work.id).unwrap().is_none());
        for table in ["resume_points", "work_file_refs"] {
            let count: i64 = db
                .conn()
                .query_row(
                    &format!("SELECT COUNT(*) FROM {table} WHERE work_id=?1"),
                    [work.id],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(count, 0, "{table} belongs to the deleted work");
        }
        for table in [
            "tasks",
            "waiting_items",
            "calendar_events",
            "activity_events",
        ] {
            let (count, detached): (i64, i64) = db
                .conn()
                .query_row(
                    &format!(
                        "SELECT COUNT(*), SUM(CASE WHEN work_id IS NULL THEN 1 ELSE 0 END) FROM {table}"
                    ),
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap();
            assert_eq!(
                (count, detached),
                (1, 1),
                "{table} should be retained without a work link"
            );
        }
    }

    #[test]
    fn resume_point_history_and_latest() {
        let db = Database::open_in_memory().unwrap();
        let work = seed_work(db.conn());
        let repo = ResumePointRepo::new(db.conn());

        let rp1 = repo
            .insert(
                work.id,
                "完成方案 V2 修订",
                "核对统计部分",
                "统计由王老师负责",
                "manual",
            )
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        let rp2 = repo
            .insert(
                work.id,
                "方案 V3 开始",
                "等待伦理材料",
                "伦理联系人：李老师",
                "manual",
            )
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
