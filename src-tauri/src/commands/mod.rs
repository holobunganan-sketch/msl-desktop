//! IPC 命令层（指南 §5：command 层只做边界转换，不塞核心业务逻辑）。

use std::path::PathBuf;

use tauri::{AppHandle, State};

use crate::app_state::AppState;
use crate::db::workspace::WorkspaceRepo;
use crate::db::{Database, DbError};
use crate::workspace::{self, watcher::FileWatcher, DirEntry, RecentFile};

/// 从 AppState 取数据库引用执行操作；统一错误信息。
fn with_db<T>(state: &State<AppState>, f: impl FnOnce(&Database) -> Result<T, DbError>) -> Result<T, String> {
    state
        .with_database(f)
        .ok_or_else(|| "数据库未初始化".to_string())?
        .map_err(|e| e.to_string())
}

/// 绑定/更新一个 workspace：保存到 DB、记录主 workspace、启动文件监听。
#[tauri::command]
pub fn bind_workspace(
    app: AppHandle,
    state: State<AppState>,
    name: String,
    path: String,
) -> Result<crate::db::workspace::Workspace, String> {
    let ws = with_db(&state, |db| {
        WorkspaceRepo::new(db.conn()).insert(&name, &path)
    })?;

    // 记录主 workspace 设置
    if let Err(e) = with_db(&state, |db| {
        crate::db::provider::AppSettingsRepo::new(db.conn()).set("main_workspace", &path)
    }) {
        eprintln!("[workspace] failed to save main_workspace setting: {e}");
    }

    // 启动 watcher（替换旧的监听）
    match FileWatcher::start(app, PathBuf::from(&path)) {
        Ok(w) => state.set_watcher(w),
        Err(e) => eprintln!("[workspace] watcher start failed: {e}"),
    }

    Ok(ws)
}

/// 列出目录单层内容（lazy loading）。
#[tauri::command]
pub fn list_dir(path: String) -> Result<Vec<DirEntry>, String> {
    workspace::list_dir(std::path::Path::new(&path)).map_err(|e| e.to_string())
}

/// 用系统默认程序打开文件/目录。
#[tauri::command]
pub fn open_file(path: String) -> Result<(), String> {
    workspace::open_path(std::path::Path::new(&path)).map_err(|e| e.to_string())
}

/// 在 Explorer 中定位。
#[tauri::command]
pub fn reveal_in_explorer(path: String) -> Result<(), String> {
    workspace::reveal_in_explorer(std::path::Path::new(&path)).map_err(|e| e.to_string())
}

/// 最近修改的文件。
#[tauri::command]
pub fn recent_files(state: State<AppState>, limit: Option<u32>) -> Result<Vec<RecentFile>, String> {
    with_db(&state, |db| workspace::recent_files(db, limit.unwrap_or(20)))
}

/// 已保存的 workspaces。
#[tauri::command]
pub fn get_workspaces(state: State<AppState>) -> Result<Vec<crate::db::workspace::Workspace>, String> {
    with_db(&state, |db| WorkspaceRepo::new(db.conn()).list())
}

/// 暂停/恢复文件监控（托盘菜单"暂停文件监控"）。
#[tauri::command]
pub fn set_watcher_paused(state: State<AppState>, paused: bool) -> Result<bool, String> {
    let p = state
        .with_watcher(|w| {
            w.set_paused(paused);
            w.is_paused()
        })
        .unwrap_or(false);
    Ok(p)
}

/// 当前 watcher 状态（暂停/监听根目录）。
#[tauri::command]
pub fn watcher_status(state: State<AppState>) -> Result<Option<crate::workspace::watcher::WatcherStatus>, String> {
    Ok(state.with_watcher(|w| w.status()))
}

// ---------- Plan / Tasks ----------

/// 创建任务。
#[tauri::command]
pub fn create_task(
    state: State<AppState>,
    work_id: Option<i64>,
    title: String,
    priority: Option<String>,
    due_at: Option<i64>,
    notes: Option<String>,
) -> Result<crate::db::task::Task, String> {
    with_db(&state, |db| {
        crate::db::task::TaskRepo::new(db.conn()).insert(
            work_id,
            &title,
            priority.as_deref().unwrap_or("normal"),
            due_at,
            notes.as_deref(),
        )
    })
}

