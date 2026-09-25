//! Persistence for analysis runs, schedules and confirmation proposals.

use super::{now_unix, DbError, DbResult};
use rusqlite::{params, Connection, OptionalExtension, Row};

#[derive(Debug, Clone, serde::Serialize)]
pub struct AnalysisRun {
    pub id: i64,
    pub trigger: String,
    pub status: String,
    pub period_start: Option<i64>,
    pub period_end: Option<i64>,
    pub provider_model_id: Option<i64>,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub source_counts_json: Option<String>,
    pub snapshot_hash: Option<String>,
    pub summary: Option<String>,
    pub brief_id: Option<i64>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub created_at: i64,
}

fn row_run(row: &Row) -> rusqlite::Result<AnalysisRun> {
    Ok(AnalysisRun {
        id: row.get(0)?,
        trigger: row.get(1)?,
        status: row.get(2)?,
        period_start: row.get(3)?,
        period_end: row.get(4)?,
        provider_model_id: row.get(5)?,
        started_at: row.get(6)?,
        finished_at: row.get(7)?,
        source_counts_json: row.get(8)?,
        snapshot_hash: row.get(9)?,
        summary: row.get(10)?,
        brief_id: row.get(11)?,
        error_code: row.get(12)?,
        error_message: row.get(13)?,
        created_at: row.get(14)?,
    })
}

