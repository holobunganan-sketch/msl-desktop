use crate::db::{Database, DbResult};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct CleanupCandidate {
    pub relative_path: String,
    pub category: String,
    pub bytes: u64,
    pub last_accessed_at: i64,
}
#[derive(Debug, Clone, Serialize)]
pub struct CleanupPlan {
    pub plan_id: String,
    pub created_at: i64,
    pub expires_at: i64,
    pub estimated_bytes: u64,
    pub estimated_count: u64,
    pub protected_counts: HashMap<String, u64>,
    pub warnings: Vec<String>,
    pub candidates: Vec<CleanupCandidate>,
}
static PLANS: OnceLock<Mutex<HashMap<String, CleanupPlan>>> = OnceLock::new();

pub fn preview(db: &Database, categories: &[String]) -> DbResult<CleanupPlan> {
    let now = crate::db::now_unix();
    let selected = if categories.is_empty() {
        vec![
            "extracted".into(),
            "chunks".into(),
            "model-catalog".into(),
            "previews".into(),
            "temp".into(),
            "logs".into(),
            "cognition".into(),
        ]
    } else {
        categories.to_vec()
    };
    let mut candidates = Vec::new();
    for e in crate::db::documents::CacheEntryRepo::new(db.conn()).list()? {
        if e.rebuildable && selected.iter().any(|c| c == &e.category) {
            candidates.push(CleanupCandidate {
                relative_path: e.relative_path,
                category: e.category,
                bytes: e.size_bytes.max(0) as u64,
                last_accessed_at: e.last_accessed_at,
            });
        }
    }
    let mut protected_counts = HashMap::new();
    for (name, sql) in [
        ("conversations", "SELECT COUNT(*) FROM qa_sessions"),
        ("expert_notes", "SELECT COUNT(*) FROM kol_notes"),
        ("expert_drafts", "SELECT COUNT(*) FROM kol_drafts"),
        ("expert_insights", "SELECT COUNT(*) FROM kol_insights"),
        (
            "pending_proposals",
            "SELECT COUNT(*) FROM ai_proposals WHERE status='pending'",
        ),
        (
            "confirmed_proposals",
            "SELECT COUNT(*) FROM ai_proposals WHERE status='confirmed'",
        ),
        (
            "kept_briefs",
            "SELECT COUNT(*) FROM daily_briefs WHERE retention_state='kept'",
        ),
        (
            "kept_reports",
            "SELECT COUNT(*) FROM reports WHERE retention_state='kept'",
        ),
        (
            "classification_memories",
            "SELECT COUNT(*) FROM classification_memories",
        ),
    ] {
        protected_counts.insert(
            name.into(),
            db.conn().query_row(sql, [], |r| r.get::<_, u64>(0))?,
        );
    }
    let estimated_bytes = candidates.iter().map(|c| c.bytes).sum();
    let plan = CleanupPlan {
        plan_id: Uuid::new_v4().to_string(),
        created_at: now,
        expires_at: now + 600,
        estimated_bytes,
        estimated_count: candidates.len() as u64,
        protected_counts,
        warnings: vec![
            "仅列出应用 cache root 下可重建文件；源工作目录、正式数据库、凭据和正式实体受保护。"
                .into(),
        ],
        candidates,
    };
    PLANS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap()
        .insert(plan.plan_id.clone(), plan.clone());
    Ok(plan)
}
pub fn take(plan_id: &str) -> Option<CleanupPlan> {
    let plans = PLANS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut lock = plans.lock().ok()?;
    let plan = lock.get(plan_id)?.clone();
    if crate::db::now_unix() > plan.expires_at {
        lock.remove(plan_id);
        None
    } else {
        Some(plan)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CleanupResult {
    pub plan_id: String,
    pub bytes_before: u64,
    pub bytes_after: u64,
    pub deleted_count: u64,
    pub skipped_count: u64,
    pub failed_count: u64,
}

pub fn execute(db: &Database, plan_id: &str) -> DbResult<CleanupResult> {
    crate::storage::paths::ensure_storage_disjoint(db)?;
    let plan = take(plan_id)
        .ok_or_else(|| crate::db::DbError::Migration("清理计划不存在或已过期".into()))?;
    let before = crate::storage::usage::get_usage(db)?.cache_bytes;
    let mut deleted = 0;
    let mut skipped = 0;
    let mut failed = 0;
    for candidate in &plan.candidates {
        let safe = crate::storage::paths::existing_cache_path(&candidate.relative_path);
        let Ok(path) = safe else {
            skipped += 1;
            continue;
        };
        let metadata = std::fs::symlink_metadata(&path);
        let Ok(meta) = metadata else {
            let _ = crate::db::documents::CacheEntryRepo::new(db.conn())
                .remove(&candidate.relative_path);
            skipped += 1;
            continue;
        };
        if !meta.file_type().is_file() {
            skipped += 1;
            continue;
        }
        if std::fs::remove_file(&path).is_ok() {
            crate::db::documents::CacheEntryRepo::new(db.conn())
                .remove(&candidate.relative_path)?;
            db.conn().execute("UPDATE document_index SET cache_rel_path=NULL,extract_status='pending' WHERE cache_rel_path=?1",[candidate.relative_path.as_str()])?;
            deleted += 1
        } else {
            failed += 1;
        }
    }
    let after = crate::storage::usage::get_usage(db)?.cache_bytes;
    let now = crate::db::now_unix();
    db.conn().execute("INSERT INTO storage_cleanup_runs(trigger,status,started_at,finished_at,bytes_before,bytes_after,deleted_counts_json) VALUES ('manual','completed',?1,?1,?2,?3,?4)",rusqlite::params![now,before,after,serde_json::json!({"deleted":deleted,"skipped":skipped,"failed":failed}).to_string()])?;
    db.conn().execute("DELETE FROM storage_cleanup_runs WHERE id NOT IN (SELECT id FROM storage_cleanup_runs ORDER BY id DESC LIMIT 50)",[])?;
    Ok(CleanupResult {
        plan_id: plan_id.into(),
        bytes_before: before,
        bytes_after: after,
        deleted_count: deleted,
        skipped_count: skipped,
        failed_count: failed,
    })
}

pub fn compact_activity_history(db: &Database, cutoff: i64) -> DbResult<u64> {
    let tx = crate::db::write_transaction(db.conn())?;
    let mut stmt=tx.prepare("SELECT date(timestamp,'unixepoch','localtime'),workspace_id,work_id,event_type,COUNT(*),MIN(timestamp),MAX(timestamp),substr(display_text,1,200) FROM activity_events WHERE timestamp < ?1 GROUP BY date(timestamp,'unixepoch','localtime'),workspace_id,work_id,event_type")?;
    let rows = stmt
        .query_map([cutoff], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<i64>>(1)?,
                r.get::<_, Option<i64>>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, i64>(5)?,
                r.get::<_, i64>(6)?,
                r.get::<_, String>(7)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    drop(stmt);
    for (date, workspace, work, event_type, count, first, last, sample) in &rows {
        tx.execute("INSERT INTO daily_activity_rollups(rollup_date,workspace_id,work_id,event_type,event_count,first_at,last_at,sample_text,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9) ON CONFLICT(rollup_date,COALESCE(workspace_id,0),COALESCE(work_id,0),event_type) DO UPDATE SET event_count=event_count+excluded.event_count,first_at=MIN(first_at,excluded.first_at),last_at=MAX(last_at,excluded.last_at),sample_text=CASE WHEN daily_activity_rollups.sample_text='' THEN excluded.sample_text ELSE daily_activity_rollups.sample_text END",rusqlite::params![date,workspace,work,event_type,count,first,last,sample,crate::db::now_unix()])?;
    }
    let deleted = tx.execute("DELETE FROM activity_events WHERE timestamp < ?1", [cutoff])? as u64;
    tx.commit()?;
    Ok(deleted)
}

pub fn compact_history(db: &Database, now: i64) -> DbResult<()> {
    let _ = compact_activity_history(db, now - 90 * 86400)?;
    db.conn().execute("DELETE FROM daily_briefs WHERE retention_state='draft' AND superseded_at IS NOT NULL AND superseded_at < ?1",[now-30*86400])?;
    db.conn().execute("DELETE FROM ai_proposals WHERE status IN ('rejected','superseded') AND decided_at IS NOT NULL AND decided_at < ?1",[now-30*86400])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    #[test]
    fn preview_protects_entities_and_expires() {
        let db = Database::open_in_memory().unwrap();
        crate::db::reports::ReportRepo::new(db.conn())
            .create("weekly", 1, 2)
            .unwrap();
        crate::db::documents::CacheEntryRepo::new(db.conn())
            .register("extracted", "extracted/synthetic", None, 10, None, None)
            .unwrap();
        let plan = preview(&db, &[]).unwrap();
        assert_eq!(plan.estimated_count, 1);
        assert!(plan.protected_counts.contains_key("pending_proposals"));
        assert!(plan
            .protected_counts
            .contains_key("classification_memories"));
        assert_eq!(plan.protected_counts.get("kept_reports"), Some(&1));
        assert!(take(&plan.plan_id).is_some());
    }
    #[test]
    fn execute_removes_only_cache_and_marks_document_pending() {
        let root = std::env::temp_dir().join(format!("msl-cleanup-{}", uuid::Uuid::new_v4()));
        let _local_app_data = crate::storage::paths::LocalAppDataTestGuard::set(&root);
        let db = Database::open_in_memory().unwrap();
        let path = crate::storage::paths::safe_cache_path("extracted/synthetic").unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"cache").unwrap();
        crate::db::documents::CacheEntryRepo::new(db.conn())
            .register("extracted", "extracted/synthetic", None, 5, None, None)
            .unwrap();
        let plan = preview(&db, &vec!["extracted".into()]).unwrap();
        let result = execute(&db, &plan.plan_id).unwrap();
        assert_eq!(result.deleted_count, 1);
        assert!(!path.exists());
        let _ = std::fs::remove_dir_all(root);
    }
    #[test]
    fn rollup_preserves_counts_and_is_idempotent() {
        let db = Database::open_in_memory().unwrap();
        let now = crate::db::now_unix();
        crate::db::activity::ActivityRepo::new(db.conn())
            .insert(
                "task.completed",
                None,
                None,
                None,
                None,
                None,
                "synthetic activity",
                None,
                None,
            )
            .unwrap();
        let deleted = compact_activity_history(&db, now + 1).unwrap();
        assert_eq!(deleted, 1);
        assert_eq!(
            db.conn()
                .query_row(
                    "SELECT SUM(event_count) FROM daily_activity_rollups",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
        assert_eq!(compact_activity_history(&db, now + 1).unwrap(), 0);
    }
}
