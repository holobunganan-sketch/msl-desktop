use super::{now_unix, DbError, DbResult};
use rusqlite::{params, Connection, OptionalExtension};

pub fn capture(
    conn: &Connection,
    content: &str,
    work_id: Option<i64>,
    kind: Option<&str>,
    entity_id: Option<i64>,
) -> DbResult<super::inbox::InboxItem> {
    if content.trim().is_empty() {
        return Err(DbError::Migration("请输入要记录的内容".into()));
    }
    let tx = super::write_transaction(conn)?;
    super::work::validate_project_scope(&tx, "task", work_id)?;
    if kind.is_some() != entity_id.is_some() {
        return Err(DbError::Migration("记录上下文不完整".into()));
    }
    if let (Some(kind), Some(id)) = (kind, entity_id) {
        let actual = entity_location(&tx, kind, id)?;
        let parent = if kind == "work" {
            Some(id)
        } else {
            actual["work_id"].as_i64()
        };
        if parent != work_id {
            return Err(DbError::Migration(
                "事项归属已变化，请重新打开后记录".into(),
            ));
        }
    }
    let item = super::inbox::InboxRepo::new(&tx).insert(content.trim())?;
    tx.execute(
        "INSERT INTO capture_context(inbox_id,work_id,entity_kind,entity_id) VALUES(?1,?2,?3,?4)",
        params![item.id, work_id, kind, entity_id],
    )?;
    tx.commit()?;
    Ok(item)
}

pub fn capture_context(conn: &Connection, id: i64) -> DbResult<Option<serde_json::Value>> {
    Ok(conn.query_row("SELECT work_id,entity_kind,entity_id FROM capture_context WHERE inbox_id=?1",[id],|row|Ok(serde_json::json!({"work_id":row.get::<_,Option<i64>>(0)?,"entity_kind":row.get::<_,Option<String>>(1)?,"entity_id":row.get::<_,Option<i64>>(2)?}))).optional()?)
}

/// Read-only projection. IDs and durable source references establish identity;
/// matching a title or an expert's name never establishes an association.
pub fn inbox_continuity(conn: &Connection, id: i64) -> DbResult<serde_json::Value> {
    use serde_json::{json, Value};
    let item = super::inbox::InboxRepo::new(conn)
        .get(id)?
        .ok_or_else(|| DbError::NotFound("inbox".into()))?;
    let context = capture_context(conn, id)?;
    let mut work_id = context.as_ref().and_then(|value| value["work_id"].as_i64());
    let mut source = if let Some(value) = &context {
        if let (Some(kind), Some(entity_id)) =
            (value["entity_kind"].as_str(), value["entity_id"].as_i64())
        {
            Some(record_location(conn, kind, entity_id, "")?)
        } else {
            None
        }
    } else {
        None
    };
    let mut pending = Vec::new();
    let mut deferred = Vec::new();
    let mut destinations = Vec::new();
    let related=super::knowledge::rows(conn,"SELECT p.id,p.status,p.deferred_at,p.title,o.kind,o.target_id FROM ai_proposals p LEFT JOIN ai_proposal_outcomes o ON o.proposal_id=p.id WHERE EXISTS(SELECT 1 FROM json_each(CASE WHEN json_valid(p.source_refs_json) THEN p.source_refs_json ELSE '[]' END) s WHERE json_extract(s.value,'$.source_type')='inbox' AND json_extract(s.value,'$.entity_id')=?1) ORDER BY p.id",&[&id])?;
    for proposal in related {
        if proposal["status"] == "pending" {
            if proposal["deferred_at"].is_null() {
                pending.push(proposal["id"].clone());
            } else {
                deferred.push(proposal["id"].clone());
            }
        }
        if let (Some(kind), Some(target)) =
            (proposal["kind"].as_str(), proposal["target_id"].as_i64())
        {
            let location =
                record_location(conn, kind, target, proposal["title"].as_str().unwrap_or(""))?;
            if !destinations
                .iter()
                .any(|v: &Value| v["entity_kind"] == kind && v["entity_id"] == target)
            {
                destinations.push(location);
            }
        }
    }
    if let (Some(kind), Some(target)) = (item.converted_to_type.as_deref(), item.converted_to_id) {
        if !destinations
            .iter()
            .any(|v| v["entity_kind"] == kind && v["entity_id"] == target)
        {
            destinations.push(record_location(conn, kind, target, "")?);
        }
    }
    // Expert notes retain the original inbox record instead of consuming it.
    for note in super::knowledge::rows(
        conn,
        "SELECT id,work_id FROM kol_notes WHERE inbox_id=?1",
        &[&id],
    )? {
        if let Some(target) = note["id"].as_i64() {
            let location = record_location(conn, "kol_note", target, "")?;
            if context.is_none() {
                work_id = note["work_id"].as_i64();
                source = Some(location.clone());
            }
            destinations.push(location);
        }
    }
    let job=super::knowledge::rows(conn,"SELECT id,status,error FROM ai_jobs WHERE command='organize_inbox_item' AND json_extract(CASE WHEN json_valid(args_json) THEN args_json ELSE '{}' END,'$.inboxId')=?1 ORDER BY id DESC LIMIT 1",&[&id])?.into_iter().next();
    Ok(
        json!({"inbox_id":id,"work_id":work_id,"source":source,"pending_ids":pending,"deferred_ids":deferred,"destinations":destinations,"last_job":job}),
    )
}

