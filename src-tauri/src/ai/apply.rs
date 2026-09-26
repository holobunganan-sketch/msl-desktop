//! Confirmation-only application of validated AI proposals.

use crate::db::{now_unix, Database, DbError, DbResult};
use rusqlite::OptionalExtension;
use serde_json::Value;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ApplyResult {
    pub proposal_id: i64,
    pub kind: String,
    pub target_id: i64,
    pub receipt_id: String,
}

fn text(payload: &Value, key: &str) -> Option<String> {
    payload.get(key).and_then(Value::as_str).map(str::to_string)
}
fn first_text(payload: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| text(payload, key).filter(|value| !value.trim().is_empty()))
}
fn required_text(payload: &Value, key: &str) -> Result<String, String> {
    text(payload, key)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("缺少字段 {key}"))
}
fn i64_value(payload: &Value, key: &str) -> Option<i64> {
    payload.get(key).and_then(Value::as_i64)
}
fn bool_value(payload: &Value, key: &str) -> bool {
    payload.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn canonical_payload(proposal: &crate::db::ai::AiProposal, value: Value) -> DbResult<Value> {
    let mut object = value
        .as_object()
        .cloned()
        .ok_or_else(|| DbError::Migration("结构化字段必须是 JSON 对象".into()))?;
    if !matches!(proposal.kind.as_str(), "inbox" | "resume_point") {
        object.insert("title".into(), Value::String(proposal.title.trim().into()));
    }
    if proposal.kind == "inbox" && !object.contains_key("content") {
        object.insert(
            "content".into(),
            Value::String(proposal.title.trim().into()),
        );
    }
    Ok(Value::Object(object))
}

fn work_summary(payload: &Value) -> Option<String> {
    if let Some(summary) = first_text(payload, &["summary", "notes", "content"]) {
        return Some(summary);
    }
    match (
        first_text(payload, &["current_state"]),
        first_text(payload, &["next_step"]),
    ) {
        (Some(current), Some(next)) => Some(format!("{current}\n下一步：{next}")),
        (Some(current), None) => Some(current),
        (None, Some(next)) => Some(format!("下一步：{next}")),
        (None, None) => None,
    }
}

fn source_inbox_ids(source_refs_json: &str) -> Vec<i64> {
    serde_json::from_str::<Value>(source_refs_json)
        .ok()
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .filter(|source| source.get("source_type").and_then(Value::as_str) == Some("inbox"))
        .filter_map(|source| source.get("entity_id").and_then(Value::as_i64))
        .collect()
}

fn link_work_evidence(
    tx: &rusqlite::Connection,
    proposal: &crate::db::ai::AiProposal,
    work_id: i64,
    now: i64,
) -> DbResult<()> {
    let mut workspace_ids = std::collections::BTreeSet::new();
    if let Some(workspace_id) = proposal.workspace_id {
        workspace_ids.insert(workspace_id);
        tx.execute(
            "UPDATE work_workspace_links SET is_primary=0 WHERE work_id=?1",
            [work_id],
        )?;
        tx.execute(
            "INSERT INTO work_workspace_links(work_id,workspace_id,is_primary,created_at) VALUES(?1,?2,1,?3) ON CONFLICT(work_id,workspace_id) DO UPDATE SET is_primary=1",
            rusqlite::params![work_id, workspace_id, now],
        )?;
    }
    let sources = serde_json::from_str::<Value>(&proposal.source_refs_json)
        .ok()
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default();
    for source in sources {
        if source.get("source_type").and_then(Value::as_str) != Some("document") {
            continue;
        }
        let workspace_id = source.get("workspace_id").and_then(Value::as_i64);
        let document_id = source.get("entity_id").and_then(Value::as_i64);
        let relative_path = source.get("relative_path").and_then(Value::as_str);
        let document = if let (Some(workspace_id), Some(document_id)) = (workspace_id, document_id)
        {
            tx.query_row(
                "SELECT workspace_id,path,relative_path FROM document_index WHERE id=?1 AND workspace_id=?2",
                rusqlite::params![document_id, workspace_id],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)),
            )
            .optional()?
        } else if let (Some(workspace_id), Some(relative_path)) = (workspace_id, relative_path) {
            tx.query_row(
                "SELECT workspace_id,path,relative_path FROM document_index WHERE workspace_id=?1 AND relative_path=?2",
                rusqlite::params![workspace_id, relative_path],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)),
            )
            .optional()?
        } else {
            None
        };
        let Some((workspace_id, path, label)) = document else {
            continue;
        };
        workspace_ids.insert(workspace_id);
        tx.execute(
            "INSERT INTO work_file_refs(work_id,workspace_id,path,label,pinned,created_at) SELECT ?1,?2,?3,?4,0,?5 WHERE NOT EXISTS(SELECT 1 FROM work_file_refs WHERE work_id=?1 AND workspace_id=?2 AND path=?3)",
            rusqlite::params![work_id, workspace_id, path, label, now],
        )?;
    }
    for workspace_id in workspace_ids {
        tx.execute(
            "INSERT INTO work_workspace_links(work_id,workspace_id,is_primary,created_at) VALUES(?1,?2,0,?3) ON CONFLICT(work_id,workspace_id) DO NOTHING",
            rusqlite::params![work_id, workspace_id, now],
        )?;
    }
    Ok(())
}

