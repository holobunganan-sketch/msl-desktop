//! document_index 与 cache_entries repository；数据库只保存索引、摘要和相对缓存路径。

use rusqlite::{params, Connection, OptionalExtension, Row};

use super::{now_unix, DbError, DbResult};

#[derive(Debug, Clone, serde::Serialize)]
pub struct DocumentIndex {
    pub id: i64,
    pub workspace_id: i64,
    pub path: String,
    pub relative_path: String,
    pub extension: String,
    pub size: i64,
    pub modified_at: i64,
    pub content_hash: Option<String>,
    pub extract_status: String,
    pub char_count: i64,
    pub cache_rel_path: Option<String>,
    pub summary: Option<String>,
    pub summary_hash: Option<String>,
    pub summary_model_id: Option<i64>,
    pub last_extracted_at: Option<i64>,
    pub last_analyzed_at: Option<i64>,
    pub last_accessed_at: Option<i64>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

fn row_to_document(row: &Row) -> rusqlite::Result<DocumentIndex> {
    Ok(DocumentIndex {
        id: row.get(0)?,
        workspace_id: row.get(1)?,
        path: row.get(2)?,
        relative_path: row.get(3)?,
        extension: row.get(4)?,
        size: row.get(5)?,
        modified_at: row.get(6)?,
        content_hash: row.get(7)?,
        extract_status: row.get(8)?,
        char_count: row.get(9)?,
        cache_rel_path: row.get(10)?,
        summary: row.get(11)?,
        summary_hash: row.get(12)?,
        summary_model_id: row.get(13)?,
        last_extracted_at: row.get(14)?,
        last_analyzed_at: row.get(15)?,
        last_accessed_at: row.get(16)?,
        error_code: row.get(17)?,
        error_message: row.get(18)?,
    })
}

pub struct DocumentIndexRepo<'a> {
    conn: &'a Connection,
}

