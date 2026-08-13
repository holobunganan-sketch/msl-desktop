//! 数据库层核心（指南 §2.3 / §16）。
//!
//! 职责：连接管理、WAL、migration、错误类型、默认路径。
//! 数据文件默认位于 `%APPDATA%\MSLDesktop\msl-desktop.db`。
//! 本模块不依赖 Tauri，便于单元测试（临时文件 / 内存库）。

pub mod activity;
pub mod brief;
pub mod calendar;
pub mod inbox;
pub mod provider;
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

/// 当前 Unix 时间戳（秒）。数据层统一使用 UTC 秒。
pub fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 应用数据目录：`%APPDATA%\MSLDesktop`（无 APPDATA 时回退到本地目录）。
pub fn default_app_data_dir() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        PathBuf::from(appdata).join(APP_DATA_DIR_NAME)
    } else {
        PathBuf::from(".").join("msl-desktop-data")
    }
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
        self.conn
            .pragma_update(None, "journal_mode", "WAL")?;
        self.conn
            .pragma_update(None, "foreign_keys", "ON")?;
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

    /// 生成唯一的临时数据库路径（测试用，用后删除）。
    fn temp_db_path(tag: &str) -> PathBuf {
        let unique = format!(
            "msl-db-test-{tag}-{}-{}",
            std::process::id(),
            now_unix()
        );
        std::env::temp_dir().join(unique).join(DB_FILE_NAME)
    }

    #[test]
    fn fresh_db_auto_creates_and_migrates() {
        let path = temp_db_path("fresh");
        let db = Database::open(&path).unwrap();

        // schema_migrations 已应用 1 条
        let version: i64 = db
            .conn()
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, 1);

        // 12 张表存在
        let table_count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' AND name != 'schema_migrations'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(table_count, 12);

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
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, 1);
        // 迁移记录只有 1 条
        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn default_path_uses_appdata() {
        let p = default_db_path();
        assert!(p.to_string_lossy().contains("MSLDesktop"));
        assert_eq!(p.file_name().unwrap().to_str().unwrap(), DB_FILE_NAME);
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
}
