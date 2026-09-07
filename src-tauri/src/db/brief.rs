//! daily_briefs repository（指南 §6.10）。

use rusqlite::{params, Connection, OptionalExtension, Row};

use super::{now_unix, DbError, DbResult};

#[derive(Debug, Clone, serde::Serialize)]
pub struct DailyBrief {
    pub id: i64,
    pub brief_date: String, // YYYY-MM-DD
    pub generated_at: i64,
    pub provider_id: Option<String>,
    pub content: String,
    pub source_snapshot_hash: Option<String>,
    pub period_start: Option<i64>,
    pub period_end: Option<i64>,
    pub locale: Option<String>,
    pub source_snapshot_json: Option<String>,
    pub ai_used: bool,
    pub warning: Option<String>,
    pub retention_state: String,
    pub analysis_run_id: Option<i64>,
    pub superseded_at: Option<i64>,
}

fn row_to_brief(row: &Row) -> rusqlite::Result<DailyBrief> {
    Ok(DailyBrief {
        id: row.get(0)?,
        brief_date: row.get(1)?,
        generated_at: row.get(2)?,
        provider_id: row.get(3)?,
        content: row.get(4)?,
        source_snapshot_hash: row.get(5)?,
        period_start: row.get(6)?,
        period_end: row.get(7)?,
        locale: row.get(8)?,
        source_snapshot_json: row.get(9)?,
        ai_used: row.get::<_, i64>(10)? != 0,
        warning: row.get(11)?,
        retention_state: row.get(12)?,
        analysis_run_id: row.get(13)?,
        superseded_at: row.get(14)?,
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

    #[allow(clippy::too_many_arguments)]
    pub fn insert_with_snapshot(
        &self,
        brief_date: &str,
        provider_id: Option<&str>,
        content: &str,
        source_snapshot_hash: Option<&str>,
        period_start: Option<i64>,
        period_end: Option<i64>,
        locale: Option<&str>,
        source_snapshot_json: Option<&str>,
        ai_used: bool,
        warning: Option<&str>,
    ) -> DbResult<DailyBrief> {
        self.conn.execute(
            "INSERT INTO daily_briefs
             (brief_date, generated_at, provider_id, content, source_snapshot_hash,
             period_start, period_end, locale, source_snapshot_json, ai_used, warning, retention_state)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'kept')",
            params![
                brief_date,
                now_unix(),
                provider_id,
                content,
                source_snapshot_hash,
                period_start,
                period_end,
                locale,
                source_snapshot_json,
                ai_used as i64,
                warning,
            ],
        )?;
        let id = self.conn.last_insert_rowid();
        self.get(id)?
            .ok_or_else(|| DbError::NotFound("daily_brief".into()))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert_with_retention(
        &self,
        brief_date: &str,
        provider_id: Option<&str>,
        content: &str,
        source_snapshot_hash: Option<&str>,
        period_start: Option<i64>,
        period_end: Option<i64>,
        locale: Option<&str>,
        source_snapshot_json: Option<&str>,
        ai_used: bool,
        warning: Option<&str>,
        retention_state: &str,
        analysis_run_id: Option<i64>,
    ) -> DbResult<DailyBrief> {
        let now = now_unix();
        if retention_state != "draft"
            && retention_state != "kept"
            && retention_state != "superseded"
        {
            return Err(DbError::Migration("invalid brief retention state".into()));
        }
        if retention_state == "draft" {
            self.conn.execute("UPDATE daily_briefs SET retention_state='superseded', superseded_at=?1 WHERE brief_date=?2 AND retention_state='draft'", params![now, brief_date])?;
        }
        self.conn.execute("INSERT INTO daily_briefs (brief_date,generated_at,provider_id,content,source_snapshot_hash,period_start,period_end,locale,source_snapshot_json,ai_used,warning,retention_state,analysis_run_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)", params![brief_date,now,provider_id,content,source_snapshot_hash,period_start,period_end,locale,source_snapshot_json,ai_used as i64,warning,retention_state,analysis_run_id])?;
        self.get(self.conn.last_insert_rowid())?
            .ok_or_else(|| DbError::NotFound("daily_brief".into()))
    }

    pub fn keep(&self, id: i64) -> DbResult<()> {
        let n = self.conn.execute(
            "UPDATE daily_briefs SET retention_state='kept', superseded_at=NULL WHERE id=?1",
            [id],
        )?;
        if n == 0 {
            return Err(DbError::NotFound("daily_brief".into()));
        }
        Ok(())
    }

    pub fn latest_visible_for_date(&self, date: &str) -> DbResult<Option<DailyBrief>> {
        self.conn.query_row("SELECT id,brief_date,generated_at,provider_id,content,source_snapshot_hash,period_start,period_end,locale,source_snapshot_json,ai_used,warning,retention_state,analysis_run_id,superseded_at FROM daily_briefs WHERE brief_date=?1 AND retention_state<>'superseded' ORDER BY CASE retention_state WHEN 'draft' THEN 0 ELSE 1 END, generated_at DESC, id DESC LIMIT 1", [date], row_to_brief).optional().map_err(DbError::from)
    }

    pub fn get(&self, id: i64) -> DbResult<Option<DailyBrief>> {
        self.conn
            .query_row(
                "SELECT id, brief_date, generated_at, provider_id, content, source_snapshot_hash,
                        period_start, period_end, locale, source_snapshot_json, ai_used, warning, retention_state, analysis_run_id, superseded_at
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
                "SELECT id, brief_date, generated_at, provider_id, content, source_snapshot_hash,
                        period_start, period_end, locale, source_snapshot_json, ai_used, warning, retention_state, analysis_run_id, superseded_at
                 FROM daily_briefs WHERE brief_date = ?1 AND retention_state <> 'superseded'
                 ORDER BY generated_at DESC, id DESC LIMIT 1",
                [brief_date],
                row_to_brief,
            )
            .optional()
            .map_err(DbError::from)
    }

    pub fn list(&self, limit: u32) -> DbResult<Vec<DailyBrief>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, brief_date, generated_at, provider_id, content, source_snapshot_hash,
                    period_start, period_end, locale, source_snapshot_json, ai_used, warning, retention_state, analysis_run_id, superseded_at
             FROM daily_briefs ORDER BY generated_at DESC, id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit as i64], row_to_brief)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
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
            .insert(
                "2026-08-13",
                Some("deepseek"),
                "昨天推进了统计方案",
                Some("hash-v1"),
            )
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
        let b2 = repo
            .insert(
                "2026-08-13",
                Some("deepseek"),
                "重新生成：补充 waiting 跟进",
                Some("hash-v2"),
            )
            .unwrap();

        let latest = repo.latest_for_date("2026-08-13").unwrap().unwrap();
        assert_eq!(latest.id, b2.id);
        assert_eq!(latest.source_snapshot_hash.as_deref(), Some("hash-v2"));
        assert!(!latest.ai_used);
        assert!(latest.source_snapshot_json.is_none());

        assert!(repo.latest_for_date("2026-08-14").unwrap().is_none());

        repo.delete(b1.id).unwrap();
        assert_eq!(
            repo.latest_for_date("2026-08-13").unwrap().unwrap().id,
            b2.id
        );
        assert_eq!(repo.list(1).unwrap().len(), 1);
        assert_eq!(repo.list(0).unwrap().len(), 0);
    }

    #[test]
    fn draft_is_superseded_and_kept_is_protected() {
        let db = Database::open_in_memory().unwrap();
        let repo = BriefRepo::new(db.conn());
        let first = repo
            .insert_with_retention(
                "2026-08-15",
                None,
                "draft 1",
                Some("h1"),
                None,
                None,
                Some("zh-CN"),
                None,
                false,
                None,
                "draft",
                None,
            )
            .unwrap();
        let second = repo
            .insert_with_retention(
                "2026-08-15",
                None,
                "draft 2",
                Some("h2"),
                None,
                None,
                Some("zh-CN"),
                None,
                false,
                None,
                "draft",
                None,
            )
            .unwrap();
        assert_eq!(
            repo.get(first.id).unwrap().unwrap().retention_state,
            "superseded"
        );
        repo.keep(second.id).unwrap();
        assert_eq!(
            repo.latest_visible_for_date("2026-08-15")
                .unwrap()
                .unwrap()
                .retention_state,
            "kept"
        );
    }
}