pub fn list_inbox_continuity(conn: &Connection) -> DbResult<Vec<serde_json::Value>> {
    super::inbox::InboxRepo::new(conn)
        .list()?
        .iter()
        .map(|item| inbox_continuity(conn, item.id))
        .collect()
}

fn record_location(
    conn: &Connection,
    kind: &str,
    id: i64,
    fallback_title: &str,
) -> DbResult<serde_json::Value> {
    let current = current_record(conn, kind, id)?;
    let record = current.as_ref().map(|value| &value["record"]);
    Ok(
        serde_json::json!({"entity_kind":kind,"entity_id":id,"available":record.is_some(),"work_id":record.and_then(|r|r["work_id"].as_i64()).or_else(||(kind=="work").then_some(id)),"expert_id":record.and_then(|r|r["expert_id"].as_i64()),"title":record.and_then(|r|r["title"].as_str().or_else(||r["content"].as_str()).or_else(||r["current_state"].as_str())).unwrap_or(fallback_title)}),
    )
}

pub fn current_record(
    conn: &Connection,
    kind: &str,
    id: i64,
) -> DbResult<Option<serde_json::Value>> {
    let table = match kind {
        "work" => "works",
        "task" => "tasks",
        "waiting" => "waiting_items",
        "calendar" => "calendar_events",
        "inbox" => "inbox_items",
        "resume_point" | "resume" => "resume_points",
        "kol_insight" => "kol_insights",
        "kol_note" => "kol_notes",
        _ => return Ok(None),
    };
    let Some(mut record) =
        super::knowledge::rows(conn, &format!("SELECT * FROM {table} WHERE id=?1"), &[&id])?
            .into_iter()
            .next()
    else {
        return Ok(None);
    };
    if kind == "work" {
        if let Some(progress) = super::work::ResumePointRepo::new(conn).latest_for_work(id)? {
            record["current_state"] = serde_json::json!(progress.current_state);
            record["next_step"] = serde_json::json!(progress.next_step);
            record["remember"] = serde_json::json!(progress.remember);
        }
    }
    if kind == "calendar" {
        record["all_day"] = serde_json::json!(record["all_day"].as_i64() == Some(1));
    }
    if kind == "kol_insight" {
        record["categories"] =
            serde_json::from_str(record["categories_json"].as_str().unwrap_or("[]"))
                .unwrap_or(serde_json::json!([]));
    }
    let token = crate::cognition::digest(&format!("{kind}:{id}:{}", record));
    Ok(Some(serde_json::json!({"record":record,"token":token})))
}

