use super::{now_unix, DbError, DbResult};
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, serde::Serialize)]
pub struct AiJob {
    pub id: i64,
    pub command: String,
    pub args: Value,
    pub status: String,
    pub result: Option<Value>,
    pub error: Option<String>,
    pub created_at: i64,
    pub finished_at: Option<i64>,
}
fn row(r: &Row) -> rusqlite::Result<AiJob> {
    let args: String = r.get(2)?;
    let result: Option<String> = r.get(4)?;
    Ok(AiJob {
        id: r.get(0)?,
        command: r.get(1)?,
        args: serde_json::from_str(&args).unwrap_or(Value::Null),
        status: r.get(3)?,
        result: result.and_then(|s| serde_json::from_str(&s).ok()),
        error: r.get(5)?,
        created_at: r.get(6)?,
        finished_at: r.get(7)?,
    })
}
const SELECT: &str =
    "SELECT id,command,args_json,status,result_json,error,created_at,finished_at FROM ai_jobs";
pub fn get(conn: &Connection, id: i64) -> DbResult<AiJob> {
    conn.query_row(&format!("{SELECT} WHERE id=?1"), [id], row)
        .map_err(DbError::from)
}
pub fn list(conn: &Connection) -> DbResult<Vec<AiJob>> {
    let mut stmt = conn.prepare(&format!(
        "{SELECT} ORDER BY (status='running') DESC,id DESC LIMIT 60"
    ))?;
    let rows = stmt.query_map([], row)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(DbError::from)
}
pub fn start(conn: &Connection, command: &str, args: &Value) -> DbResult<(AiJob, bool)> {
    let serialized = args.to_string();
    if serialized.len() > 100_000 {
        return Err(DbError::Migration("后台任务输入过长".into()));
    }
    let key = format!(
        "{:x}",
        Sha256::digest(format!("{command}:{serialized}").as_bytes())
    );
    let tx = crate::db::write_transaction(conn)?;
    if let Some(id) = tx
        .query_row(
            "SELECT id FROM ai_jobs WHERE request_key=?1 AND status='running'",
            [&key],
            |r| r.get::<_, i64>(0),
        )
        .optional()?
    {
        let job = get(&tx, id)?;
        tx.commit()?;
        return Ok((job, false));
    }
    let running: i64 = tx.query_row(
        "SELECT COUNT(*) FROM ai_jobs WHERE status='running'",
        [],
        |r| r.get(0),
    )?;
    if running >= 4 {
        return Err(DbError::Migration(
            "已有 4 个后台任务，请等待其中一个完成".into(),
        ));
    }
    tx.execute(
        "DELETE FROM ai_jobs WHERE status!='running' AND finished_at<?1",
        [now_unix() - 7 * 86400],
    )?;
    tx.execute("DELETE FROM ai_jobs WHERE id IN (SELECT id FROM ai_jobs WHERE status!='running' ORDER BY id DESC LIMIT -1 OFFSET 200)",[])?;
    tx.execute("INSERT INTO ai_jobs(command,request_key,args_json,status,created_at) VALUES (?1,?2,?3,'running',?4)",params![command,key,serialized,now_unix()])?;
    let job = get(&tx, tx.last_insert_rowid())?;
    tx.commit()?;
    Ok((job, true))
}
pub fn finish(conn: &Connection, id: i64, result: Result<Value, String>) -> DbResult<()> {
    let (status, value, error) = match result {
        Ok(v) => ("completed", Some(v.to_string()), None),
        Err(e) => (
            "failed",
            None,
            Some(e.chars().take(1000).collect::<String>()),
        ),
    };
    conn.execute("UPDATE ai_jobs SET status=?1,result_json=?2,error=?3,finished_at=?4 WHERE id=?5 AND status='running'",params![status,value,error,now_unix(),id])?;
    Ok(())
}
/// Call once on process startup, never when opening another DB connection.
pub fn recover(conn: &Connection) -> DbResult<()> {
    conn.execute("UPDATE qa_turns SET status='interrupted',error='应用已退出，可重试这一轮。问题和范围已保留。',finished_at=?1 WHERE status IN ('running','pending')",[now_unix()])?;
    conn.execute("UPDATE ai_jobs SET status='interrupted',error='应用已退出，任务中断。可重新发起；已保存的工作不会丢失。',finished_at=?1 WHERE status='running'",[now_unix()])?;
    conn.execute("UPDATE analysis_runs SET status='failed',error_code='interrupted',error_message='应用已退出，分析中断，可重新发起',finished_at=?1 WHERE status='running'",[now_unix()])?;
    conn.execute("UPDATE reports SET status='failed',error_code='interrupted',error_message='应用已退出，报告生成中断，可重新发起' WHERE status='running'",[])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn receipt_retention_never_removes_active_jobs_or_formal_outputs() {
        let db = crate::db::Database::open_in_memory().unwrap();
        let (active, _) = start(db.conn(), "running-task", &serde_json::json!({})).unwrap();
        let report = crate::db::reports::ReportRepo::new(db.conn())
            .create("weekly", 1, 2)
            .unwrap();
        for n in 0..205 {
            let (job, _) = start(db.conn(), "synthetic", &serde_json::json!({"n":n})).unwrap();
            finish(db.conn(), job.id, Ok(serde_json::json!(n))).unwrap();
        }
        assert_eq!(get(db.conn(), active.id).unwrap().status, "running");
        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM ai_jobs", [], |r| r.get(0))
            .unwrap();
        assert!(count <= 202);
        assert!(crate::db::reports::ReportRepo::new(db.conn())
            .get(report.id)
            .unwrap()
            .is_some());
    }
    #[test]
    fn jobs_are_deduplicated_and_survive_database_reopen() {
        let dir = std::env::temp_dir().join(format!("job-test-{}", uuid::Uuid::new_v4()));
        let path = dir.join("test.db");
        let db = crate::db::Database::open(&path).unwrap();
        let (a, new) = start(
            db.conn(),
            "translation",
            &serde_json::json!({"input":"synthetic"}),
        )
        .unwrap();
        assert!(new);
        let (b, new) = start(
            db.conn(),
            "translation",
            &serde_json::json!({"input":"synthetic"}),
        )
        .unwrap();
        assert!(!new);
        assert_eq!(a.id, b.id);
        db.close().unwrap();
        let db = crate::db::Database::open(&path).unwrap();
        assert_eq!(get(db.conn(), a.id).unwrap().status, "running");
        finish(db.conn(), a.id, Ok(serde_json::json!("translation"))).unwrap();
        recover(db.conn()).unwrap();
        assert_eq!(
            get(db.conn(), a.id).unwrap().result,
            Some(serde_json::json!("translation"))
        );
        let (next, _) = start(db.conn(), "analysis", &serde_json::json!({})).unwrap();
        recover(db.conn()).unwrap();
        assert_eq!(get(db.conn(), next.id).unwrap().status, "interrupted");
        db.close().unwrap();
        std::fs::remove_dir_all(dir).unwrap();
    }
}
