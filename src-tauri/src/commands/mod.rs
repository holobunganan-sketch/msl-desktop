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

/// 记录一条管理活动（指南 §6.9：task.created / task.completed / work.created 等）。
/// 失败只记日志，不影响主操作。
fn record_activity(
    state: &State<AppState>,
    event_type: &str,
    work_id: Option<i64>,
    entity_type: &str,
    entity_id: i64,
    display_text: &str,
) {
    let result = state.with_database(|db| {
        crate::db::activity::ActivityRepo::new(db.conn()).insert(
            event_type,
            None,
            work_id,
            Some(entity_type),
            Some(entity_id),
            None,
            display_text,
            None,
            None,
        )
    });
    if let Some(Err(e)) = result {
        eprintln!("[activity] failed to record {event_type}: {e}");
    }
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
    .map(|task| {
        record_activity(
            &state,
            "task.created",
            task.work_id,
            "task",
            task.id,
            &format!("创建任务 {}", task.title),
        );
        task
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
    with_db(&state, |db| {
        crate::db::task::TaskRepo::new(db.conn()).complete(id)
    })?;
    // 记录 task.completed
    if let Some(Ok(Some(t))) = state.with_database(|db| crate::db::task::TaskRepo::new(db.conn()).get(id)) {
        record_activity(&state, "task.completed", t.work_id, "task", t.id, &format!("完成 {}", t.title));
    }
    Ok(())
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
    .map(|w| {
        record_activity(
            &state,
            "waiting.created",
            w.work_id,
            "waiting",
            w.id,
            &format!("创建等待事项 {}", w.title),
        );
        w
    })
}

/// 解决 waiting item。
#[tauri::command]
pub fn resolve_waiting(state: State<AppState>, id: i64) -> Result<(), String> {
    with_db(&state, |db| crate::db::task::WaitingRepo::new(db.conn()).resolve(id))
}

/// 列出 waiting（可按 status / work 过滤）。
#[tauri::command]
pub fn list_waiting(
    state: State<AppState>,
    status: Option<String>,
    work_id: Option<i64>,
) -> Result<Vec<crate::db::task::WaitingItem>, String> {
    with_db(&state, |db| {
        crate::db::task::WaitingRepo::new(db.conn()).list(status.as_deref(), work_id)
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

/// 按时间范围列出日历事件（[start, end]，可指定 work）。
#[tauri::command]
pub fn list_calendar_events(
    state: State<AppState>,
    start: i64,
    end: i64,
    work_id: Option<i64>,
) -> Result<Vec<crate::db::calendar::CalendarEvent>, String> {
    if let Some(w) = work_id {
        with_db(&state, |db| {
            crate::db::calendar::CalendarRepo::new(db.conn()).list_by_work(w)
        })
    } else {
        with_db(&state, |db| {
            crate::db::calendar::CalendarRepo::new(db.conn()).list_between(start, end)
        })
    }
}

// ---------- Works / Resume Points / 文件关联 ----------

/// 创建 Work。
#[tauri::command]
pub fn create_work(
    state: State<AppState>,
    title: String,
    status: Option<String>,
) -> Result<crate::db::work::Work, String> {
    with_db(&state, |db| {
        crate::db::work::WorkRepo::new(db.conn()).insert(&title, status.as_deref().unwrap_or("active"))
    })
    .map(|work| {
        record_activity(&state, "work.created", Some(work.id), "work", work.id, &format!("创建 Work {}", work.title));
        work
    })
}

/// 更新 Work（标题/状态/摘要）。
#[tauri::command]
pub fn update_work(
    state: State<AppState>,
    id: i64,
    title: String,
    status: String,
    summary: Option<String>,
) -> Result<(), String> {
    with_db(&state, |db| {
        crate::db::work::WorkRepo::new(db.conn()).update(id, &title, &status, summary.as_deref())
    })
}

/// 归档 Work。
#[tauri::command]
pub fn archive_work(state: State<AppState>, id: i64) -> Result<(), String> {
    with_db(&state, |db| crate::db::work::WorkRepo::new(db.conn()).archive(id))
}

/// 列出 Works（可按 status 过滤，默认按最近更新排序）。
#[tauri::command]
pub fn list_works(
    state: State<AppState>,
    status: Option<String>,
) -> Result<Vec<crate::db::work::Work>, String> {
    with_db(&state, |db| crate::db::work::WorkRepo::new(db.conn()).list(status.as_deref()))
}

/// Work 详情聚合：work + 最新 Resume Point + 历史 + 文件 + tasks + waiting + calendar + 最近活动。
#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkDetail {
    pub work: crate::db::work::Work,
    pub latest_resume: Option<crate::db::work::ResumePoint>,
    pub resume_history: Vec<crate::db::work::ResumePoint>,
    pub files: Vec<crate::db::workspace::WorkFileRef>,
    pub tasks: Vec<crate::db::task::Task>,
    pub waiting: Vec<crate::db::task::WaitingItem>,
    pub calendar: Vec<crate::db::calendar::CalendarEvent>,
    pub recent_activity: Vec<crate::db::activity::ActivityEvent>,
}

/// 获取 Work 详情（一次取齐，用于 Work 页面 10 秒恢复上下文）。
#[tauri::command]
pub fn get_work_detail(state: State<AppState>, id: i64) -> Result<WorkDetail, String> {
    with_db(&state, |db| {
        let work_repo = crate::db::work::WorkRepo::new(db.conn());
        let work = work_repo
            .get(id)?
            .ok_or_else(|| DbError::NotFound("work".into()))?;

        let resume_repo = crate::db::work::ResumePointRepo::new(db.conn());
        let latest_resume = resume_repo.latest_for_work(id)?;
        let resume_history = resume_repo.list_by_work(id)?;

        let files = crate::db::workspace::WorkFileRefRepo::new(db.conn()).list_by_work(id)?;
        let tasks = crate::db::task::TaskRepo::new(db.conn()).list(None, Some(id))?;
        let waiting = crate::db::task::WaitingRepo::new(db.conn()).list(None, Some(id))?;
        let calendar = crate::db::calendar::CalendarRepo::new(db.conn()).list_by_work(id)?;
        let recent_activity = crate::db::activity::ActivityRepo::new(db.conn())
            .query(None, None, Some(id), None, None, Some(30))?;

        Ok(WorkDetail {
            work,
            latest_resume,
            resume_history,
            files,
            tasks,
            waiting,
            calendar,
            recent_activity,
        })
    })
}

// ---------- Resume Points ----------

/// 创建 Resume Point（source: manual）。
#[tauri::command]
pub fn create_resume_point(
    state: State<AppState>,
    work_id: i64,
    current_state: String,
    next_step: String,
    remember: String,
) -> Result<crate::db::work::ResumePoint, String> {
    with_db(&state, |db| {
        crate::db::work::ResumePointRepo::new(db.conn())
            .insert(work_id, &current_state, &next_step, &remember, "manual")
    })
    .map(|rp| {
        record_activity(
            &state,
            "resume_point.created",
            Some(rp.work_id),
            "resume_point",
            rp.id,
            "更新 Resume Point",
        );
        rp
    })
}

/// 某 Work 的 Resume Point 历史（新→旧）。
#[tauri::command]
pub fn list_resume_points(
    state: State<AppState>,
    work_id: i64,
) -> Result<Vec<crate::db::work::ResumePoint>, String> {
    with_db(&state, |db| {
        crate::db::work::ResumePointRepo::new(db.conn()).list_by_work(work_id)
    })
}

/// 删除 Resume Point。
#[tauri::command]
pub fn delete_resume_point(state: State<AppState>, id: i64) -> Result<(), String> {
    with_db(&state, |db| {
        crate::db::work::ResumePointRepo::new(db.conn()).delete(id)
    })
}

// ---------- Work 文件关联 ----------

/// 关联文件到 Work（只存路径引用）。
#[tauri::command]
pub fn add_work_file_ref(
    state: State<AppState>,
    work_id: i64,
    workspace_id: Option<i64>,
    path: String,
    label: Option<String>,
) -> Result<crate::db::workspace::WorkFileRef, String> {
    with_db(&state, |db| {
        crate::db::workspace::WorkFileRefRepo::new(db.conn())
            .insert(work_id, workspace_id, &path, label.as_deref())
    })
}

/// 更新文件关联（label / pinned）。
#[tauri::command]
pub fn update_work_file_ref(
    state: State<AppState>,
    id: i64,
    label: Option<String>,
    pinned: bool,
) -> Result<(), String> {
    with_db(&state, |db| {
        crate::db::workspace::WorkFileRefRepo::new(db.conn())
            .update(id, label.as_deref(), pinned)
    })
}

/// 移除文件关联。
#[tauri::command]
pub fn remove_work_file_ref(state: State<AppState>, id: i64) -> Result<(), String> {
    with_db(&state, |db| {
        crate::db::workspace::WorkFileRefRepo::new(db.conn()).delete(id)
    })
}
