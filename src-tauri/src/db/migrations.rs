//! 显式 SQL migration（指南 §2.3 / §16）。
//!
//! 机制：
//! - `schema_migrations` 表记录已应用的版本；
//! - 每个 migration 在一个事务中执行（SQL + 版本记录）；
//! - 重复执行安全：已应用版本自动跳过；
//! - 每次 schema 变化必须追加新 migration 文件，并登记到 `MIGRATIONS`。

use rusqlite::{Connection, OptionalExtension};

use super::{now_unix, DbError, DbResult};

pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

/// 迁移列表（按 version 升序）。
/// 新增 schema 变化时：新建 `migrations/NNNN_name.sql` 并在此登记。
pub const MIGRATIONS: &[Migration] = &[
    // Entries are executed in their declared order; append new versions below.
    Migration {
        version: 1,
        name: "init",
        sql: include_str!("../../migrations/0001_init.sql"),
    },
    Migration {
        version: 2,
        name: "workbench_reliability",
        sql: include_str!("../../migrations/0002_workbench_reliability.sql"),
    },
    Migration {
        version: 3,
        name: "ai_provider_catalog",
        sql: include_str!("../../migrations/0003_ai_provider_catalog.sql"),
    },
    Migration {
        version: 4,
        name: "document_intelligence",
        sql: include_str!("../../migrations/0004_document_intelligence.sql"),
    },
    Migration {
        version: 5,
        name: "ai_secretary",
        sql: include_str!("../../migrations/0005_ai_secretary.sql"),
    },
    Migration {
        version: 6,
        name: "storage_governance",
        sql: include_str!("../../migrations/0006_storage_governance.sql"),
    },
    Migration {
        version: 7,
        name: "decisions_reports",
        sql: include_str!("../../migrations/0007_decisions_reports.sql"),
    },
    Migration {
        version: 8,
        name: "classification_memory",
        sql: include_str!("../../migrations/0008_classification_memory.sql"),
    },
    Migration {
        version: 9,
        name: "background_jobs",
        sql: include_str!("../../migrations/0009_background_jobs.sql"),
    },
    Migration {
        version: 10,
        name: "cognition_flow",
        sql: include_str!("../../migrations/0010_cognition_flow.sql"),
    },
    Migration {
        version: 11,
        name: "capture_context",
        sql: include_str!("../../migrations/0011_capture_context.sql"),
    },
    Migration {
        version: 12,
        name: "ai_efficiency",
        sql: include_str!("../../migrations/0012_ai_efficiency.sql"),
    },
    Migration {
        version: 13,
        name: "workspace_lifecycle",
        sql: include_str!("../../migrations/0013_workspace_lifecycle.sql"),
    },
    Migration {
        version: 14,
        name: "knowledge_kol",
        sql: include_str!("../../migrations/0014_knowledge_kol.sql"),
    },
    Migration {
        version: 15,
        name: "expert_department",
        sql: include_str!("../../migrations/0015_expert_department.sql"),
    },
    Migration {
        version: 16,
        name: "safe_deletion",
        sql: include_str!("../../migrations/0016_safe_deletion.sql"),
    },
    Migration {
        version: 17,
        name: "expert_materials",
        sql: include_str!("../../migrations/0017_expert_materials.sql"),
    },
    Migration {
        version: 18,
        name: "sync_foundation",
        sql: include_str!("../../migrations/0018_sync_foundation.sql"),
    },
    Migration {
        version: 19,
        name: "sync_lifecycle",
        sql: include_str!("../../migrations/0019_sync_lifecycle.sql"),
    },
    Migration {
        version: 20,
        name: "ai_readable_documents",
        sql: include_str!("../../migrations/0020_ai_readable_documents.sql"),
    },
    Migration {
        version: 21,
        name: "sync_edit_capture",
        sql: include_str!("../../migrations/0021_sync_edit_capture.sql"),
    },
    Migration {
        version: 22,
        name: "secretary_rounds",
        sql: include_str!("../../migrations/0022_secretary_rounds.sql"),
    },
    Migration {
        version: 23,
        name: "proposal_outcomes",
        sql: include_str!("../../migrations/0023_proposal_outcomes.sql"),
    },
    Migration {
        version: 24,
        name: "recovery_evidence",
        sql: include_str!("../../migrations/0024_recovery_evidence.sql"),
    },
];

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

    crate::sync::rows::install_capture(conn)?;
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
