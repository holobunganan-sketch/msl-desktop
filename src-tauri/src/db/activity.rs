//! activity_events repository（指南 §6.9，工作事实时间线核心）。

use rusqlite::{Connection, OptionalExtension, Row, params};

use super::{DbError, DbResult, now_unix};

#[derive(Debug, Clone)]
pub struct ActivityEvent {
    pub id: i64,
    pub timestamp: i64,
    pub event_type: String, // file.created / task.completed / ...
    pub workspace_id: Option<i64>,
    pub work_id: Option<i64>,
    pub entity_type: Option<String>,
    pub entity_id: Option<i64>,
    pub path: Option<String>,
    pub display_text: String,
    pub metadata_json: Option<String>,
    pub dedupe_key: Option<String>,
}

fn row_to_activity(row: &Row) -> rusqlite::Result<ActivityEvent> {
    Ok(ActivityEvent {
        id: row.get(0)?,
        timestamp: row.get(1)?,
        event_type: row.get(2)?,
        workspace_id: row.get(3)?,
        work_id: row.get(4)?,
        entity_type: row.get(5)?,
        entity_id: row.get(6)?,
        path: row.get(7)?,
        display_text: row.get(8)?,
        metadata_json: row.get(9)?,
        dedupe_key: row.get(10)?,
    })
}

pub struct ActivityRepo<'a> {
    conn: &'a Connection,
}

impl<'a> ActivityRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// 记录一条活动。
    #[allow(clippy::too_many_arguments)]
    pub fn insert(
        &self,
        event_type: &str,
        workspace_id: Option<i64>,
        work_id: Option<i64>,
        entity_type: Option<&str>,
        entity_id: Option<i64>,
        path: Option<&str>,
        display_text: &str,
        metadata_json: Option<&str>,
        dedupe_key: Option<&str>,
    ) -> DbResult<ActivityEvent> {
        let ts = now_unix();
        self.conn.execute(
            "INSERT INTO activity_events
               (timestamp, event_type, workspace_id, work_id, entity_type, entity_id,
                path, display_text, metadata_json, dedupe_key)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![ts, event_type, workspace_id, work_id, entity_type, entity_id,
                    path, display_text, metadata_json, dedupe_key],
        )?;
        let id = self.conn.last_insert_rowid();
        self.get(id)?
            .ok_or_else(|| DbError::NotFound("activity_event".into()))
    }

    pub fn get(&self, id: i64) -> DbResult<Option<ActivityEvent>> {
        self.conn
            .query_row(
                "SELECT id, timestamp, event_type, workspace_id, work_id, entity_type,
                        entity_id, path, display_text, metadata_json, dedupe_key
                 FROM activity_events WHERE id = ?1",
                [id],
                row_to_activity,
            )
            .optional()
            .map_err(DbError::from)
    }

    /// 按条件查询：时间范围 + 可选 work / event_type / path。
    /// 顺序：时间倒序（最新在前）。
    pub fn query(
        &self,
        from_ts: Option<i64>,
        to_ts: Option<i64>,
        work_id: Option<i64>,
        event_type: Option<&str>,
        path: Option<&str>,
        limit: Option<u32>,
    ) -> DbResult<Vec<ActivityEvent>> {
        let mut sql = String::from(
            "SELECT id, timestamp, event_type, workspace_id, work_id, entity_type,
                    entity_id, path, display_text, metadata_json, dedupe_key
             FROM activity_events WHERE 1=1",
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(f) = from_ts {
            sql.push_str(" AND timestamp >= ?");
            params.push(Box::new(f));
        }
        if let Some(t) = to_ts {
            sql.push_str(" AND timestamp <= ?");
            params.push(Box::new(t));
        }
        if let Some(w) = work_id {
            sql.push_str(" AND work_id = ?");
            params.push(Box::new(w));
        }
        if let Some(e) = event_type {
            sql.push_str(" AND event_type = ?");
            params.push(Box::new(e.to_string()));
        }
        if let Some(p) = path {
            sql.push_str(" AND path = ?");
            params.push(Box::new(p.to_string()));
        }
        sql.push_str(" ORDER BY timestamp DESC, id DESC");
        if let Some(l) = limit {
            sql.push_str(&format!(" LIMIT {l}"));
        }
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), row_to_activity)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }

    /// 按 dedupe_key 查重（用于文件事件 coalescing）。
    pub fn find_by_dedupe_key(&self, dedupe_key: &str) -> DbResult<Option<ActivityEvent>> {
        self.conn
            .query_row(
                "SELECT id, timestamp, event_type, workspace_id, work_id, entity_type,
                        entity_id, path, display_text, metadata_json, dedupe_key
                 FROM activity_events WHERE dedupe_key = ?1 ORDER BY id DESC LIMIT 1",
                [dedupe_key],
                row_to_activity,
            )
            .optional()
            .map_err(DbError::from)
    }

    /// 删除单条（管理用）。
    pub fn delete(&self, id: i64) -> DbResult<()> {
        let affected = self
            .conn
            .execute("DELETE FROM activity_events WHERE id = ?1", [id])?;
        if affected == 0 {
            return Err(DbError::NotFound("activity_event".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn activity_insert_and_query() {
        let db = Database::open_in_memory().unwrap();
        let repo = ActivityRepo::new(db.conn());
        // 先创建 work，满足外键约束
        crate::db::work::WorkRepo::new(db.conn())
            .insert("老年破伤风 IIT", "active")
            .unwrap();

        let a = repo
            .insert("file.modified", None, Some(1), Some("file"), None,
                    Some("C:\\Work\\IIT\\方案V3.docx"), "修改了 方案V3.docx", None, Some("dedupe:1"))
            .unwrap();
        repo.insert("task.completed", None, Some(1), Some("task"), Some(7),
                    None, "完成 核对入排标准", None, None)
            .unwrap();
        assert!(a.id > 0);

        // 按 work 过滤
        let by_work = repo.query(None, None, Some(1), None, None, None).unwrap();
        assert_eq!(by_work.len(), 2);
        // 最新在前
        assert_eq!(by_work[0].event_type, "task.completed");

        // 按 event_type 过滤
        let files = repo.query(None, None, None, Some("file.modified"), None, None).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path.as_deref(), Some("C:\\Work\\IIT\\方案V3.docx"));

        // 按 path 过滤
        let by_path = repo.query(None, None, None, None, Some("C:\\Work\\IIT\\方案V3.docx"), None).unwrap();
        assert_eq!(by_path.len(), 1);

        // dedupe 查重
        let dup = repo.find_by_dedupe_key("dedupe:1").unwrap().unwrap();
        assert_eq!(dup.id, a.id);

        // limit
        let limited = repo.query(None, None, None, None, None, Some(1)).unwrap();
        assert_eq!(limited.len(), 1);
    }
}
