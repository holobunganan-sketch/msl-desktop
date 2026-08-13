//! 显式 SQL migration（指南 §2.3 / §16）。
//!
//! 机制：
//! - `schema_migrations` 表记录已应用的版本；
//! - 每个 migration 在一个事务中执行（SQL + 版本记录）；
//! - 重复执行安全：已应用版本自动跳过；
//! - 每次 schema 变化必须追加新 migration 文件，并登记到 `MIGRATIONS`。

use rusqlite::{Connection, OptionalExtension};

use super::{DbError, DbResult, now_unix};

pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

/// 迁移列表（按 version 升序）。
/// 新增 schema 变化时：新建 `migrations/NNNN_name.sql` 并在此登记。
pub const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "init",
    sql: include_str!("../../migrations/0001_init.sql"),
}];

/// 执行所有未应用的迁移（幂等、事务化）。
pub fn run(conn: &mut Connection) -> DbResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
           version    INTEGER PRIMARY KEY,
           name       TEXT NOT NULL,
           applied_at INTEGER NOT NULL
         );",
    )?;

    for m in MIGRATIONS {
        let applied: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
                [m.version],
                |row| row.get(0),
            )
            .optional()?
            .unwrap_or(false);

        if applied {
            continue;
        }

        // 单事务执行迁移 SQL 与版本记录
        let tx = conn.transaction().map_err(DbError::from)?;
        tx.execute_batch(m.sql).map_err(DbError::from)?;
        tx.execute(
            "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![m.version, m.name, now_unix()],
        )
        .map_err(DbError::from)?;
        tx.commit().map_err(DbError::from)?;
    }

    Ok(())
}

/// 已应用的最高版本（测试/诊断用）。
#[allow(dead_code)]
pub fn current_version(conn: &Connection) -> DbResult<i64> {
    conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |row| row.get(0),
    )
    .map_err(DbError::from)
}