/// 更新任务（标题/优先级/截止/备注）。
#[tauri::command]
pub fn update_task(
    state: State<AppState>,
    id: i64,
    title: String,
    priority: Option<String>,
    due_at: Option<i64>,
    notes: Option<String>,
) -> Result<(), String> {
    with_db(&state, |db| {
        crate::db::task::TaskRepo::new(db.conn()).update(
            id,
            &title,
            priority.as_deref().unwrap_or("normal"),
            due_at,
            notes.as_deref(),
        )
    })
}

/// 完成任务。
#[tauri::command]
pub fn complete_task(state: State<AppState>, id: i64) -> Result<(), String> {
    with_db(&state, |db| crate::db::task::TaskRepo::new(db.conn()).complete(id))
}

/// 列出任务（可按 status / work 过滤）。
#[tauri::command]
pub fn list_tasks(
    state: State<AppState>,
    status: Option<String>,
    work_id: Option<i64>,
) -> Result<Vec<crate::db::task::Task>, String> {
    with_db(&state, |db| {
        crate::db::task::TaskRepo::new(db.conn()).list(status.as_deref(), work_id)
    })
}

/// 删除任务。
#[tauri::command]
pub fn delete_task(state: State<AppState>, id: i64) -> Result<(), String> {
    with_db(&state, |db| crate::db::task::TaskRepo::new(db.conn()).delete(id))
}

// ---------- Waiting ----------

/// 创建 waiting item。
#[tauri::command]
pub fn create_waiting(
    state: State<AppState>,
    work_id: Option<i64>,
    title: String,
    waiting_for: Option<String>,
    follow_up_at: Option<i64>,
    notes: Option<String>,
) -> Result<crate::db::task::WaitingItem, String> {
    with_db(&state, |db| {
        crate::db::task::WaitingRepo::new(db.conn()).insert(
            work_id,
            &title,
            waiting_for.as_deref().unwrap_or(""),
            follow_up_at,
            notes.as_deref(),
        )
    })
}

/// 解决 waiting item。
#[tauri::command]
pub fn resolve_waiting(state: State<AppState>, id: i64) -> Result<(), String> {
    with_db(&state, |db| crate::db::task::WaitingRepo::new(db.conn()).resolve(id))
}

/// 列出 waiting（可按 status 过滤）。
#[tauri::command]
pub fn list_waiting(
    state: State<AppState>,
    status: Option<String>,
) -> Result<Vec<crate::db::task::WaitingItem>, String> {
    with_db(&state, |db| {
        crate::db::task::WaitingRepo::new(db.conn()).list(status.as_deref(), None)
    })
}

/// 删除 waiting item。
#[tauri::command]
pub fn delete_waiting(state: State<AppState>, id: i64) -> Result<(), String> {
    with_db(&state, |db| crate::db::task::WaitingRepo::new(db.conn()).delete(id))
}

// ---------- Inbox / Quick Capture ----------

/// Quick Capture：写入 Inbox。
#[tauri::command]
pub fn create_inbox_item(state: State<AppState>, content: String) -> Result<crate::db::inbox::InboxItem, String> {
    let content = content.trim().to_string();
    if content.is_empty() {
        return Err("内容为空".to_string());
    }
    with_db(&state, |db| crate::db::inbox::InboxRepo::new(db.conn()).insert(&content))
}

/// 列出 Inbox（未处理在前）。
#[tauri::command]
pub fn list_inbox(state: State<AppState>) -> Result<Vec<crate::db::inbox::InboxItem>, String> {
    with_db(&state, |db| crate::db::inbox::InboxRepo::new(db.conn()).list())
}

/// Inbox → Task。
#[tauri::command]
pub fn convert_inbox_to_task(
    state: State<AppState>,
    inbox_id: i64,
    work_id: Option<i64>,
    priority: Option<String>,
    due_at: Option<i64>,
    notes: Option<String>,
) -> Result<crate::db::task::Task, String> {
    with_db(&state, |db| {
        let inbox = crate::db::inbox::InboxRepo::new(db.conn())
            .get(inbox_id)?
            .ok_or_else(|| DbError::NotFound("inbox_item".into()))?;
        let task = crate::db::task::TaskRepo::new(db.conn()).insert(
            work_id,
            &inbox.content,
            priority.as_deref().unwrap_or("normal"),
            due_at,
            notes.as_deref(),
        )?;
        crate::db::inbox::InboxRepo::new(db.conn())
            .mark_processed(inbox_id, "task", task.id)?;
        Ok(task)
    })
}

