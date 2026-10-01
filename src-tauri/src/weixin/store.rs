use crate::db::{jobs::AiJob, DbError, DbResult};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;

const MENU: &str = "秘书在这里，请回复序号：\n一、全局整理\n二、生成每日简报\n三、查看整理进度\n四、查看待确认数量\n五、退出菜单\n可回复 1–5；菜单 10 分钟内有效。普通文字会原样记入收件箱，工作内容与建议请在桌面端查看。";

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
    pub ack: String,
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
        let Some((id, text, created_ms)) = trusted_text(binding, msg, now) else {
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
        let expired = tx.execute("UPDATE weixin_binding SET menu_mode=NULL,menu_expires_at=NULL WHERE id=1 AND menu_expires_at<=?1", [now])? > 0;
        let (mode, after_ms, after_id, protocol_after_ms): (Option<String>, i64, String, i64) = tx.query_row(
            "SELECT menu_mode,menu_after_ms,menu_after_id,menu_protocol_after_ms FROM weixin_binding WHERE id=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )?;
        // IDs are validated uint64 values. They break ties when the service emits
        // multiple messages in the same millisecond, including one poll batch.
        let fresh = (created_ms, id.parse::<u64>().unwrap_or(0))
            > (after_ms, after_id.parse::<u64>().unwrap_or(0));
        let choice = if fresh && mode.as_deref() == Some("menu") {
            menu_choice(text.trim())
        } else {
            None
        };
        let mut action = "capture";
        let mut ack = "已记入收件箱。".to_string();
        let mut inbox_id = None;
        let mut job_id = None;
        let mut job = None;
        if recent >= 60 {
            action = "rate_limited";
            ack = "消息较频繁，请稍后重新发送。".into();
        } else if text.trim() == "召唤秘书" {
            action = "menu";
            let switch_wait = (protocol_after_ms / 1000 + 1).saturating_sub(now);
            if protocol_after_ms > 0 && switch_wait > 0 {
                ack = format!("升级后的菜单正在切换，请约{switch_wait}秒后重新发送“召唤秘书”；普通文字仍会记录。");
            } else if created_ms <= protocol_after_ms {
                ack = "菜单已更新，请重新发送“召唤秘书”打开当前菜单；普通文字仍会记录。".into();
            } else if fresh {
                // Offline delivery cannot renew old authorization; cap future clock skew.
                let expires_at = now.min(created_ms / 1000) + 600;
                tx.execute(
                    "UPDATE weixin_binding SET menu_mode=?1,menu_expires_at=?2 WHERE id=1",
                    params![
                        if expires_at > now { Some("menu") } else { None },
                        expires_at
                    ],
                )?;
                ack = if expires_at > now {
                    MENU
                } else {
                    "菜单已过期，请重新发送“召唤秘书”。"
                }
                .into();
            } else {
                ack = "菜单已更新，请按当前菜单操作；需要时可重新发送“召唤秘书”。".into();
            }
        } else if let Some(choice) = choice {
            match choice {
                1 | 2 => {
                    let last: Option<i64> = tx.query_row(
                        if choice == 1 {
                            "SELECT last_global_at FROM weixin_binding WHERE id=1"
                        } else {
                            "SELECT last_brief_at FROM weixin_binding WHERE id=1"
                        },
                        [],
                        |r| r.get(0),
                    )?;
                    if last.is_some_and(|last| now.saturating_sub(last) < 60) {
                        action = "rate_limited";
                        ack = "此操作刚刚提交，请稍后重试；进度可在桌面端查看。".into();
                    } else {
                        let (command, args) = if choice == 1 {
                            ("run_analysis_now", serde_json::json!({"trigger":"manual"}))
                        } else {
                            ("generate_brief", brief_args(now)?)
                        };
                        // Shared job deduplication and queue limit; entity changes
                        // continue through the desktop proposal confirmation flow.
                        match crate::db::jobs::start_in_transaction(&tx, command, &args) {
                            Ok((started, created)) => {
                                tx.execute(
                                    if choice == 1 {
                                        "UPDATE weixin_binding SET last_global_at=?1 WHERE id=1"
                                    } else {
                                        "UPDATE weixin_binding SET last_brief_at=?1 WHERE id=1"
                                    },
                                    [now],
                                )?;
                                action = if choice == 1 { "global" } else { "brief" };
                                ack = if choice == 1 {
                                    "已提交后台整理请求；处理进度和待确认建议请在桌面端查看。"
                                } else {
                                    "已提交今日简报请求；简报内容和处理进度请在桌面端查看。"
                                }
                                .into();
                                job_id = Some(started.id);
                                job = created.then_some(started);
                            }
                            Err(DbError::Migration(_)) => {
                                action = "queue_busy";
                                ack = "后台任务较多，请稍后重新回复序号。".into();
                            }
                            Err(error) => return Err(error),
                        }
                    }
                }
                3 => {
                    action = "job_status";
                    let counts: (i64, i64, i64, i64) = tx.query_row(
                        "SELECT COUNT(CASE WHEN status='running' THEN 1 END),COUNT(CASE WHEN status='completed' THEN 1 END),COUNT(CASE WHEN status='failed' THEN 1 END),COUNT(CASE WHEN status='interrupted' THEN 1 END) FROM ai_jobs", [],
                        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                    )?;
                    ack = format!("本机保留的后台任务：进行中 {}，已完成 {}，失败 {}，已中断 {}。详情请在桌面端查看。", counts.0, counts.1, counts.2, counts.3);
                }
                4 => {
                    action = "pending_status";
                    let counts: (i64, i64) = tx.query_row(
                        "SELECT COUNT(CASE WHEN deferred_at IS NULL THEN 1 END),COUNT(CASE WHEN deferred_at IS NOT NULL THEN 1 END) FROM ai_proposals WHERE status='pending'", [],
                        |r| Ok((r.get(0)?, r.get(1)?)),
                    )?;
                    ack = format!(
                        "待确认 {} 条，暂缓 {} 条。请在桌面端查看并确认建议。",
                        counts.0, counts.1
                    );
                }
                5 => {
                    action = "exit_menu";
                    tx.execute(
                        "UPDATE weixin_binding SET menu_mode=NULL,menu_expires_at=NULL WHERE id=1",
                        [],
                    )?;
                    ack = "已退出菜单，之后发送的普通文字会记入收件箱。需要时请发送“召唤秘书”。"
                        .into();
                }
                _ => unreachable!(),
            }
        } else {
            let item = crate::db::inbox::InboxRepo::new(&tx).insert(text)?;
            tx.execute("INSERT INTO capture_context(inbox_id,work_id,entity_kind,entity_id) VALUES(?1,NULL,NULL,NULL)",[item.id])?;
            inbox_id = Some(item.id);
            if expired && menu_choice(text.trim()).is_some() {
                ack = "菜单已过期，这条原话已记入收件箱；如需操作，请重新发送“召唤秘书”。".into();
            }
        }
        if fresh && action != "rate_limited" {
            tx.execute(
                "UPDATE weixin_binding SET menu_after_ms=?1,menu_after_id=?2 WHERE id=1",
                params![created_ms, id],
            )?;
        }
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

fn menu_choice(text: &str) -> Option<u8> {
    match text {
        "1" | "一" | "１" => Some(1),
        "2" | "二" | "２" => Some(2),
        "3" | "三" | "３" => Some(3),
        "4" | "四" | "４" => Some(4),
        "5" | "五" | "５" => Some(5),
        _ => None,
    }
}

fn brief_args(now: i64) -> DbResult<Value> {
    use chrono::{Local, TimeZone};
    let invalid = || DbError::Migration("本机日期无效，请在桌面端生成简报".into());
    let local = Local.timestamp_opt(now, 0).single().ok_or_else(invalid)?;
    let date = local.date_naive();
    let period_start = Local
        .from_local_datetime(
            &date
                .pred_opt()
                .ok_or_else(invalid)?
                .and_hms_opt(0, 0, 0)
                .ok_or_else(invalid)?,
        )
        .earliest()
        .ok_or_else(invalid)?
        .timestamp();
    let today_start = Local
        .from_local_datetime(&date.and_hms_opt(0, 0, 0).ok_or_else(invalid)?)
        .earliest()
        .ok_or_else(invalid)?
        .timestamp();
    let today_end = Local
        .from_local_datetime(
            &date
                .succ_opt()
                .ok_or_else(invalid)?
                .and_hms_opt(0, 0, 0)
                .ok_or_else(invalid)?,
        )
        .earliest()
        .ok_or_else(invalid)?
        .timestamp();
    Ok(
        serde_json::json!({"date":date.to_string(),"periodStart":period_start,"periodEnd":today_start,
        "todayStart":today_start,"todayEnd":today_end,"locale":"zh-CN","force":false}),
    )
}

fn trusted_text<'a>(binding: &Binding, msg: &'a Value, now: i64) -> Option<(String, &'a str, i64)> {
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
    let created_ms = msg["create_time_ms"].as_i64()?;
    let created = created_ms.checked_div(1000)?;
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
    Some((id, text, created_ms))
}
