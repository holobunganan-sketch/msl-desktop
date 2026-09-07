//! Persistence for model-generated weekly and monthly reports.

use rusqlite::{params, Connection, OptionalExtension, Row};

use super::{now_unix, DbError, DbResult};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Report {
    pub id: i64,
    pub kind: String,
    pub period_start: i64,
    pub period_end: i64,
    pub status: String,
    pub provider_model_id: Option<i64>,
    pub content: Option<String>,
    pub snapshot_hash: Option<String>,
    pub source_counts_json: String,
    pub source_report_ids_json: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub retention_state: String,
    pub generated_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

fn row_report(row: &Row<'_>) -> rusqlite::Result<Report> {
    Ok(Report {
        id: row.get(0)?,
        kind: row.get(1)?,
        period_start: row.get(2)?,
        period_end: row.get(3)?,
        status: row.get(4)?,
        provider_model_id: row.get(5)?,
        content: row.get(6)?,
        snapshot_hash: row.get(7)?,
        source_counts_json: row.get(8)?,
        source_report_ids_json: row.get(9)?,
        error_code: row.get(10)?,
        error_message: row.get(11)?,
        retention_state: row.get(12)?,
        generated_at: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

const REPORT_SELECT: &str = "SELECT id,kind,period_start,period_end,status,provider_model_id,content,snapshot_hash,source_counts_json,source_report_ids_json,error_code,error_message,retention_state,generated_at,created_at,updated_at FROM reports";

pub struct ReportRepo<'a> {
    conn: &'a Connection,
}

impl<'a> ReportRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn create(&self, kind: &str, period_start: i64, period_end: i64) -> DbResult<Report> {
        if !["weekly", "monthly"].contains(&kind) || period_end <= period_start {
            return Err(DbError::Migration("invalid report period or kind".into()));
        }
        let now = now_unix();
        self.conn.execute(
            "INSERT INTO reports(kind,period_start,period_end,status,created_at,updated_at) VALUES(?1,?2,?3,'running',?4,?4)",
            params![kind, period_start, period_end, now],
        )?;
        self.get(self.conn.last_insert_rowid())?
            .ok_or_else(|| DbError::NotFound("report".into()))
    }

    pub fn get(&self, id: i64) -> DbResult<Option<Report>> {
        self.conn
            .query_row(&format!("{} WHERE id=?1", REPORT_SELECT), [id], row_report)
            .optional()
            .map_err(DbError::from)
    }

    pub fn complete(
        &self,
        id: i64,
        provider_model_id: i64,
        content: &str,
        snapshot_hash: &str,
        source_counts_json: &str,
        source_report_ids_json: &str,
    ) -> DbResult<Report> {
        let now = now_unix();
        let changed = self.conn.execute(
            "UPDATE reports SET status='completed',provider_model_id=?1,content=?2,snapshot_hash=?3,source_counts_json=?4,source_report_ids_json=?5,error_code=NULL,error_message=NULL,generated_at=?6,updated_at=?6 WHERE id=?7 AND status='running'",
            params![provider_model_id, content, snapshot_hash, source_counts_json, source_report_ids_json, now, id],
        )?;
        if changed == 0 {
            return Err(DbError::Migration("report is no longer running".into()));
        }
        self.get(id)?
            .ok_or_else(|| DbError::NotFound("report".into()))
    }

    pub fn fail(&self, id: i64, code: &str, message: &str) -> DbResult<Report> {
        let now = now_unix();
        let short = message.chars().take(500).collect::<String>();
        let changed = self.conn.execute(
            "UPDATE reports SET status='failed',error_code=?1,error_message=?2,generated_at=?3,updated_at=?3 WHERE id=?4 AND status='running'",
            params![code, short, now, id],
        )?;
        if changed == 0 {
            return Err(DbError::Migration("report is no longer running".into()));
        }
        self.get(id)?
            .ok_or_else(|| DbError::NotFound("report".into()))
    }

    pub fn list(&self, kind: Option<&str>, limit: usize) -> DbResult<Vec<Report>> {
        let limit = limit.clamp(1, 200) as i64;
        let mut values = Vec::new();
        if let Some(kind) = kind {
            let mut stmt = self.conn.prepare(&format!(
                "{} WHERE kind=?1 ORDER BY id DESC LIMIT ?2",
                REPORT_SELECT
            ))?;
            let rows = stmt.query_map(params![kind, limit], row_report)?;
            for row in rows {
                values.push(row?);
            }
        } else {
            let mut stmt = self
                .conn
                .prepare(&format!("{} ORDER BY id DESC LIMIT ?1", REPORT_SELECT))?;
            let rows = stmt.query_map([limit], row_report)?;
            for row in rows {
                values.push(row?);
            }
        }
        Ok(values)
    }

    pub fn list_overlapping_weekly(
        &self,
        period_start: i64,
        period_end: i64,
        limit: usize,
    ) -> DbResult<Vec<Report>> {
        let mut stmt = self.conn.prepare(&format!(
            "{} WHERE kind='weekly' AND status='completed' AND period_start < ?2 AND period_end > ?1 ORDER BY period_start ASC,id ASC LIMIT ?3",
            REPORT_SELECT
        ))?;
        let rows = stmt.query_map(
            params![period_start, period_end, limit.clamp(1, 100) as i64],
            row_report,
        )?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }

    pub fn clear_history(&self, kind: &str) -> DbResult<usize> {
        if kind != "weekly" {
            return Err(DbError::Migration(
                "only weekly report history can be cleared".into(),
            ));
        }
        self.conn
            .execute(
                "DELETE FROM reports WHERE kind='weekly' AND status<>'running'",
                [],
            )
            .map_err(DbError::from)
    }

    pub fn keep(&self, id: i64) -> DbResult<()> {
        let changed = self.conn.execute(
            "UPDATE reports SET retention_state='kept',updated_at=?1 WHERE id=?2",
            params![now_unix(), id],
        )?;
        if changed == 0 {
            return Err(DbError::NotFound("report".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReportSchedule {
    pub id: i64,
    pub weekly_enabled: bool,
    /// Monday=0, Sunday=6.
    pub weekly_weekday: i64,
    pub weekly_hour: i64,
    pub weekly_minute: i64,
    pub last_weekly_period_key: Option<String>,
    pub monthly_enabled: bool,
    pub monthly_day: i64,
    pub monthly_hour: i64,
    pub monthly_minute: i64,
    pub last_monthly_period_key: Option<String>,
    pub updated_at: i64,
}

pub struct ReportScheduleRepo<'a> {
    conn: &'a Connection,
}

impl<'a> ReportScheduleRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    pub fn get(&self) -> DbResult<ReportSchedule> {
        self.conn
            .query_row(
                "SELECT id,weekly_enabled,weekly_weekday,weekly_hour,weekly_minute,last_weekly_period_key,monthly_enabled,monthly_day,monthly_hour,monthly_minute,last_monthly_period_key,updated_at FROM report_schedule_state WHERE id=1",
                [],
                |row| {
                    Ok(ReportSchedule {
                        id: row.get(0)?,
                        weekly_enabled: row.get::<_, i64>(1)? != 0,
                        weekly_weekday: row.get(2)?,
                        weekly_hour: row.get(3)?,
                        weekly_minute: row.get(4)?,
                        last_weekly_period_key: row.get(5)?,
                        monthly_enabled: row.get::<_, i64>(6)? != 0,
                        monthly_day: row.get(7)?,
                        monthly_hour: row.get(8)?,
                        monthly_minute: row.get(9)?,
                        last_monthly_period_key: row.get(10)?,
                        updated_at: row.get(11)?,
                    })
                },
            )
            .map_err(DbError::from)
    }

    pub fn save(&self, schedule: &ReportSchedule) -> DbResult<()> {
        if !(0..=6).contains(&schedule.weekly_weekday)
            || !(0..=23).contains(&schedule.weekly_hour)
            || !(0..=59).contains(&schedule.weekly_minute)
            || !(1..=28).contains(&schedule.monthly_day)
            || !(0..=23).contains(&schedule.monthly_hour)
            || !(0..=59).contains(&schedule.monthly_minute)
        {
            return Err(DbError::Migration("invalid report schedule".into()));
        }
        self.conn.execute(
            "UPDATE report_schedule_state SET weekly_enabled=?1,weekly_weekday=?2,weekly_hour=?3,weekly_minute=?4,last_weekly_period_key=?5,monthly_enabled=?6,monthly_day=?7,monthly_hour=?8,monthly_minute=?9,last_monthly_period_key=?10,updated_at=?11 WHERE id=1",
            params![schedule.weekly_enabled as i64,schedule.weekly_weekday,schedule.weekly_hour,schedule.weekly_minute,schedule.last_weekly_period_key,schedule.monthly_enabled as i64,schedule.monthly_day,schedule.monthly_hour,schedule.monthly_minute,schedule.last_monthly_period_key,now_unix()],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn report_lifecycle_and_overlapping_weekly_lookup_are_persisted() {
        let db = Database::open_in_memory().unwrap();
        db.conn().execute_batch("INSERT INTO provider_settings(display_name,provider_type,base_url,model,enabled,created_at,updated_at) VALUES('Mock','openai_compatible','http://127.0.0.1','',1,1,1); INSERT INTO provider_models(provider_id,model_id,display_name,protocol,endpoint_path,source,created_at,updated_at) VALUES(1,'mock','Mock','chat_completions','/chat/completions','manual',1,1);").unwrap();
        let repo = ReportRepo::new(db.conn());
        let weekly = repo.create("weekly", 100, 200).unwrap();
        assert_eq!(weekly.status, "running");
        repo.complete(
            weekly.id,
            1,
            "1. 完成项目复盘",
            "snapshot-a",
            r#"{"works":1}"#,
            "[]",
        )
        .unwrap();
        let monthly = repo.create("monthly", 50, 250).unwrap();
        repo.fail(monthly.id, "route_unavailable", "未配置模型")
            .unwrap();

        let overlap = repo.list_overlapping_weekly(50, 250, 20).unwrap();
        assert_eq!(overlap.len(), 1);
        assert_eq!(overlap[0].content.as_deref(), Some("1. 完成项目复盘"));
        let rows = repo.list(None, 20).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].status, "failed");
    }

    #[test]
    fn report_schedule_defaults_and_updates_are_persisted() {
        let db = Database::open_in_memory().unwrap();
        let repo = ReportScheduleRepo::new(db.conn());
        let mut schedule = repo.get().unwrap();
        assert!(schedule.weekly_enabled);
        assert_eq!(schedule.weekly_weekday, 6);
        assert_eq!(schedule.weekly_hour, 17);
        assert_eq!(schedule.monthly_day, 1);
        assert_eq!(schedule.monthly_hour, 9);
        schedule.weekly_weekday = 4;
        schedule.weekly_hour = 16;
        schedule.last_weekly_period_key = Some("2026-08-10/2026-08-17".into());
        repo.save(&schedule).unwrap();
        let saved = repo.get().unwrap();
        assert_eq!(saved.weekly_weekday, 4);
        assert_eq!(saved.weekly_hour, 16);
        assert_eq!(
            saved.last_weekly_period_key.as_deref(),
            Some("2026-08-10/2026-08-17")
        );
    }

    #[test]
    fn clearing_weekly_history_keeps_running_and_monthly_reports() {
        let db = Database::open_in_memory().unwrap();
        db.conn().execute_batch("INSERT INTO provider_settings(display_name,provider_type,base_url,model,enabled,created_at,updated_at) VALUES('Mock','openai_compatible','http://127.0.0.1','',1,1,1); INSERT INTO provider_models(provider_id,model_id,display_name,protocol,endpoint_path,source,created_at,updated_at) VALUES(1,'mock','Mock','chat_completions','/chat/completions','manual',1,1);").unwrap();
        let repo = ReportRepo::new(db.conn());
        let completed = repo.create("weekly", 100, 200).unwrap();
        repo.complete(completed.id, 1, "1. 完成", "hash", "{}", "[]")
            .unwrap();
        let failed = repo.create("weekly", 200, 300).unwrap();
        repo.fail(failed.id, "provider_failed", "temporary")
            .unwrap();
        let running = repo.create("weekly", 300, 400).unwrap();
        let monthly = repo.create("monthly", 100, 400).unwrap();
        repo.fail(monthly.id, "provider_failed", "temporary")
            .unwrap();

        assert_eq!(repo.clear_history("weekly").unwrap(), 2);
        let remaining = repo.list(None, 20).unwrap();
        assert_eq!(remaining.len(), 2);
        assert!(remaining.iter().any(|item| item.id == running.id));
        assert!(remaining.iter().any(|item| item.id == monthly.id));
        assert!(repo.clear_history("monthly").is_err());
    }
}
