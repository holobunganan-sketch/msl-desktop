//! calendar_events repository（指南 §6.8）。

use rusqlite::{params, Connection, OptionalExtension, Row};

use super::{now_unix, DbError, DbResult};

#[derive(Debug, Clone, serde::Serialize)]
pub struct CalendarEvent {
    pub id: i64,
    pub work_id: Option<i64>,
    pub title: String,
    pub start_at: i64,
    pub end_at: Option<i64>,
    pub all_day: bool,
    pub location: Option<String>,
    pub notes: Option<String>,
    pub kind: String, // meeting|kol_visit|deadline|travel|work_block|other
    pub created_at: i64,
    pub updated_at: i64,
}

fn row_to_event(row: &Row) -> rusqlite::Result<CalendarEvent> {
    Ok(CalendarEvent {
        id: row.get(0)?,
        work_id: row.get(1)?,
        title: row.get(2)?,
        start_at: row.get(3)?,
        end_at: row.get(4)?,
        all_day: row.get::<_, i64>(5)? != 0,
        location: row.get(6)?,
        notes: row.get(7)?,
        kind: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

pub struct CalendarRepo<'a> {
    conn: &'a Connection,
}

impl<'a> CalendarRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// 新增事件。
    pub fn insert(
        &self,
        work_id: Option<i64>,
        title: &str,
        start_at: i64,
        end_at: Option<i64>,
        all_day: bool,
        kind: &str,
        location: Option<&str>,
        notes: Option<&str>,
    ) -> DbResult<CalendarEvent> {
        let now = now_unix();
        self.conn.execute(
            "INSERT INTO calendar_events
               (work_id, title, start_at, end_at, all_day, kind, location, notes, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
            params![work_id, title, start_at, end_at, all_day as i64, kind, location, notes, now],
        )?;
        let id = self.conn.last_insert_rowid();
        self.get(id)?
            .ok_or_else(|| DbError::NotFound("calendar_event".into()))
    }

    pub fn get(&self, id: i64) -> DbResult<Option<CalendarEvent>> {
        self.conn
            .query_row(
                "SELECT id, work_id, title, start_at, end_at, all_day, location, notes, kind, created_at, updated_at
                 FROM calendar_events WHERE id = ?1",
                [id],
                row_to_event,
            )
            .optional()
            .map_err(DbError::from)
    }

    /// 按时间范围列出（含起止边界）。
    pub fn list_between(&self, start: i64, end: i64) -> DbResult<Vec<CalendarEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, work_id, title, start_at, end_at, all_day, location, notes, kind, created_at, updated_at
             FROM calendar_events
             WHERE start_at <= ?2 AND (end_at IS NULL OR end_at >= ?1)
             ORDER BY start_at, id",
        )?;
        let rows = stmt.query_map(params![start, end], row_to_event)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }

    /// 列出某 Work 的事件。
    pub fn list_by_work(&self, work_id: i64) -> DbResult<Vec<CalendarEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, work_id, title, start_at, end_at, all_day, location, notes, kind, created_at, updated_at
             FROM calendar_events WHERE work_id = ?1 ORDER BY start_at, id",
        )?;
        let rows = stmt.query_map([work_id], row_to_event)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }

    /// 更新事件字段。
    pub fn update(
        &self,
        id: i64,
        work_id: Option<i64>,
        title: &str,
        start_at: i64,
        end_at: Option<i64>,
        all_day: bool,
        kind: &str,
        location: Option<&str>,
        notes: Option<&str>,
    ) -> DbResult<()> {
        let affected = self.conn.execute(
            "UPDATE calendar_events
             SET work_id = ?1, title = ?2, start_at = ?3, end_at = ?4, all_day = ?5, kind = ?6,
                 location = ?7, notes = ?8, updated_at = ?9
             WHERE id = ?10",
            params![
                work_id,
                title,
                start_at,
                end_at,
                all_day as i64,
                kind,
                location,
                notes,
                now_unix(),
                id
            ],
        )?;
        if affected == 0 {
            return Err(DbError::NotFound("calendar_event".into()));
        }
        Ok(())
    }

    pub fn delete(&self, id: i64) -> DbResult<()> {
        let affected = self
            .conn
            .execute("DELETE FROM calendar_events WHERE id = ?1", [id])?;
        if affected == 0 {
            return Err(DbError::NotFound("calendar_event".into()));
        }
        Ok(())
    }

    /// 按标题/地点/备注模糊搜索。
    pub fn search(&self, like: &str, limit: usize) -> DbResult<Vec<CalendarEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, work_id, title, start_at, end_at, all_day, location, notes, kind, created_at, updated_at
             FROM calendar_events
             WHERE title LIKE ?1 ESCAPE '\\'
                OR (location IS NOT NULL AND location LIKE ?1 ESCAPE '\\')
                OR (notes IS NOT NULL AND notes LIKE ?1 ESCAPE '\\')
             ORDER BY start_at LIMIT ?2",
        )?;
        let rows = stmt.query_map(rusqlite::params![like, limit as i64], row_to_event)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn calendar_crud_and_range() {
        let db = Database::open_in_memory().unwrap();
        let repo = CalendarRepo::new(db.conn());

        let day0 = now_unix();
        let e1 = repo
            .insert(
                None,
                "专家拜访 张教授",
                day0,
                Some(day0 + 3600),
                false,
                "kol_visit",
                Some("医院"),
                None,
            )
            .unwrap();
        let e2 = repo
            .insert(
                None,
                "方案提交 deadline",
                day0 + 86400,
                None,
                true,
                "deadline",
                None,
                None,
            )
            .unwrap();
        assert!(e1.all_day == false && e2.all_day);

        // 范围查询：只覆盖当天
        let range = repo.list_between(day0, day0 + 86399).unwrap();
        assert_eq!(range.len(), 1);
        assert_eq!(range[0].id, e1.id);

        let all = repo.list_between(day0, day0 + 172800).unwrap();
        assert_eq!(all.len(), 2);

        repo.update(
            e1.id,
            None,
            "专家拜访 张教授（改期）",
            day0 + 7200,
            None,
            false,
            "kol_visit",
            Some("线上"),
            None,
        )
        .unwrap();
        let updated = repo.get(e1.id).unwrap().unwrap();
        assert_eq!(updated.title, "专家拜访 张教授（改期）");
        assert_eq!(updated.location.as_deref(), Some("线上"));

        repo.delete(e2.id).unwrap();
        assert_eq!(repo.list_by_work(1).unwrap().len(), 0);
    }
}
