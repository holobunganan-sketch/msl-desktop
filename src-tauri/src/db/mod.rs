//! 数据库层核心（指南 §2.3 / §16）。
//!
//! 职责：连接管理、WAL、migration、错误类型、默认路径。
//! 数据文件默认位于 `%APPDATA%\MSLDesktop\msl-desktop.db`。
//! 本模块不依赖 Tauri，便于单元测试（临时文件 / 内存库）。

pub mod activity;
pub mod ai;
pub mod ai_documents;
pub mod brief;
pub mod calendar;
pub mod documents;
pub mod flow;
pub mod inbox;
pub mod jobs;
pub mod knowledge;
pub mod kol;
pub mod memory;
pub mod provider;
pub mod qa;
pub mod reports;
pub mod source_lifecycle;
pub mod task;
pub mod work;
pub mod workspace;

mod migrations;

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::Connection;

/// 数据库文件名（指南 §2.3）。
pub const DB_FILE_NAME: &str = "msl-desktop.db";
/// 应用数据目录名。
pub const APP_DATA_DIR_NAME: &str = "MSLDesktop";

pub fn latest_schema_version() -> i64 {
    migrations::MIGRATIONS
        .last()
        .map(|m| m.version)
        .unwrap_or(0)
}

/// 当前 Unix 时间戳（秒）。数据层统一使用 UTC 秒。
pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn resolve_data_directory(appdata: Option<std::ffi::OsString>) -> Result<PathBuf, String> {
    let base = appdata
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .ok_or("Windows 用户数据目录不可用，已停止初始化，避免在程序目录创建个人数据")?;
    Ok(base.join(APP_DATA_DIR_NAME))
}

/// Application data is always outside the executable's working directory.
pub fn default_app_data_dir() -> PathBuf {
    resolve_data_directory(std::env::var_os("APPDATA"))
        .expect("User data path must be validated at startup")
}

/// 默认数据库路径：`%APPDATA%\MSLDesktop\msl-desktop.db`。
pub fn default_db_path() -> PathBuf {
    default_app_data_dir().join(DB_FILE_NAME)
}

/// 数据库错误：统一包装 IO / SQLite / migration / NotFound 错误。
#[derive(Debug)]
pub enum DbError {
    Io(std::io::Error),
    Sqlite(rusqlite::Error),
    Migration(String),
    NotFound(String),
}

impl fmt::Display for DbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DbError::Io(e) => write!(f, "io error: {e}"),
            DbError::Sqlite(e) => write!(f, "sqlite error: {e}"),
            DbError::Migration(msg) => write!(f, "migration error: {msg}"),
            DbError::NotFound(what) => write!(f, "not found: {what}"),
        }
    }
}

impl std::error::Error for DbError {}

impl From<std::io::Error> for DbError {
    fn from(e: std::io::Error) -> Self {
        DbError::Io(e)
    }
}

impl From<rusqlite::Error> for DbError {
    fn from(e: rusqlite::Error) -> Self {
        DbError::Sqlite(e)
    }
}

pub type DbResult<T> = Result<T, DbError>;

pub fn write_transaction(conn: &Connection) -> DbResult<rusqlite::Transaction<'_>> {
    // Reserve the writer before reading: a deferred WAL transaction cannot wait
    // when another writer invalidates its read snapshot before the first write.
    Ok(rusqlite::Transaction::new_unchecked(
        conn,
        rusqlite::TransactionBehavior::Immediate,
    )?)
}

/// SQLite 连接封装。
pub struct Database {
    conn: Connection,
}

