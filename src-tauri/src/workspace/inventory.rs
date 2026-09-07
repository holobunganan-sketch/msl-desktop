//! 工作目录 metadata inventory：只读取目录项 metadata，不读取文件正文。

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::db::workspace::{WorkspaceFileState, WorkspaceFileStateRepo};
use crate::db::{now_unix, Database, DbError, DbResult};

pub const MAX_FILES: usize = 50_000;

#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub workspace_id: i64,
    pub path: String,
    pub relative_path: String,
    pub modified_at: i64,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct InventorySnapshot {
    pub files: Vec<FileMetadata>,
    pub skipped: u64,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ReconcileReport {
    pub workspace_id: i64,
    pub scanned: u64,
    pub baseline_count: u64,
    pub created: u64,
    pub modified: u64,
    pub deleted: u64,
    pub skipped: u64,
    pub warning: Option<String>,
    pub first_scan: bool,
    pub scanned_at: i64,
}

fn modified_seconds(meta: &std::fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 递归扫描工作目录。每个文件只取路径、修改时间、大小；不可读项会计入 skipped。
pub fn scan(root: &Path, workspace_id: i64) -> std::io::Result<InventorySnapshot> {
    let root = root.canonicalize()?;
    if !root.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "工作目录不可读或不是目录",
        ));
    }

    let mut stack = vec![root.clone()];
    let mut files = Vec::new();
    let mut skipped = 0_u64;
    let mut capped = false;

    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(value) => value,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(value) => value,
                Err(_) => {
                    skipped += 1;
                    continue;
                }
            };
            let path = entry.path();
            if crate::workspace::watcher::is_temp_file(&path) {
                continue;
            }
            let metadata = match entry.metadata() {
                Ok(value) => value,
                Err(_) => {
                    skipped += 1;
                    continue;
                }
            };
            if metadata.is_dir() {
                stack.push(path);
                continue;
            }
            if !metadata.is_file() {
                continue;
            }
            if files.len() >= MAX_FILES {
                capped = true;
                break;
            }
            let relative_path = path
                .strip_prefix(&root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            files.push(FileMetadata {
                workspace_id,
                path: path.to_string_lossy().into_owned(),
                relative_path,
                modified_at: modified_seconds(&metadata),
                size: metadata.len(),
            });
        }
        if capped {
            break;
        }
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));
    let warning = if capped {
        Some(format!("扫描已达到 {} 个文件上限，结果不完整", MAX_FILES))
    } else if skipped > 0 {
        Some(format!("有 {} 个目录或文件无法读取", skipped))
    } else {
        None
    };
    Ok(InventorySnapshot {
        files,
        skipped,
        warning,
    })
}

fn already_recorded(tx: &rusqlite::Transaction<'_>, dedupe_key: &str) -> rusqlite::Result<bool> {
    tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM activity_events WHERE dedupe_key = ?1)",
        [dedupe_key],
        |row| row.get(0),
    )
}

fn insert_activity(
    tx: &rusqlite::Transaction<'_>,
    event_type: &str,
    workspace_id: i64,
    path: Option<&str>,
    display_text: &str,
    metadata_json: &str,
    dedupe_key: &str,
) -> rusqlite::Result<bool> {
    if already_recorded(tx, dedupe_key)? {
        return Ok(false);
    }
    tx.execute(
        "INSERT INTO activity_events
          (timestamp, event_type, workspace_id, work_id, entity_type, entity_id,
           path, display_text, metadata_json, dedupe_key)
         VALUES (?1, ?2, ?3, NULL, 'file', NULL, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            now_unix(),
            event_type,
            workspace_id,
            path,
            display_text,
            metadata_json,
            dedupe_key
        ],
    )?;
    Ok(true)
}

