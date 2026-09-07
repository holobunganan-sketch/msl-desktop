//! 提醒调度（指南 §23：waiting follow-up / task deadline / calendar reminder）。
//!
//! - 纯逻辑 `collect_due_reminders` 可测试；
//! - 调度循环 30 秒一次轻量 DB 查询（Resident Core 的一部分），
//!   空闲时无持续 CPU 尖峰；
//! - 通知通过 tauri-plugin-notification（Windows toast）；
//! - 进程内去重（同一事件只通知一次，重启后窗口内会重新检查）。

use std::collections::HashSet;
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::db::Database;

/// 检查周期：30 秒（"精巧"：低频轮询 + 轻量查询）。
pub const CHECK_INTERVAL: Duration = Duration::from_secs(30);

/// 单条提醒。
#[derive(Debug, Clone, Serialize)]
pub struct Reminder {
    pub kind: String, // waiting | task | calendar
    pub title: String,
    pub body: String,
    /// 去重键（kind + id）。
    pub dedupe_key: String,
}

/// 收集到期提醒（纯逻辑，可测试）。
/// `now` 当前秒；`lead_secs` 提前提醒窗口。
pub fn collect_due_reminders(db: &Database, now: i64, lead_secs: i64) -> Vec<Reminder> {
    let window_end = now + lead_secs;
    let mut out = Vec::new();

    // waiting：open 且 follow_up_at 落在 [now, now+lead]
    if let Ok(items) = crate::db::task::WaitingRepo::new(db.conn()).list(Some("open"), None) {
        for w in items {
            if let Some(f) = w.follow_up_at {
                if f >= now && f <= window_end {
                    out.push(Reminder {
                        kind: "waiting".into(),
                        title: format!("跟进：{}", w.title),
                        body: format!("等待{}，{}", w.waiting_for, fmt_time(f)),
                        dedupe_key: format!("waiting-{}", w.id),
                    });
                }
            }
        }
    }

    // task：未完成且 due_at 落在窗口内
    if let Ok(tasks) = crate::db::task::TaskRepo::new(db.conn()).list(None, None) {
        for t in tasks {
            if t.status == "done" {
                continue;
            }
            if let Some(d) = t.due_at {
                if d >= now && d <= window_end {
                    out.push(Reminder {
                        kind: "task".into(),
                        title: format!("任务截止：{}", t.title),
                        body: fmt_time(d),
                        dedupe_key: format!("task-{}", t.id),
                    });
                }
            }
        }
    }

    // calendar：start_at 落在窗口内
    if let Ok(events) =
        crate::db::calendar::CalendarRepo::new(db.conn()).list_between(now, window_end)
    {
        for e in events {
            out.push(Reminder {
                kind: "calendar".into(),
                title: format!("日程：{}", e.title),
                body: fmt_time(e.start_at),
                dedupe_key: format!("calendar-{}", e.id),
            });
        }
    }

    out
}

fn fmt_time(ts: i64) -> String {
    // 简短本地时间（用前端更精确；这里给粗粒度时间即可）
    if let Some(d) = chrono_like(ts) {
        d
    } else {
        format!("{ts}")
    }
}

/// 无 chrono 依赖的本地时间格式化（Windows 系统时区）。
fn chrono_like(ts: i64) -> Option<String> {
    // 用 std 无法直接取本地时区；回退：显示 UTC 时间
    let days = ts.div_euclid(86400);
    let secs = ts.rem_euclid(86400);
    let (h, m) = (secs / 3600, (secs % 3600) / 60);
    Some(format!("{days}d {h:02}:{m:02} (UTC)"))
}

/// 通知已发送的去重集合（进程内）。
#[derive(Default)]
pub struct NotifiedSet(pub Mutex<HashSet<String>>);

/// 启动提醒调度循环（挂到 Resident Core）。
pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(CHECK_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            if !reminders_enabled(&app) {
                continue;
            }
            let Some(state) = app.try_state::<crate::app_state::AppState>() else {
                continue;
            };
            let lead_minutes = state
                .with_database(|db| {
                    crate::db::provider::AppSettingsRepo::new(db.conn())
                        .get("reminder_lead_minutes")
                })
                .and_then(|r| r.ok().flatten())
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(60);
            let lead_secs = lead_minutes.saturating_mul(60);

            let due = state
                .with_database(|db| collect_due_reminders(db, crate::db::now_unix(), lead_secs))
                .unwrap_or_default();

            for r in due {
                // 进程内去重
                let already = {
                    let set = state.notified_set();
                    !set.lock().unwrap().insert(r.dedupe_key.clone())
                };
                if already {
                    continue;
                }
                send_notification(&app, &r.title, &r.body);
            }
        }
    });
}

fn reminders_enabled(app: &AppHandle) -> bool {
    let Some(state) = app.try_state::<crate::app_state::AppState>() else {
        return false;
    };
    state
        .with_database(|db| {
            crate::db::provider::AppSettingsRepo::new(db.conn()).get("notifications_enabled")
        })
        .and_then(|r| r.ok().flatten())
        .map(|v| v != "false")
        .unwrap_or(true)
}

/// 发送系统通知（tauri-plugin-notification，Windows toast）。
fn send_notification(app: &AppHandle, title: &str, body: &str) {
    use tauri_plugin_notification::NotificationExt;
    let _ = app.notification().builder().title(title).body(body).show();
}

/// 立即执行一次提醒检查（调试/验收用；不经过调度循环）。
pub fn check_once(app: &AppHandle) -> Vec<Reminder> {
    let Some(state) = app.try_state::<crate::app_state::AppState>() else {
        return Vec::new();
    };
    let due = state
        .with_database(|db| collect_due_reminders(db, crate::db::now_unix(), 60 * 60))
        .unwrap_or_default();
    for r in &due {
        send_notification(app, &r.title, &r.body);
    }
    due
}
