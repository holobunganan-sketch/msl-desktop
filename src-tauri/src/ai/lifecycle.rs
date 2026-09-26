//! Individual advice lifecycle. User decisions and source files stay separate.
use crate::db::{
    ai::{AiProposal, ProposalRepo},
    now_unix, Database, DbError, DbResult,
};
use rusqlite::{params, Connection};

fn current(conn: &Connection, id: i64, expected: i64) -> DbResult<AiProposal> {
    let p = ProposalRepo::new(conn)
        .get(id)?
        .ok_or_else(|| DbError::NotFound("建议记录".into()))?;
    if p.updated_at != expected || p.status == "deleted" {
        return Err(DbError::Migration("这条记录已变化，请刷新后操作".into()));
    }
    Ok(p)
}

pub fn delete(db: &Database, id: i64, expected: i64) -> DbResult<()> {
    let tx = crate::db::write_transaction(db.conn())?;
    current(&tx, id, expected)?;
    // Retain an internal tombstone to propagate deletion and prevent recurrence.
    // Adopted entities are intentionally untouched.
    tx.execute("UPDATE ai_proposals SET status='deleted',decided_at=?1,updated_at=MAX(updated_at+1,?1) WHERE id=?2", params![now_unix(),id])?;
    tx.commit()?;
    Ok(())
}

pub fn resolve(db: &Database, id: i64, expected: i64) -> DbResult<AiProposal> {
    let tx = crate::db::write_transaction(db.conn())?;
    let p = current(&tx, id, expected)?;
    if p.status != "confirmed" {
        return Err(DbError::Migration("请先采纳建议，再标记已解决".into()));
    }
    match (p.applied_kind.as_deref(), p.applied_id) {
        (Some("task"), Some(target)) => {
            tx.execute("UPDATE tasks SET status='done',completed_at=COALESCE(completed_at,?1),updated_at=MAX(updated_at+1,?1) WHERE id=?2", params![now_unix(),target])?;
        }
        (Some("waiting"), Some(target)) => {
            tx.execute("UPDATE waiting_items SET status='resolved',resolved_at=COALESCE(resolved_at,?1),updated_at=MAX(updated_at+1,?1) WHERE id=?2", params![now_unix(),target])?;
        }
        _ => (), // Older records may lack an unambiguous target; close advice only.
    }
    tx.execute("UPDATE ai_proposals SET status='resolved',decided_at=?1,updated_at=MAX(updated_at+1,?1) WHERE id=?2 AND status='confirmed'",params![now_unix(),id])?;
    let result = ProposalRepo::new(&tx).get(id)?.unwrap();
    tx.commit()?;
    Ok(result)
}

pub fn remove_item(db: &Database, kind: &str, id: i64) -> DbResult<()> {
    let tx = crate::db::write_transaction(db.conn())?;
    remove_item_in_transaction(&tx, kind, id)?;
    tx.commit()?;
    Ok(())
}
pub(crate) fn remove_item_in_transaction(tx: &Connection, kind: &str, id: i64) -> DbResult<()> {
    let table = match kind {
        "task" => "tasks",
        "waiting" => "waiting_items",
        "calendar" => "calendar_events",
        "inbox" => "inbox_items",
        "resume_point" => "resume_points",
        _ => return Err(DbError::Migration("不支持的记录类型".into())),
    };
    let exists: bool = tx.query_row(
        &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=?1)"),
        [id],
        |r| r.get(0),
    )?;
    if !exists {
        return Err(DbError::NotFound("记录".into()));
    }
    crate::db::source_lifecycle::retire(&tx, &[format!("{kind}:{id}")])?;
    tx.execute("UPDATE ai_proposals SET status='deleted',decided_at=?1,updated_at=MAX(updated_at+1,?1) WHERE status<>'deleted' AND id IN (SELECT proposal_id FROM ai_proposal_outcomes WHERE kind=?2 AND target_id=?3)",params![now_unix(),kind,id])?;
    tx.execute(&format!("DELETE FROM {table} WHERE id=?1"), [id])?;
    Ok(())
}

