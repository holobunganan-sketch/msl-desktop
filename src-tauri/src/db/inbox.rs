//! inbox_items repository（指南 §6.7）。

use rusqlite::{Connection, OptionalExtension, Row, params};

use super::{DbError, DbResult, now_unix};

#[derive(Debug, Clone)]
pub struct InboxItem {
    pub id: i64,
    pub content: String,
    pub created_at: i64,
    pub processed_at: Option<i64>,
    pub converted_to_type: Option<String>,
    pub converted_to_id: Option<i64>,
}

fn row_to_inbox(row: &Row) -> rusqlite::Result<InboxItem> {
    Ok(InboxItem {
        id: row.get(0)?,
        content: row.get(1)?,
        created_at: row.get(2)?,
        processed_at: row.get(3)?,
        converted_to_type: row.get(4)?,
        converted_to_id: row.get(5)?,
    })
}

pub struct InboxRepo<'a> {
    conn: &'a Connection,
}

impl<'a> InboxRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Quick Capture 写入 Inbox。
    pub fn insert(&self, content: &str) -> DbResult<InboxItem> {
        let now = now_unix();
        self.conn.execute(
            "INSERT INTO inbox_items (content, created_at) VALUES (?1, ?2)",
            params![content, now],
        )?;
        let id = self.conn.last_insert_rowid();
        self.get(id)?
            .ok_or_else(|| DbError::NotFound("inbox_item".into()))
    }

    pub fn get(&self, id: i64) -> DbResult<Option<InboxItem>> {
        self.conn
            .query_row(
                "SELECT id, content, created_at, processed_at, converted_to_type, converted_to_id
                 FROM inbox_items WHERE id = ?1",
                [id],
                row_to_inbox,
            )
            .optional()
            .map_err(DbError::from)
    }

    /// 列出（默认未处理在前）。
    pub fn list(&self) -> DbResult<Vec<InboxItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, content, created_at, processed_at, converted_to_type, converted_to_id
             FROM inbox_items ORDER BY (processed_at IS NOT NULL), created_at DESC, id DESC",
        )?;
        let rows = stmt.query_map([], row_to_inbox)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }

    /// 标记已处理并记录转换目标（例如转成 task/waiting/calendar）。
    pub fn mark_processed(&self, id: i64, converted_to_type: &str, converted_to_id: i64) -> DbResult<()> {
        let affected = self.conn.execute(
            "UPDATE inbox_items SET processed_at = ?1, converted_to_type = ?2, converted_to_id = ?3 WHERE id = ?4",
            params![now_unix(), converted_to_type, converted_to_id, id],
        )?;
        if affected == 0 {
            return Err(DbError::NotFound("inbox_item".into()));
        }
        Ok(())
    }

    pub fn delete(&self, id: i64) -> DbResult<()> {
        let affected = self
            .conn
            .execute("DELETE FROM inbox_items WHERE id = ?1", [id])?;
        if affected == 0 {
            return Err(DbError::NotFound("inbox_item".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn inbox_crud_and_process() {
        let db = Database::open_in_memory().unwrap();
        let repo = InboxRepo::new(db.conn());

        let a = repo.insert("周五问张教授确认中心启动时间").unwrap();
        repo.insert("需要联系伦理老师").unwrap();

        assert_eq!(repo.list().unwrap().len(), 2);

        repo.mark_processed(a.id, "task", 42).unwrap();
        let processed = repo.get(a.id).unwrap().unwrap();
        assert!(processed.processed_at.is_some());
        assert_eq!(processed.converted_to_type.as_deref(), Some("task"));
        assert_eq!(processed.converted_to_id, Some(42));

        // 未处理在前
        let list = repo.list().unwrap();
        assert_eq!(list[0].id, a.id + 1); // 第二个未处理的排第一

        repo.delete(a.id).unwrap();
        assert_eq!(repo.list().unwrap().len(), 1);
    }
}