/// 比较当前目录与上一次快照，并在一次 SQLite 事务中更新快照及活动。
pub fn reconcile(db: &Database, workspace_id: i64, root: &Path) -> DbResult<ReconcileReport> {
    let snapshot = scan(root, workspace_id).map_err(DbError::from)?;
    let conn = db.conn();
    // File I/O stays outside the short write transaction. Reserve the writer
    // before reading the previous baseline so concurrent scans cannot upgrade
    // an invalidated WAL read snapshot.
    let tx = crate::db::write_transaction(conn)?;
    let existing = WorkspaceFileStateRepo::new(&tx).list(workspace_id)?;
    let has_baseline: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM activity_events WHERE workspace_id = ?1 AND event_type = 'workspace.baseline')",
        [workspace_id],
        |row| row.get::<_, bool>(0),
    )?;
    let first_scan = !has_baseline;
    let old_by_path: HashMap<String, WorkspaceFileState> = existing
        .into_iter()
        .map(|state| (state.path.clone(), state))
        .collect();
    let current_paths: HashSet<&str> = snapshot.files.iter().map(|f| f.path.as_str()).collect();
    let mut created = 0_u64;
    let mut modified = 0_u64;
    let mut deleted = 0_u64;

    for file in &snapshot.files {
        let old = old_by_path.get(&file.path);
        let kind = match old {
            None if !first_scan => Some(("file.created", "新建文件")),
            Some(old) if old.modified_at != file.modified_at || old.size != file.size => {
                Some(("file.modified", "修改文件"))
            }
            _ => None,
        };
        if let Some((event_type, verb)) = kind {
            let dedupe = format!(
                "reconcile|{workspace_id}|{event_type}|{}|{}|{}",
                file.path, file.modified_at, file.size
            );
            let metadata = r#"{"source":"reconcile"}"#;
            let _ = insert_activity(
                &tx,
                event_type,
                workspace_id,
                Some(&file.path),
                &format!("{verb} {}", file.relative_path),
                metadata,
                &dedupe,
            )?;
            match event_type {
                "file.created" => created += 1,
                "file.modified" => modified += 1,
                _ => {}
            }
        }
        tx.execute(
            "INSERT INTO workspace_file_state (workspace_id, path, modified_at, size, seen_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(workspace_id, path) DO UPDATE SET
               modified_at = excluded.modified_at,
               size = excluded.size,
               seen_at = excluded.seen_at",
            rusqlite::params![
                workspace_id,
                file.path,
                file.modified_at,
                file.size as i64,
                now_unix()
            ],
        )?;
    }

    for old in old_by_path.values() {
        if current_paths.contains(old.path.as_str()) {
            continue;
        }
        let dedupe = format!(
            "reconcile|{workspace_id}|file.deleted|{}|{}|{}",
            old.path, old.modified_at, old.size
        );
        if !first_scan {
            let inserted = insert_activity(
                &tx,
                "file.deleted",
                workspace_id,
                Some(&old.path),
                &format!(
                    "删除文件 {}",
                    PathBuf::from(&old.path)
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| old.path.clone()),
                ),
                r#"{"source":"reconcile"}"#,
                &dedupe,
            )?;
            // Only count a newly inserted event, not a repeated dedupe lookup.
            if inserted {
                deleted += 1;
            }
        }
        tx.execute(
            "DELETE FROM workspace_file_state WHERE workspace_id = ?1 AND path = ?2",
            rusqlite::params![workspace_id, old.path],
        )?;
    }

    if first_scan {
        let baseline_key = format!("workspace.baseline|{workspace_id}");
        let warning = snapshot.warning.clone().unwrap_or_default();
        let _ = insert_activity(
            &tx,
            "workspace.baseline",
            workspace_id,
            None,
            &format!("工作目录基线：{} 个文件，跳过 {} 个", snapshot.files.len(), snapshot.skipped),
            &serde_json::json!({"source":"baseline","skipped":snapshot.skipped,"warning":snapshot.warning}).to_string(),
            &baseline_key,
        )?;
        let _ = warning;
    } else {
        let reconcile_key = format!("workspace.reconcile|{workspace_id}|{}", now_unix());
        let _ = insert_activity(
            &tx,
            "workspace.reconcile",
            workspace_id,
            None,
            &format!(
                "工作目录扫描：新增 {}，修改 {}，删除 {}，跳过 {}",
                created, modified, deleted, snapshot.skipped
            ),
            &serde_json::json!({
                "source": "reconcile",
                "created": created,
                "modified": modified,
                "deleted": deleted,
                "skipped": snapshot.skipped,
                "warning": snapshot.warning
            })
            .to_string(),
            &reconcile_key,
        )?;
    }
    tx.commit()?;

    Ok(ReconcileReport {
        workspace_id,
        scanned: snapshot.files.len() as u64,
        baseline_count: snapshot.files.len() as u64,
        created,
        modified,
        deleted,
        skipped: snapshot.skipped,
        warning: snapshot.warning,
        first_scan,
        scanned_at: now_unix(),
    })
}