pub fn confirm_proposal(
    db: &Database,
    id: i64,
    expected_updated_at: i64,
    edited_payload: Option<Value>,
) -> DbResult<ApplyResult> {
    crate::ai::receipts::confirm_single(db, id, expected_updated_at, edited_payload)
}

pub(crate) fn confirm_in_transaction(
    tx: &rusqlite::Connection,
    id: i64,
    expected_updated_at: i64,
    edited_payload: Option<Value>,
) -> DbResult<ApplyResult> {
    let proposal = crate::db::ai::ProposalRepo::new(tx)
        .get(id)?
        .ok_or_else(|| DbError::NotFound("proposal".into()))?;
    if proposal.status != "pending" || proposal.updated_at != expected_updated_at {
        return Err(DbError::Migration(
            "proposal is stale or no longer pending".into(),
        ));
    }
    let payload = edited_payload.unwrap_or_else(|| {
        serde_json::from_str(&proposal.payload_json).unwrap_or(Value::Object(Default::default()))
    });
    let payload = canonical_payload(&proposal, payload)?;
    crate::ai::schema::validate_payload_fields(&proposal.kind, &payload)
        .map_err(DbError::Migration)?;
    crate::db::work::validate_project_scope(tx, &proposal.kind, proposal.work_id)?;
    if let (Some(start), Some(end)) = (
        i64_value(&payload, "start_at"),
        i64_value(&payload, "end_at"),
    ) {
        if end < start {
            return Err(DbError::Migration("结束时间不能早于开始时间".into()));
        }
    }
    let now = now_unix();
    let target_id = match (proposal.kind.as_str(), proposal.operation.as_str()) {
        ("work", "create") => {
            let title = proposal.title.trim();
            let status = text(&payload, "status").unwrap_or_else(|| "active".into());
            let summary = work_summary(&payload);
            tx.execute("INSERT INTO works(title,status,summary,created_at,updated_at) VALUES (?1,?2,?3,?4,?4)",rusqlite::params![title,status,summary,now])?;
            tx.last_insert_rowid()
        }
        ("task", "create") => {
            let title = proposal.title.trim();
            let priority = text(&payload, "priority").unwrap_or_else(|| "normal".into());
            let notes = first_text(&payload, &["notes", "summary", "next_step", "content"]);
            tx.execute("INSERT INTO tasks(work_id,title,status,priority,due_at,notes,created_at,updated_at) VALUES (?1,?2,'next',?3,?4,?5,?6,?6)",rusqlite::params![proposal.work_id,title,priority,i64_value(&payload,"due_at"),notes,now])?;
            tx.last_insert_rowid()
        }
        ("waiting", "create") => {
            let title = proposal.title.trim();
            let waiting_for = text(&payload, "waiting_for").unwrap_or_default();
            let notes = first_text(&payload, &["notes", "summary", "next_step", "content"]);
            tx.execute("INSERT INTO waiting_items(work_id,title,waiting_for,started_at,follow_up_at,status,notes,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,'open',?6,?4,?4)",rusqlite::params![proposal.work_id,title,waiting_for,now,i64_value(&payload,"follow_up_at"),notes])?;
            tx.last_insert_rowid()
        }
        ("calendar", "create") => {
            let title = proposal.title.trim();
            let start = i64_value(&payload, "start_at")
                .ok_or_else(|| DbError::Migration("缺少字段 start_at".into()))?;
            let end = i64_value(&payload, "end_at");
            let kind = text(&payload, "kind").unwrap_or_else(|| "other".into());
            tx.execute("INSERT INTO calendar_events(work_id,title,start_at,end_at,all_day,location,notes,kind,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?9)",rusqlite::params![proposal.work_id,title,start,end,bool_value(&payload,"all_day") as i64,text(&payload,"location"),text(&payload,"notes"),kind,now])?;
            tx.last_insert_rowid()
        }
        ("inbox", "create") => {
            let content = required_text(&payload, "content").map_err(DbError::Migration)?;
            tx.execute(
                "INSERT INTO inbox_items(content,created_at) VALUES (?1,?2)",
                rusqlite::params![content, now],
            )?;
            tx.last_insert_rowid()
        }
        ("resume_point", "create") => {
            let work_id = proposal
                .work_id
                .or_else(|| i64_value(&payload, "work_id"))
                .ok_or_else(|| DbError::Migration("resume_point 缺少 work_id".into()))?;
            let current = text(&payload, "current_state").unwrap_or_default();
            let next = text(&payload, "next_step").unwrap_or_default();
            let remember = text(&payload, "remember").unwrap_or_default();
            tx.execute("INSERT INTO resume_points(work_id,current_state,next_step,remember,source,created_at) VALUES (?1,?2,?3,?4,'ai_draft_confirmed',?5)",rusqlite::params![work_id,current,next,remember,now])?;
            tx.last_insert_rowid()
        }
        (kind, "update") => {
            let target = proposal
                .target_id
                .ok_or_else(|| DbError::Migration("update 缺少 target_id".into()))?;
            match kind {
                "work" => {
                    let current = tx
                        .query_row(
                            "SELECT status,summary FROM works WHERE id=?1",
                            [target],
                            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
                        )
                        .optional()?
                        .ok_or_else(|| DbError::NotFound("work target".into()))?;
                    let status = text(&payload, "status").unwrap_or(current.0);
                    let summary = work_summary(&payload).or(current.1);
                    tx.execute(
                        "UPDATE works SET title=?1,status=?2,summary=?3,updated_at=?4 WHERE id=?5",
                        rusqlite::params![proposal.title.trim(), status, summary, now, target],
                    )?;
                }
                "task" => {
                    let current = tx
                        .query_row(
                            "SELECT priority,due_at,notes FROM tasks WHERE id=?1",
                            [target],
                            |row| {
                                Ok((
                                    row.get::<_, String>(0)?,
                                    row.get::<_, Option<i64>>(1)?,
                                    row.get::<_, Option<String>>(2)?,
                                ))
                            },
                        )
                        .optional()?
                        .ok_or_else(|| DbError::NotFound("task target".into()))?;
                    let priority = text(&payload, "priority").unwrap_or(current.0);
                    let due_at = if payload.get("due_at").is_some() {
                        i64_value(&payload, "due_at")
                    } else {
                        current.1
                    };
                    let notes = first_text(&payload, &["notes", "summary", "next_step", "content"])
                        .or(current.2);
                    tx.execute("UPDATE tasks SET work_id=?1,title=?2,priority=?3,due_at=?4,notes=?5,updated_at=MAX(updated_at+1,?6) WHERE id=?7",rusqlite::params![proposal.work_id,proposal.title.trim(),priority,due_at,notes,now,target])?;
                }
                "waiting" => {
                    let current = tx.query_row("SELECT waiting_for,started_at,follow_up_at,notes FROM waiting_items WHERE id=?1",[target],|row|Ok((row.get::<_,String>(0)?,row.get::<_,i64>(1)?,row.get::<_,Option<i64>>(2)?,row.get::<_,Option<String>>(3)?))).optional()?.ok_or_else(||DbError::NotFound("waiting target".into()))?;
                    let waiting_for = text(&payload, "waiting_for").unwrap_or(current.0);
                    let started_at = i64_value(&payload, "started_at").unwrap_or(current.1);
                    let follow_up_at = if payload.get("follow_up_at").is_some() {
                        i64_value(&payload, "follow_up_at")
                    } else {
                        current.2
                    };
                    let notes = first_text(&payload, &["notes", "summary", "next_step", "content"])
                        .or(current.3);
                    tx.execute("UPDATE waiting_items SET work_id=?1,title=?2,waiting_for=?3,started_at=?4,follow_up_at=?5,notes=?6,updated_at=MAX(updated_at+1,?7) WHERE id=?8",rusqlite::params![proposal.work_id,proposal.title.trim(),waiting_for,started_at,follow_up_at,notes,now,target])?;
                }
                "calendar" => {
                    let current = tx.query_row("SELECT start_at,end_at,all_day,location,notes,kind FROM calendar_events WHERE id=?1",[target],|row|Ok((row.get::<_,i64>(0)?,row.get::<_,Option<i64>>(1)?,row.get::<_,i64>(2)? != 0,row.get::<_,Option<String>>(3)?,row.get::<_,Option<String>>(4)?,row.get::<_,String>(5)?))).optional()?.ok_or_else(||DbError::NotFound("calendar target".into()))?;
                    let start_at = i64_value(&payload, "start_at").unwrap_or(current.0);
                    let end_at = if payload.get("end_at").is_some() {
                        i64_value(&payload, "end_at")
                    } else {
                        current.1
                    };
                    let all_day = payload
                        .get("all_day")
                        .and_then(Value::as_bool)
                        .unwrap_or(current.2);
                    let location = text(&payload, "location").or(current.3);
                    let notes =
                        first_text(&payload, &["notes", "summary", "content"]).or(current.4);
                    let event_kind = text(&payload, "kind").unwrap_or(current.5);
                    tx.execute("UPDATE calendar_events SET work_id=?1,title=?2,start_at=?3,end_at=?4,all_day=?5,location=?6,notes=?7,kind=?8,updated_at=?9 WHERE id=?10",rusqlite::params![proposal.work_id,proposal.title.trim(),start_at,end_at,all_day as i64,location,notes,event_kind,now,target])?;
                }
                "inbox" => {
                    let current = tx
                        .query_row(
                            "SELECT content FROM inbox_items WHERE id=?1",
                            [target],
                            |row| row.get::<_, String>(0),
                        )
                        .optional()?
                        .ok_or_else(|| DbError::NotFound("inbox target".into()))?;
                    let content =
                        first_text(&payload, &["content", "summary", "notes"]).unwrap_or(current);
                    tx.execute(
                        "UPDATE inbox_items SET content=?1 WHERE id=?2",
                        rusqlite::params![content, target],
                    )?;
                }
                "resume_point" => {
                    let current = tx.query_row("SELECT work_id,current_state,next_step,remember FROM resume_points WHERE id=?1",[target],|row|Ok((row.get::<_,i64>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?))).optional()?.ok_or_else(||DbError::NotFound("resume point target".into()))?;
                    let work_id = proposal
                        .work_id
                        .or_else(|| i64_value(&payload, "work_id"))
                        .unwrap_or(current.0);
                    let current_state = text(&payload, "current_state").unwrap_or(current.1);
                    let next_step = text(&payload, "next_step").unwrap_or(current.2);
                    let remember = text(&payload, "remember").unwrap_or(current.3);
                    tx.execute("UPDATE resume_points SET work_id=?1,current_state=?2,next_step=?3,remember=?4 WHERE id=?5",rusqlite::params![work_id,current_state,next_step,remember,target])?;
                }
                _ => return Err(DbError::Migration("该 kind 暂不支持 update".into())),
            }
            if tx.changes() == 0 {
                return Err(DbError::NotFound(format!("{kind} target")));
            }
            target
        }
        _ => return Err(DbError::Migration("不支持的 proposal operation".into())),
    };
    if proposal.kind == "task"
        && (payload.get("scheduled_start").is_some() || payload.get("scheduled_end").is_some())
    {
        let current = crate::db::task::TaskRepo::new(&tx)
            .get(target_id)?
            .ok_or_else(|| DbError::NotFound("task".into()))?;
        let start = if payload.get("scheduled_start").is_some() {
            i64_value(&payload, "scheduled_start")
        } else {
            current.scheduled_start
        };
        let end = if start.is_none() {
            None
        } else if payload.get("scheduled_end").is_some() {
            i64_value(&payload, "scheduled_end")
        } else {
            current.scheduled_end
        };
        crate::db::flow::schedule_in_transaction(&tx, target_id, start, end)?;
    }
    if proposal.kind == "work" {
        link_work_evidence(&tx, &proposal, target_id, now)?;
    }
    if let Some(status) = text(&payload, "status") {
        match proposal.kind.as_str() {
            "task" => {
                tx.execute("UPDATE tasks SET status=?1,completed_at=CASE WHEN ?1='done' THEN COALESCE(completed_at,?2) ELSE NULL END,updated_at=MAX(updated_at+1,?2) WHERE id=?3",rusqlite::params![status,now,target_id])?;
            }
            "waiting" => {
                tx.execute("UPDATE waiting_items SET status=?1,resolved_at=CASE WHEN ?1='resolved' THEN COALESCE(resolved_at,?2) ELSE NULL END,updated_at=MAX(updated_at+1,?2) WHERE id=?3",rusqlite::params![status,now,target_id])?;
            }
            "work" => {
                tx.execute("UPDATE works SET archived_at=CASE WHEN ?1='archived' THEN COALESCE(archived_at,?2) ELSE NULL END WHERE id=?3",rusqlite::params![status,now,target_id])?;
            }
            _ => (),
        }
    }
    if proposal.kind != "inbox" {
        for inbox_id in source_inbox_ids(&proposal.source_refs_json) {
            tx.execute("UPDATE inbox_items SET processed_at=COALESCE(processed_at,?1),converted_to_type=COALESCE(converted_to_type,?2),converted_to_id=COALESCE(converted_to_id,?3) WHERE id=?4",rusqlite::params![now,proposal.kind,target_id,inbox_id])?;
        }
    }
    let feedback = if proposal.kind == proposal.suggested_kind
        && proposal.work_id == proposal.suggested_work_id
    {
        crate::db::memory::ClassificationFeedback::Accepted
    } else {
        crate::db::memory::ClassificationFeedback::Corrected
    };
    crate::db::memory::record_feedback(
        &tx,
        &proposal,
        Some(&proposal.kind),
        proposal.work_id,
        &payload,
        feedback,
    )?;
    tx.execute("UPDATE ai_proposals SET status='confirmed',payload_json=?1,decided_at=?2,updated_at=?2 WHERE id=?3 AND status='pending' AND updated_at=?4",rusqlite::params![payload.to_string(),now,id,expected_updated_at])?;
    if tx.changes() == 0 {
        return Err(DbError::Migration("proposal became stale".into()));
    }
    tx.execute("INSERT INTO ai_proposal_outcomes(proposal_id,kind,target_id,created_at) VALUES (?1,?2,?3,?4)",rusqlite::params![id,proposal.kind,target_id,now])?;
    tx.execute("INSERT INTO activity_events(timestamp,event_type,entity_type,entity_id,display_text,metadata_json) VALUES (?1,'ai.proposal.confirmed','proposal',?2,?3,?4)",rusqlite::params![now,id,format!("confirmed {} proposal",proposal.kind),serde_json::json!({"kind":proposal.kind,"target_id":target_id}).to_string()])?;
    Ok(ApplyResult {
        proposal_id: id,
        kind: proposal.kind,
        target_id,
        receipt_id: String::new(),
    })
}

