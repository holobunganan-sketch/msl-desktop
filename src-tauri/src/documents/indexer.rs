//! 工作目录 document_index 增量索引。

use std::path::Path;

use crate::db::documents::{CacheEntryRepo, DocumentIndexRepo};
use crate::db::{Database, DbResult};
use crate::documents::extract_path;
use crate::storage::paths::write_cache_atomically;
use crate::workspace::inventory;

pub fn reindex_workspace(db: &Database, workspace_id: i64, root: &Path) -> DbResult<IndexReport> {
    crate::storage::paths::ensure_storage_disjoint(db)?;
    let snapshot = inventory::scan(root, workspace_id).map_err(crate::db::DbError::from)?;
    let docs = DocumentIndexRepo::new(db.conn());
    let cache = CacheEntryRepo::new(db.conn());
    let mut ready = 0_u64;
    let mut reused = 0_u64;
    let mut failed = 0_u64;
    let current: std::collections::HashSet<String> = snapshot
        .files
        .iter()
        .map(|file| file.path.clone())
        .collect();
    for file in snapshot.files {
        let extension = Path::new(&file.path)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if let Some(existing) = docs.get(workspace_id, &file.path)? {
            let cache_available = existing.extract_status != "ready"
                || existing
                    .cache_rel_path
                    .as_deref()
                    .is_some_and(|rel| crate::storage::paths::existing_cache_path(rel).is_ok());
            if existing.modified_at == file.modified_at
                && existing.size == file.size as i64
                && existing.extract_status != "pending"
                && cache_available
            {
                if let Some(cache_path) = existing.cache_rel_path.as_deref() {
                    cache.touch(cache_path)?;
                }
                reused += 1;
                continue;
            }
        }
        let extracted = extract_path(Path::new(&file.path));
        if extracted.status == crate::documents::ExtractStatus::Ready {
            let relative_cache = format!("extracted/{}.txt", extracted.content_hash);
            if let Ok(target) = write_cache_atomically(&relative_cache, extracted.text.as_bytes()) {
                cache.register(
                    "extracted",
                    &relative_cache,
                    Some(&extracted.content_hash),
                    target.metadata().map(|meta| meta.len()).unwrap_or(0) as i64,
                    Some("document"),
                    None,
                )?;
                docs.upsert(
                    workspace_id,
                    &file.path,
                    &file.relative_path,
                    &extension,
                    file.size as i64,
                    file.modified_at,
                    Some(&extracted.content_hash),
                    "ready",
                    extracted.char_count as i64,
                    Some(&relative_cache),
                    None,
                    None,
                )?;
                db.conn().execute("UPDATE document_index SET summary=?1,summary_hash=?2,summary_model_id=NULL WHERE workspace_id=?3 AND path=?4",
                    rusqlite::params![crate::cognition::bounded(&extracted.text,360),extracted.content_hash,workspace_id,file.path])?;
                ready += 1;
            } else {
                docs.upsert(
                    workspace_id,
                    &file.path,
                    &file.relative_path,
                    &extension,
                    file.size as i64,
                    file.modified_at,
                    None,
                    "failed_parse",
                    0,
                    None,
                    Some("cache_write"),
                    Some("无法写入正文缓存"),
                )?;
                failed += 1;
            }
        } else {
            docs.upsert(
                workspace_id,
                &file.path,
                &file.relative_path,
                &extension,
                file.size as i64,
                file.modified_at,
                (!extracted.content_hash.is_empty()).then_some(extracted.content_hash.as_str()),
                &serde_json::to_value(&extracted.status)
                    .unwrap_or_else(|_| serde_json::json!("failed_parse"))
                    .as_str()
                    .unwrap_or("failed_parse"),
                extracted.char_count as i64,
                None,
                Some("extract"),
                extracted.warnings.first().map(String::as_str),
            )?;
            failed += 1;
        }
    }
    for document in docs.list(workspace_id)? {
        if !current.contains(&document.path) {
            docs.delete(workspace_id, &document.path)?;
        }
    }
    Ok(IndexReport {
        workspace_id,
        scanned: current.len() as u64,
        ready,
        reused,
        failed,
        skipped: snapshot.skipped,
    })
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexReport {
    pub workspace_id: i64,
    pub scanned: u64,
    pub ready: u64,
    pub reused: u64,
    pub failed: u64,
    pub skipped: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::workspace::WorkspaceRepo;
    use crate::db::Database;

    #[test]
    fn unchanged_files_are_reused_and_deleted_files_leave_no_index() {
        let root = std::env::temp_dir().join(format!("msl-indexer-{}", uuid::Uuid::new_v4()));
        let local =
            std::env::temp_dir().join(format!("msl-indexer-local-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("note.txt"), "hello").unwrap();
        let _local_app_data = crate::storage::paths::LocalAppDataTestGuard::set(&local);
        let db = Database::open_in_memory().unwrap();
        let workspace = WorkspaceRepo::new(db.conn())
            .insert("test", root.to_string_lossy().as_ref())
            .unwrap();
        let first = reindex_workspace(&db, workspace.id, &root).unwrap();
        assert_eq!(first.ready, 1);
        let second = reindex_workspace(&db, workspace.id, &root).unwrap();
        assert_eq!(second.reused, 1);
        std::fs::remove_file(root.join("note.txt")).unwrap();
        reindex_workspace(&db, workspace.id, &root).unwrap();
        assert!(crate::db::documents::DocumentIndexRepo::new(db.conn())
            .list(workspace.id)
            .unwrap()
            .is_empty());
        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(local);
    }
}
