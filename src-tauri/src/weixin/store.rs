use crate::db::{jobs::AiJob, DbError, DbResult};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

#[derive(Clone)]
pub(crate) struct Binding {
    pub bot_id: String,
    pub owner_id: String,
    pub base_url: String,
    pub credential_ref: String,
    pub bound_at: i64,
    pub enabled: bool,
    pub cursor: String,
}
pub(crate) struct Receipt {
    pub ack: &'static str,
    pub context: String,
    pub job: Option<AiJob>,
}
pub(crate) fn binding(conn: &Connection) -> DbResult<Option<Binding>> {
    Ok(conn.query_row("SELECT bot_id,owner_id,base_url,credential_ref,bound_at,enabled,cursor FROM weixin_binding WHERE id=1",[],|r|Ok(Binding{bot_id:r.get(0)?,owner_id:r.get(1)?,base_url:r.get(2)?,credential_ref:r.get(3)?,bound_at:r.get(4)?,enabled:r.get(5)?,cursor:r.get(6)?})).optional()?)
}

pub(crate) fn ingest_batch(
    conn: &Connection,
    binding: &Binding,
    messages: &[Value],
    next: &str,
    now: i64,
) -> DbResult<Vec<Receipt>> {
    if messages.len() > 1000 || next.len() > 65536 {
        return Err(DbError::Migration("微信消息批次超出安全限制".into()));
    }
    let tx = crate::db::write_transaction(conn)?;
    let active:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM weixin_binding WHERE id=1 AND enabled=1 AND bot_id=?1 AND owner_id=?2 AND bound_at=?3)",params![binding.bot_id,binding.owner_id,binding.bound_at],|r|r.get(0))?;
    if !active || !binding.enabled {
        return Ok(vec![]);
    }
    let mut receipts = vec![];
    for msg in messages {
        let Some((id, text)) = trusted_text(binding, msg, now) else {
            continue;
        };
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM weixin_receipts WHERE bot_id=?1 AND message_id=?2)",
            params![binding.bot_id, id],
            |r| r.get(0),
        )?;
        if exists {
            continue;
        }
        let recent: i64 = tx.query_row(
            "SELECT COUNT(*) FROM weixin_receipts WHERE bot_id=?1 AND received_at>?2",
            params![binding.bot_id, now - 60],
            |r| r.get(0),
        )?;
        let global = text == "全局交给秘书整理一遍";
        let last_global: Option<i64> = tx.query_row(
            "SELECT last_global_at FROM weixin_binding WHERE id=1",
            [],
            |r| r.get(0),
        )?;
        let limited = recent >= 60 || (global && last_global.is_some_and(|t| now - t < 60));
        let (action, ack, inbox_id, job_id, job) = if limited {
            (
                "rate_limited",
                "消息较频繁，请稍后重新发送。",
                None,
                None,
                None,
            )
        } else if global {
            // Reuse the same job request and deduplication as the desktop button.
            // This explicit owner command is a manual request; proposals still
            // go through the desktop's normal confirmation flow.
            match crate::db::jobs::start_in_transaction(
                &tx,
                "run_analysis_now",
                &serde_json::json!({"trigger":"manual"}),
            ) {
                Ok((job, created)) => {
                    tx.execute(
                        "UPDATE weixin_binding SET last_global_at=?1 WHERE id=1",
                        [now],
                    )?;
                    (
                        "global",
                        "已提交后台整理请求；处理进度和待确认建议请在桌面端查看。",
                        None,
                        Some(job.id),
                        created.then_some(job),
                    )
                }
                Err(DbError::Migration(_)) => (
                    "queue_busy",
                    "后台任务较多，请稍后重新发送整理命令。",
                    None,
                    None,
                    None,
                ),
                Err(e) => return Err(e),
            }
        } else {
            let item = crate::db::inbox::InboxRepo::new(&tx).insert(text)?;
            tx.execute("INSERT INTO capture_context(inbox_id,work_id,entity_kind,entity_id) VALUES(?1,NULL,NULL,NULL)",[item.id])?;
            ("capture", "已记入收件箱。", Some(item.id), None, None)
        };
        tx.execute("INSERT INTO weixin_receipts(bot_id,message_id,received_at,action,inbox_id,job_id) VALUES(?1,?2,?3,?4,?5,?6)",params![binding.bot_id,id,now,action,inbox_id,job_id])?;
        tx.execute(
            "UPDATE weixin_binding SET last_received_at=?1 WHERE id=1",
            [now],
        )?;
        receipts.push(Receipt {
            ack,
            context: msg["context_token"]
                .as_str()
                .filter(|s| s.len() <= 16384)
                .unwrap_or("")
                .to_owned(),
            job,
        });
    }
    // A failed write rolls back the entire batch, including its cursor and jobs.
    if !next.is_empty() {
        tx.execute("UPDATE weixin_binding SET cursor=?1 WHERE id=1", [next])?;
    }
    tx.commit()?;
    Ok(receipts)
}

fn trusted_text<'a>(binding: &Binding, msg: &'a Value, now: i64) -> Option<(String, &'a str)> {
    if msg["from_user_id"].as_str() != Some(binding.owner_id.as_str())
        || msg["to_user_id"]
            .as_str()
            .is_some_and(|to| !to.is_empty() && to != binding.bot_id)
        || (!msg["to_user_id"].is_null() && !msg["to_user_id"].is_string())
        || msg["group_id"].as_str().is_some_and(|s| !s.is_empty())
        || (!msg["group_id"].is_null() && !msg["group_id"].is_string())
        || msg["message_type"].as_i64() != Some(1)
        || msg["message_state"].as_i64() != Some(2)
        || msg["delete_time_ms"].as_i64().unwrap_or(0) > 0
    {
        return None;
    }
    let created = msg["create_time_ms"].as_i64()?.checked_div(1000)?;
    if created < binding.bound_at || created > now + 300 {
        return None;
    }
    let id = match &msg["message_id"] {
        Value::String(value) if value.len() <= 20 && value.bytes().all(|b| b.is_ascii_digit()) => {
            value.parse::<u64>().ok()
        }
        value => value.as_u64(),
    }
    .filter(|n| *n > 0)?
    .to_string();
    let items = msg["item_list"].as_array()?;
    if items.len() != 1 || items[0]["type"].as_i64() != Some(1) {
        return None;
    }
    let text = items[0]["text_item"]["text"].as_str()?;
    if text.trim().is_empty() || text.len() > 20000 {
        return None;
    }
    Some((id, text))
}
