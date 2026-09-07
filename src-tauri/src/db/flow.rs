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
    conn.execute("UPDATE tasks SET scheduled_start=?1,scheduled_end=?2,status=CASE WHEN status IN ('next','scheduled') THEN CASE WHEN ?1 IS NULL THEN 'next' ELSE 'scheduled' END ELSE status END,updated_at=?3 WHERE id=?4",params![start,end,now_unix(),id])?;
    conn.execute("INSERT INTO activity_events(timestamp,event_type,work_id,entity_type,entity_id,display_text) VALUES(?1,'task.scheduled',?2,'task',?3,?4)",params![now_unix(),task.work_id,id,if start.is_some(){format!("安排任务 {}",task.title)}else{format!("取消任务时间安排 {}",task.title)}])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{task::TaskRepo, work::WorkRepo, Database};

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
