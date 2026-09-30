//! Project classification and explicit clinical study links. Ownership never changes.
use super::{
    now_unix,
    work::{Work, WorkRepo},
    DbError, DbResult,
};
use rusqlite::{params, Connection, OptionalExtension};

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProjectRelation {
    pub id: i64,
    pub entity_kind: String,
    pub entity_id: i64,
    pub clinical_work_id: Option<i64>,
    pub revision: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

fn relation(row: &rusqlite::Row) -> rusqlite::Result<ProjectRelation> {
    Ok(ProjectRelation {
        id: row.get(0)?,
        entity_kind: row.get(1)?,
        entity_id: row.get(2)?,
        clinical_work_id: row.get(3)?,
        revision: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}
pub fn get(conn: &Connection, kind: &str, id: i64) -> DbResult<Option<ProjectRelation>> {
    conn.query_row("SELECT id,entity_kind,entity_id,clinical_work_id,revision,created_at,updated_at FROM project_relations WHERE entity_kind=?1 AND entity_id=?2",params![kind,id],relation).optional().map_err(DbError::from)
}
pub fn by_clinical_project(conn: &Connection, id: i64) -> DbResult<Vec<ProjectRelation>> {
    let mut stmt=conn.prepare("SELECT id,entity_kind,entity_id,clinical_work_id,revision,created_at,updated_at FROM project_relations WHERE clinical_work_id=?1 ORDER BY updated_at DESC,id")?;
    let result = stmt
        .query_map([id], relation)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(result)
}
pub fn all(conn: &Connection) -> DbResult<Vec<ProjectRelation>> {
    let mut stmt=conn.prepare("SELECT id,entity_kind,entity_id,clinical_work_id,revision,created_at,updated_at FROM project_relations ORDER BY id")?;
    let result = stmt
        .query_map([], relation)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(result)
}
pub fn table_for_kind(kind: &str) -> DbResult<&'static str> {
    match kind {
        "task" => Ok("tasks"),
        "waiting" => Ok("waiting_items"),
        "calendar" => Ok("calendar_events"),
        "inbox" => Ok("inbox_items"),
        _ => Err(DbError::Migration("此记录类型不支持关联临床研究".into())),
    }
}
pub fn kind_for_table(table: &str) -> Option<&'static str> {
    match table {
        "tasks" => Some("task"),
        "waiting_items" => Some("waiting"),
        "calendar_events" => Some("calendar"),
        "inbox_items" => Some("inbox"),
        _ => None,
    }
}
pub fn validate_link(
    conn: &Connection,
    kind: &str,
    id: i64,
    clinical_id: Option<i64>,
) -> DbResult<()> {
    let table = table_for_kind(kind)?;
    let owner: Option<Option<i64>> = if kind == "inbox" {
        conn.query_row("SELECT c.work_id FROM inbox_items i LEFT JOIN capture_context c ON c.inbox_id=i.id WHERE i.id=?1",[id],|r|r.get(0)).optional()?
    } else {
        conn.query_row(
            &format!("SELECT work_id FROM {table} WHERE id=?1"),
            [id],
            |r| r.get(0),
        )
        .optional()?
    };
    let owner = owner.ok_or_else(|| DbError::NotFound("事项已不存在，请刷新".into()))?;
    if let Some(target) = clinical_id {
        let work = WorkRepo::new(conn)
            .get(target)?
            .ok_or_else(|| DbError::NotFound("关联的临床研究已不存在".into()))?;
        if work.category.as_deref() != Some("clinical") || work.status == "archived" {
            return Err(DbError::Migration("请选择未归档的临床研究项目".into()));
        }
        if let Some(owner) = owner {
            if WorkRepo::new(conn)
                .get(owner)?
                .is_some_and(|w| w.category.as_deref() == Some("clinical"))
            {
                return Err(DbError::Migration(
                    "该事项已归属临床研究项目，无需再关联另一项研究".into(),
                ));
            }
        }
    }
    Ok(())
}
pub fn set(
    conn: &Connection,
    kind: &str,
    id: i64,
    clinical_id: Option<i64>,
    expected_revision: i64,
) -> DbResult<Option<ProjectRelation>> {
    let tx = if conn.is_autocommit() {
        Some(super::write_transaction(conn)?)
    } else {
        None
    };
    validate_link(conn, kind, id, clinical_id)?;
    let current = get(conn, kind, id)?;
    if current.as_ref().map_or(0, |r| r.revision) != expected_revision {
        return Err(DbError::Migration(
            "临床研究关联已更新，请刷新后重新确认".into(),
        ));
    }
    if current.as_ref().and_then(|r| r.clinical_work_id) != clinical_id {
        conn.execute("INSERT INTO project_relations(entity_kind,entity_id,clinical_work_id,created_at,updated_at) VALUES(?1,?2,?3,?4,?4) ON CONFLICT(entity_kind,entity_id) DO UPDATE SET clinical_work_id=excluded.clinical_work_id,updated_at=MAX(project_relations.updated_at+1,excluded.updated_at)",params![kind,id,clinical_id,now_unix()])?;
    }
    let result = get(conn, kind, id)?;
    if let Some(tx) = tx {
        tx.commit()?;
    }
    Ok(result)
}
pub fn set_category(
    conn: &Connection,
    id: i64,
    category: Option<&str>,
    expected_revision: i64,
) -> DbResult<Work> {
    if category.is_some_and(|v| !matches!(v, "clinical" | "non_clinical")) {
        return Err(DbError::Migration("项目分类无效".into()));
    }
    let tx = if conn.is_autocommit() {
        Some(super::write_transaction(conn)?)
    } else {
        None
    };
    let current = WorkRepo::new(conn)
        .get(id)?
        .ok_or_else(|| DbError::NotFound("work".into()))?;
    if current.revision != expected_revision {
        return Err(DbError::Migration(
            "项目已更新，请刷新后重新确认分类".into(),
        ));
    }
    if current.category.as_deref() != category {
        if category != Some("clinical") && !by_clinical_project(conn, id)?.is_empty() {
            return Err(DbError::Migration(
                "此项目仍有关联事项，请先解除临床研究关联再更改分类".into(),
            ));
        }
        if category == Some("clinical") {
            let outbound:i64=conn.query_row("SELECT COUNT(*) FROM project_relations r WHERE r.clinical_work_id IS NOT NULL AND ((r.entity_kind='task' AND EXISTS(SELECT 1 FROM tasks WHERE id=r.entity_id AND work_id=?1)) OR (r.entity_kind='waiting' AND EXISTS(SELECT 1 FROM waiting_items WHERE id=r.entity_id AND work_id=?1)) OR (r.entity_kind='calendar' AND EXISTS(SELECT 1 FROM calendar_events WHERE id=r.entity_id AND work_id=?1)) OR (r.entity_kind='inbox' AND EXISTS(SELECT 1 FROM capture_context WHERE inbox_id=r.entity_id AND work_id=?1)))",[id],|row|row.get(0))?;
            if outbound > 0 {
                return Err(DbError::Migration(
                    "项目内仍有事项关联其他临床研究，请先核对并解除关联，再将本项目改为临床研究"
                        .into(),
                ));
            }
        }
        conn.execute("UPDATE works SET category=?1,updated_at=MAX(updated_at+1,?2) WHERE id=?3 AND revision=?4",params![category,now_unix(),id,expected_revision])?;
    }
    let result = WorkRepo::new(conn)
        .get(id)?
        .ok_or_else(|| DbError::NotFound("work".into()))?;
    if let Some(tx) = tx {
        tx.commit()?;
    }
    Ok(result)
}

/// Carry an explicit inbox association into its confirmed destination. The
/// source note remains traceable; no additional task or calendar row is created.
pub fn inherit_from_inbox(conn: &Connection, inbox_id: i64, kind: &str, id: i64) -> DbResult<()> {
    if !matches!(kind, "task" | "waiting" | "calendar") {
        return Ok(());
    }
    let Some(target) = get(conn, "inbox", inbox_id)?.and_then(|r| r.clinical_work_id) else {
        return Ok(());
    };
    if get(conn, kind, id)?.is_some() {
        return Ok(());
    }
    let table = table_for_kind(kind)?;
    let owner: Option<Option<i64>> = conn
        .query_row(
            &format!("SELECT work_id FROM {table} WHERE id=?1"),
            [id],
            |r| r.get(0),
        )
        .optional()?;
    let Some(owner) = owner else {
        return Ok(());
    };
    if owner == Some(target) {
        return Ok(());
    }
    if let Some(owner) = owner {
        if WorkRepo::new(conn)
            .get(owner)?
            .is_some_and(|w| w.category.as_deref() == Some("clinical"))
        {
            return Err(DbError::Migration(
                "原记录关联了另一项临床研究，请先核对归属和研究关联".into(),
            ));
        }
    }
    conn.execute("INSERT INTO project_relations(entity_kind,entity_id,clinical_work_id,created_at,updated_at) VALUES(?1,?2,?3,?4,?4)",params![kind,id,target,now_unix()])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inbox_conversion_preserves_explicit_clinical_association_without_duplicate_records() {
        let db = super::super::Database::open_in_memory().unwrap();
        let conn = db.conn();
        let study = WorkRepo::new(conn).insert("研究", "active").unwrap();
        set_category(conn, study.id, Some("clinical"), study.revision).unwrap();
        let inbox = super::super::inbox::InboxRepo::new(conn);
        let source = inbox.insert("专家跟进").unwrap();
        set(conn, "inbox", source.id, Some(study.id), 0).unwrap();
        let task = super::super::task::TaskRepo::new(conn)
            .insert(None, "跟进", "normal", None, None)
            .unwrap();
        inbox.mark_processed(source.id, "task", task.id).unwrap();
        assert_eq!(
            get(conn, "task", task.id)
                .unwrap()
                .unwrap()
                .clinical_work_id,
            Some(study.id)
        );
        assert_eq!(
            conn.query_row("SELECT count(*) FROM tasks", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        WorkRepo::new(conn).delete(study.id).unwrap();
        assert!(get(conn, "task", task.id)
            .unwrap()
            .unwrap()
            .clinical_work_id
            .is_none());
        assert_eq!(
            conn.query_row("SELECT count(*) FROM tasks", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert!(inbox.get(source.id).unwrap().is_some());
    }
    #[test]
    fn classification_and_link_revisions_protect_ownership_and_user_edits() {
        let db = super::super::Database::open_in_memory().unwrap();
        let conn = db.conn();
        let repo = WorkRepo::new(conn);
        let owner = repo.insert("交流计划", "active").unwrap();
        let study = repo.insert("研究项目", "active").unwrap();
        assert!(owner.category.is_none());
        set_category(conn, owner.id, Some("non_clinical"), owner.revision).unwrap();
        set_category(conn, study.id, Some("clinical"), study.revision).unwrap();
        assert!(set_category(conn, study.id, Some("non_clinical"), study.revision).is_err());
        let task = super::super::task::TaskRepo::new(conn)
            .insert(Some(owner.id), "交流跟进", "normal", None, None)
            .unwrap();
        let link = set(conn, "task", task.id, Some(study.id), 0)
            .unwrap()
            .unwrap();
        assert_eq!(
            super::super::task::TaskRepo::new(conn)
                .get(task.id)
                .unwrap()
                .unwrap()
                .work_id,
            Some(owner.id)
        );
        assert!(set(conn, "task", task.id, None, 0).is_err());
        let removed = set(conn, "task", task.id, None, link.revision)
            .unwrap()
            .unwrap();
        assert!(set(conn, "task", task.id, Some(study.id), link.revision).is_err());
        assert!(removed.clinical_work_id.is_none());
        assert!(set(conn, "task", 999999, Some(study.id), 0).is_err());
        repo.archive(study.id).unwrap();
        assert!(set(conn, "task", task.id, Some(study.id), removed.revision).is_err());
    }
    #[test]
    fn helpers_reuse_confirmation_transaction_and_reject_nonclinical_targets() {
        let db = super::super::Database::open_in_memory().unwrap();
        let conn = db.conn();
        let work = WorkRepo::new(conn).insert("项目", "active").unwrap();
        let item = super::super::inbox::InboxRepo::new(conn)
            .insert("记下讨论")
            .unwrap();
        assert!(set(conn, "inbox", item.id, Some(work.id), 0).is_err());
        let tx = super::super::write_transaction(conn).unwrap();
        set_category(&tx, work.id, Some("clinical"), work.revision).unwrap();
        set(&tx, "inbox", item.id, Some(work.id), 0).unwrap();
        tx.rollback().unwrap();
        assert!(get(conn, "inbox", item.id).unwrap().is_none());
        assert!(WorkRepo::new(conn)
            .get(work.id)
            .unwrap()
            .unwrap()
            .category
            .is_none());
    }
}