pub fn suppress(conn: &Connection, proposal: &super::schema::ProposalContract) -> DbResult<bool> {
    if let Some(id) = proposal.related_proposal_id {
        let active: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM ai_proposals WHERE id=?1 AND status IN ('pending','confirmed') AND work_id IS ?2)",params![id,proposal.work_id],|r|r.get(0))?;
        if !active {
            return Ok(true);
        }
    }
    // An explicit update must never silently reopen a completed target.
    if let Some(target) = proposal
        .target_id
        .filter(|_| proposal.operation == "update")
    {
        if proposal.kind != "work" && conn.query_row("SELECT EXISTS(SELECT 1 FROM ai_proposal_outcomes o JOIN ai_proposals p ON p.id=o.proposal_id WHERE o.kind=?1 AND o.target_id=?2 AND p.status IN ('resolved','deleted'))",params![proposal.kind,target],|r|r.get::<_,bool>(0))? {
            return Ok(true);
        }
        let closed: bool = match proposal.kind.as_str() {
            "task" => conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM tasks WHERE id=?1 AND status='done')",
                [target],
                |r| r.get(0),
            )?,
            "waiting" => conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM waiting_items WHERE id=?1 AND status='resolved')",
                [target],
                |r| r.get(0),
            )?,
            _ => false,
        };
        if closed {
            return Ok(true);
        }
    }
    let normalize = |s: &str| {
        s.chars()
            .filter(|c| c.is_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect::<String>()
    };
    let mut stmt = conn.prepare(
        "SELECT title,id FROM ai_proposals WHERE work_id IS ?1 AND status IN ('resolved','deleted')",
    )?;
    let scopes = super::rounds::proposal_scopes(
        conn,
        &proposal.kind,
        proposal.target_id,
        proposal.work_id,
        &serde_json::json!(proposal.source_refs),
    )?;
    for row in stmt.query_map([proposal.work_id], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
    })? {
        let (title, id) = row?;
        let previous =
            super::rounds::evidence_scopes(conn, &serde_json::json!({"proposal_id":id}))?;
        let same = proposal.work_id.is_some()
            || !scopes.is_disjoint(&previous)
            || (scopes.is_empty() && previous.is_empty());
        if same && normalize(&title) == normalize(&proposal.title) {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn filter_active_context(
    db: &Database,
    snapshot: &mut super::analysis_snapshot::AnalysisSnapshot,
) -> DbResult<()> {
    let mut stmt = db.conn().prepare("SELECT o.kind,o.target_id FROM ai_proposal_outcomes o JOIN ai_proposals p ON p.id=o.proposal_id WHERE p.status IN ('resolved','deleted')")?;
    let closed = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?
        .collect::<rusqlite::Result<std::collections::BTreeSet<_>>>()?;
    // Appointments remain real scheduling constraints even after their advice
    // has been closed. Strip issue text while preserving occupied time slots.
    let mut slots = Vec::new();
    if let Some(focused) = snapshot.focused_work.as_mut() {
        if let Some(events) = focused["calendar"].as_array() {
            for event in events.iter().filter(|e| {
                e["id"]
                    .as_i64()
                    .is_some_and(|id| closed.contains(&("calendar".into(), id)))
            }) {
                slots.push(serde_json::json!({"start_at":event["start_at"],"end_at":event["end_at"],"all_day":event["all_day"]}));
            }
        }
        for (key, kind, done) in [
            ("tasks", "task", "done"),
            ("waiting", "waiting", "resolved"),
            ("resume_history", "resume_point", ""),
            ("calendar", "calendar", ""),
        ] {
            if let Some(items) = focused[key].as_array_mut() {
                items.retain(|item| {
                    item["status"] != done
                        && !item["id"]
                            .as_i64()
                            .is_some_and(|id| closed.contains(&(kind.into(), id)))
                });
            }
        }
    }
    for fact in &snapshot.brief.calendar {
        if fact
            .entity_id
            .is_some_and(|id| closed.contains(&("calendar".into(), id)))
        {
            slots.push(serde_json::json!({"start_at":fact.start_at,"end_at":fact.end_at}));
        }
    }
    if !slots.is_empty() {
        snapshot
            .round_history
            .push(serde_json::json!({"scheduling_constraints_only":slots}));
    }
    for (facts, kind) in [
        (&mut snapshot.brief.tasks_open, "task"),
        (&mut snapshot.brief.waiting, "waiting"),
        (&mut snapshot.brief.calendar, "calendar"),
        (&mut snapshot.brief.resume_points, "resume_point"),
        (&mut snapshot.brief.inbox, "inbox"),
    ] {
        facts.retain(|fact| {
            !fact
                .entity_id
                .is_some_and(|id| closed.contains(&(kind.into(), id)))
        });
    }
    snapshot.brief.source_counts.tasks_open = snapshot.brief.tasks_open.len() as u32;
    snapshot.brief.source_counts.waiting = snapshot.brief.waiting.len() as u32;
    snapshot.brief.source_counts.calendar = snapshot.brief.calendar.len() as u32;
    snapshot.brief.source_counts.inbox = snapshot.brief.inbox.len() as u32;
    snapshot.brief.source_counts.resume_points = snapshot.brief.resume_points.len() as u32;
    for entry in &mut snapshot.project_cognition {
        let work = entry["scope"]
            .as_str()
            .and_then(|s| s.strip_prefix("work-"))
            .and_then(|s| s.parse::<i64>().ok());
        let resume_closed = if let Some(work) = work {
            crate::db::work::ResumePointRepo::new(db.conn())
                .latest_for_work(work)?
                .is_some_and(|r| closed.contains(&("resume_point".into(), r.id)))
        } else {
            false
        };
        if let Some(text) = entry["entry"].as_str() {
            let mut section = "";
            let filtered = text
                .lines()
                .filter(|line| {
                    if line.starts_with("## ") {
                        section = if line.contains("任务与进度") {
                            "task"
                        } else if line.contains("等待与阻塞") {
                            "waiting"
                        } else if line.contains("日程与约定") {
                            "calendar"
                        } else {
                            ""
                        };
                    }
                    if let Some(id) = line
                        .strip_prefix("- #")
                        .and_then(|s| s.split_whitespace().next())
                        .and_then(|s| s.parse::<i64>().ok())
                    {
                        if closed.contains(&(section.into(), id)) {
                            return false;
                        }
                    }
                    !line.contains("[done]")
                        && !line.contains("[resolved]")
                        && !(resume_closed && line.starts_with("  - 进展："))
                })
                .collect::<Vec<_>>()
                .join("\n");
            entry["entry"] = serde_json::Value::String(filtered);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn advice(db: &Database) -> AiProposal {
        let work = crate::db::work::WorkRepo::new(db.conn())
            .insert("Synthetic project", "active")
            .unwrap();
        let run = super::super::analysis::create_run(db, "manual", 0, 1).unwrap();
        ProposalRepo::new(db.conn())
            .upsert_pending(
                run,
                "task",
                "create",
                None,
                Some(work.id),
                None,
                "test",
                "Synthetic follow-up",
                "{}",
                "test",
                "[]",
                None,
            )
            .unwrap()
            .unwrap()
    }
    #[test]
    fn delete_is_checked_and_keeps_adopted_task() {
        let db = Database::open_in_memory().unwrap();
        let p = advice(&db);
        assert!(delete(&db, p.id, p.updated_at + 1).is_err());
        let result = super::super::apply::confirm_proposal(&db, p.id, p.updated_at, None).unwrap();
        let p = ProposalRepo::new(db.conn()).get(p.id).unwrap().unwrap();
        delete(&db, p.id, p.updated_at).unwrap();
        assert!(ProposalRepo::new(db.conn())
            .list(None, 200)
            .unwrap()
            .is_empty());
        assert!(ProposalRepo::new(db.conn())
            .list_since(0, None, 200)
            .unwrap()
            .is_empty());
        assert!(crate::db::task::TaskRepo::new(db.conn())
            .get(result.target_id)
            .unwrap()
            .is_some());
        assert!(delete(&db, p.id, p.updated_at).is_err());
    }
    #[test]
    fn resolve_is_explicit_and_completes_only_its_target() {
        let db = Database::open_in_memory().unwrap();
        let p = advice(&db);
        assert!(resolve(&db, p.id, p.updated_at).is_err());
        let result = super::super::apply::confirm_proposal(&db, p.id, p.updated_at, None).unwrap();
        let p = ProposalRepo::new(db.conn()).get(p.id).unwrap().unwrap();
        assert_eq!(p.applied_id, Some(result.target_id));
        assert_eq!(resolve(&db, p.id, p.updated_at).unwrap().status, "resolved");
        assert_eq!(
            crate::db::task::TaskRepo::new(db.conn())
                .get(result.target_id)
                .unwrap()
                .unwrap()
                .status,
            "done"
        );
        assert!(resolve(&db, p.id, p.updated_at).is_err());
    }
    #[test]
    fn deleted_advice_cannot_reappear_with_a_new_dedupe_key() {
        let db = Database::open_in_memory().unwrap();
        let p = advice(&db);
        delete(&db, p.id, p.updated_at).unwrap();
        let proposal = serde_json::from_value(serde_json::json!({"kind":"task","operation":"create","title":p.title,"work_id":p.work_id,"reason":"test","source_refs":[]})).unwrap();
        assert!(super::super::analysis::insert_proposal(
            &db,
            super::super::analysis::create_run(&db, "manual", 0, 1).unwrap(),
            &proposal,
            "new-key"
        )
        .unwrap()
        .is_none());
    }
    #[test]
    fn deleting_item_retires_its_insight_and_undo_does_not_resurrect_it() {
        let db = Database::open_in_memory().unwrap();
        let p = advice(&db);
        let result = super::super::apply::confirm_proposal(&db, p.id, p.updated_at, None).unwrap();
        remove_item(&db, "task", result.target_id).unwrap();
        assert!(ProposalRepo::new(db.conn())
            .list(None, 100)
            .unwrap()
            .is_empty());
        assert!(super::super::receipts::undo(&db, &result.receipt_id).is_err());
    }
    #[test]
    fn resolved_insight_leaves_active_context_and_cannot_be_reopened() {
        let db = Database::open_in_memory().unwrap();
        let p = advice(&db);
        let result = super::super::apply::confirm_proposal(&db, p.id, p.updated_at, None).unwrap();
        let p = ProposalRepo::new(db.conn()).get(p.id).unwrap().unwrap();
        resolve(&db, p.id, p.updated_at).unwrap();
        let run = super::super::analysis::create_run(&db, "manual", 0, i64::MAX).unwrap();
        let tickets =
            super::super::rounds::reserve(&db, run, &[format!("work:{}", p.work_id.unwrap())])
                .unwrap();
        assert_eq!(tickets.len(), 1);
        let snapshot = super::super::analysis_snapshot::build(
            &db,
            "work_draft",
            "2026-09-20",
            0,
            i64::MAX,
            0,
            i64::MAX,
            "zh-CN",
        )
        .unwrap();
        let snapshot =
            super::super::analysis_snapshot::focus_work(&db, snapshot, p.work_id.unwrap()).unwrap();
        let snapshot = super::super::rounds::restrict(&db, snapshot, tickets).unwrap();
        assert!(snapshot.focused_work.unwrap()["tasks"]
            .as_array()
            .unwrap()
            .is_empty());
        assert!(snapshot.round_history[0]["opinions"]
            .as_array()
            .unwrap()
            .is_empty());
        assert_eq!(snapshot.round_history[0]["closed_opinions"][0]["id"], p.id);
        assert!(snapshot.round_history[0]["closed_opinions"][0]
            .get("previous_arrangement")
            .is_none());
        assert!(snapshot.project_cognition.iter().all(|v| !v["entry"]
            .as_str()
            .unwrap_or("")
            .contains("Synthetic follow-up")));
        let reworded = serde_json::from_value(serde_json::json!({"kind":"task","operation":"create","work_id":p.work_id,"title":"Same issue reworded","related_proposal_id":p.id,"reason":"test","source_refs":[]})).unwrap();
        assert!(suppress(db.conn(), &reworded).unwrap());
        let reopen = serde_json::from_value(serde_json::json!({"kind":"task","operation":"update","target_id":result.target_id,"work_id":p.work_id,"title":"Same issue reworded","reason":"test","source_refs":[]})).unwrap();
        assert!(suppress(db.conn(), &reopen).unwrap());
        let unrelated = serde_json::from_value(serde_json::json!({"kind":"task","operation":"create","work_id":p.work_id,"title":"Genuinely different next step","reason":"test","source_refs":[]})).unwrap();
        assert!(!suppress(db.conn(), &unrelated).unwrap());
    }
    #[test]
    fn deleting_history_during_analysis_invalidates_late_response() {
        let db = Database::open_in_memory().unwrap();
        let p = advice(&db);
        let result = super::super::apply::confirm_proposal(&db, p.id, p.updated_at, None).unwrap();
        crate::db::task::TaskRepo::new(db.conn())
            .complete(result.target_id)
            .unwrap();
        let p = ProposalRepo::new(db.conn()).get(p.id).unwrap().unwrap();
        let run = super::super::analysis::create_run(&db, "manual", 0, 1).unwrap();
        let tickets =
            super::super::rounds::reserve(&db, run, &[format!("work:{}", p.work_id.unwrap())])
                .unwrap();
        assert_eq!(tickets.len(), 1);
        assert!(super::super::rounds::valid(db.conn(), run, &tickets).unwrap());
        delete(&db, p.id, p.updated_at).unwrap();
        assert!(!super::super::rounds::valid(db.conn(), run, &tickets).unwrap());
    }
    #[test]
    fn regression_legacy_receipt_can_be_undone_and_readopted() {
        let db = Database::open_in_memory().unwrap();
        let p = advice(&db);
        let result = super::super::apply::confirm_proposal(&db, p.id, p.updated_at, None).unwrap();
        let json: String = db
            .conn()
            .query_row(
                "SELECT changes_json FROM proposal_receipts WHERE id=?1",
                [&result.receipt_id],
                |r| r.get(0),
            )
            .unwrap();
        let mut changes: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
        changes.retain(|c| c["table"] != "ai_proposal_outcomes");
        db.conn()
            .execute(
                "UPDATE proposal_receipts SET changes_json=?1 WHERE id=?2",
                params![serde_json::to_string(&changes).unwrap(), result.receipt_id],
            )
            .unwrap();
        super::super::receipts::undo(&db, &result.receipt_id).unwrap();
        let p = ProposalRepo::new(db.conn()).get(p.id).unwrap().unwrap();
        assert_eq!(p.status, "pending");
        assert!(super::super::apply::confirm_proposal(&db, p.id, p.updated_at, None).is_ok());
    }
    #[test]
    fn regression_calendar_and_deleted_task_leave_all_active_projections() {
        let db = Database::open_in_memory().unwrap();
        let p = advice(&db);
        let work = p.work_id.unwrap();
        let task = super::super::apply::confirm_proposal(&db, p.id, p.updated_at, None).unwrap();
        let p = ProposalRepo::new(db.conn()).get(p.id).unwrap().unwrap();
        delete(&db, p.id, p.updated_at).unwrap();
        let run = super::super::analysis::create_run(&db, "manual", 0, i64::MAX).unwrap();
        let cal = ProposalRepo::new(db.conn())
            .upsert_pending(
                run,
                "calendar",
                "create",
                None,
                Some(work),
                None,
                "cal",
                "Calendar issue",
                "{\"start_at\":2000000000}",
                "test",
                "[]",
                None,
            )
            .unwrap()
            .unwrap();
        let event =
            super::super::apply::confirm_proposal(&db, cal.id, cal.updated_at, None).unwrap();
        let cal = ProposalRepo::new(db.conn()).get(cal.id).unwrap().unwrap();
        resolve(&db, cal.id, cal.updated_at).unwrap();
        let snapshot = super::super::analysis_snapshot::build(
            &db,
            "work_draft",
            "2026-09-20",
            0,
            i64::MAX,
            0,
            i64::MAX,
            "zh-CN",
        )
        .unwrap();
        let mut snapshot =
            super::super::analysis_snapshot::focus_work(&db, snapshot, work).unwrap();
        filter_active_context(&db, &mut snapshot).unwrap();
        assert!(snapshot
            .brief
            .tasks_open
            .iter()
            .all(|f| f.entity_id != Some(task.target_id)));
        assert!(snapshot
            .brief
            .calendar
            .iter()
            .all(|f| f.entity_id != Some(event.target_id)));
        assert!(snapshot.focused_work.unwrap()["calendar"]
            .as_array()
            .unwrap()
            .is_empty());
        assert!(snapshot
            .project_cognition
            .iter()
            .all(|v| !v.to_string().contains("Calendar issue")
                && !v.to_string().contains("Synthetic follow-up")));
        assert!(crate::db::calendar::CalendarRepo::new(db.conn())
            .get(event.target_id)
            .unwrap()
            .is_some());
    }
}