/// 实时 watcher 事件成功写入后同步单个路径的 metadata 快照。
pub fn sync_live_state(
    db: &Database,
    event_type: &str,
    path: &Path,
    old_path: Option<&Path>,
) -> DbResult<()> {
    let repo = WorkspaceFileStateRepo::new(db.conn());
    if let Some(old) = old_path {
        if let Some(ws) = repo.workspace_for_path(&old.to_string_lossy())? {
            repo.delete(ws.id, &old.to_string_lossy())?;
        }
    }
    let Some(ws) = repo.workspace_for_path(&path.to_string_lossy())? else {
        return Ok(());
    };
    if event_type == "file.deleted" {
        return repo.delete(ws.id, &path.to_string_lossy());
    }
    match std::fs::metadata(path) {
        Ok(meta) if meta.is_file() => repo.upsert(
            ws.id,
            &path.to_string_lossy(),
            modified_seconds(&meta),
            meta.len(),
        ),
        Ok(_) => Ok(()),
        Err(err) => {
            eprintln!(
                "[workspace] live metadata failed for {}: {err}",
                path.display()
            );
            repo.delete(ws.id, &path.to_string_lossy())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::activity::ActivityRepo;
    use crate::db::workspace::WorkspaceRepo;

    fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "msl-inventory-{tag}-{}-{}",
            std::process::id(),
            now_unix()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn baseline_and_offline_reconcile_are_quiet_and_repeatable() {
        let root = temp_dir("reconcile");
        std::fs::write(root.join("keep.md"), "v1").unwrap();
        std::fs::write(root.join("~$temp.docx"), "temp").unwrap();
        let db = Database::open_in_memory().unwrap();
        let ws = WorkspaceRepo::new(db.conn())
            .insert("test", root.to_string_lossy().as_ref())
            .unwrap();
        let first = reconcile(&db, ws.id, &root).unwrap();
        assert!(first.first_scan);
        assert_eq!(first.created, 0);
        assert_eq!(
            ActivityRepo::new(db.conn())
                .query(None, None, None, Some("file.created"), None, None)
                .unwrap()
                .len(),
            0
        );

        std::fs::write(root.join("keep.md"), "v2-longer").unwrap();
        std::fs::write(root.join("new.md"), "new").unwrap();
        std::fs::remove_file(root.join("keep.md")).unwrap();
        let second = reconcile(&db, ws.id, &root).unwrap();
        assert_eq!(second.created, 1);
        assert_eq!(second.deleted, 1);
        assert_eq!(second.modified, 0);
        let third = reconcile(&db, ws.id, &root).unwrap();
        assert_eq!(third.created + third.modified + third.deleted, 0);
        let events = ActivityRepo::new(db.conn())
            .query(None, None, None, None, None, None)
            .unwrap();
        assert_eq!(
            events
                .iter()
                .filter(|e| e.event_type == "workspace.baseline")
                .count(),
            1
        );
        assert_eq!(
            events
                .iter()
                .filter(|e| e.event_type == "file.created")
                .count(),
            1
        );
        assert_eq!(
            events
                .iter()
                .filter(|e| e.event_type == "file.deleted")
                .count(),
            1
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