/// Inbox → Waiting。
#[tauri::command]
pub fn convert_inbox_to_waiting(
    state: State<AppState>,
    inbox_id: i64,
    work_id: Option<i64>,
    waiting_for: Option<String>,
    follow_up_at: Option<i64>,
) -> Result<crate::db::task::WaitingItem, String> {
    with_db(&state, |db| {
        let inbox = crate::db::inbox::InboxRepo::new(db.conn())
            .get(inbox_id)?
            .ok_or_else(|| DbError::NotFound("inbox_item".into()))?;
        let waiting = crate::db::task::WaitingRepo::new(db.conn()).insert(
            work_id,
            &inbox.content,
            waiting_for.as_deref().unwrap_or(""),
            follow_up_at,
            None,
        )?;
        crate::db::inbox::InboxRepo::new(db.conn())
            .mark_processed(inbox_id, "waiting", waiting.id)?;
        Ok(waiting)
    })
}

/// Inbox → Calendar。
#[tauri::command]
pub fn convert_inbox_to_calendar(
    state: State<AppState>,
    inbox_id: i64,
    work_id: Option<i64>,
    start_at: i64,
    end_at: Option<i64>,
    all_day: bool,
    kind: Option<String>,
) -> Result<crate::db::calendar::CalendarEvent, String> {
    with_db(&state, |db| {
        let inbox = crate::db::inbox::InboxRepo::new(db.conn())
            .get(inbox_id)?
            .ok_or_else(|| DbError::NotFound("inbox_item".into()))?;
        let event = crate::db::calendar::CalendarRepo::new(db.conn()).insert(
            work_id,
            &inbox.content,
            start_at,
            end_at,
            all_day,
            kind.as_deref().unwrap_or("other"),
            None,
            None,
        )?;
        crate::db::inbox::InboxRepo::new(db.conn())
            .mark_processed(inbox_id, "calendar", event.id)?;
        Ok(event)
    })
}

/// 删除 Inbox 项。
#[tauri::command]
pub fn delete_inbox_item(state: State<AppState>, id: i64) -> Result<(), String> {
    with_db(&state, |db| crate::db::inbox::InboxRepo::new(db.conn()).delete(id))
}

// ---------- Calendar ----------

/// 创建日历事件。
#[tauri::command]
pub fn create_calendar_event(
    state: State<AppState>,
    work_id: Option<i64>,
    title: String,
    start_at: i64,
    end_at: Option<i64>,
    all_day: bool,
    kind: Option<String>,
    location: Option<String>,
    notes: Option<String>,
) -> Result<crate::db::calendar::CalendarEvent, String> {
    with_db(&state, |db| {
        crate::db::calendar::CalendarRepo::new(db.conn()).insert(
            work_id,
            &title,
            start_at,
            end_at,
            all_day,
            kind.as_deref().unwrap_or("other"),
            location.as_deref(),
            notes.as_deref(),
        )
    })
}

/// 更新日历事件。
#[tauri::command]
pub fn update_calendar_event(
    state: State<AppState>,
    id: i64,
    title: String,
    start_at: i64,
    end_at: Option<i64>,
    all_day: bool,
    kind: Option<String>,
    location: Option<String>,
    notes: Option<String>,
) -> Result<(), String> {
    with_db(&state, |db| {
        crate::db::calendar::CalendarRepo::new(db.conn()).update(
            id,
            &title,
            start_at,
            end_at,
            all_day,
            kind.as_deref().unwrap_or("other"),
            location.as_deref(),
            notes.as_deref(),
        )
    })
}

/// 删除日历事件。
#[tauri::command]
pub fn delete_calendar_event(state: State<AppState>, id: i64) -> Result<(), String> {
    with_db(&state, |db| crate::db::calendar::CalendarRepo::new(db.conn()).delete(id))
}

/// 按时间范围列出日历事件（[start, end]）。
#[tauri::command]
pub fn list_calendar_events(
    state: State<AppState>,
    start: i64,
    end: i64,
) -> Result<Vec<crate::db::calendar::CalendarEvent>, String> {
    with_db(&state, |db| {
        crate::db::calendar::CalendarRepo::new(db.conn()).list_between(start, end)
    })
}