pub struct AnalysisRunRepo<'a> {
    conn: &'a Connection,
}
impl<'a> AnalysisRunRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
    pub fn create(
        &self,
        trigger: &str,
        period_start: Option<i64>,
        period_end: Option<i64>,
    ) -> DbResult<AnalysisRun> {
        let now = now_unix();
        self.conn.execute("INSERT INTO analysis_runs (trigger,status,period_start,period_end,started_at,created_at) VALUES (?1,'running',?2,?3,?4,?4)", params![trigger,period_start,period_end,now])?;
        self.get(self.conn.last_insert_rowid())?
            .ok_or_else(|| DbError::NotFound("analysis_run".into()))
    }
    pub fn get(&self, id: i64) -> DbResult<Option<AnalysisRun>> {
        self.conn.query_row("SELECT id,trigger,status,period_start,period_end,provider_model_id,started_at,finished_at,source_counts_json,snapshot_hash,summary,brief_id,error_code,error_message,created_at FROM analysis_runs WHERE id=?1",[id],row_run).optional().map_err(DbError::from)
    }
    pub fn finish(
        &self,
        id: i64,
        status: &str,
        summary: Option<&str>,
        error: Option<(&str, &str)>,
    ) -> DbResult<()> {
        self.conn.execute("UPDATE analysis_runs SET status=?1,summary=?2,error_code=?3,error_message=?4,finished_at=?5 WHERE id=?6",params![status,summary,error.map(|v|v.0),error.map(|v|v.1.chars().take(300).collect::<String>()),now_unix(),id])?;
        Ok(())
    }
    pub fn list(&self, limit: usize) -> DbResult<Vec<AnalysisRun>> {
        let mut stmt=self.conn.prepare("SELECT id,trigger,status,period_start,period_end,provider_model_id,started_at,finished_at,source_counts_json,snapshot_hash,summary,brief_id,error_code,error_message,created_at FROM analysis_runs ORDER BY id DESC LIMIT ?1")?;
        let rows = stmt.query_map([limit.clamp(1, 200) as i64], row_run)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }
    pub fn expire_stale_running(&self, cutoff_started_at: i64) -> DbResult<usize> {
        self.conn
            .execute(
                "UPDATE analysis_runs SET status='failed',error_code='interrupted',error_message='应用中断或旧版本未执行该分析',finished_at=?1 WHERE status='running' AND started_at < ?2",
                params![now_unix(), cutoff_started_at],
            )
            .map_err(DbError::from)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AiProposal {
    pub id: i64,
    pub analysis_run_id: Option<i64>,
    pub kind: String,
    pub suggested_kind: String,
    pub operation: String,
    pub target_id: Option<i64>,
    pub work_id: Option<i64>,
    pub suggested_work_id: Option<i64>,
    pub workspace_id: Option<i64>,
    pub dedupe_key: String,
    pub title: String,
    pub payload_json: String,
    pub reason: String,
    pub source_refs_json: String,
    pub confidence: Option<f64>,
    pub user_edited: bool,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub decided_at: Option<i64>,
    pub deferred_at: Option<i64>,
    pub applied_kind: Option<String>,
    pub applied_id: Option<i64>,
}
fn row_proposal(row: &Row) -> rusqlite::Result<AiProposal> {
    Ok(AiProposal {
        id: row.get(0)?,
        analysis_run_id: row.get(1)?,
        kind: row.get(2)?,
        suggested_kind: row.get(3)?,
        operation: row.get(4)?,
        target_id: row.get(5)?,
        work_id: row.get(6)?,
        suggested_work_id: row.get(7)?,
        workspace_id: row.get(8)?,
        dedupe_key: row.get(9)?,
        title: row.get(10)?,
        payload_json: row.get(11)?,
        reason: row.get(12)?,
        source_refs_json: row.get(13)?,
        confidence: row.get(14)?,
        user_edited: row.get::<_, i64>(15)? != 0,
        status: row.get(16)?,
        created_at: row.get(17)?,
        updated_at: row.get(18)?,
        decided_at: row.get(19)?,
        deferred_at: row.get(20)?,
        applied_kind: row.get(21)?,
        applied_id: row.get(22)?,
    })
}
const PROPOSAL_SELECT:&str="SELECT id,analysis_run_id,kind,suggested_kind,operation,target_id,work_id,suggested_work_id,workspace_id,dedupe_key,title,payload_json,reason,source_refs_json,confidence,user_edited,status,created_at,updated_at,decided_at,deferred_at,(SELECT kind FROM ai_proposal_outcomes WHERE proposal_id=ai_proposals.id),(SELECT target_id FROM ai_proposal_outcomes WHERE proposal_id=ai_proposals.id) FROM ai_proposals";
pub struct ProposalRepo<'a> {
    conn: &'a Connection,
}
impl<'a> ProposalRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
    pub fn get(&self, id: i64) -> DbResult<Option<AiProposal>> {
        self.conn
            .query_row(
                &format!("{} WHERE id=?1", PROPOSAL_SELECT),
                [id],
                row_proposal,
            )
            .optional()
            .map_err(DbError::from)
    }
    pub fn list(&self, status: Option<&str>, limit: usize) -> DbResult<Vec<AiProposal>> {
        let (sql, params): (String, Vec<Box<dyn rusqlite::ToSql>>) = if let Some(status) = status {
            (
                format!(
                    "{} WHERE status=?1 AND status<>'deleted' ORDER BY updated_at DESC,id DESC LIMIT ?2",
                    PROPOSAL_SELECT
                ),
                vec![
                    Box::new(status.to_string()),
                    Box::new(limit.clamp(1, 200) as i64),
                ],
            )
        } else {
            (
                format!(
                    "{} WHERE status<>'deleted' ORDER BY updated_at DESC,id DESC LIMIT ?1",
                    PROPOSAL_SELECT
                ),
                vec![Box::new(limit.clamp(1, 200) as i64)],
            )
        };
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params.iter()), row_proposal)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }
    pub fn list_latest_run_pending(&self, limit: usize) -> DbResult<Vec<AiProposal>> {
        let sql = format!(
            "{} WHERE status='pending' AND deferred_at IS NULL AND analysis_run_id=(
             SELECT ar.id FROM analysis_runs ar
             WHERE ar.status='completed'
             AND EXISTS(SELECT 1 FROM ai_proposals p WHERE p.analysis_run_id=ar.id AND p.status='pending' AND p.deferred_at IS NULL)
             ORDER BY COALESCE(ar.finished_at,ar.started_at) DESC,ar.id DESC LIMIT 1
             ) ORDER BY confidence DESC,updated_at DESC,id DESC LIMIT ?1",
            PROPOSAL_SELECT
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([limit.clamp(1, 100) as i64], row_proposal)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }
    pub fn list_since(
        &self,
        cutoff_created_at: i64,
        status: Option<&str>,
        limit: usize,
    ) -> DbResult<Vec<AiProposal>> {
        let limit = limit.clamp(1, 500) as i64;
        let mut rows = Vec::new();
        if let Some(status) = status {
            let sql = format!(
                "{} WHERE created_at>=?1 AND status=?2 AND status<>'deleted' ORDER BY created_at DESC,id DESC LIMIT ?3",
                PROPOSAL_SELECT
            );
            let mut stmt = self.conn.prepare(&sql)?;
            let mapped = stmt.query_map(params![cutoff_created_at, status, limit], row_proposal)?;
            for row in mapped {
                rows.push(row?);
            }
        } else {
            let sql = format!(
                "{} WHERE created_at>=?1 AND status<>'deleted' ORDER BY created_at DESC,id DESC LIMIT ?2",
                PROPOSAL_SELECT
            );
            let mut stmt = self.conn.prepare(&sql)?;
            let mapped = stmt.query_map(params![cutoff_created_at, limit], row_proposal)?;
            for row in mapped {
                rows.push(row?);
            }
        }
        Ok(rows)
    }
    pub fn upsert_pending(
        &self,
        run_id: i64,
        kind: &str,
        operation: &str,
        target_id: Option<i64>,
        work_id: Option<i64>,
        workspace_id: Option<i64>,
        dedupe_key: &str,
        title: &str,
        payload_json: &str,
        reason: &str,
        source_refs_json: &str,
        confidence: Option<f64>,
    ) -> DbResult<Option<AiProposal>> {
        let now = now_unix();
        let payload = serde_json::from_str::<serde_json::Value>(payload_json).unwrap_or_default();
        let mut sources = serde_json::from_str::<Vec<serde_json::Value>>(source_refs_json)
            .unwrap_or_default()
            .into_iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>();
        sources.sort();
        let signature = crate::cognition::digest(&format!("{payload}|{}", sources.join("|")));
        let decided: bool=self.conn.query_row("SELECT EXISTS(SELECT 1 FROM ai_proposals WHERE dedupe_key=?1 AND status IN ('confirmed','rejected','superseded') AND input_signature=?2)",params![dedupe_key,signature],|r|r.get(0))?;
        if decided {
            return Ok(None);
        }
        let existing = self
            .conn
            .query_row(
                &format!(
                    "{} WHERE dedupe_key=?1 AND status='pending'",
                    PROPOSAL_SELECT
                ),
                [dedupe_key],
                row_proposal,
            )
            .optional()?;
        if let Some(existing) = existing {
            // A pending opinion belongs to the user's current round. Never
            // replace it with a later model response, even before user editing.
            return Ok(Some(existing));
        }
        self.conn.execute("INSERT INTO ai_proposals (analysis_run_id,kind,suggested_kind,operation,target_id,work_id,suggested_work_id,workspace_id,dedupe_key,title,payload_json,reason,source_refs_json,confidence,status,created_at,updated_at) VALUES (?1,?2,?2,?3,?4,?5,?5,?6,?7,?8,?9,?10,?11,?12,'pending',?13,?13)",params![run_id,kind,operation,target_id,work_id,workspace_id,dedupe_key,title,payload_json,reason,source_refs_json,confidence,now])?;
        let id = self.conn.last_insert_rowid();
        self.conn.execute(
            "UPDATE ai_proposals SET input_signature=?1 WHERE id=?2",
            params![signature, id],
        )?;
        self.get(id)
    }
    pub fn update_draft(
        &self,
        id: i64,
        expected_updated_at: i64,
        title: &str,
        payload_json: &str,
    ) -> DbResult<AiProposal> {
        let next = now_unix().max(expected_updated_at.saturating_add(1));
        let changed=self.conn.execute("UPDATE ai_proposals SET title=?1,payload_json=?2,user_edited=1,updated_at=?3 WHERE id=?4 AND status='pending' AND updated_at=?5",params![title,payload_json,next,id,expected_updated_at])?;
        if changed == 0 {
            return Err(DbError::Migration(
                "proposal is stale or no longer pending".into(),
            ));
        }
        self.get(id)?
            .ok_or_else(|| DbError::NotFound("proposal".into()))
    }
    pub fn update_classification(
        &self,
        id: i64,
        expected_updated_at: i64,
        kind: &str,
        work_id: Option<i64>,
        title: &str,
        payload_json: &str,
    ) -> DbResult<AiProposal> {
        if ![
            "work",
            "task",
            "waiting",
            "calendar",
            "inbox",
            "resume_point",
        ]
        .contains(&kind)
        {
            return Err(DbError::Migration("invalid proposal kind".into()));
        }
        if title.trim().is_empty() {
            return Err(DbError::Migration("proposal title is required".into()));
        }
        let payload = serde_json::from_str::<serde_json::Value>(payload_json)
            .map_err(|_| DbError::Migration("proposal payload must be valid json".into()))?;
        crate::ai::schema::validate_payload_fields(kind, &payload).map_err(DbError::Migration)?;
        crate::db::work::validate_project_scope(self.conn, kind, work_id)?;
        let work_id = if matches!(kind, "work" | "inbox") {
            None
        } else {
            work_id
        };
        let next = now_unix().max(expected_updated_at.saturating_add(1));
        let changed = self.conn.execute(
            "UPDATE ai_proposals SET operation=CASE WHEN kind<>?1 THEN 'create' ELSE operation END,target_id=CASE WHEN kind<>?1 THEN NULL ELSE target_id END,kind=?1,work_id=?2,title=?3,payload_json=?4,user_edited=1,deferred_at=NULL,updated_at=?5 WHERE id=?6 AND status='pending' AND updated_at=?7",
            params![kind,work_id,title.trim(),payload_json,next,id,expected_updated_at],
        )?;
        if changed == 0 {
            return Err(DbError::Migration(
                "proposal is stale or no longer pending".into(),
            ));
        }
        self.conn.execute(
            "DELETE FROM secretary_proposal_scopes WHERE proposal_id=?1",
            [id],
        )?;
        self.get(id)?
            .ok_or_else(|| DbError::NotFound("proposal".into()))
    }
    pub fn defer(&self, id: i64, expected_updated_at: i64) -> DbResult<AiProposal> {
        let next = now_unix().max(expected_updated_at.saturating_add(1));
        let changed = self.conn.execute(
            "UPDATE ai_proposals SET deferred_at=?1,updated_at=?1 WHERE id=?2 AND status='pending' AND updated_at=?3",
            params![next,id,expected_updated_at],
        )?;
        if changed == 0 {
            return Err(DbError::Migration(
                "proposal is stale or no longer pending".into(),
            ));
        }
        self.get(id)?
            .ok_or_else(|| DbError::NotFound("proposal".into()))
    }
    pub fn set_status(&self, id: i64, status: &str, expected_updated_at: i64) -> DbResult<()> {
        if !["pending", "confirmed", "rejected", "superseded"].contains(&status) {
            return Err(DbError::Migration("invalid proposal status".into()));
        }
        let n=self.conn.execute("UPDATE ai_proposals SET status=?1,decided_at=CASE WHEN ?1='pending' THEN NULL ELSE ?2 END,updated_at=?2 WHERE id=?3 AND status='pending' AND updated_at=?4",params![status,now_unix(),id,expected_updated_at])?;
        if n == 0 {
            return Err(DbError::Migration(
                "proposal is stale or no longer pending".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AnalysisSchedule {
    pub id: i64,
    pub enabled: bool,
    pub interval_minutes: i64,
    pub daily_enabled: bool,
    pub daily_hour: i64,
    pub daily_minute: i64,
    pub last_interval_run_at: Option<i64>,
    pub last_daily_local_date: Option<String>,
    pub updated_at: i64,
}
pub struct ScheduleRepo<'a> {
    conn: &'a Connection,
}
impl<'a> ScheduleRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
    pub fn get(&self) -> DbResult<AnalysisSchedule> {
        self.conn.query_row("SELECT id,enabled,interval_minutes,daily_enabled,daily_hour,daily_minute,last_interval_run_at,last_daily_local_date,updated_at FROM analysis_schedule_state WHERE id=1",[],|r|Ok(AnalysisSchedule{id:r.get(0)?,enabled:r.get::<_,i64>(1)? != 0,interval_minutes:r.get(2)?,daily_enabled:r.get::<_,i64>(3)? != 0,daily_hour:r.get(4)?,daily_minute:r.get(5)?,last_interval_run_at:r.get(6)?,last_daily_local_date:r.get(7)?,updated_at:r.get(8)?})).map_err(DbError::from)
    }
    pub fn save(&self, s: &AnalysisSchedule) -> DbResult<()> {
        if !(30..=1440).contains(&s.interval_minutes)
            || !(0..=23).contains(&s.daily_hour)
            || !(0..=59).contains(&s.daily_minute)
        {
            return Err(DbError::Migration("invalid schedule".into()));
        }
        self.conn.execute("UPDATE analysis_schedule_state SET enabled=?1,interval_minutes=?2,daily_enabled=?3,daily_hour=?4,daily_minute=?5,last_interval_run_at=?6,last_daily_local_date=?7,updated_at=?8 WHERE id=1",params![s.enabled as i64,s.interval_minutes,s.daily_enabled as i64,s.daily_hour,s.daily_minute,s.last_interval_run_at,s.last_daily_local_date,now_unix()])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_draft_can_be_edited_before_confirmation() {
        let db = crate::db::Database::open_in_memory().unwrap();
        let work = crate::db::work::WorkRepo::new(db.conn())
            .insert("Synthetic", "active")
            .unwrap();
        let run = AnalysisRunRepo::new(db.conn())
            .create("manual", None, None)
            .unwrap();
        let repo = ProposalRepo::new(db.conn());
        let p = repo
            .upsert_pending(
                run.id,
                "resume_point",
                "create",
                None,
                Some(work.id),
                None,
                "progress-edit",
                "Progress",
                r#"{"current_state":"received"}"#,
                "",
                "[]",
                None,
            )
            .unwrap()
            .unwrap();
        let edited = repo
            .update_classification(
                p.id,
                p.updated_at,
                "resume_point",
                Some(work.id),
                "Progress",
                r#"{"current_state":"reviewed"}"#,
            )
            .unwrap();
        assert_eq!(edited.kind, "resume_point");
        assert!(edited.payload_json.contains("reviewed"));
        assert_eq!(edited.work_id, Some(work.id));
    }

    #[test]
    fn decided_identical_proposal_is_suppressed_but_new_evidence_is_allowed() {
        let db = crate::db::Database::open_in_memory().unwrap();
        let run = AnalysisRunRepo::new(db.conn())
            .create("manual", None, None)
            .unwrap();
        let repo = ProposalRepo::new(db.conn());
        let first = repo
            .upsert_pending(
                run.id,
                "task",
                "create",
                None,
                None,
                None,
                "history-key",
                "Follow up",
                r#"{"notes":"original"}"#,
                "reason",
                "[]",
                Some(0.8),
            )
            .unwrap()
            .unwrap();
        repo.set_status(first.id, "rejected", first.updated_at)
            .unwrap();
        assert!(repo
            .upsert_pending(
                run.id,
                "task",
                "create",
                None,
                None,
                None,
                "history-key",
                "Follow up",
                r#"{"notes":"original"}"#,
                "rewritten reason",
                "[]",
                Some(0.9)
            )
            .unwrap()
            .is_none());
        assert!(repo
            .upsert_pending(
                run.id,
                "task",
                "create",
                None,
                None,
                None,
                "history-key",
                "Follow up",
                r#"{"notes":"new material evidence"}"#,
                "reason",
                "[]",
                Some(0.9)
            )
            .unwrap()
            .is_some());
    }
    use crate::db::Database;

    fn proposal_for_run(repo: &ProposalRepo<'_>, run_id: i64, key: &str) -> AiProposal {
        repo.upsert_pending(
            run_id,
            "task",
            "create",
            None,
            None,
            None,
            key,
            key,
            &format!(r#"{{"title":"{key}"}}"#),
            "reason",
            "[]",
            Some(0.8),
        )
        .unwrap()
        .unwrap()
    }

    #[test]
    fn latest_run_pending_isolated_from_older_and_deferred_proposals() {
        let db = Database::open_in_memory().unwrap();
        let runs = AnalysisRunRepo::new(db.conn());
        let old = runs.create("manual", Some(0), Some(1)).unwrap();
        runs.finish(old.id, "completed", Some("old"), None).unwrap();
        let latest = runs.create("manual", Some(1), Some(2)).unwrap();
        runs.finish(latest.id, "completed", Some("latest"), None)
            .unwrap();
        let pending_run = runs.create("manual", Some(2), Some(3)).unwrap();

        let repo = ProposalRepo::new(db.conn());
        proposal_for_run(&repo, old.id, "old");
        let visible = proposal_for_run(&repo, latest.id, "visible");
        let deferred = proposal_for_run(&repo, latest.id, "deferred");
        proposal_for_run(&repo, pending_run.id, "running");
        repo.defer(deferred.id, deferred.updated_at).unwrap();

        let rows = repo.list_latest_run_pending(20).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, visible.id);
    }

    #[test]
    fn no_output_run_keeps_unfinished_round_visible() {
        let db = Database::open_in_memory().unwrap();
        let runs = AnalysisRunRepo::new(db.conn());
        let old = runs.create("manual", Some(0), Some(1)).unwrap();
        runs.finish(old.id, "completed", Some("old"), None).unwrap();
        let repo = ProposalRepo::new(db.conn());
        proposal_for_run(&repo, old.id, "old-pending");

        let latest = runs.create("manual", Some(1), Some(2)).unwrap();
        runs.finish(latest.id, "completed", Some("no suggestions"), None)
            .unwrap();

        assert_eq!(repo.list_latest_run_pending(20).unwrap().len(), 1);
    }

    #[test]
    fn recent_list_filters_by_cutoff_and_optional_status() {
        let db = Database::open_in_memory().unwrap();
        let run = AnalysisRunRepo::new(db.conn())
            .create("manual", Some(0), Some(1))
            .unwrap();
        let repo = ProposalRepo::new(db.conn());
        let old = proposal_for_run(&repo, run.id, "old-seven-day");
        let recent = proposal_for_run(&repo, run.id, "recent-seven-day");
        db.conn()
            .execute(
                "UPDATE ai_proposals SET created_at=10,updated_at=10 WHERE id=?1",
                [old.id],
            )
            .unwrap();
        db.conn()
            .execute(
                "UPDATE ai_proposals SET created_at=100,updated_at=100 WHERE id=?1",
                [recent.id],
            )
            .unwrap();

        let rows = repo.list_since(50, None, 20).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, recent.id);
        assert!(repo
            .list_since(50, Some("confirmed"), 20)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn classification_edit_updates_kind_scope_payload_and_uses_lock() {
        let db = Database::open_in_memory().unwrap();
        let run = AnalysisRunRepo::new(db.conn())
            .create("manual", Some(0), Some(1))
            .unwrap();
        let work = crate::db::work::WorkRepo::new(db.conn())
            .insert("长期项目", "active")
            .unwrap();
        let repo = ProposalRepo::new(db.conn());
        let proposal = proposal_for_run(&repo, run.id, "classify");

        let edited = repo
            .update_classification(
                proposal.id,
                proposal.updated_at,
                "waiting",
                Some(work.id),
                "等待医学部确认",
                r#"{"title":"等待医学部确认","waiting_for":"医学部"}"#,
            )
            .unwrap();
        assert_eq!(edited.kind, "waiting");
        assert_eq!(edited.work_id, Some(work.id));
        assert_eq!(edited.title, "等待医学部确认");
        assert!(edited.user_edited);
        assert!(repo
            .update_classification(
                proposal.id,
                proposal.updated_at,
                "task",
                None,
                "stale",
                "{}",
            )
            .is_err());
    }

    #[test]
    fn defer_hides_home_proposal_without_writing_business_entities() {
        let db = Database::open_in_memory().unwrap();
        let run = AnalysisRunRepo::new(db.conn())
            .create("manual", Some(0), Some(1))
            .unwrap();
        AnalysisRunRepo::new(db.conn())
            .finish(run.id, "completed", Some("done"), None)
            .unwrap();
        let repo = ProposalRepo::new(db.conn());
        let proposal = proposal_for_run(&repo, run.id, "defer-only");
        let before: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))
            .unwrap();

        let deferred = repo.defer(proposal.id, proposal.updated_at).unwrap();
        assert!(deferred.deferred_at.is_some());
        assert_eq!(deferred.status, "pending");
        assert!(repo.list_latest_run_pending(20).unwrap().is_empty());
        let after: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))
            .unwrap();
        assert_eq!(after, before);
    }
    #[test]
    fn user_edited_pending_is_not_overwritten_and_stale_is_rejected() {
        let db = Database::open_in_memory().unwrap();
        let r = AnalysisRunRepo::new(db.conn())
            .create("manual", Some(0), Some(1))
            .unwrap();
        let repo = ProposalRepo::new(db.conn());
        let p = repo
            .upsert_pending(
                r.id,
                "task",
                "create",
                None,
                None,
                None,
                "d",
                "old",
                "{}",
                "r",
                "[]",
                Some(0.5),
            )
            .unwrap()
            .unwrap();
        let edited = repo
            .update_draft(p.id, p.updated_at, "edited", "{\"title\":\"edited\"}")
            .unwrap();
        let same = repo
            .upsert_pending(
                r.id,
                "task",
                "create",
                None,
                None,
                None,
                "d",
                "new",
                "{\"title\":\"new\"}",
                "new",
                "[]",
                Some(0.9),
            )
            .unwrap()
            .unwrap();
        assert_eq!(same.title, "edited");
        assert!(repo
            .update_draft(p.id, p.updated_at, "stale", "{}")
            .is_err());
        assert!(edited.user_edited);
    }

    #[test]
    fn stale_running_analyses_are_finalized_before_they_are_listed() {
        let db = Database::open_in_memory().unwrap();
        let run = AnalysisRunRepo::new(db.conn())
            .create("manual", Some(0), Some(1))
            .unwrap();
        db.conn()
            .execute(
                "UPDATE analysis_runs SET started_at=1 WHERE id=?1",
                [run.id],
            )
            .unwrap();
        assert_eq!(
            AnalysisRunRepo::new(db.conn())
                .expire_stale_running(601)
                .unwrap(),
            1
        );
        let updated = AnalysisRunRepo::new(db.conn())
            .get(run.id)
            .unwrap()
            .unwrap();
        assert_eq!(updated.status, "failed");
        assert_eq!(updated.error_code.as_deref(), Some("interrupted"));
        assert!(updated.finished_at.is_some());
    }
}