pub fn restore_deferred(
    conn: &Connection,
    id: i64,
    expected_updated_at: i64,
) -> DbResult<super::ai::AiProposal> {
    let tx = super::write_transaction(conn)?;
    let updated = now_unix().max(expected_updated_at.saturating_add(1));
    if tx.execute("UPDATE ai_proposals SET deferred_at=NULL,updated_at=?1 WHERE id=?2 AND updated_at=?3 AND status='pending' AND deferred_at IS NOT NULL",params![updated,id,expected_updated_at])?!=1 {
        return Err(DbError::Migration("建议已变化或已处理，请刷新后再查看".into()));
    }
    let proposal = super::ai::ProposalRepo::new(&tx)
        .get(id)?
        .ok_or_else(|| DbError::NotFound("proposal".into()))?;
    tx.commit()?;
    Ok(proposal)
}

pub fn entity_location(conn: &Connection, kind: &str, id: i64) -> DbResult<serde_json::Value> {
    let query = match kind {
        "work" => "SELECT id,NULL FROM works WHERE id=?1",
        "task" => "SELECT work_id,scheduled_start FROM tasks WHERE id=?1",
        "waiting" => "SELECT work_id,follow_up_at FROM waiting_items WHERE id=?1",
        "calendar" => "SELECT work_id,start_at FROM calendar_events WHERE id=?1",
        "resume" | "resume_point" => "SELECT work_id,NULL FROM resume_points WHERE id=?1",
        _ => return Err(DbError::Migration("无法定位该类型".into())),
    };
    conn.query_row(query,[id],|r|Ok(serde_json::json!({"work_id":r.get::<_,Option<i64>>(0)?,"at":r.get::<_,Option<i64>>(1)?}))).optional()?.ok_or_else(||DbError::NotFound("该记录已不存在".into()))
}