impl Database {
    /// 打开（或创建）数据库文件并执行 migration。
    /// 父目录不存在时自动创建。
    pub fn open(path: &Path) -> DbResult<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let conn = Connection::open(path)?;
        let mut db = Self { conn };
        db.configure()?;
        db.migrate()?;
        Ok(db)
    }

    /// 打开内存数据库（仅用于测试）。
    pub fn open_in_memory() -> DbResult<Self> {
        let conn = Connection::open_in_memory()?;
        let mut db = Self { conn };
        db.configure()?;
        db.migrate()?;
        Ok(db)
    }

    /// 基础 PRAGMA：WAL、外键、busy timeout。
    fn configure(&self) -> DbResult<()> {
        self.conn.pragma_update(None, "journal_mode", "WAL")?;
        self.conn.pragma_update(None, "foreign_keys", "ON")?;
        self.conn.busy_timeout(Duration::from_secs(5))?;
        Ok(())
    }

    /// 执行所有未应用的 migration（幂等）。
    pub fn migrate(&mut self) -> DbResult<()> {
        migrations::run(&mut self.conn)
    }

    /// 只读访问底层连接（repository 使用）。
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// 应用退出前调用：WAL checkpoint（TRUNCATE）后关闭连接。
    pub fn close(self) -> DbResult<()> {
        // TRUNCATE 将 WAL 内容合并回主库并截断 WAL，保证退出时数据完整。
        self.conn
            .pragma_update(None, "wal_checkpoint", "TRUNCATE")?;
        drop(self.conn);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knowledge_schema_preserves_durable_conversations_and_kol_records() {
        let db = Database::open_in_memory().unwrap();
        let found: i64 = db.conn().query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('qa_sessions','qa_turns','kol_experts','kol_notes','kol_drafts','kol_insights','kol_actions','kol_projects')", [], |r| r.get(0)).unwrap();
        assert_eq!(found, 8, "knowledge persistence is unavailable");
    }

    #[test]
    fn write_transaction_reserves_writer_before_reading_shared_state() {
        let root = std::env::temp_dir().join(format!("msl-wal-race-{}", uuid::Uuid::new_v4()));
        let db = Database::open(&root.join("test.db")).unwrap();
        let other = Database::open(&root.join("test.db")).unwrap();
        other
            .conn()
            .busy_timeout(Duration::from_millis(25))
            .unwrap();
        let tx = write_transaction(db.conn()).unwrap();
        tx.query_row("SELECT COUNT(*) FROM workspaces", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap();
        let competing=other.conn().execute("INSERT INTO workspaces(name,root_path,created_at,updated_at) VALUES ('other','C:/synthetic',0,0)",[]);
        assert!(competing.is_err(),"writer was not reserved before the read; later upgrade can fail with SQLITE_BUSY_SNAPSHOT");
        tx.execute("INSERT INTO workspaces(name,root_path,created_at,updated_at) VALUES ('first','C:/synthetic',0,0)",[]).unwrap();
        tx.commit().unwrap();
        other.close().unwrap();
        db.close().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    /// 生成唯一的临时数据库路径（测试用，用后删除）。
    fn temp_db_path(tag: &str) -> PathBuf {
        let unique = format!("msl-db-test-{tag}-{}-{}", std::process::id(), now_unix());
        std::env::temp_dir().join(unique).join(DB_FILE_NAME)
    }

    #[test]
    fn fresh_db_auto_creates_and_migrates() {
        let path = temp_db_path("fresh");
        let db = Database::open(&path).unwrap();

        // schema_migrations 已应用到最新版本
        let version: i64 = db
            .conn()
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(version, 23);

        // 业务表包含 Provider catalog、文档智能、AI secretary 与周期报告。
        let table_count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' AND name != 'schema_migrations'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(table_count, 67); // Includes secretary round checkpoints and accepted-advice outcomes.

        // WAL 已启用
        let journal: String = db
            .conn()
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(journal.to_ascii_lowercase(), "wal");

        db.close().unwrap();
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn reopen_preserves_data() {
        let path = temp_db_path("reopen");
        {
            let db = Database::open(&path).unwrap();
            let ws = workspace::WorkspaceRepo::new(db.conn())
                .insert("主工作目录", "C:\\Work")
                .unwrap();
            let work = work::WorkRepo::new(db.conn())
                .insert("老年破伤风 IIT", "active")
                .unwrap();
            activity::ActivityRepo::new(db.conn())
                .insert(
                    "task.completed",
                    Some(ws.id),
                    Some(work.id),
                    Some("task"),
                    Some(9),
                    None,
                    "完成 核对入排标准",
                    None,
                    None,
                )
                .unwrap();
            db.close().unwrap();
        }
        // 重新打开：数据仍在
        {
            let db = Database::open(&path).unwrap();
            let works = workspace::WorkspaceRepo::new(db.conn()).list().unwrap();
            assert_eq!(works.len(), 1);
            assert_eq!(works[0].root_path, "C:\\Work");

            let acts = activity::ActivityRepo::new(db.conn())
                .query(None, None, None, None, None, None)
                .unwrap();
            assert_eq!(acts.len(), 1);
            assert_eq!(acts[0].display_text, "完成 核对入排标准");
            db.close().unwrap();
        }
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn migration_is_idempotent() {
        let mut db = Database::open_in_memory().unwrap();
        db.migrate().unwrap();
        db.migrate().unwrap();
        let version: i64 = db
            .conn()
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(version, 23);
        // Each registered migration is recorded exactly once.
        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 23);
    }

    #[test]
    fn migration_v1_to_v3_preserves_provider_and_adds_schema() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../../migrations/0001_init.sql"))
            .unwrap();
        conn.execute_batch(
            "CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, name TEXT NOT NULL, applied_at INTEGER NOT NULL);
             INSERT INTO schema_migrations VALUES (1, 'init', 0);
             INSERT INTO provider_settings (display_name, provider_type, base_url, model, enabled, created_at, updated_at)
             VALUES ('legacy', 'openai_compatible', 'http://localhost', 'local', 1, 1, 1);",
        )
        .unwrap();

        migrations::run(&mut conn).unwrap();
        assert_eq!(migrations::current_version(&conn).unwrap(), 23);
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM provider_settings", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            conn.query_row(
                "SELECT credential_ref FROM provider_settings WHERE id = 1",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "provider-1"
        );
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM workspace_file_state", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM provider_models", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        let columns: Vec<String> = conn
            .prepare("PRAGMA table_info(daily_briefs)")
            .unwrap()
            .query_map([], |r| r.get(1))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert!(columns.iter().any(|c| c == "source_snapshot_json"));
    }

    #[test]
    fn migration_v2_to_v3_preserves_provider_and_migrates_model_catalog() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../../migrations/0001_init.sql"))
            .unwrap();
        conn.execute_batch(include_str!(
            "../../migrations/0002_workbench_reliability.sql"
        ))
        .unwrap();
        conn.execute_batch(
            "CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, name TEXT NOT NULL, applied_at INTEGER NOT NULL);
             INSERT INTO schema_migrations VALUES (1, 'init', 0);
             INSERT INTO schema_migrations VALUES (2, 'workbench_reliability', 0);
             INSERT INTO provider_settings (display_name, provider_type, base_url, model, enabled, credential_ref, created_at, updated_at)
             VALUES ('legacy', 'openai_compatible', 'https://api.deepseek.com/', 'deepseek-v4-flash', 1, 'provider-test', 1, 1);",
        )
        .unwrap();

        migrations::run(&mut conn).unwrap();
        assert_eq!(migrations::current_version(&conn).unwrap(), 23);
        assert_eq!(
            conn.query_row(
                "SELECT template_kind FROM provider_settings WHERE id = 1",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "deepseek"
        );
        assert_eq!(
            conn.query_row(
                "SELECT credential_ref FROM provider_settings WHERE id = 1",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "provider-test"
        );
        assert_eq!(
            conn.query_row(
                "SELECT protocol || '|' || endpoint_path || '|' || source FROM provider_models WHERE provider_id = 1 AND model_id = 'deepseek-v4-flash'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "chat_completions|/chat/completions|legacy"
        );

        migrations::run(&mut conn).unwrap();
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM provider_models", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        conn.execute(
            "INSERT INTO ai_task_routes (task_kind, provider_model_id, updated_at) VALUES ('general', 1, 0)",
            [],
        )
        .unwrap();
        conn.execute("DELETE FROM provider_settings WHERE id = 1", [])
            .unwrap();
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM provider_models", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            conn.query_row(
                "SELECT provider_model_id FROM ai_task_routes WHERE task_kind = 'general'",
                [],
                |r| r.get::<_, Option<i64>>(0)
            )
            .unwrap(),
            None
        );
    }

    #[test]
    fn migration_v3_to_v4_adds_document_index_without_body_columns() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../../migrations/0001_init.sql"))
            .unwrap();
        conn.execute_batch(include_str!(
            "../../migrations/0002_workbench_reliability.sql"
        ))
        .unwrap();
        conn.execute_batch(include_str!(
            "../../migrations/0003_ai_provider_catalog.sql"
        ))
        .unwrap();
        conn.execute_batch(
            "CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, name TEXT NOT NULL, applied_at INTEGER NOT NULL);
             INSERT INTO schema_migrations VALUES (1, 'init', 0);
             INSERT INTO schema_migrations VALUES (2, 'workbench_reliability', 0);
             INSERT INTO schema_migrations VALUES (3, 'ai_provider_catalog', 0);
             INSERT INTO workspaces (name, root_path, created_at, updated_at) VALUES ('w', 'C:/w', 0, 0);
             INSERT INTO works (title, status, created_at, updated_at) VALUES ('work', 'active', 0, 0);",
        ).unwrap();
        migrations::run(&mut conn).unwrap();
        assert_eq!(migrations::current_version(&conn).unwrap(), 23);
        for table in ["work_workspace_links", "document_index", "cache_entries"] {
            assert_eq!(
                conn.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [table],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                1
            );
        }
        let columns: Vec<String> = conn
            .prepare("PRAGMA table_info(document_index)")
            .unwrap()
            .query_map([], |r| r.get(1))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert!(!columns
            .iter()
            .any(|c| ["full_content", "raw_body", "prompt_body"].contains(&c.as_str())));
        conn.execute("INSERT INTO work_workspace_links (work_id, workspace_id, is_primary, created_at) VALUES (1, 1, 1, 0)", []).unwrap();
        conn.execute("INSERT INTO document_index (workspace_id, path, relative_path, extension, size, modified_at) VALUES (1, 'C:/w/a.txt', 'a.txt', 'txt', 1, 0)", []).unwrap();
        conn.execute("INSERT INTO cache_entries (category, relative_path, size_bytes, created_at, last_accessed_at) VALUES ('extracted', 'extracted/x', 1, 0, 0)", []).unwrap();
        assert!(conn.execute("INSERT INTO document_index (workspace_id, path, relative_path, extension, size, modified_at) VALUES (1, 'C:/w/a.txt', 'a.txt', 'txt', 1, 0)", []).is_err());
    }

    #[test]
    fn migration_v4_to_v5_adds_secretary_defaults_and_keeps_briefs() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../../migrations/0001_init.sql"))
            .unwrap();
        conn.execute_batch(include_str!(
            "../../migrations/0002_workbench_reliability.sql"
        ))
        .unwrap();
        conn.execute_batch(include_str!(
            "../../migrations/0003_ai_provider_catalog.sql"
        ))
        .unwrap();
        conn.execute_batch(include_str!(
            "../../migrations/0004_document_intelligence.sql"
        ))
        .unwrap();
        conn.execute_batch("CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, name TEXT NOT NULL, applied_at INTEGER NOT NULL); INSERT INTO schema_migrations VALUES (1,'init',0),(2,'workbench_reliability',0),(3,'ai_provider_catalog',0),(4,'document_intelligence',0); INSERT INTO daily_briefs (brief_date,generated_at,content) VALUES ('2026-08-14',0,'kept brief');").unwrap();
        migrations::run(&mut conn).unwrap();
        assert_eq!(migrations::current_version(&conn).unwrap(), 23);
        let schedule: (i64, i64, i64, i64) = conn.query_row("SELECT enabled, interval_minutes, daily_hour, daily_minute FROM analysis_schedule_state WHERE id=1", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).unwrap();
        assert_eq!(schedule, (1, 180, 6, 0));
        assert_eq!(
            conn.query_row(
                "SELECT retention_state FROM daily_briefs LIMIT 1",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "kept"
        );
        for table in ["analysis_runs", "ai_proposals"] {
            assert_eq!(
                conn.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [table],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                1
            );
        }
        conn.execute_batch("INSERT INTO ai_proposals (kind,operation,dedupe_key,title,payload_json,created_at,updated_at) VALUES ('task','create','same','a','{}',0,0);").unwrap();
        assert!(conn.execute("INSERT INTO ai_proposals (kind,operation,dedupe_key,title,payload_json,created_at,updated_at) VALUES ('task','create','same','b','{}',0,0)", []).is_err());
        migrations::run(&mut conn).unwrap();
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            23
        );
    }

    #[test]
    fn migration_v5_to_v6_preserves_existing_storage_setting() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../../migrations/0001_init.sql"))
            .unwrap();
        conn.execute_batch(include_str!(
            "../../migrations/0002_workbench_reliability.sql"
        ))
        .unwrap();
        conn.execute_batch(include_str!(
            "../../migrations/0003_ai_provider_catalog.sql"
        ))
        .unwrap();
        conn.execute_batch(include_str!(
            "../../migrations/0004_document_intelligence.sql"
        ))
        .unwrap();
        conn.execute_batch(include_str!("../../migrations/0005_ai_secretary.sql"))
            .unwrap();
        conn.execute_batch("CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, name TEXT NOT NULL, applied_at INTEGER NOT NULL); INSERT INTO schema_migrations VALUES (1,'init',0),(2,'workbench_reliability',0),(3,'ai_provider_catalog',0),(4,'document_intelligence',0),(5,'ai_secretary',0); INSERT OR REPLACE INTO app_settings(key,value,updated_at) VALUES ('cache_limit_bytes','123',1);").unwrap();
        migrations::run(&mut conn).unwrap();
        assert_eq!(migrations::current_version(&conn).unwrap(), 23);
        assert_eq!(
            conn.query_row(
                "SELECT value FROM app_settings WHERE key='cache_limit_bytes'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "123"
        );
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='daily_activity_rollups'", [], |r| r.get::<_, i64>(0)).unwrap(), 1);
    }

    #[test]
    fn migration_v7_to_v8_preserves_proposals_and_initializes_memory_schema() {
        let mut conn = Connection::open_in_memory().unwrap();
        for sql in [
            include_str!("../../migrations/0001_init.sql"),
            include_str!("../../migrations/0002_workbench_reliability.sql"),
            include_str!("../../migrations/0003_ai_provider_catalog.sql"),
            include_str!("../../migrations/0004_document_intelligence.sql"),
            include_str!("../../migrations/0005_ai_secretary.sql"),
            include_str!("../../migrations/0006_storage_governance.sql"),
            include_str!("../../migrations/0007_decisions_reports.sql"),
        ] {
            conn.execute_batch(sql).unwrap();
        }
        conn.execute_batch(
            "CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, name TEXT NOT NULL, applied_at INTEGER NOT NULL);
             INSERT INTO schema_migrations VALUES
               (1,'init',0),(2,'workbench_reliability',0),(3,'ai_provider_catalog',0),
               (4,'document_intelligence',0),(5,'ai_secretary',0),
               (6,'storage_governance',0),(7,'decisions_reports',0);
             INSERT INTO ai_proposals
               (kind,operation,dedupe_key,title,payload_json,created_at,updated_at)
             VALUES ('task','create','kept-proposal','保留建议','{}',1,1);",
        )
        .unwrap();

        migrations::run(&mut conn).unwrap();

        assert_eq!(migrations::current_version(&conn).unwrap(), 23);
        assert_eq!(
            conn.query_row(
                "SELECT kind || '|' || suggested_kind FROM ai_proposals WHERE dedupe_key='kept-proposal'",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
            "task|task"
        );
        assert_eq!(
            conn.query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='classification_memories'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
            1
        );
    }

    #[test]
    fn default_path_uses_appdata() {
        let p = default_db_path();
        assert!(p.to_string_lossy().contains("MSLDesktop"));
        assert_eq!(p.file_name().unwrap().to_str().unwrap(), DB_FILE_NAME);
    }

    #[test]
    fn migration_v22_backfills_explicit_advice_targets_without_losing_existing_work() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        conn.execute_batch("CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY,name TEXT NOT NULL,applied_at INTEGER NOT NULL);").unwrap();
        for migration in migrations::MIGRATIONS.iter().filter(|m| m.version <= 22) {
            conn.execute_batch(migration.sql).unwrap();
            conn.execute(
                "INSERT INTO schema_migrations VALUES(?1,?2,0)",
                rusqlite::params![migration.version, migration.name],
            )
            .unwrap();
        }
        assert_eq!(migrations::current_version(&conn).unwrap(), 22);
        conn.execute_batch(r#"
            INSERT INTO works(id,title,status,summary,created_at,updated_at) VALUES(10,'Synthetic preserved project','active','Original summary',1,2);
            INSERT INTO tasks(id,work_id,title,status,priority,notes,created_at,updated_at,completed_at) VALUES
              (70,10,'Matching title does not identify a target','next','high','Preserve this task',3,4,NULL),
              (71,10,'Completed synthetic task','done','normal','Preserve completed task',5,6,6);
            INSERT INTO ai_proposals(id,kind,operation,work_id,dedupe_key,title,payload_json,status,created_at,updated_at) VALUES
              (90,'task','create',10,'explicit-done','Matching title does not identify a target','{}','confirmed',7,8),
              (91,'task','create',10,'unidentified','Matching title does not identify a target','{}','confirmed',7,8),
              (92,'task','create',10,'pending','Pending suggestion','{}','pending',7,8),
              (93,'task','create',10,'malformed','Malformed old metadata','{}','confirmed',7,8),
              (94,'task','create',10,'explicit-open','Still in progress','{}','confirmed',7,8);
            INSERT INTO activity_events(timestamp,event_type,work_id,entity_type,entity_id,metadata_json) VALUES
              (9,'ai.proposal.confirmed',10,'proposal',90,'{"kind":"task","target_id":71}'),
              (9,'ai.proposal.confirmed',10,'proposal',92,'{"kind":"task","target_id":70}'),
              (9,'ai.proposal.confirmed',10,'proposal',93,'not-json'),
              (9,'ai.proposal.confirmed',10,'proposal',94,'{"kind":"task","target_id":71}'),
              (10,'ai.proposal.confirmed',10,'proposal',94,'{"kind":"task","target_id":70}');
        "#).unwrap();
        let read_rows = |sql: &str| -> Vec<String> {
            conn.prepare(sql)
                .unwrap()
                .query_map([], |row| row.get(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        };
        let work_sql = "SELECT json_array(id,title,status,summary,created_at,updated_at,archived_at,revision) FROM works ORDER BY id";
        let task_sql = "SELECT json_array(id,work_id,title,status,priority,due_at,scheduled_start,scheduled_end,notes,created_at,updated_at,completed_at) FROM tasks ORDER BY id";
        let works_before = read_rows(work_sql);
        let tasks_before = read_rows(task_sql);
        let events_before: i64 = conn
            .query_row("SELECT COUNT(*) FROM activity_events", [], |r| r.get(0))
            .unwrap();
        migrations::run(&mut conn).unwrap();
        let outcomes: Vec<(i64, String, i64)> = conn
            .prepare(
                "SELECT proposal_id,kind,target_id FROM ai_proposal_outcomes ORDER BY proposal_id",
            )
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(
            outcomes,
            vec![(90, "task".into(), 71), (94, "task".into(), 70)]
        );
        assert_eq!(
            ai::ProposalRepo::new(&conn)
                .get(90)
                .unwrap()
                .unwrap()
                .status,
            "resolved"
        );
        assert_eq!(
            ai::ProposalRepo::new(&conn)
                .get(94)
                .unwrap()
                .unwrap()
                .status,
            "confirmed"
        );
        assert!(
            ai::ProposalRepo::new(&conn)
                .get(91)
                .unwrap()
                .unwrap()
                .applied_id
                .is_none(),
            "a matching title must never invent a legacy relationship"
        );
        for _ in 0..2 {
            migrations::run(&mut conn).unwrap();
        }
        let read_rows = |sql: &str| -> Vec<String> {
            conn.prepare(sql)
                .unwrap()
                .query_map([], |row| row.get(0))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        };
        assert_eq!(read_rows(work_sql), works_before);
        assert_eq!(read_rows(task_sql), tasks_before);
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM activity_events", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            events_before
        );
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM ai_proposals", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            5
        );
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM ai_proposal_outcomes", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            2
        );
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            23
        );
        let violations: i64 = conn
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(violations, 0);
    }

    #[test]
    fn foreign_keys_enforced() {
        let db = Database::open_in_memory().unwrap();
        // 插入不存在的 work_id 的 resume_point 应失败
        let err = db
            .conn()
            .execute(
                "INSERT INTO resume_points (work_id, current_state, next_step, remember, source, created_at)
                 VALUES (999, '', '', '', 'manual', 0)",
                [],
            )
            .unwrap_err();
        assert!(err.to_string().contains("FOREIGN KEY"));
    }

    /// Release audit only: the caller must provide a disposable copy, never the formal path.
    #[test]
    #[ignore]
    fn disposable_formal_database_copy_migrates_without_entity_loss() {
        let path = std::env::var("MSL_FORMAL_DB_COPY")
            .map(PathBuf::from)
            .expect("MSL_FORMAL_DB_COPY must point to a disposable database copy");
        let canonical = path.canonicalize().expect("database copy must exist");
        assert!(canonical
            .to_string_lossy()
            .contains("formal-migration-copy"));
        let before = Connection::open(&canonical).unwrap();
        let counts_before = [
            "works",
            "tasks",
            "waiting_items",
            "calendar_events",
            "inbox_items",
        ]
        .map(|table| {
            before
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap()
        });
        drop(before);
        let db = Database::open(&canonical).unwrap();
        assert_eq!(migrations::current_version(db.conn()).unwrap(), 23);
        let counts_after = [
            "works",
            "tasks",
            "waiting_items",
            "calendar_events",
            "inbox_items",
        ]
        .map(|table| {
            db.conn()
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap()
        });
        assert_eq!(counts_after, counts_before);
        assert_eq!(
            db.conn()
                .query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "ok"
        );
        assert!(knowledge::rows(db.conn(), "PRAGMA foreign_key_check", &[])
            .unwrap()
            .is_empty());
        drop(db);
        let db = Database::open(&canonical).unwrap();
        assert_eq!(migrations::current_version(db.conn()).unwrap(), 23);
        for table in ["reports", "report_schedule_state"] {
            assert_eq!(
                db.conn()
                    .query_row(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                        [table],
                        |row| row.get::<_, i64>(0)
                    )
                    .unwrap(),
                1
            );
        }
    }
}
