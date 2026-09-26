use super::{Database, DbError, DbResult};
use rusqlite::params;
use serde_json::{json, Value};

fn table(kind: &str) -> DbResult<(&'static str, &'static str, &'static str)> {
    match kind {
        "task" => Ok(("tasks", "completed_at", "done")),
        "waiting" => Ok(("waiting_items", "resolved_at", "resolved")),
        _ => Err(DbError::Migration("不支持的完成类型".into())),
    }
}
fn snapshot(conn: &rusqlite::Connection, kind: &str, id: i64) -> DbResult<Value> {
    let (table, _, _) = table(kind)?;
    let entity =
        super::knowledge::rows(conn, &format!("SELECT * FROM {table} WHERE id=?1"), &[&id])?
            .into_iter()
            .next()
            .ok_or_else(|| DbError::NotFound("事项".into()))?;
    let proposals=super::knowledge::rows(conn,"SELECT p.* FROM ai_proposals p JOIN ai_proposal_outcomes o ON o.proposal_id=p.id WHERE o.kind=?1 AND o.target_id=?2 ORDER BY p.id",&[&kind,&id])?;
    Ok(json!({"entity":entity,"proposals":proposals}))
}
fn invalidate(conn: &rusqlite::Connection, kind: &str, id: i64) -> DbResult<()> {
    for scope in crate::ai::rounds::proposal_scopes(conn, kind, Some(id), None, &json!([]))? {
        conn.execute(
            "UPDATE secretary_rounds SET epoch=epoch+1,active_run=NULL WHERE scope=?1",
            [scope],
        )?;
    }
    Ok(())
}
pub fn complete(db: &Database, kind: &str, id: i64) -> DbResult<Value> {
    let (table, finished, status) = table(kind)?;
    let tx = super::write_transaction(db.conn())?;
    let before = snapshot(&tx, kind, id)?;
    if before["entity"]["status"] == status {
        return Err(DbError::Migration("事项已经完成，请刷新".into()));
    }
    let now = super::now_unix();
    tx.execute(&format!("UPDATE {table} SET status=?1,{finished}=?2,updated_at=MAX(updated_at+1,?2) WHERE id=?3"),params![status,now,id])?;
    let after = snapshot(&tx, kind, id)?;
    let receipt = uuid::Uuid::new_v4().to_string();
    tx.execute("INSERT INTO manual_completion_receipts(id,entity_kind,entity_id,title,before_json,after_json,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![receipt,kind,id,before["entity"]["title"].as_str(),before.to_string(),after.to_string(),now])?;
    tx.execute("DELETE FROM manual_completion_receipts WHERE id NOT IN (SELECT id FROM manual_completion_receipts ORDER BY created_at DESC,rowid DESC LIMIT 100)",[])?;
    invalidate(&tx, kind, id)?;
    tx.execute("INSERT INTO activity_events(timestamp,event_type,entity_type,entity_id,work_id,display_text) VALUES(?1,?2,?3,?4,?5,?6)",params![now,format!("{kind}.completed"),kind,id,before["entity"]["work_id"].as_i64(),format!("完成 {}",before["entity"]["title"].as_str().unwrap_or("事项"))])?;
    tx.commit()?;
    Ok(
        json!({"id":receipt,"entity_kind":kind,"entity_id":id,"title":before["entity"]["title"],"created_at":now,"undone_at":null}),
    )
}
pub fn undo(db: &Database, id: &str) -> DbResult<()> {
    let tx = super::write_transaction(db.conn())?;
    let row = super::knowledge::rows(
        &tx,
        "SELECT * FROM manual_completion_receipts WHERE id=?1 AND undone_at IS NULL",
        &[&id],
    )?
    .into_iter()
    .next()
    .ok_or_else(|| DbError::NotFound("可撤销记录".into()))?;
    let kind = row["entity_kind"]
        .as_str()
        .ok_or_else(|| DbError::Migration("凭据类型无效".into()))?;
    let entity_id = row["entity_id"].as_i64().unwrap_or(0);
    let before: Value = serde_json::from_str(row["before_json"].as_str().unwrap_or(""))
        .map_err(|e| DbError::Migration(e.to_string()))?;
    let after: Value = serde_json::from_str(row["after_json"].as_str().unwrap_or(""))
        .map_err(|e| DbError::Migration(e.to_string()))?;
    if snapshot(&tx, kind, entity_id)? != after {
        return Err(DbError::Migration(
            "事项或相关建议已有后续修改，无法安全撤销".into(),
        ));
    }
    let (table, finished, _) = table(kind)?;
    let now = super::now_unix();
    let entity = &before["entity"];
    tx.execute(&format!("UPDATE {table} SET status=?1,{finished}=?2,updated_at=MAX(updated_at+1,?3) WHERE id=?4"),params![entity["status"].as_str(),entity[finished].as_i64(),now,entity_id])?;
    for p in before["proposals"].as_array().into_iter().flatten() {
        tx.execute("UPDATE ai_proposals SET status=?1,decided_at=?2,updated_at=MAX(updated_at+1,?3) WHERE id=?4",params![p["status"].as_str(),p["decided_at"].as_i64(),now,p["id"].as_i64()])?;
    }
    invalidate(&tx, kind, entity_id)?;
    tx.execute(
        "UPDATE manual_completion_receipts SET undone_at=?1 WHERE id=?2",
        params![now, id],
    )?;
    tx.commit()?;
    Ok(())
}
pub fn list(db: &Database) -> DbResult<Vec<Value>> {
    super::knowledge::rows(db.conn(),"SELECT id,entity_kind,entity_id,title,created_at,undone_at FROM manual_completion_receipts ORDER BY created_at DESC,rowid DESC LIMIT 100",&[])
}
pub fn delete(db: &Database, kind: &str, id: i64, confirmed: bool, expected: i64) -> DbResult<()> {
    let tx = super::write_transaction(db.conn())?;
    let row = snapshot(&tx, kind, id)?;
    if !confirmed || row["entity"]["updated_at"].as_i64() != Some(expected) {
        return Err(DbError::Migration(
            "请确认删除当前版本；事项已变化时需刷新后确认".into(),
        ));
    }
    crate::ai::lifecycle::remove_item_in_transaction(&tx, kind, id)?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn schedule_advances_delete_version_even_with_same_second_or_clock_skew() {
        let db = Database::open_in_memory().unwrap();
        let task = crate::db::task::TaskRepo::new(db.conn())
            .insert(None, "Synthetic", "normal", None, None)
            .unwrap();
        let version = super::super::now_unix() + 60;
        db.conn()
            .execute(
                "UPDATE tasks SET updated_at=?1 WHERE id=?2",
                params![version, task.id],
            )
            .unwrap();
        crate::db::flow::schedule(db.conn(), task.id, Some(100), None).unwrap();
        let current = crate::db::task::TaskRepo::new(db.conn())
            .get(task.id)
            .unwrap()
            .unwrap();
        assert!(current.updated_at > version);
        assert!(delete(&db, "task", task.id, true, version).is_err());
    }
    #[test]
    fn undo_restores_related_advice_and_invalidates_inflight_round_after_restart() {
        let path = std::env::temp_dir()
            .join(format!("receipt-{}", uuid::Uuid::new_v4()))
            .join("db.sqlite");
        let db = Database::open(&path).unwrap();
        let t = crate::db::task::TaskRepo::new(db.conn())
            .insert(None, "Synthetic", "normal", None, None)
            .unwrap();
        let run = crate::ai::analysis::create_run(&db, "manual", 0, 1).unwrap();
        let p = crate::db::ai::ProposalRepo::new(db.conn())
            .upsert_pending(
                run,
                "task",
                "create",
                None,
                None,
                None,
                "synthetic",
                "Synthetic",
                "{}",
                "",
                "[]",
                None,
            )
            .unwrap()
            .unwrap();
        db.conn()
            .execute(
                "UPDATE ai_proposals SET status='confirmed' WHERE id=?1",
                [p.id],
            )
            .unwrap();
        db.conn().execute("INSERT INTO ai_proposal_outcomes(proposal_id,kind,target_id,created_at) VALUES(?1,'task',?2,1)",params![p.id,t.id]).unwrap();
        let receipt = complete(&db, "task", t.id).unwrap();
        assert_eq!(
            db.conn()
                .query_row("SELECT status FROM ai_proposals WHERE id=?1", [p.id], |r| r
                    .get::<_, String>(0))
                .unwrap(),
            "resolved"
        );
        drop(db);
        let db = Database::open(&path).unwrap();
        assert_eq!(list(&db).unwrap()[0]["id"], receipt["id"]);
        let run = crate::ai::analysis::create_run(&db, "manual", 0, 1).unwrap();
        let tickets = crate::ai::rounds::reserve(&db, run, &[format!("tasks:{}", t.id)]).unwrap();
        assert_eq!(tickets.len(), 1);
        undo(&db, receipt["id"].as_str().unwrap()).unwrap();
        assert!(!crate::ai::rounds::valid(db.conn(), run, &tickets).unwrap());
        assert_eq!(
            db.conn()
                .query_row("SELECT status FROM ai_proposals WHERE id=?1", [p.id], |r| r
                    .get::<_, String>(0))
                .unwrap(),
            "confirmed"
        );
        let receipt = complete(&db, "task", t.id).unwrap();
        db.conn()
            .execute(
                "UPDATE ai_proposals SET reason='Later edit' WHERE id=?1",
                [p.id],
            )
            .unwrap();
        assert!(undo(&db, receipt["id"].as_str().unwrap()).is_err());
    }
    #[test]
    fn waiting_undo_preserves_prior_status_and_proposal_edit_conflicts() {
        let db = Database::open_in_memory().unwrap();
        db.conn().execute("INSERT INTO waiting_items(title,status,started_at,created_at,updated_at) VALUES('Wait','open',1,1,1)",[]).unwrap();
        let receipt = complete(&db, "waiting", 1).unwrap();
        undo(&db, receipt["id"].as_str().unwrap()).unwrap();
        assert_eq!(
            db.conn()
                .query_row("SELECT status FROM waiting_items WHERE id=1", [], |r| {
                    r.get::<_, String>(0)
                })
                .unwrap(),
            "open"
        );
    }
    #[test]
    fn manual_completion_is_durable_and_undo_rejects_same_second_edits() {
        for changed in [false, true] {
            let db = Database::open_in_memory().unwrap();
            let t = crate::db::task::TaskRepo::new(db.conn())
                .insert(None, "Synthetic", "normal", None, None)
                .unwrap();
            let result = complete(&db, "task", t.id);
            assert!(result.is_ok());
            let receipt = result.unwrap();
            assert_eq!(list(&db).unwrap().len(), 1);
            if changed {
                db.conn()
                    .execute("UPDATE tasks SET notes='Later edit' WHERE id=?1", [t.id])
                    .unwrap();
            }
            let result = undo(&db, receipt["id"].as_str().unwrap());
            assert_eq!(result.is_err(), changed);
            assert_eq!(
                crate::db::task::TaskRepo::new(db.conn())
                    .get(t.id)
                    .unwrap()
                    .unwrap()
                    .status,
                if changed { "done" } else { "next" }
            );
        }
    }
    #[test]
    fn delete_requires_explicit_current_confirmation() {
        let db = Database::open_in_memory().unwrap();
        let t = crate::db::task::TaskRepo::new(db.conn())
            .insert(None, "Synthetic", "normal", None, None)
            .unwrap();
        assert!(delete(&db, "task", t.id, false, t.updated_at).is_err());
        assert!(delete(&db, "task", t.id, true, t.updated_at - 1).is_err());
        assert!(delete(&db, "task", t.id, true, t.updated_at).is_ok());
        assert!(crate::db::task::TaskRepo::new(db.conn())
            .get(t.id)
            .unwrap()
            .is_none());
    }
}