pub fn schedule(conn: &Connection, id: i64, start: Option<i64>, end: Option<i64>) -> DbResult<()> {
    let tx = super::write_transaction(conn)?;
    schedule_in_transaction(&tx, id, start, end)?;
    tx.commit()?;
    Ok(())
}
pub(crate) fn schedule_in_transaction(
    conn: &Connection,
    id: i64,
    start: Option<i64>,
    end: Option<i64>,
) -> DbResult<()> {
    if start.is_some_and(|v| v <= 0) || end.is_some_and(|v| start.is_none_or(|s| v < s)) {
        return Err(DbError::Migration("请检查安排的开始和结束时间".into()));
    }
    let task = super::task::TaskRepo::new(conn)
        .get(id)?
        .ok_or_else(|| DbError::NotFound("task".into()))?;
    conn.execute("UPDATE tasks SET scheduled_start=?1,scheduled_end=?2,status=CASE WHEN status IN ('next','scheduled') THEN CASE WHEN ?1 IS NULL THEN 'next' ELSE 'scheduled' END ELSE status END,updated_at=MAX(updated_at+1,?3) WHERE id=?4",params![start,end,now_unix(),id])?;
    conn.execute("INSERT INTO activity_events(timestamp,event_type,work_id,entity_type,entity_id,display_text) VALUES(?1,'task.scheduled',?2,'task',?3,?4)",params![now_unix(),task.work_id,id,if start.is_some(){format!("安排任务 {}",task.title)}else{format!("取消任务时间安排 {}",task.title)}])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{task::TaskRepo, work::WorkRepo, Database};

    #[test]
    fn continuity_keeps_all_outcomes_and_deferred_suggestions_without_name_matching() {
        let db = Database::open_in_memory().unwrap();
        let project = WorkRepo::new(db.conn())
            .insert("Same title", "active")
            .unwrap();
        WorkRepo::new(db.conn())
            .insert("Same title", "active")
            .unwrap();
        let note = capture(
            db.conn(),
            "Synthetic follow-up",
            Some(project.id),
            Some("work"),
            Some(project.id),
        )
        .unwrap();
        let run = crate::db::ai::AnalysisRunRepo::new(db.conn())
            .create("manual", None, None)
            .unwrap();
        let repo = crate::db::ai::ProposalRepo::new(db.conn());
        let mut proposals = Vec::new();
        for n in 0..3 {
            proposals.push(
                repo.upsert_pending(
                    run.id,
                    "task",
                    "create",
                    None,
                    Some(project.id),
                    None,
                    &format!("continuity-{n}"),
                    &format!("Follow-up {n}"),
                    "{}",
                    "Synthetic",
                    &serde_json::json!([{"source_type":"inbox","entity_id":note.id}]).to_string(),
                    None,
                )
                .unwrap()
                .unwrap(),
            );
        }
        let first =
            crate::ai::apply::confirm_proposal(&db, proposals[0].id, proposals[0].updated_at, None)
                .unwrap();
        crate::ai::apply::confirm_proposal(&db, proposals[1].id, proposals[1].updated_at, None)
            .unwrap();
        repo.defer(proposals[2].id, proposals[2].updated_at)
            .unwrap();
        let state = inbox_continuity(db.conn(), note.id).unwrap();
        assert_eq!(state["work_id"], project.id);
        assert_eq!(state["pending_ids"], serde_json::json!([]));
        assert_eq!(state["deferred_ids"], serde_json::json!([proposals[2].id]));
        assert_eq!(state["destinations"].as_array().unwrap().len(), 2);
        TaskRepo::new(db.conn()).delete(first.target_id).unwrap();
        let state = inbox_continuity(db.conn(), note.id).unwrap();
        assert!(state["destinations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["entity_id"] == first.target_id && item["available"] == false));
    }

    #[test]
    fn restoring_deferred_keeps_original_draft_and_never_starts_analysis() {
        let db = Database::open_in_memory().unwrap();
        let run = crate::db::ai::AnalysisRunRepo::new(db.conn())
            .create("manual", None, None)
            .unwrap();
        let repo = crate::db::ai::ProposalRepo::new(db.conn());
        let proposal = repo
            .upsert_pending(
                run.id,
                "task",
                "create",
                None,
                None,
                None,
                "later",
                "Original",
                "{}",
                "Synthetic",
                "[]",
                None,
            )
            .unwrap()
            .unwrap();
        let deferred = repo.defer(proposal.id, proposal.updated_at).unwrap();
        assert!(restore_deferred(db.conn(), proposal.id, proposal.updated_at).is_err());
        let restored = restore_deferred(db.conn(), proposal.id, deferred.updated_at).unwrap();
        assert!(restored.deferred_at.is_none());
        assert_eq!(restored.id, proposal.id);
        assert_eq!(restored.payload_json, proposal.payload_json);
        assert_eq!(restored.status, "pending");
        assert_eq!(
            db.conn()
                .query_row("SELECT COUNT(*) FROM analysis_runs", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            db.conn()
                .query_row("SELECT COUNT(*) FROM ai_jobs", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
    }

    #[test]
    fn current_record_token_detects_changes_even_without_timestamp_changes() {
        let db = Database::open_in_memory().unwrap();
        let work = WorkRepo::new(db.conn())
            .insert("Synthetic", "active")
            .unwrap();
        let point = crate::db::work::ResumePointRepo::new(db.conn())
            .insert(work.id, "Old", "Next", "", "manual")
            .unwrap();
        let before = current_record(db.conn(), "resume_point", point.id)
            .unwrap()
            .unwrap();
        db.conn()
            .execute(
                "UPDATE resume_points SET current_state='Changed' WHERE id=?1",
                [point.id],
            )
            .unwrap();
        let after = current_record(db.conn(), "resume_point", point.id)
            .unwrap()
            .unwrap();
        assert_ne!(before["token"], after["token"]);
        assert_eq!(after["record"]["current_state"], "Changed");
    }

    #[test]
    fn continuity_preserves_the_explicit_expert_note_project_without_matching_names() {
        let db = Database::open_in_memory().unwrap();
        let project = WorkRepo::new(db.conn())
            .insert("Same title", "active")
            .unwrap();
        WorkRepo::new(db.conn())
            .insert("Same title", "active")
            .unwrap();
        let expert = super::super::kol::save_expert(
            &db,
            None,
            None,
            "Same name",
            "Synthetic institution",
            None,
            "",
            &[project.id],
            false,
        )
        .unwrap();
        let original = super::super::inbox::InboxRepo::new(db.conn())
            .insert("Synthetic exchange")
            .unwrap();
        let note = super::super::kol::capture(
            &db,
            expert["id"].as_i64().unwrap(),
            Some(project.id),
            Some(original.id),
            &original.content,
            1800000000,
        )
        .unwrap();
        let context = inbox_continuity(db.conn(), original.id).unwrap();
        assert_eq!(context["work_id"], project.id);
        assert_eq!(context["source"]["entity_kind"], "kol_note");
        assert_eq!(context["source"]["expert_id"], expert["id"]);
        assert_eq!(context["destinations"][0]["entity_id"], note["id"]);
    }

    #[test]
    fn capture_keeps_project_context_without_creating_formal_entities() {
        let db = Database::open_in_memory().unwrap();
        let project = WorkRepo::new(db.conn())
            .insert("Synthetic project", "active")
            .unwrap();
        let note = capture(
            db.conn(),
            "Received feedback",
            Some(project.id),
            Some("work"),
            Some(project.id),
        )
        .unwrap();
        let context = capture_context(db.conn(), note.id).unwrap().unwrap();
        assert_eq!(context["work_id"], project.id);
        assert_eq!(context["entity_id"], project.id);
        let snapshot = crate::ai::analysis_snapshot::build(
            &db,
            "global_analysis",
            "2026-09-05",
            0,
            now_unix() + 1,
            0,
            now_unix() + 1,
            "zh-CN",
        )
        .unwrap();
        let value = serde_json::to_value(snapshot).unwrap();
        assert_eq!(value["capture_contexts"][0]["inbox_id"], note.id);
        assert!(TaskRepo::new(db.conn())
            .list(None, None)
            .unwrap()
            .is_empty());
        assert!(capture(db.conn(), "Invalid", Some(9999), None, None).is_err());
        assert_eq!(
            crate::db::inbox::InboxRepo::new(db.conn())
                .list()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn schedule_changes_one_task_and_preserves_project_notes_and_completion() {
        let db = Database::open_in_memory().unwrap();
        let project = WorkRepo::new(db.conn())
            .insert("Synthetic project", "active")
            .unwrap();
        let repo = TaskRepo::new(db.conn());
        let task = repo
            .insert(
                Some(project.id),
                "Visit",
                "normal",
                None,
                Some("Original context"),
            )
            .unwrap();
        schedule(db.conn(), task.id, Some(2000), Some(3000)).unwrap();
        let updated = repo.get(task.id).unwrap().unwrap();
        assert_eq!(updated.work_id, Some(project.id));
        assert_eq!(updated.notes.as_deref(), Some("Original context"));
        assert_eq!(updated.scheduled_start, Some(2000));
        assert_eq!(updated.status, "scheduled");
        assert!(schedule(db.conn(), task.id, Some(3000), Some(2000)).is_err());
        assert_eq!(
            repo.get(task.id).unwrap().unwrap().scheduled_start,
            Some(2000)
        );
        repo.complete(task.id).unwrap();
        schedule(db.conn(), task.id, None, None).unwrap();
        assert_eq!(repo.get(task.id).unwrap().unwrap().status, "done");
        let count: i64 = db
            .conn()
            .query_row("SELECT count(*) FROM calendar_events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
        assert_eq!(repo.list(None, None).unwrap().len(), 1);
    }
}
