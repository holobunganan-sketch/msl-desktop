//! Transactional confirmation groups with conflict-checked undo. Database only.

use crate::db::{now_unix, Database, DbError, DbResult};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const TABLES: &[&str] = &[
    "works",
    "tasks",
    "waiting_items",
    "calendar_events",
    "inbox_items",
    "resume_points",
    "work_workspace_links",
    "work_file_refs",
    "project_relations",
    "kol_drafts",
    "kol_insights",
    "kol_projects",
    "classification_memories",
    "ai_proposals",
    "ai_proposal_outcomes",
];
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Change {
    table: String,
    key: Value,
    before: Option<Value>,
    after: Option<Value>,
}
#[derive(Debug, Serialize)]
pub struct ConfirmationGroup {
    pub receipt_id: String,
    pub results: Vec<super::apply::ApplyResult>,
}
#[derive(Debug, Serialize)]
pub struct ReceiptCard {
    pub id: String,
    pub proposal_ids: Vec<i64>,
    pub created_at: i64,
    pub undone_at: Option<i64>,
}

fn quote(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}
fn columns(conn: &Connection, table: &str) -> DbResult<Vec<(String, bool)>> {
    if !TABLES.contains(&table) {
        return Err(DbError::Migration("撤销记录的实体类型无效".into()));
    }
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", quote(table)))?;
    let rows = stmt.query_map([], |r| {
        Ok((r.get::<_, String>(1)?, r.get::<_, i64>(5)? > 0))
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}
fn json_row(cols: &[(String, bool)], prefix: &str, keys: bool) -> String {
    format!(
        "json_object({})",
        cols.iter()
            .filter(|(_, pk)| !keys || *pk)
            .map(|(name, _)| format!("'{}',{}{}", name.replace('\'', "''"), prefix, quote(name)))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn start_capture(conn: &Connection) -> DbResult<()> {
    conn.execute_batch("CREATE TEMP TABLE _msl_capture(seq INTEGER PRIMARY KEY,table_name TEXT,key_json TEXT,before_json TEXT,after_json TEXT);")?;
    for table in TABLES {
        let cols = columns(conn, table)?;
        for (event, key, before, after) in [
            (
                "INSERT",
                json_row(&cols, "NEW.", true),
                "NULL".into(),
                json_row(&cols, "NEW.", false),
            ),
            (
                "UPDATE",
                json_row(&cols, "NEW.", true),
                json_row(&cols, "OLD.", false),
                json_row(&cols, "NEW.", false),
            ),
            (
                "DELETE",
                json_row(&cols, "OLD.", true),
                json_row(&cols, "OLD.", false),
                "NULL".into(),
            ),
        ] {
            conn.execute_batch(&format!("CREATE TEMP TRIGGER _msl_{table}_{event} AFTER {event} ON main.{table} BEGIN INSERT INTO _msl_capture(table_name,key_json,before_json,after_json) VALUES ('{table}',{key},{before},{after}); END;"))?;
        }
    }
    Ok(())
}
fn finish_capture(conn: &Connection) -> DbResult<Vec<Change>> {
    let mut stmt = conn.prepare(
        "SELECT table_name,key_json,before_json,after_json FROM _msl_capture ORDER BY seq",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, Option<String>>(3)?,
        ))
    })?;
    let mut changes = Vec::new();
    for row in rows {
        let (table, key, before, after) = row?;
        let parse = |s: &str| {
            serde_json::from_str(s).map_err(|_| DbError::Migration("撤销凭据解析失败".into()))
        };
        changes.push(Change {
            table,
            key: parse(&key)?,
            before: before.as_deref().map(parse).transpose()?,
            after: after.as_deref().map(parse).transpose()?,
        });
    }
    drop(stmt);
    // Multiple writes (including revision triggers) form one user action. Store
    // each row's original state and its actual final state, not intermediate
    // trigger snapshots that would conflict with our own undo.
    let mut consolidated: Vec<Change> = Vec::new();
    for change in changes {
        if let Some(existing) = consolidated
            .iter_mut()
            .find(|entry| entry.table == change.table && entry.key == change.key)
        {
            existing.after = change.after;
        } else {
            consolidated.push(change);
        }
    }
    for change in &mut consolidated {
        let cols = columns(conn, &change.table)?;
        let keys = change
            .key
            .as_object()
            .ok_or_else(|| DbError::Migration("撤销凭据缺少主键".into()))?;
        let condition = keys
            .keys()
            .map(|key| format!("{}=?", quote(key)))
            .collect::<Vec<_>>()
            .join(" AND ");
        let current: Option<String> = conn
            .query_row(
                &format!(
                    "SELECT {} FROM {} WHERE {condition}",
                    json_row(&cols, "", false),
                    quote(&change.table)
                ),
                rusqlite::params_from_iter(keys.values().map(sql_value)),
                |row| row.get(0),
            )
            .optional()?;
        change.after = current
            .map(|value| serde_json::from_str(&value))
            .transpose()
            .map_err(|_| DbError::Migration("撤销最终状态解析失败".into()))?;
    }
    consolidated.retain(|change| change.before != change.after);
    for table in TABLES {
        for event in ["INSERT", "UPDATE", "DELETE"] {
            conn.execute_batch(&format!("DROP TRIGGER _msl_{table}_{event};"))?;
        }
    }
    conn.execute_batch("DROP TABLE _msl_capture;")?;
    Ok(consolidated)
}
fn execute(db: &Database, items: &[(i64, i64, Option<Value>)]) -> DbResult<ConfirmationGroup> {
    if items.is_empty() || items.len() > 20 {
        return Err(DbError::Migration("请选择 1 到 20 条已审阅建议".into()));
    }
    let unique = items
        .iter()
        .map(|i| i.0)
        .collect::<std::collections::HashSet<_>>();
    if unique.len() != items.len() {
        return Err(DbError::Migration("选择中有重复建议".into()));
    }
    let tx = crate::db::write_transaction(db.conn())?;
    start_capture(&tx)?;
    let id = uuid::Uuid::new_v4().to_string();
    let mut results = Vec::new();
    for (proposal, expected, payload) in items {
        let mut result =
            super::apply::confirm_in_transaction(&tx, *proposal, *expected, payload.clone())?;
        result.receipt_id = id.clone();
        results.push(result);
    }
    let changes = finish_capture(&tx)?;
    for result in &results {
        super::rounds::acknowledge_result(&tx, result)?;
    }
    // Release new project material only after every acknowledgement, so batch
    // order cannot overwrite the destination project's analysis opportunity.
    for result in &results {
        super::rounds::release_new_association(&tx, result)?;
    }
    tx.execute("INSERT INTO proposal_receipts(id,proposal_ids_json,changes_json,created_at) VALUES (?1,?2,?3,?4)",params![id,serde_json::json!(items.iter().map(|i|i.0).collect::<Vec<_>>()).to_string(),serde_json::to_string(&changes).map_err(|_|DbError::Migration("撤销凭据写入失败".into()))?,now_unix()])?;
    tx.commit()?;
    Ok(ConfirmationGroup {
        receipt_id: id,
        results,
    })
}
pub fn confirm_batch(db: &Database, items: &[(i64, i64)]) -> DbResult<ConfirmationGroup> {
    execute(
        db,
        &items
            .iter()
            .map(|(id, ts)| (*id, *ts, None))
            .collect::<Vec<_>>(),
    )
}
pub fn confirm_single(
    db: &Database,
    id: i64,
    expected: i64,
    payload: Option<Value>,
) -> DbResult<super::apply::ApplyResult> {
    Ok(execute(db, &[(id, expected, payload)])?.results.remove(0))
}
fn sql_value(value: &Value) -> rusqlite::types::Value {
    match value {
        Value::Null => rusqlite::types::Value::Null,
        Value::Bool(b) => rusqlite::types::Value::Integer(i64::from(*b)),
        Value::Number(n) => n
            .as_i64()
            .map(rusqlite::types::Value::Integer)
            .unwrap_or_else(|| rusqlite::types::Value::Real(n.as_f64().unwrap_or(0.))),
        Value::String(s) => rusqlite::types::Value::Text(s.clone()),
        _ => rusqlite::types::Value::Text(value.to_string()),
    }
}
pub fn undo(db: &Database, receipt: &str) -> DbResult<()> {
    let tx = crate::db::write_transaction(db.conn())?;
    let json: Option<String> = tx
        .query_row(
            "SELECT changes_json FROM proposal_receipts WHERE id=?1 AND undone_at IS NULL",
            [receipt],
            |r| r.get(0),
        )
        .optional()?;
    let changes: Vec<Change> = serde_json::from_str(
        &json.ok_or_else(|| DbError::Migration("记录不存在或已经撤销".into()))?,
    )
    .map_err(|_| DbError::Migration("撤销凭据无效".into()))?;
    // Check later external links BEFORE restoring categories. The category
    // lifecycle trigger may unlink records, so checking after restoration would
    // miss a user's later association and could silently remove it.
    for change in &changes {
        if change.table != "works" {
            continue;
        }
        let removes_clinical = change.before.is_none()
            || (change
                .after
                .as_ref()
                .is_some_and(|row| row["category"] == "clinical")
                && !change
                    .before
                    .as_ref()
                    .is_some_and(|row| row["category"] == "clinical"));
        if !removes_clinical {
            continue;
        }
        let id = change.key["id"].as_i64().unwrap_or(0);
        for relation in crate::db::project_relations::by_clinical_project(&tx, id)? {
            let reversed_here = changes.iter().any(|other| {
                other.table == "project_relations"
                    && other.key["id"].as_i64() == Some(relation.id)
                    && other
                        .after
                        .as_ref()
                        .is_some_and(|row| row["clinical_work_id"].as_i64() == Some(id))
                    && !other
                        .before
                        .as_ref()
                        .is_some_and(|row| row["clinical_work_id"].as_i64() == Some(id))
            });
            if !reversed_here {
                return Err(DbError::Migration(
                    "项目已有后续临床研究关联，无法安全撤销。未修改任何记录。".into(),
                ));
            }
        }
    }
    for change in changes.into_iter().rev() {
        let cols = columns(&tx, &change.table)?;
        let keys = change
            .key
            .as_object()
            .ok_or_else(|| DbError::Migration("撤销凭据缺少主键".into()))?;
        if keys.is_empty()
            || keys
                .keys()
                .any(|k| !cols.iter().any(|(name, pk)| name == k && *pk))
        {
            return Err(DbError::Migration("撤销主键无效".into()));
        }
        let condition = keys
            .keys()
            .map(|k| format!("{}=?", quote(k)))
            .collect::<Vec<_>>()
            .join(" AND ");
        let bindings = keys.values().map(sql_value).collect::<Vec<_>>();
        let current: Option<String> = tx
            .query_row(
                &format!(
                    "SELECT {} FROM {} WHERE {condition}",
                    json_row(&cols, "", false),
                    quote(&change.table)
                ),
                rusqlite::params_from_iter(bindings.iter()),
                |r| r.get(0),
            )
            .optional()?;
        let mut current = current
            .map(|s| serde_json::from_str::<Value>(&s))
            .transpose()
            .map_err(|_| DbError::Migration("当前记录解析失败".into()))?;
        // A legacy receipt predates the nullable category column. Null added by
        // the migration is not a user edit; any actual classification still
        // participates in the exact conflict check (as does revision).
        if change.table == "works"
            && change
                .after
                .as_ref()
                .is_some_and(|row| row.get("category").is_none())
        {
            if let Some(row) = current.as_mut().and_then(Value::as_object_mut) {
                if row.get("category").is_some_and(Value::is_null) {
                    row.remove("category");
                }
            }
        }
        if current != change.after {
            return Err(DbError::Migration(
                "这些事项已被后续修改，无法安全撤销。未覆盖任何新记录。".into(),
            ));
        }
        if let Some(before) = change.before {
            let row = before
                .as_object()
                .ok_or_else(|| DbError::Migration("撤销原值无效".into()))?;
            let names = cols
                .iter()
                .map(|(name, _)| quote(name))
                .collect::<Vec<_>>()
                .join(",");
            let placeholders = cols.iter().map(|_| "?").collect::<Vec<_>>().join(",");
            let values = cols
                .iter()
                .map(|(name, _)| sql_value(&row[name]))
                .collect::<Vec<_>>();
            let updates = cols
                .iter()
                .filter(|(_, pk)| !*pk)
                .map(|(name, _)| format!("{}=excluded.{}", quote(name), quote(name)))
                .collect::<Vec<_>>()
                .join(",");
            tx.execute(&format!("INSERT INTO {} ({names}) VALUES ({placeholders}) ON CONFLICT DO UPDATE SET {updates}",quote(&change.table)),rusqlite::params_from_iter(values))?;
        } else {
            if change.table == "works" {
                let id = change.key["id"].as_i64().unwrap_or(0);
                for table in [
                    "tasks",
                    "waiting_items",
                    "calendar_events",
                    "resume_points",
                    "work_workspace_links",
                    "work_file_refs",
                ] {
                    let referenced: bool = tx.query_row(
                        &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE work_id=?1)"),
                        [id],
                        |r| r.get(0),
                    )?;
                    if referenced {
                        return Err(DbError::Migration(
                            "项目已有后续关联事项，无法安全撤销创建".into(),
                        ));
                    }
                }
            }
            tx.execute(
                &format!("DELETE FROM {} WHERE {condition}", quote(&change.table)),
                rusqlite::params_from_iter(bindings),
            )?;
        }
    }
    // Pre-v23 receipts have no outcome-table change. Remove their migrated
    // link only after the complete receipt has passed conflict checks.
    tx.execute("DELETE FROM ai_proposal_outcomes WHERE proposal_id IN (SELECT value FROM json_each((SELECT proposal_ids_json FROM proposal_receipts WHERE id=?1))) AND proposal_id IN (SELECT id FROM ai_proposals WHERE status='pending')",[receipt])?;
    tx.execute(
        "UPDATE proposal_receipts SET undone_at=?1 WHERE id=?2",
        params![now_unix(), receipt],
    )?;
    tx.execute("INSERT INTO activity_events(timestamp,event_type,display_text,metadata_json) VALUES (?1,'ai.confirmation.undone','撤销已确认建议',?2)",params![now_unix(),serde_json::json!({"receipt_id":receipt}).to_string()])?;
    tx.commit()?;
    Ok(())
}
pub fn list(db: &Database) -> DbResult<Vec<ReceiptCard>> {
    let mut stmt=db.conn().prepare("SELECT id,proposal_ids_json,created_at,undone_at FROM proposal_receipts ORDER BY created_at DESC,rowid DESC LIMIT 30")?;
    let rows = stmt.query_map([], |r| {
        Ok(ReceiptCard {
            id: r.get(0)?,
            proposal_ids: serde_json::from_str(&r.get::<_, String>(1)?).unwrap_or_default(),
            created_at: r.get(2)?,
            undone_at: r.get(3)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[cfg(test)]
mod tests {
    use crate::db::{
        ai::{AnalysisRunRepo, ProposalRepo},
        Database,
    };
    #[test]
    fn clinical_link_receipt_tracks_user_edits_and_restores_unlinked_state() {
        let db = Database::open_in_memory().unwrap();
        let conn = db.conn();
        let study = crate::db::work::WorkRepo::new(conn)
            .insert("Synthetic study", "active")
            .unwrap();
        crate::db::project_relations::set_category(
            conn,
            study.id,
            Some("clinical"),
            study.revision,
        )
        .unwrap();
        let task = crate::db::task::TaskRepo::new(conn)
            .insert(None, "Synthetic followup", "normal", None, None)
            .unwrap();
        super::start_capture(conn).unwrap();
        let link = crate::db::project_relations::set(conn, "task", task.id, Some(study.id), 0)
            .unwrap()
            .unwrap();
        let changes = super::finish_capture(conn).unwrap();
        assert!(
            changes
                .iter()
                .any(|change| change.table == "project_relations"),
            "The clinical link must be part of undo, not silently left behind"
        );
        conn.execute("INSERT INTO proposal_receipts(id,proposal_ids_json,changes_json,created_at) VALUES('clinical-link','[]',?1,1)",[serde_json::to_string(&changes).unwrap()]).unwrap();
        super::undo(&db, "clinical-link").unwrap();
        assert!(crate::db::project_relations::get(conn, "task", task.id)
            .unwrap()
            .is_none());
        assert!(crate::db::task::TaskRepo::new(conn)
            .get(task.id)
            .unwrap()
            .is_some());

        super::start_capture(conn).unwrap();
        crate::db::project_relations::set(conn, "task", task.id, Some(study.id), 0).unwrap();
        let changed = super::finish_capture(conn).unwrap();
        conn.execute("INSERT INTO proposal_receipts(id,proposal_ids_json,changes_json,created_at) VALUES('clinical-link-conflict','[]',?1,1)",[serde_json::to_string(&changed).unwrap()]).unwrap();
        crate::db::project_relations::set(conn, "task", task.id, None, link.revision).unwrap();
        assert!(super::undo(&db, "clinical-link-conflict").is_err());
    }
    #[test]
    fn clinical_project_undo_preserves_later_external_links() {
        let db = Database::open_in_memory().unwrap();
        let conn = db.conn();
        super::start_capture(conn).unwrap();
        let study = crate::db::work::WorkRepo::new(conn)
            .insert("Synthetic clinical study", "active")
            .unwrap();
        crate::db::project_relations::set_category(
            conn,
            study.id,
            Some("clinical"),
            study.revision,
        )
        .unwrap();
        let changes = super::finish_capture(conn).unwrap();
        conn.execute("INSERT INTO proposal_receipts(id,proposal_ids_json,changes_json,created_at) VALUES('clinical-project','[]',?1,1)",[serde_json::to_string(&changes).unwrap()]).unwrap();
        let task = crate::db::task::TaskRepo::new(conn)
            .insert(None, "Later external followup", "normal", None, None)
            .unwrap();
        crate::db::project_relations::set(conn, "task", task.id, Some(study.id), 0).unwrap();
        assert!(
            super::undo(&db, "clinical-project").is_err(),
            "Undo must not silently detach a user's later clinical link"
        );
        assert!(crate::db::work::WorkRepo::new(conn)
            .get(study.id)
            .unwrap()
            .is_some());
        assert_eq!(
            crate::db::project_relations::get(conn, "task", task.id)
                .unwrap()
                .unwrap()
                .clinical_work_id,
            Some(study.id)
        );
        let relation = crate::db::project_relations::get(conn, "task", task.id)
            .unwrap()
            .unwrap();
        crate::db::project_relations::set(conn, "task", task.id, None, relation.revision).unwrap();
        super::undo(&db, "clinical-project").unwrap();
        assert!(crate::db::work::WorkRepo::new(conn)
            .get(study.id)
            .unwrap()
            .is_none());
    }
    #[test]
    fn legacy_project_receipts_remain_undoable_after_nullable_category_migration() {
        let db = Database::open_in_memory().unwrap();
        let conn = db.conn();
        super::start_capture(conn).unwrap();
        let work = crate::db::work::WorkRepo::new(conn)
            .insert("Synthetic legacy project", "active")
            .unwrap();
        let mut changes = super::finish_capture(conn).unwrap();
        for change in &mut changes {
            if change.table == "works" {
                if let Some(row) = change
                    .before
                    .as_mut()
                    .and_then(serde_json::Value::as_object_mut)
                {
                    row.remove("category");
                }
                if let Some(row) = change
                    .after
                    .as_mut()
                    .and_then(serde_json::Value::as_object_mut)
                {
                    row.remove("category");
                }
            }
        }
        conn.execute("INSERT INTO proposal_receipts(id,proposal_ids_json,changes_json,created_at) VALUES('legacy-project','[]',?1,1)",[serde_json::to_string(&changes).unwrap()]).unwrap();
        super::undo(&db, "legacy-project").unwrap();
        assert!(crate::db::work::WorkRepo::new(conn)
            .get(work.id)
            .unwrap()
            .is_none());
    }
    #[test]
    fn created_work_undo_preserves_later_children_and_restores_clean_creation() {
        let db = Database::open_in_memory().unwrap();
        let run = AnalysisRunRepo::new(db.conn())
            .create("manual", None, None)
            .unwrap();
        let repo = ProposalRepo::new(db.conn());
        let proposal = repo
            .upsert_pending(
                run.id,
                "work",
                "create",
                None,
                None,
                None,
                "new-project-undo",
                "Synthetic project",
                "{}",
                "",
                "[]",
                None,
            )
            .unwrap()
            .unwrap();
        let group = super::confirm_batch(&db, &[(proposal.id, proposal.updated_at)]).unwrap();
        let work_id = group.results[0].target_id;
        let task = crate::db::task::TaskRepo::new(db.conn())
            .insert(Some(work_id), "Later user task", "normal", None, None)
            .unwrap();
        assert!(super::undo(&db, &group.receipt_id).is_err());
        assert!(crate::db::work::WorkRepo::new(db.conn())
            .get(work_id)
            .unwrap()
            .is_some());
        db.conn()
            .execute("DELETE FROM tasks WHERE id=?1", [task.id])
            .unwrap();
        super::undo(&db, &group.receipt_id).unwrap();
        assert!(crate::db::work::WorkRepo::new(db.conn())
            .get(work_id)
            .unwrap()
            .is_none());
        assert_eq!(repo.get(proposal.id).unwrap().unwrap().status, "pending");
    }
    #[test]
    fn confirmed_followup_can_resolve_waiting_and_complete_task_then_undo() {
        let db = Database::open_in_memory().unwrap();
        let task = crate::db::task::TaskRepo::new(db.conn())
            .insert(None, "Reply arrived", "normal", None, None)
            .unwrap();
        let waiting = crate::db::task::WaitingRepo::new(db.conn())
            .insert(None, "Await reply", "Colleague", None, None)
            .unwrap();
        let run = AnalysisRunRepo::new(db.conn())
            .create("manual", None, None)
            .unwrap();
        let repo = ProposalRepo::new(db.conn());
        let a = repo
            .upsert_pending(
                run.id,
                "task",
                "update",
                Some(task.id),
                None,
                None,
                "complete",
                "Reply arrived",
                r#"{"status":"done"}"#,
                "Explicit user observation",
                "[]",
                None,
            )
            .unwrap()
            .unwrap();
        let b = repo
            .upsert_pending(
                run.id,
                "waiting",
                "update",
                Some(waiting.id),
                None,
                None,
                "resolve",
                "Await reply",
                r#"{"status":"resolved"}"#,
                "Reply has arrived",
                "[]",
                None,
            )
            .unwrap()
            .unwrap();
        let result =
            super::confirm_batch(&db, &[(a.id, a.updated_at), (b.id, b.updated_at)]).unwrap();
        assert_eq!(
            crate::db::task::TaskRepo::new(db.conn())
                .get(task.id)
                .unwrap()
                .unwrap()
                .status,
            "done"
        );
        assert_eq!(
            crate::db::task::WaitingRepo::new(db.conn())
                .get(waiting.id)
                .unwrap()
                .unwrap()
                .status,
            "resolved"
        );
        super::undo(&db, &result.receipt_id).unwrap();
        assert_eq!(
            crate::db::task::TaskRepo::new(db.conn())
                .get(task.id)
                .unwrap()
                .unwrap()
                .status,
            "next"
        );
        assert_eq!(
            crate::db::task::WaitingRepo::new(db.conn())
                .get(waiting.id)
                .unwrap()
                .unwrap()
                .status,
            "open"
        );
    }
    #[test]
    fn group_confirm_undo_restores_entities_feedback_and_inbox() {
        let db = Database::open_in_memory().unwrap();
        let run = AnalysisRunRepo::new(db.conn())
            .create("manual", None, None)
            .unwrap();
        let repo = ProposalRepo::new(db.conn());
        let a = repo
            .upsert_pending(
                run.id, "task", "create", None, None, None, "group-a", "Prepare", "{}", "", "[]",
                None,
            )
            .unwrap()
            .unwrap();
        let b = repo
            .upsert_pending(
                run.id, "waiting", "create", None, None, None, "group-b", "Wait", "{}", "", "[]",
                None,
            )
            .unwrap()
            .unwrap();
        let result =
            super::confirm_batch(&db, &[(a.id, a.updated_at), (b.id, b.updated_at)]).unwrap();
        assert_eq!(result.results.len(), 2);
        assert_eq!(
            crate::db::memory::stats(db.conn()).unwrap().feedback_count,
            2
        );
        super::undo(&db, &result.receipt_id).unwrap();
        assert_eq!(
            crate::db::memory::stats(db.conn()).unwrap().feedback_count,
            0
        );
        assert_eq!(
            db.conn()
                .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            db.conn()
                .query_row("SELECT COUNT(*) FROM waiting_items", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(repo.get(a.id).unwrap().unwrap().status, "pending");
        assert!(super::undo(&db, &result.receipt_id).is_err());
    }
    #[test]
    fn group_failure_rolls_back_and_undo_refuses_later_changes() {
        let db = Database::open_in_memory().unwrap();
        let run = AnalysisRunRepo::new(db.conn())
            .create("manual", None, None)
            .unwrap();
        let repo = ProposalRepo::new(db.conn());
        let a = repo
            .upsert_pending(
                run.id, "task", "create", None, None, None, "atomic-a", "Prepare", "{}", "", "[]",
                None,
            )
            .unwrap()
            .unwrap();
        let b = repo
            .upsert_pending(
                run.id,
                "calendar",
                "create",
                None,
                None,
                None,
                "atomic-b",
                "Missing date",
                "{}",
                "",
                "[]",
                None,
            )
            .unwrap()
            .unwrap();
        assert!(super::confirm_batch(&db, &[(a.id, a.updated_at), (b.id, b.updated_at)]).is_err());
        assert_eq!(repo.get(a.id).unwrap().unwrap().status, "pending");
        assert_eq!(
            db.conn()
                .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
        let result = super::confirm_batch(&db, &[(a.id, a.updated_at)]).unwrap();
        db.conn()
            .execute("UPDATE tasks SET title='user changed later'", [])
            .unwrap();
        assert!(super::undo(&db, &result.receipt_id).is_err());
        assert_eq!(
            db.conn()
                .query_row("SELECT title FROM tasks LIMIT 1", [], |r| r
                    .get::<_, String>(0))
                .unwrap(),
            "user changed later"
        );
    }
}