pub fn reject_proposal(
    db: &Database,
    id: i64,
    expected_updated_at: i64,
    reason: Option<&str>,
) -> DbResult<()> {
    let proposal = crate::db::ai::ProposalRepo::new(db.conn())
        .get(id)?
        .ok_or_else(|| DbError::NotFound("proposal".into()))?;
    if proposal.status != "pending" || proposal.updated_at != expected_updated_at {
        return Err(DbError::Migration(
            "proposal is stale or no longer pending".into(),
        ));
    }
    let payload =
        serde_json::from_str(&proposal.payload_json).unwrap_or(Value::Object(Default::default()));
    let tx = crate::db::write_transaction(db.conn())?;
    let now = now_unix();
    let n=tx.execute("UPDATE ai_proposals SET status='rejected',decided_at=?1,updated_at=?1 WHERE id=?2 AND status='pending' AND updated_at=?3",rusqlite::params![now,id,expected_updated_at])?;
    if n == 0 {
        return Err(DbError::Migration(
            "proposal is stale or no longer pending".into(),
        ));
    }
    let value = reason.unwrap_or("unspecified");
    let code = if [
        "wrong_category",
        "misunderstood",
        "duplicate",
        "already_done",
        "not_now",
        "unspecified",
    ]
    .contains(&value)
    {
        value
    } else {
        "unspecified"
    };
    tx.execute("INSERT INTO review_decisions(proposal_id,reason_code,note,created_at) VALUES (?1,?2,?3,?4)",rusqlite::params![id,code,if code==value {""}else{value},now])?;
    if matches!(code, "wrong_category" | "misunderstood") {
        crate::db::memory::record_feedback(
            &tx,
            &proposal,
            None,
            None,
            &payload,
            crate::db::memory::ClassificationFeedback::Rejected,
        )?;
    }
    tx.execute("INSERT INTO activity_events(timestamp,event_type,entity_type,entity_id,display_text) VALUES (?1,'ai.proposal.rejected','proposal',?2,'AI proposal rejected')",rusqlite::params![now,id])?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirmed_task_schedule_is_the_same_entity_and_undo_restores_time() {
        let db = Database::open_in_memory().unwrap();
        let task = crate::db::task::TaskRepo::new(db.conn())
            .insert(None, "Synthetic visit", "normal", None, None)
            .unwrap();
        let run = crate::db::ai::AnalysisRunRepo::new(db.conn())
            .create("manual", None, None)
            .unwrap();
        let p = crate::db::ai::ProposalRepo::new(db.conn())
            .upsert_pending(
                run.id,
                "task",
                "update",
                Some(task.id),
                None,
                None,
                "schedule-test",
                "Synthetic visit",
                r#"{"scheduled_start":2000,"scheduled_end":3000}"#,
                "Explicit date",
                "[]",
                None,
            )
            .unwrap()
            .unwrap();
        let result = confirm_proposal(&db, p.id, p.updated_at, None).unwrap();
        let repo = crate::db::task::TaskRepo::new(db.conn());
        assert_eq!(
            repo.get(task.id).unwrap().unwrap().scheduled_start,
            Some(2000)
        );
        assert_eq!(repo.list(None, None).unwrap().len(), 1);
        crate::ai::receipts::undo(&db, &result.receipt_id).unwrap();
        assert_eq!(repo.get(task.id).unwrap().unwrap().scheduled_start, None);
    }

    #[test]
    fn reclassifying_into_project_progress_preserves_project_and_can_be_undone() {
        let db = Database::open_in_memory().unwrap();
        let work = crate::db::work::WorkRepo::new(db.conn())
            .insert("Existing project", "active")
            .unwrap();
        let run = crate::db::ai::AnalysisRunRepo::new(db.conn())
            .create("manual", None, None)
            .unwrap();
        let repo = crate::db::ai::ProposalRepo::new(db.conn());
        let p = repo
            .upsert_pending(
                run.id,
                "task",
                "create",
                None,
                None,
                None,
                "scope-progress",
                "Progress",
                "{\"status\":\"next\"}",
                "",
                "[]",
                None,
            )
            .unwrap()
            .unwrap();
        assert!(repo
            .update_classification(
                p.id,
                p.updated_at,
                "resume_point",
                None,
                "Progress",
                "{\"current_state\":\"Received feedback\"}"
            )
            .is_err());
        let edited = repo
            .update_classification(
                p.id,
                p.updated_at,
                "resume_point",
                Some(work.id),
                "Progress",
                "{\"current_state\":\"Received feedback\"}",
            )
            .unwrap();
        let result = confirm_proposal(&db, edited.id, edited.updated_at, None).unwrap();
        assert_eq!(
            crate::db::work::WorkRepo::new(db.conn())
                .list(None)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            crate::db::work::ResumePointRepo::new(db.conn())
                .get(result.target_id)
                .unwrap()
                .unwrap()
                .work_id,
            work.id
        );
        assert_eq!(
            crate::db::work::WorkRepo::new(db.conn())
                .get(work.id)
                .unwrap()
                .unwrap()
                .title,
            "Existing project"
        );
        assert!(crate::db::task::TaskRepo::new(db.conn())
            .list(None, None)
            .unwrap()
            .is_empty());
        crate::ai::receipts::undo(&db, &result.receipt_id).unwrap();
        assert!(crate::db::work::ResumePointRepo::new(db.conn())
            .get(result.target_id)
            .unwrap()
            .is_none());
        assert_eq!(repo.get(p.id).unwrap().unwrap().status, "pending");
        assert_eq!(
            crate::db::work::WorkRepo::new(db.conn())
                .list(None)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn draft_scope_rejects_missing_or_archived_projects_without_writes() {
        let db = Database::open_in_memory().unwrap();
        let work = crate::db::work::WorkRepo::new(db.conn())
            .insert("Archive", "archived")
            .unwrap();
        let run = crate::db::ai::AnalysisRunRepo::new(db.conn())
            .create("manual", None, None)
            .unwrap();
        let repo = crate::db::ai::ProposalRepo::new(db.conn());
        for kind in ["task", "waiting", "calendar"] {
            let p = repo
                .upsert_pending(
                    run.id, kind, "create", None, None, None, kind, "Item", "{}", "", "[]", None,
                )
                .unwrap()
                .unwrap();
            for id in [work.id, 99999] {
                assert!(repo
                    .update_classification(p.id, p.updated_at, kind, Some(id), "Item", "{}")
                    .is_err());
                assert_eq!(repo.get(p.id).unwrap().unwrap().updated_at, p.updated_at);
            }
            assert!(repo
                .update_classification(p.id, p.updated_at, kind, None, "Item", "{}")
                .is_ok());
        }
    }

    #[test]
    fn project_children_can_be_reassigned_or_detached_without_duplicate_entities() {
        let db = Database::open_in_memory().unwrap();
        let works = crate::db::work::WorkRepo::new(db.conn());
        let a = works.insert("Project A", "active").unwrap();
        let b = works.insert("Project B", "active").unwrap();
        let run = crate::db::ai::AnalysisRunRepo::new(db.conn())
            .create("manual", None, None)
            .unwrap();
        let repo = crate::db::ai::ProposalRepo::new(db.conn());
        for (kind, table, fields) in [
            ("task", "tasks", r#"{"status":"done","notes":"Keep notes"}"#),
            (
                "waiting",
                "waiting_items",
                r#"{"status":"resolved","waiting_for":"Team"}"#,
            ),
            (
                "calendar",
                "calendar_events",
                r#"{"start_at":100,"end_at":200,"notes":"Keep notes"}"#,
            ),
        ] {
            let p = repo
                .upsert_pending(
                    run.id,
                    kind,
                    "create",
                    None,
                    Some(a.id),
                    None,
                    &format!("create-{kind}"),
                    "Child",
                    fields,
                    "",
                    "[]",
                    None,
                )
                .unwrap()
                .unwrap();
            let result = confirm_proposal(&db, p.id, p.updated_at, None).unwrap();
            for (i, parent) in [Some(b.id), None].into_iter().enumerate() {
                let update = repo
                    .upsert_pending(
                        run.id,
                        kind,
                        "update",
                        Some(result.target_id),
                        Some(a.id),
                        None,
                        &format!("update-{kind}-{i}"),
                        "Child",
                        "{}",
                        "",
                        "[]",
                        None,
                    )
                    .unwrap()
                    .unwrap();
                let draft = repo
                    .update_classification(
                        update.id,
                        update.updated_at,
                        kind,
                        parent,
                        "Child",
                        "{}",
                    )
                    .unwrap();
                confirm_proposal(&db, draft.id, draft.updated_at, None).unwrap();
                let actual: Option<i64> = db
                    .conn()
                    .query_row(
                        &format!("SELECT work_id FROM {table} WHERE id=?1"),
                        [result.target_id],
                        |row| row.get(0),
                    )
                    .unwrap();
                assert_eq!(actual, parent);
                let count: i64 = db
                    .conn()
                    .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                        row.get(0)
                    })
                    .unwrap();
                assert_eq!(count, 1);
            }
        }
        let task = crate::db::task::TaskRepo::new(db.conn())
            .list(None, None)
            .unwrap()
            .remove(0);
        assert_eq!(task.status, "done");
        assert_eq!(task.notes.as_deref(), Some("Keep notes"));
        assert_eq!(works.list(None).unwrap().len(), 2);
    }

    #[test]
    fn neutral_rejection_does_not_teach_an_incorrect_category() {
        let db = Database::open_in_memory().unwrap();
        let run = crate::db::ai::AnalysisRunRepo::new(db.conn())
            .create("manual", None, None)
            .unwrap();
        let proposal = crate::db::ai::ProposalRepo::new(db.conn())
            .upsert_pending(
                run.id,
                "task",
                "create",
                None,
                None,
                None,
                "neutral",
                "Prepare slides",
                "{}",
                "Useful task",
                "[]",
                None,
            )
            .unwrap()
            .unwrap();
        reject_proposal(&db, proposal.id, proposal.updated_at, Some("not_now")).unwrap();
        assert_eq!(
            crate::db::memory::stats(db.conn()).unwrap().feedback_count,
            0
        );
        assert_eq!(
            crate::db::ai::ProposalRepo::new(db.conn())
                .get(proposal.id)
                .unwrap()
                .unwrap()
                .reason,
            "Useful task"
        );
    }
    use crate::db::work::WorkRepo;
    #[test]
    fn confirmation_is_atomic_and_rejection_writes_no_work() {
        let db = Database::open_in_memory().unwrap();
        let run = crate::db::ai::AnalysisRunRepo::new(db.conn())
            .create("manual", None, None)
            .unwrap();
        let p = crate::db::ai::ProposalRepo::new(db.conn())
            .upsert_pending(
                run.id,
                "work",
                "create",
                None,
                None,
                None,
                "k",
                "Draft",
                "{\"title\":\"Draft\"}",
                "r",
                "[]",
                Some(0.8),
            )
            .unwrap()
            .unwrap();
        let result = confirm_proposal(&db, p.id, p.updated_at, None).unwrap();
        assert!(result.target_id > 0);
        assert_eq!(WorkRepo::new(db.conn()).list(None).unwrap().len(), 1);
        let p2 = crate::db::ai::ProposalRepo::new(db.conn())
            .upsert_pending(
                run.id,
                "work",
                "create",
                None,
                None,
                None,
                "k2",
                "Bad",
                "{}",
                "r",
                "[]",
                Some(0.2),
            )
            .unwrap()
            .unwrap();
        reject_proposal(&db, p2.id, p2.updated_at, Some("不采用")).unwrap();
        assert_eq!(WorkRepo::new(db.conn()).list(None).unwrap().len(), 1);
    }

    #[test]
    fn update_uses_the_card_title_when_payload_omits_title() {
        let db = Database::open_in_memory().unwrap();
        let work = WorkRepo::new(db.conn()).insert("旧标题", "active").unwrap();
        let run = crate::db::ai::AnalysisRunRepo::new(db.conn())
            .create("manual", None, None)
            .unwrap();
        let proposal = crate::db::ai::ProposalRepo::new(db.conn())
            .upsert_pending(
                run.id,
                "work",
                "update",
                Some(work.id),
                Some(work.id),
                None,
                "work-title-fallback",
                "更新后的工作标题",
                r#"{"current_state":"进展待确认","next_step":"补充当前状态"}"#,
                "工作内容需要补充",
                "[]",
                Some(0.7),
            )
            .unwrap()
            .unwrap();

        confirm_proposal(&db, proposal.id, proposal.updated_at, None).unwrap();

        let updated = WorkRepo::new(db.conn()).get(work.id).unwrap().unwrap();
        assert_eq!(updated.title, "更新后的工作标题");
        assert_eq!(
            updated.summary.as_deref(),
            Some("进展待确认\n下一步：补充当前状态")
        );
    }

    #[test]
    fn confirmed_and_rejected_decisions_are_saved_as_classification_memory() {
        let db = Database::open_in_memory().unwrap();
        let run = crate::db::ai::AnalysisRunRepo::new(db.conn())
            .create("manual", None, None)
            .unwrap();
        let repo = crate::db::ai::ProposalRepo::new(db.conn());
        let corrected = repo
            .upsert_pending(
                run.id,
                "task",
                "create",
                None,
                None,
                None,
                "memory-corrected",
                "等待医学部确认材料",
                "{}",
                "等待外部回复",
                r#"[{"source_type":"inbox","entity_id":1}]"#,
                Some(0.6),
            )
            .unwrap()
            .unwrap();
        let corrected = repo
            .update_classification(
                corrected.id,
                corrected.updated_at,
                "waiting",
                None,
                "等待医学部确认材料",
                r#"{"waiting_for":"医学部"}"#,
            )
            .unwrap();
        confirm_proposal(&db, corrected.id, corrected.updated_at, None).unwrap();

        let rejected = repo
            .upsert_pending(
                run.id,
                "work",
                "create",
                None,
                None,
                None,
                "memory-rejected",
                "单次电话确认",
                "{}",
                "建议作为长期工作",
                "[]",
                Some(0.4),
            )
            .unwrap()
            .unwrap();
        reject_proposal(
            &db,
            rejected.id,
            rejected.updated_at,
            Some("wrong_category"),
        )
        .unwrap();

        let rows: Vec<(String, Option<String>, String)> = db
            .conn()
            .prepare(
                "SELECT suggested_kind, preferred_kind, last_feedback FROM classification_memories ORDER BY id",
            )
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0],
            ("task".into(), Some("waiting".into()), "corrected".into())
        );
        assert_eq!(rows[1], ("work".into(), None, "rejected".into()));
    }

    #[test]
    fn existing_task_and_inbox_updates_accept_model_payloads_without_title() {
        let db = Database::open_in_memory().unwrap();
        let task = crate::db::task::TaskRepo::new(db.conn())
            .insert(None, "旧任务", "normal", None, None)
            .unwrap();
        let inbox = crate::db::inbox::InboxRepo::new(db.conn())
            .insert("含有错字的原内容")
            .unwrap();
        let run = crate::db::ai::AnalysisRunRepo::new(db.conn())
            .create("manual", None, None)
            .unwrap();
        let repo = crate::db::ai::ProposalRepo::new(db.conn());
        let task_proposal = repo
            .upsert_pending(
                run.id,
                "task",
                "update",
                Some(task.id),
                None,
                None,
                "task-update-no-title",
                "拜访李院长",
                r#"{"summary":"确认拜访要求","next_step":"约定完成日期"}"#,
                "补充任务信息",
                "[]",
                Some(0.9),
            )
            .unwrap()
            .unwrap();
        confirm_proposal(&db, task_proposal.id, task_proposal.updated_at, None).unwrap();
        let updated_task = crate::db::task::TaskRepo::new(db.conn())
            .get(task.id)
            .unwrap()
            .unwrap();
        assert_eq!(updated_task.title, "拜访李院长");
        assert_eq!(updated_task.notes.as_deref(), Some("确认拜访要求"));

        let inbox_proposal = repo
            .upsert_pending(
                run.id,
                "inbox",
                "update",
                Some(inbox.id),
                None,
                None,
                "inbox-update-no-title",
                "完成测试",
                r#"{"content":"完成测试"}"#,
                "修正错字",
                "[]",
                Some(0.8),
            )
            .unwrap()
            .unwrap();
        confirm_proposal(&db, inbox_proposal.id, inbox_proposal.updated_at, None).unwrap();
        assert_eq!(
            crate::db::inbox::InboxRepo::new(db.conn())
                .get(inbox.id)
                .unwrap()
                .unwrap()
                .content,
            "完成测试"
        );
    }

    #[test]
    fn confirmed_work_uses_ai_document_evidence_to_link_workspace_and_files() {
        let db = Database::open_in_memory().unwrap();
        let workspace = crate::db::workspace::WorkspaceRepo::new(db.conn())
            .insert("医学项目", "C:/medical-project")
            .unwrap();
        crate::db::documents::DocumentIndexRepo::new(db.conn())
            .upsert(
                workspace.id,
                "C:/medical-project/plan.docx",
                "plan.docx",
                "docx",
                100,
                1,
                Some("hash-plan"),
                "ready",
                20,
                None,
                None,
                None,
            )
            .unwrap();
        let run = crate::db::ai::AnalysisRunRepo::new(db.conn())
            .create("workspace_import", None, None)
            .unwrap();
        let proposal = crate::db::ai::ProposalRepo::new(db.conn())
            .upsert_pending(
                run.id,
                "work",
                "create",
                None,
                None,
                Some(workspace.id),
                "ai-file-link",
                "医学项目",
                r#"{"summary":"自动整理的项目"}"#,
                "目录内容符合长期项目",
                r#"[{"source_type":"document","entity_id":1,"workspace_id":1,"relative_path":"plan.docx","content_hash":"hash-plan"}]"#,
                Some(0.9),
            )
            .unwrap()
            .unwrap();

        let result = confirm_proposal(&db, proposal.id, proposal.updated_at, None).unwrap();
        let workspace_links: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM work_workspace_links WHERE work_id=?1 AND workspace_id=?2",
                rusqlite::params![result.target_id, workspace.id],
                |row| row.get(0),
            )
            .unwrap();
        let file_links: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM work_file_refs WHERE work_id=?1 AND workspace_id=?2 AND path='C:/medical-project/plan.docx'",
                rusqlite::params![result.target_id, workspace.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(workspace_links, 1);
        assert_eq!(file_links, 1);
    }
}
