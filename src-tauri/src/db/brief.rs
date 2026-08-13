//! daily_briefs repository（指南 §6.10）。

use rusqlite::{Connection, OptionalExtension, Row, params};

use super::{DbError, DbResult, now_unix};

#[derive(Debug, Clone)]
pub struct DailyBrief {
    pub id: i64,
    pub brief_date: String, // YYYY-MM-DD
    pub generated_at: i64,
    pub provider_id: Option<String>,
    pub content: String,
    pub source_snapshot_hash: Option<String>,
}

fn row_to_brief(row: &Row) -> rusqlite::Result<DailyBrief> {
    Ok(DailyBrief {
        id: row.get(0)?,
        brief_date: row.get(1)?,
        generated_at: row.get(2)?,
        provider_id: row.get(3)?,
        content: row.get(4)?,
        source_snapshot_hash: row.get(5)?,
    })
}

pub struct BriefRepo<'a> {
    conn: &'a Connection,
}

impl<'a> BriefRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// 保存一份 Brief。
    pub fn insert(
        &self,
        brief_date: &str,
        provider_id: Option<&str>,
        content: &str,
        source_snapshot_hash: Option<&str>,
    ) -> DbResult<DailyBrief> {
        self.conn.execute(
            "INSERT INTO daily_briefs (brief_date, generated_at, provider_id, content, source_snapshot_hash)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![brief_date, now_unix(), provider_id, content, source_snapshot_hash],
        )?;
        let id = self.conn.last_insert_rowid();
        self.get(id)?
            .ok_or_else(|| DbError::NotFound("daily_brief".into()))
    }

    pub fn get(&self, id: i64) -> DbResult<Option<DailyBrief>> {
        self.conn
            .query_row(
                "SELECT id, brief_date, generated_at, provider_id, content, source_snapshot_hash
                 FROM daily_briefs WHERE id = ?1",
                [id],
                row_to_brief,
            )
            .optional()
            .map_err(DbError::from)
    }

    /// 某日期最新的 Brief（同日输入无变化时复用）。
    pub fn latest_for_date(&self, brief_date: &str) -> DbResult<Option<DailyBrief>> {
        self.conn
            .query_row(
                "SELECT id, brief_date, generated_at, provider_id, content, source_snapshot_hash
                 FROM daily_briefs WHERE brief_date = ?1
                 ORDER BY generated_at DESC, id DESC LIMIT 1",
                [brief_date],
                row_to_brief,
            )
            .optional()
            .map_err(DbError::from)
    }

    pub fn delete(&self, id: i64) -> DbResult<()> {
        let affected = self
            .conn
            .execute("DELETE FROM daily_briefs WHERE id = ?1", [id])?;
        if affected == 0 {
            return Err(DbError::NotFound("daily_brief".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn brief_latest_for_date() {
        let db = Database::open_in_memory().unwrap();
        let repo = BriefRepo::new(db.conn());

        let b1 = repo
            .insert("2026-08-13", Some("deepseek"), "昨天推进了统计方案", Some("hash-v1"))
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        let b2 = repo
            .insert("2026-08-13", Some("deepseek"), "重新生成：补充 waiting 跟进", Some("hash-v2"))
            .unwrap();

        let latest = repo.latest_for_date("2026-08-13").unwrap().unwrap();
        assert_eq!(latest.id, b2.id);
        assert_eq!(latest.source_snapshot_hash.as_deref(), Some("hash-v2"));

        assert!(repo.latest_for_date("2026-08-14").unwrap().is_none());

        repo.delete(b1.id).unwrap();
        assert_eq!(repo.latest_for_date("2026-08-13").unwrap().unwrap().id, b2.id);
    }
}