impl<'a> DocumentIndexRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
    fn select() -> &'static str {
        "SELECT id, workspace_id, path, relative_path, extension, size, modified_at, content_hash, extract_status, char_count, cache_rel_path, summary, summary_hash, summary_model_id, last_extracted_at, last_analyzed_at, last_accessed_at, error_code, error_message FROM document_index"
    }
    pub fn get(&self, workspace_id: i64, path: &str) -> DbResult<Option<DocumentIndex>> {
        self.conn
            .query_row(
                &format!("{} WHERE workspace_id=?1 AND path=?2", Self::select()),
                params![workspace_id, path],
                row_to_document,
            )
            .optional()
            .map_err(DbError::from)
    }
    pub fn list(&self, workspace_id: i64) -> DbResult<Vec<DocumentIndex>> {
        let mut stmt = self.conn.prepare(&format!(
            "{} WHERE workspace_id=?1 ORDER BY relative_path",
            Self::select()
        ))?;
        let rows = stmt.query_map([workspace_id], row_to_document)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }
    #[allow(clippy::too_many_arguments)]
    pub fn upsert(
        &self,
        workspace_id: i64,
        path: &str,
        relative_path: &str,
        extension: &str,
        size: i64,
        modified_at: i64,
        content_hash: Option<&str>,
        extract_status: &str,
        char_count: i64,
        cache_rel_path: Option<&str>,
        error_code: Option<&str>,
        error_message: Option<&str>,
    ) -> DbResult<()> {
        self.conn.execute(
            "INSERT INTO document_index (workspace_id,path,relative_path,extension,size,modified_at,content_hash,extract_status,char_count,cache_rel_path,last_accessed_at,error_code,error_message)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)
             ON CONFLICT(workspace_id,path) DO UPDATE SET relative_path=excluded.relative_path,extension=excluded.extension,size=excluded.size,modified_at=excluded.modified_at,content_hash=excluded.content_hash,extract_status=excluded.extract_status,char_count=excluded.char_count,cache_rel_path=excluded.cache_rel_path,last_accessed_at=excluded.last_accessed_at,error_code=excluded.error_code,error_message=excluded.error_message,last_extracted_at=CASE WHEN excluded.extract_status='ready' THEN excluded.last_accessed_at ELSE document_index.last_extracted_at END",
            params![workspace_id,path,relative_path,extension,size,modified_at,content_hash,extract_status,char_count,cache_rel_path,now_unix(),error_code,error_message.map(|value| value.chars().take(300).collect::<String>())]
        )?;
        Ok(())
    }
    pub fn delete(&self, workspace_id: i64, path: &str) -> DbResult<()> {
        self.conn.execute(
            "DELETE FROM document_index WHERE workspace_id=?1 AND path=?2",
            params![workspace_id, path],
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CacheEntry {
    pub id: i64,
    pub category: String,
    pub relative_path: String,
    pub content_hash: Option<String>,
    pub size_bytes: i64,
    pub rebuildable: bool,
    pub owner_type: Option<String>,
    pub owner_id: Option<i64>,
    pub created_at: i64,
    pub last_accessed_at: i64,
    pub expires_at: Option<i64>,
}

pub struct CacheEntryRepo<'a> {
    conn: &'a Connection,
}
impl<'a> CacheEntryRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
    pub fn register(
        &self,
        category: &str,
        relative_path: &str,
        content_hash: Option<&str>,
        size_bytes: i64,
        owner_type: Option<&str>,
        owner_id: Option<i64>,
    ) -> DbResult<()> {
        self.conn.execute("INSERT INTO cache_entries (category,relative_path,content_hash,size_bytes,rebuildable,owner_type,owner_id,created_at,last_accessed_at) VALUES (?1,?2,?3,?4,1,?5,?6,?7,?7) ON CONFLICT(relative_path) DO UPDATE SET content_hash=excluded.content_hash,size_bytes=excluded.size_bytes,owner_type=excluded.owner_type,owner_id=excluded.owner_id,last_accessed_at=excluded.last_accessed_at", params![category,relative_path,content_hash,size_bytes,owner_type,owner_id,now_unix()])?;
        Ok(())
    }
    pub fn touch(&self, relative_path: &str) -> DbResult<()> {
        self.conn.execute(
            "UPDATE cache_entries SET last_accessed_at=?1 WHERE relative_path=?2",
            params![now_unix(), relative_path],
        )?;
        Ok(())
    }
    pub fn list(&self) -> DbResult<Vec<CacheEntry>> {
        let mut stmt=self.conn.prepare("SELECT id,category,relative_path,content_hash,size_bytes,rebuildable,owner_type,owner_id,created_at,last_accessed_at,expires_at FROM cache_entries ORDER BY last_accessed_at ASC")?;
        let rows = stmt.query_map([], |r| {
            Ok(CacheEntry {
                id: r.get(0)?,
                category: r.get(1)?,
                relative_path: r.get(2)?,
                content_hash: r.get(3)?,
                size_bytes: r.get(4)?,
                rebuildable: r.get::<_, i64>(5)? != 0,
                owner_type: r.get(6)?,
                owner_id: r.get(7)?,
                created_at: r.get(8)?,
                last_accessed_at: r.get(9)?,
                expires_at: r.get(10)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }
    pub fn remove(&self, relative_path: &str) -> DbResult<()> {
        self.conn.execute(
            "DELETE FROM cache_entries WHERE relative_path=?1",
            [relative_path],
        )?;
        Ok(())
    }
}

pub struct WorkWorkspaceLinkRepo<'a> {
    conn: &'a Connection,
}
impl<'a> WorkWorkspaceLinkRepo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }
    pub fn link(&self, work_id: i64, workspace_id: i64, primary: bool) -> DbResult<()> {
        let tx = crate::db::write_transaction(self.conn)?;
        if primary {
            tx.execute(
                "UPDATE work_workspace_links SET is_primary=0 WHERE work_id=?1",
                [work_id],
            )?;
        }
        tx.execute("INSERT INTO work_workspace_links (work_id,workspace_id,is_primary,created_at) VALUES (?1,?2,?3,?4) ON CONFLICT(work_id,workspace_id) DO UPDATE SET is_primary=excluded.is_primary", params![work_id,workspace_id,primary as i64,now_unix()])?;
        tx.commit()?;
        Ok(())
    }
    pub fn unlink(&self, work_id: i64, workspace_id: i64) -> DbResult<()> {
        self.conn.execute(
            "DELETE FROM work_workspace_links WHERE work_id=?1 AND workspace_id=?2",
            params![work_id, workspace_id],
        )?;
        Ok(())
    }
    pub fn list_by_work(&self, work_id: i64) -> DbResult<Vec<i64>> {
        let mut stmt=self.conn.prepare("SELECT workspace_id FROM work_workspace_links WHERE work_id=?1 ORDER BY is_primary DESC, workspace_id")?;
        let rows = stmt.query_map([work_id], |r| r.get(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }
    pub fn list_by_workspace(&self, workspace_id: i64) -> DbResult<Vec<i64>> {
        let mut stmt=self.conn.prepare("SELECT work_id FROM work_workspace_links WHERE workspace_id=?1 ORDER BY is_primary DESC, work_id")?;
        let rows = stmt.query_map([workspace_id], |r| r.get(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(DbError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::work::WorkRepo;
    use crate::db::workspace::WorkspaceRepo;
    use crate::db::Database;

    #[test]
    fn work_workspace_link_primary_and_unlink_preserve_entities() {
        let db = Database::open_in_memory().unwrap();
        let workspace = WorkspaceRepo::new(db.conn())
            .insert("workspace", "C:/workspace")
            .unwrap();
        let work = WorkRepo::new(db.conn()).insert("work", "active").unwrap();
        let repo = WorkWorkspaceLinkRepo::new(db.conn());
        repo.link(work.id, workspace.id, true).unwrap();
        assert_eq!(repo.list_by_work(work.id).unwrap(), vec![workspace.id]);
        repo.unlink(work.id, workspace.id).unwrap();
        assert!(repo.list_by_work(work.id).unwrap().is_empty());
        assert!(WorkspaceRepo::new(db.conn())
            .get(workspace.id)
            .unwrap()
            .is_some());
        assert!(WorkRepo::new(db.conn()).get(work.id).unwrap().is_some());
    }
}
