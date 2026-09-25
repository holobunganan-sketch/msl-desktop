//! IPC 命令层（指南 §5：command 层只做边界转换，不塞核心业务逻辑）。

use std::path::{Path, PathBuf};

use tauri::{AppHandle, State};

use crate::app_state::AppState;
use crate::db::workspace::WorkspaceRepo;
use crate::db::{Database, DbError, DbResult};
use crate::workspace::{self, inventory, watcher::FileWatcher, DirEntry, RecentFile};

pub mod ai_secretary;
pub mod backup;
pub mod flow;
pub mod jobs;
pub mod knowledge;
pub mod materials;
pub mod provider_catalog;
pub mod reports;
pub mod sync;
pub use ai_secretary::{
    confirm_ai_proposal, list_ai_proposals, reject_ai_proposal, update_ai_proposal_draft,
};
pub use provider_catalog::{
    create_provider_template, list_ai_task_routes, list_provider_connections, list_provider_models,
    refresh_provider_models, save_ai_task_route, save_provider_connection, save_provider_model,
    set_provider_model_enabled, test_provider_model,
};

/// 从 AppState 取数据库引用执行操作；统一错误信息。
pub(crate) fn with_db<T>(
    state: &State<AppState>,
    f: impl FnOnce(&Database) -> Result<T, DbError>,
) -> Result<T, String> {
    state
        .with_database(f)
        .ok_or_else(|| "数据库未初始化".to_string())?
        .map_err(|e| e.to_string())
}

fn required(value: String, field: &str) -> Result<String, String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        return Err(format!("{field} 不能为空"));
    }
    Ok(trimmed)
}

fn optional_trim(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let trimmed = v.trim().to_string();
        (!trimmed.is_empty()).then_some(trimmed)
    })
}

fn allowed(value: &str, field: &str, values: &[&str]) -> Result<(), String> {
    if values.contains(&value) {
        Ok(())
    } else {
        Err(format!("{field} 非法：{value}"))
    }
}

fn valid_time_range(start_at: i64, end_at: Option<i64>) -> Result<(), String> {
    if let Some(end) = end_at {
        if end < start_at {
            return Err("结束时间不能早于开始时间".into());
        }
    }
    Ok(())
}

fn ensure_work_active(state: &State<AppState>, work_id: Option<i64>) -> Result<(), String> {
    let Some(id) = work_id else { return Ok(()) };
    let work = with_db(&state, |db| {
        crate::db::work::WorkRepo::new(db.conn()).get(id)
    })?;
    match work {
        Some(w) if w.status != "archived" => Ok(()),
        Some(_) => Err("work_id 对应的 Work 已归档".into()),
        None => Err("work_id 无效：Work 不存在".into()),
    }
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
    let name = required(name, "工作目录名称")?;
    let path = required(path, "工作目录路径")?;
    let root = std::path::Path::new(&path);
    crate::storage::paths::validate_workspace_root(root)?;
    if !root.is_dir() {
        return Err("工作目录路径不存在或不是目录".into());
    }
    let path = root
        .canonicalize()
        .map_err(|e| format!("工作目录不可读：{e}"))?
        .to_string_lossy()
        .into_owned();
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
    match FileWatcher::start(app.clone(), PathBuf::from(&path)) {
        Ok(w) => state.set_watcher(w),
        Err(e) => eprintln!("[workspace] watcher start failed: {e}"),
    }

    spawn_workspace_reconcile(app, ws.id, path.clone());

    Ok(ws)
}

fn with_background_database<T>(
    path: &Path,
    action: impl FnOnce(&Database) -> DbResult<T>,
) -> DbResult<T> {
    let db = Database::open(path)?;
    action(&db)
}

pub(crate) fn spawn_workspace_reconcile(_app: AppHandle, workspace_id: i64, root: String) {
    std::thread::spawn(move || {
        let result = with_background_database(&crate::db::default_db_path(), |db| {
            inventory::reconcile(db, workspace_id, std::path::Path::new(&root))?;
            let report = crate::documents::indexer::reindex_workspace(
                db,
                workspace_id,
                std::path::Path::new(&root),
            )?;
            crate::cognition::refresh_all(db)?;
            Ok(report)
        });
        if let Err(err) = result {
            eprintln!("[documents] background reconcile/index failed: {err}");
        }
    });
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
    with_db(&state, |db| {
        workspace::recent_files(db, limit.unwrap_or(20))
    })
}

/// 已保存的 workspaces。
#[tauri::command]
pub fn get_workspaces(
    state: State<AppState>,
) -> Result<Vec<crate::db::workspace::Workspace>, String> {
    with_db(&state, |db| WorkspaceRepo::new(db.conn()).list())
}

#[tauri::command]
pub fn get_project_directories(
    state: State<AppState>,
) -> Result<Vec<crate::db::workspace::ProjectDirectory>, String> {
    with_db(&state, |db| {
        WorkspaceRepo::new(db.conn()).project_directories()
    })
}

fn stop_retired_watcher(state: &State<'_, AppState>) {
    let path = state.with_watcher(|w| w.root().to_string_lossy().into_owned());
    if let Some(path) = path {
        let active = with_db(state, |db| {
            Ok(WorkspaceRepo::new(db.conn())
                .list()?
                .iter()
                .any(|w| w.enabled && w.root_path == path))
        })
        .unwrap_or(false);
        if !active {
            state.clear_watcher_for(&path);
        }
    }
}

#[tauri::command]
pub fn remove_workspace(state: State<AppState>, workspace_id: i64) -> Result<(), String> {
    with_db(&state, |db| {
        WorkspaceRepo::new(db.conn()).delete(workspace_id)
    })?;
    stop_retired_watcher(&state);
    Ok(())
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
pub fn watcher_status(
    state: State<AppState>,
) -> Result<Option<crate::workspace::watcher::WatcherStatus>, String> {
    Ok(state.with_watcher(|w| w.status()))
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkspaceSyncStatus {
    pub workspace_id: Option<i64>,
    pub root: Option<String>,
    pub paused: bool,
    pub watching: bool,
    pub baseline_count: u64,
    pub last_scan: Option<i64>,
    pub last_warning: Option<String>,
}

/// 返回工作目录同步健康度：监听状态、快照文件数、上次扫描时间和警告。
#[tauri::command]
pub fn workspace_sync_status(
    state: State<AppState>,
    workspace_id: Option<i64>,
) -> Result<WorkspaceSyncStatus, String> {
    stop_retired_watcher(&state);
    let watcher = state.with_watcher(|w| w.status());
    let (watched_root, paused) = watcher
        .map(|status| (status.root, status.paused))
        .unwrap_or((None, false));
    let selected = with_db(&state, |db| {
        if let Some(id) = workspace_id {
            return Ok(Some(WorkspaceRepo::new(db.conn()).active(id)?));
        }
        Ok(watched_root.as_deref().and_then(|path| {
            WorkspaceRepo::new(db.conn())
                .list()
                .ok()?
                .into_iter()
                .find(|ws| ws.enabled && ws.root_path == path)
        }))
    })?;
    let root = selected.as_ref().map(|w| w.root_path.clone());
    let watching = root.is_some() && watched_root == root;
    let status = with_db(&state, |db| {
        let workspace = selected;
        let baseline_count = workspace
            .as_ref()
            .map(|ws| crate::db::workspace::WorkspaceFileStateRepo::new(db.conn()).count(ws.id))
            .transpose()?
            .unwrap_or(0);
        let events = crate::db::activity::ActivityRepo::new(db.conn()).query(
            None,
            None,
            None,
            None,
            None,
            Some(200),
        )?;
        let mut last_scan = None;
        let mut last_warning = None;
        for event in events {
            if !matches!(
                event.event_type.as_str(),
                "workspace.baseline" | "workspace.reconcile"
            ) {
                continue;
            }
            if workspace
                .as_ref()
                .is_some_and(|ws| event.workspace_id == Some(ws.id))
            {
                last_scan.get_or_insert(event.timestamp);
                if let Some(meta) = event.metadata_json {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&meta) {
                        last_warning = value
                            .get("warning")
                            .and_then(|v| v.as_str())
                            .filter(|v| !v.is_empty())
                            .map(str::to_string);
                    }
                }
            }
        }
        Ok((
            workspace.map(|ws| ws.id),
            baseline_count,
            last_scan,
            last_warning,
        ))
    })?;
    Ok(WorkspaceSyncStatus {
        workspace_id: status.0,
        root,
        paused: watching && paused,
        watching,
        baseline_count: status.1,
        last_scan: status.2,
        last_warning: status.3,
    })
}

/// 立即执行一次 metadata reconcile；正文不会被读取。
#[tauri::command]
pub fn workspace_rescan(
    state: State<AppState>,
    workspace_id: Option<i64>,
) -> Result<inventory::ReconcileReport, String> {
    let watched_root = state.with_watcher(|w| w.root().to_string_lossy().into_owned());
    with_db(&state, |db| {
        let repo = WorkspaceRepo::new(db.conn());
        let workspace = if let Some(id) = workspace_id {
            repo.active(id)?
        } else {
            repo.list()?
                .into_iter()
                .find(|w| w.enabled && Some(&w.root_path) == watched_root.as_ref())
                .ok_or_else(|| DbError::NotFound("尚未选择工作目录".into()))?
        };
        inventory::reconcile(db, workspace.id, std::path::Path::new(&workspace.root_path))
    })
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkspaceDocumentStatus {
    pub workspace_id: i64,
    pub total: u64,
    pub ready: u64,
    pub pending: u64,
    pub unsupported: u64,
    pub failed: u64,
    pub needs_ocr: u64,
    pub too_large: u64,
    pub last_indexed_at: Option<i64>,
}

#[tauri::command]
pub fn list_workspace_documents(
    state: State<AppState>,
    workspace_id: i64,
) -> Result<Vec<crate::db::documents::DocumentIndex>, String> {
    with_db(&state, |db| {
        crate::db::documents::DocumentIndexRepo::new(db.conn()).list(workspace_id)
    })
}

#[tauri::command]
pub fn workspace_document_status(
    state: State<AppState>,
    workspace_id: i64,
) -> Result<WorkspaceDocumentStatus, String> {
    with_db(&state, |db| {
        let items = crate::db::documents::DocumentIndexRepo::new(db.conn()).list(workspace_id)?;
        let mut result = WorkspaceDocumentStatus {
            workspace_id,
            total: items.len() as u64,
            ready: 0,
            pending: 0,
            unsupported: 0,
            failed: 0,
            needs_ocr: 0,
            too_large: 0,
            last_indexed_at: None,
        };
        for item in items {
            match item.extract_status.as_str() {
                "ready" => result.ready += 1,
                "pending" => result.pending += 1,
                "unsupported" => result.unsupported += 1,
                "needs_ocr" => result.needs_ocr += 1,
                "too_large" => result.too_large += 1,
                _ => result.failed += 1,
            }
            result.last_indexed_at = result.last_indexed_at.max(item.last_extracted_at);
        }
        Ok(result)
    })
}

#[tauri::command]
pub fn reindex_workspace_documents(
    state: State<AppState>,
    workspace_id: i64,
) -> Result<crate::documents::indexer::IndexReport, String> {
    let workspace = with_db(&state, |db| {
        WorkspaceRepo::new(db.conn()).active(workspace_id)
    })?;
    with_db(&state, |db| {
        crate::documents::indexer::reindex_workspace(
            db,
            workspace_id,
            std::path::Path::new(&workspace.root_path),
        )
    })
}

#[tauri::command]
pub fn link_work_workspace(
    state: State<AppState>,
    work_id: i64,
    workspace_id: i64,
    is_primary: bool,
) -> Result<(), String> {
    ensure_work_active(&state, Some(work_id))?;
    with_db(&state, |db| {
        crate::db::work::WorkRepo::new(db.conn())
            .get(work_id)?
            .ok_or_else(|| DbError::NotFound("work".into()))?;
        WorkspaceRepo::new(db.conn())
            .get(workspace_id)?
            .ok_or_else(|| DbError::NotFound("workspace".into()))?;
        crate::db::documents::WorkWorkspaceLinkRepo::new(db.conn()).link(
            work_id,
            workspace_id,
            is_primary,
        )
    })
}

#[tauri::command]
pub fn unlink_work_workspace(
    state: State<AppState>,
    work_id: i64,
    workspace_id: i64,
) -> Result<(), String> {
    with_db(&state, |db| {
        crate::db::documents::WorkWorkspaceLinkRepo::new(db.conn()).unlink(work_id, workspace_id)
    })?;
    stop_retired_watcher(&state);
    Ok(())
}

#[tauri::command]
pub fn list_work_workspaces(state: State<AppState>, work_id: i64) -> Result<Vec<i64>, String> {
    with_db(&state, |db| {
        crate::db::documents::WorkWorkspaceLinkRepo::new(db.conn()).list_by_work(work_id)
    })
}

/// Associate a project folder without replacing the global watched directory.
#[tauri::command]
pub fn attach_work_folder(
    app: AppHandle,
    state: State<AppState>,
    work_id: i64,
    path: String,
) -> Result<crate::db::workspace::Workspace, String> {
    ensure_work_active(&state, Some(work_id))?;
    let root = Path::new(&path);
    crate::storage::paths::validate_workspace_root(root)?;
    if !root.is_dir() {
        return Err("请选择一个有效文件夹".into());
    }
    let root = root
        .canonicalize()
        .map_err(|e| format!("文件夹不可读：{e}"))?;
    let path = root.to_string_lossy().to_string();
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "项目目录".into());
    let ws = with_db(&state, |db| {
        let tx = crate::db::write_transaction(db.conn())?;
        let repo = WorkspaceRepo::new(&tx);
        let existing = repo
            .list()?
            .into_iter()
            .find(|w| w.root_path.eq_ignore_ascii_case(&path));
        let ws = match existing {
            Some(w) => w,
            None => repo.insert(&name, &path)?,
        };
        tx.execute("INSERT INTO work_workspace_links(work_id,workspace_id,is_primary,created_at) VALUES (?1,?2,0,?3) ON CONFLICT(work_id,workspace_id) DO NOTHING",rusqlite::params![work_id,ws.id,crate::db::now_unix()])?;
        tx.commit()?;
        Ok(ws)
    })?;
    spawn_workspace_reconcile(app, ws.id, path);
    Ok(ws)
}

#[tauri::command]
pub fn list_workspace_works(state: State<AppState>, workspace_id: i64) -> Result<Vec<i64>, String> {
    with_db(&state, |db| {
        crate::db::documents::WorkWorkspaceLinkRepo::new(db.conn()).list_by_workspace(workspace_id)
    })
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
    let title = required(title, "任务标题")?;
    let priority = priority.unwrap_or_else(|| "normal".into());
    allowed(&priority, "任务优先级", &["low", "normal", "high"])?;
    ensure_work_active(&state, work_id)?;
    let notes = optional_trim(notes);
    with_db(&state, |db| {
        crate::db::task::TaskRepo::new(db.conn()).insert(
            work_id,
            &title,
            &priority,
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
    work_id: Option<i64>,
    title: String,
    priority: Option<String>,
    due_at: Option<i64>,
    notes: Option<String>,
) -> Result<(), String> {
    let title = required(title, "任务标题")?;
    let priority = priority.unwrap_or_else(|| "normal".into());
    allowed(&priority, "任务优先级", &["low", "normal", "high"])?;
    ensure_work_active(&state, work_id)?;
    let notes = optional_trim(notes);
    with_db(&state, |db| {
        crate::db::task::TaskRepo::new(db.conn()).update(
            id,
            work_id,
            &title,
            &priority,
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
    if let Some(Ok(Some(t))) =
        state.with_database(|db| crate::db::task::TaskRepo::new(db.conn()).get(id))
    {
        record_activity(
            &state,
            "task.completed",
            t.work_id,
            "task",
            t.id,
            &format!("完成 {}", t.title),
        );
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
    with_db(&state, |db| {
        crate::ai::lifecycle::remove_item(db, "task", id)
    })
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
    let title = required(title, "等待事项标题")?;
    ensure_work_active(&state, work_id)?;
    let waiting_for = optional_trim(waiting_for).unwrap_or_default();
    let notes = optional_trim(notes);
    with_db(&state, |db| {
        crate::db::task::WaitingRepo::new(db.conn()).insert(
            work_id,
            &title,
            &waiting_for,
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
    with_db(&state, |db| {
        crate::db::task::WaitingRepo::new(db.conn()).resolve(id)
    })
}

/// 编辑 waiting item。
#[tauri::command]
pub fn update_waiting(
    state: State<AppState>,
    id: i64,
    work_id: Option<i64>,
    title: String,
    waiting_for: Option<String>,
    started_at: i64,
    follow_up_at: Option<i64>,
    notes: Option<String>,
) -> Result<(), String> {
    let title = required(title, "等待事项标题")?;
    ensure_work_active(&state, work_id)?;
    let waiting_for = optional_trim(waiting_for).unwrap_or_default();
    let notes = optional_trim(notes);
    with_db(&state, |db| {
        crate::db::task::WaitingRepo::new(db.conn()).update(
            id,
            work_id,
            &title,
            &waiting_for,
            started_at,
            follow_up_at,
            notes.as_deref(),
        )
    })
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
    with_db(&state, |db| {
        crate::ai::lifecycle::remove_item(db, "waiting", id)
    })
}

// ---------- Inbox / Quick Capture ----------

/// Quick Capture：写入 Inbox。
#[tauri::command]
pub fn create_inbox_item(
    state: State<AppState>,
    content: String,
) -> Result<crate::db::inbox::InboxItem, String> {
    let content = content.trim().to_string();
    if content.is_empty() {
        return Err("收件箱内容不能为空".to_string());
    }
    with_db(&state, |db| {
        crate::db::inbox::InboxRepo::new(db.conn()).insert(&content)
    })
}

/// 列出 Inbox（未处理在前）。
#[tauri::command]
pub fn list_inbox(state: State<AppState>) -> Result<Vec<crate::db::inbox::InboxItem>, String> {
    with_db(&state, |db| {
        crate::db::inbox::InboxRepo::new(db.conn()).list()
    })
}

/// Inbox → Task。
#[tauri::command]
pub fn convert_inbox_to_task(
    state: State<AppState>,
    inbox_id: i64,
    work_id: Option<i64>,
    title: Option<String>,
    priority: Option<String>,
    due_at: Option<i64>,
    notes: Option<String>,
) -> Result<crate::db::task::Task, String> {
    let priority = priority.unwrap_or_else(|| "normal".into());
    allowed(&priority, "任务优先级", &["low", "normal", "high"])?;
    ensure_work_active(&state, work_id)?;
    let notes = optional_trim(notes);
    with_db(&state, |db| {
        let inbox = crate::db::inbox::InboxRepo::new(db.conn())
            .get(inbox_id)?
            .ok_or_else(|| DbError::NotFound("inbox_item".into()))?;
        if inbox.processed_at.is_some() {
            return Err(DbError::Migration(
                "这条收件箱内容已经处理，请勿重复转换".into(),
            ));
        }
        let tx = crate::db::write_transaction(db.conn())?;
        let title = optional_trim(title).unwrap_or_else(|| inbox.content.clone());
        let task = crate::db::task::TaskRepo::new(db.conn()).insert(
            work_id,
            &title,
            &priority,
            due_at,
            notes.as_deref().or(Some(inbox.content.as_str())),
        )?;
        crate::db::inbox::InboxRepo::new(db.conn()).mark_processed(inbox_id, "task", task.id)?;
        tx.commit()?;
        Ok(task)
    })
}

/// Inbox → Waiting。
#[tauri::command]
pub fn convert_inbox_to_waiting(
    state: State<AppState>,
    inbox_id: i64,
    work_id: Option<i64>,
    title: Option<String>,
    waiting_for: Option<String>,
    follow_up_at: Option<i64>,
) -> Result<crate::db::task::WaitingItem, String> {
    ensure_work_active(&state, work_id)?;
    let waiting_for = optional_trim(waiting_for).unwrap_or_default();
    with_db(&state, |db| {
        let inbox = crate::db::inbox::InboxRepo::new(db.conn())
            .get(inbox_id)?
            .ok_or_else(|| DbError::NotFound("inbox_item".into()))?;
        if inbox.processed_at.is_some() {
            return Err(DbError::Migration(
                "这条收件箱内容已经处理，请勿重复转换".into(),
            ));
        }
        let tx = crate::db::write_transaction(db.conn())?;
        let title = optional_trim(title).unwrap_or_else(|| inbox.content.clone());
        let waiting = crate::db::task::WaitingRepo::new(db.conn()).insert(
            work_id,
            &title,
            &waiting_for,
            follow_up_at,
            Some(&inbox.content),
        )?;
        crate::db::inbox::InboxRepo::new(db.conn())
            .mark_processed(inbox_id, "waiting", waiting.id)?;
        tx.commit()?;
        Ok(waiting)
    })
}

/// Inbox → Calendar。
#[tauri::command]
pub fn convert_inbox_to_calendar(
    state: State<AppState>,
    inbox_id: i64,
    work_id: Option<i64>,
    title: Option<String>,
    start_at: i64,
    end_at: Option<i64>,
    all_day: bool,
    kind: Option<String>,
) -> Result<crate::db::calendar::CalendarEvent, String> {
    let kind = kind.unwrap_or_else(|| "other".into());
    allowed(
        &kind,
        "日程类型",
        &[
            "meeting",
            "kol_visit",
            "deadline",
            "travel",
            "work_block",
            "other",
        ],
    )?;
    valid_time_range(start_at, end_at)?;
    ensure_work_active(&state, work_id)?;
    with_db(&state, |db| {
        let inbox = crate::db::inbox::InboxRepo::new(db.conn())
            .get(inbox_id)?
            .ok_or_else(|| DbError::NotFound("inbox_item".into()))?;
        if inbox.processed_at.is_some() {
            return Err(DbError::Migration(
                "这条收件箱内容已经处理，请勿重复转换".into(),
            ));
        }
        let tx = crate::db::write_transaction(db.conn())?;
        let title = optional_trim(title).unwrap_or_else(|| inbox.content.clone());
        let event = crate::db::calendar::CalendarRepo::new(db.conn()).insert(
            work_id,
            &title,
            start_at,
            end_at,
            all_day,
            &kind,
            None,
            Some(&inbox.content),
        )?;
        crate::db::inbox::InboxRepo::new(db.conn())
            .mark_processed(inbox_id, "calendar", event.id)?;
        tx.commit()?;
        Ok(event)
    })
}

/// 删除 Inbox 项。
#[tauri::command]
pub fn convert_inbox_to_resume_point(
    state: State<AppState>,
    inbox_id: i64,
    work_id: i64,
    current_state: String,
) -> Result<crate::db::work::ResumePoint, String> {
    ensure_work_active(&state, Some(work_id))?;
    let current_state = required(current_state, "项目进度")?;
    with_db(&state, |db| {
        crate::db::inbox::convert_to_progress(db.conn(), inbox_id, work_id, &current_state)
    })
}

/// 删除 Inbox 项。
#[tauri::command]
pub fn delete_inbox_item(state: State<AppState>, id: i64) -> Result<(), String> {
    with_db(&state, |db| {
        crate::ai::lifecycle::remove_item(db, "inbox", id)
    })
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
    let title = required(title, "日程标题")?;
    let kind = kind.unwrap_or_else(|| "other".into());
    allowed(
        &kind,
        "日程类型",
        &[
            "meeting",
            "kol_visit",
            "deadline",
            "travel",
            "work_block",
            "other",
        ],
    )?;
    valid_time_range(start_at, end_at)?;
    ensure_work_active(&state, work_id)?;
    let location = optional_trim(location);
    let notes = optional_trim(notes);
    with_db(&state, |db| {
        crate::db::calendar::CalendarRepo::new(db.conn()).insert(
            work_id,
            &title,
            start_at,
            end_at,
            all_day,
            &kind,
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
    work_id: Option<i64>,
    title: String,
    start_at: i64,
    end_at: Option<i64>,
    all_day: bool,
    kind: Option<String>,
    location: Option<String>,
    notes: Option<String>,
) -> Result<(), String> {
    let title = required(title, "日程标题")?;
    let kind = kind.unwrap_or_else(|| "other".into());
    allowed(
        &kind,
        "日程类型",
        &[
            "meeting",
            "kol_visit",
            "deadline",
            "travel",
            "work_block",
            "other",
        ],
    )?;
    valid_time_range(start_at, end_at)?;
    ensure_work_active(&state, work_id)?;
    let location = optional_trim(location);
    let notes = optional_trim(notes);
    with_db(&state, |db| {
        crate::db::calendar::CalendarRepo::new(db.conn()).update(
            id,
            work_id,
            &title,
            start_at,
            end_at,
            all_day,
            &kind,
            location.as_deref(),
            notes.as_deref(),
        )
    })
}

/// 删除日历事件。
#[tauri::command]
pub fn delete_calendar_event(state: State<AppState>, id: i64) -> Result<(), String> {
    with_db(&state, |db| {
        crate::ai::lifecycle::remove_item(db, "calendar", id)
    })
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
    let title = required(title, "Work 标题")?;
    let status = status.unwrap_or_else(|| "active".into());
    allowed(
        &status,
        "Work 状态",
        &["active", "paused", "waiting", "done", "archived"],
    )?;
    with_db(&state, |db| {
        crate::db::work::WorkRepo::new(db.conn()).insert(&title, &status)
    })
    .map(|work| {
        record_activity(
            &state,
            "work.created",
            Some(work.id),
            "work",
            work.id,
            &format!("创建 Work {}", work.title),
        );
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
    let title = required(title, "Work 标题")?;
    allowed(
        &status,
        "Work 状态",
        &["active", "paused", "waiting", "done", "archived"],
    )?;
    let summary = optional_trim(summary);
    with_db(&state, |db| {
        crate::db::work::WorkRepo::new(db.conn()).update(id, &title, &status, summary.as_deref())
    })
}

/// 归档 Work。
#[tauri::command]
pub fn archive_work(state: State<AppState>, id: i64) -> Result<(), String> {
    with_db(&state, |db| {
        crate::db::work::WorkRepo::new(db.conn()).archive(id)
    })
}

/// 删除 Work。保留任务、等待事项和日程，并解除它们与 Work 的关联。
#[tauri::command]
pub fn delete_work(
    state: State<AppState>,
    id: i64,
    confirmation_name: String,
    expected_revision: i64,
) -> Result<(), String> {
    let title = with_db(&state, |db| {
        let repo = crate::db::work::WorkRepo::new(db.conn());
        let title = repo
            .get(id)?
            .ok_or_else(|| DbError::NotFound("work".into()))?
            .title;
        repo.delete_confirmed(id, &confirmation_name, expected_revision)?;
        Ok(title)
    })?;
    stop_retired_watcher(&state);
    record_activity(
        &state,
        "work.deleted",
        None,
        "work",
        id,
        &format!("删除 Work {title}"),
    );
    Ok(())
}

/// 列出 Works（可按 status 过滤，默认按最近更新排序）。
#[tauri::command]
pub fn list_works(
    state: State<AppState>,
    status: Option<String>,
) -> Result<Vec<crate::db::work::Work>, String> {
    with_db(&state, |db| {
        crate::db::work::WorkRepo::new(db.conn()).list(status.as_deref())
    })
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
        let recent_activity = crate::db::activity::ActivityRepo::new(db.conn()).query(
            None,
            None,
            Some(id),
            None,
            None,
            Some(30),
        )?;

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
    let current_state = current_state.trim().to_string();
    let next_step = next_step.trim().to_string();
    let remember = remember.trim().to_string();
    if current_state.is_empty() && next_step.is_empty() {
        return Err("当前状态或下一步至少填写一项".into());
    }
    ensure_work_active(&state, Some(work_id))?;
    with_db(&state, |db| {
        crate::db::work::ResumePointRepo::new(db.conn()).insert(
            work_id,
            &current_state,
            &next_step,
            &remember,
            "manual",
        )
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
        crate::ai::lifecycle::remove_item(db, "resume_point", id)
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
    let path = required(path, "文件路径")?;
    if !std::path::Path::new(&path).exists() {
        return Err("文件路径不存在，未创建关联".into());
    }
    ensure_work_active(&state, Some(work_id))?;
    let label = optional_trim(label);
    with_db(&state, |db| {
        crate::db::workspace::WorkFileRefRepo::new(db.conn()).insert(
            work_id,
            workspace_id,
            &path,
            label.as_deref(),
        )
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
        crate::db::workspace::WorkFileRefRepo::new(db.conn()).update(id, label.as_deref(), pinned)
    })
}

/// 移除文件关联。
#[tauri::command]
pub fn remove_work_file_ref(state: State<AppState>, id: i64) -> Result<(), String> {
    with_db(&state, |db| {
        crate::db::workspace::WorkFileRefRepo::new(db.conn()).delete(id)
    })
}

// ---------- AI Provider / Morning Brief（指南 §22） ----------

/// 列出已保存的 Providers（不含 API Key）。
#[tauri::command]
pub fn list_providers(
    state: State<AppState>,
) -> Result<Vec<crate::db::provider::ProviderSetting>, String> {
    with_db(&state, |db| {
        crate::db::provider::ProviderRepo::new(db.conn()).list()
    })
}

/// 保存 Provider（含 API Key → Windows Credential Manager）。
/// 传入 `api_key: Some("")` 表示不修改 key（编辑场景）。
#[tauri::command]
pub fn save_provider(
    state: State<AppState>,
    id: Option<i64>,
    display_name: String,
    provider_type: String,
    base_url: String,
    model: String,
    enabled: bool,
    api_key: Option<String>,
) -> Result<crate::db::provider::ProviderSetting, String> {
    let display_name = required(display_name, "Provider 显示名称")?;
    let provider_type = required(provider_type, "Provider 类型")?;
    let base_url = required(base_url, "Provider Base URL")?;
    let model = required(model, "Provider 模型")?;
    let saved = match id {
        Some(pid) => {
            with_db(&state, |db| {
                crate::db::provider::ProviderRepo::new(db.conn()).update(
                    pid,
                    &display_name,
                    &provider_type,
                    &base_url,
                    &model,
                    enabled,
                )
            })?;
            with_db(&state, |db| {
                crate::db::provider::ProviderRepo::new(db.conn()).get(pid)
            })?
            .ok_or_else(|| "Provider 不存在".to_string())?
        }
        None => with_db(&state, |db| {
            crate::db::provider::ProviderRepo::new(db.conn()).insert(
                &display_name,
                &provider_type,
                &base_url,
                &model,
                enabled,
            )
        })?,
    };

    // 保存 API Key（Some("") 视为不修改）
    if let Some(key) = api_key {
        if !key.is_empty() {
            crate::ai::provider::save_api_key(&saved.credential_ref, &key)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(saved)
}

/// 删除 Provider（并清除 keyring 中的 key）。
#[tauri::command]
pub fn delete_provider(state: State<AppState>, id: i64) -> Result<(), String> {
    let provider = with_db(&state, |db| {
        crate::db::provider::ProviderRepo::new(db.conn()).get(id)
    })?
    .ok_or_else(|| "Provider 不存在".to_string())?;
    with_db(&state, |db| {
        crate::db::provider::ProviderRepo::new(db.conn()).delete(id)
    })?;
    let _ = crate::ai::provider::delete_api_key(&provider.credential_ref);
    Ok(())
}

/// 查询某 Provider 是否已配置 API Key（不返回 key 本身）。
#[tauri::command]
pub fn provider_has_key(state: State<AppState>, id: i64) -> Result<bool, String> {
    // 确认 provider 存在
    let exists = with_db(&state, |db| {
        crate::db::provider::ProviderRepo::new(db.conn()).get(id)
    })?
    .is_some();
    if !exists {
        return Ok(false);
    }
    let provider = with_db(&state, |db| {
        crate::db::provider::ProviderRepo::new(db.conn()).get(id)
    })?
    .ok_or_else(|| "Provider 不存在".to_string())?;
    Ok(crate::ai::provider::has_api_key(&provider.credential_ref))
}

/// 测试连接：使用 keyring 中的 key 发送最小请求。
#[tauri::command]
pub async fn test_provider_connection(
    state: State<'_, AppState>,
    id: i64,
) -> Result<String, String> {
    let provider = with_db(&state, |db| {
        crate::db::provider::ProviderRepo::new(db.conn()).get(id)
    })?
    .ok_or_else(|| "Provider 不存在".to_string())?;

    let key = crate::ai::provider::get_api_key(&provider.credential_ref)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "未配置 API Key".to_string())?;

    let resp = crate::ai::provider::test_connection(&provider, &key)
        .await
        .map_err(|e| e.to_string())?;
    Ok(format!(
        "连接成功（model={}）：{}",
        resp.model.unwrap_or_default(),
        resp.content
    ))
}

fn validate_brief_range(start: i64, end: i64) -> Result<(), String> {
    if start >= end {
        return Err("Brief 时间范围必须满足开始早于结束".into());
    }
    if end.saturating_sub(start) > 90 * 86_400 {
        return Err("Brief 时间范围不能超过 90 天".into());
    }
    Ok(())
}

async fn generate_brief_inner(
    state: &AppState,
    date: &str,
    period_start: i64,
    period_end: i64,
    today_start: i64,
    today_end: i64,
    locale: &str,
    force: bool,
) -> Result<crate::ai::brief::BriefResult, String> {
    validate_brief_range(period_start, period_end)?;
    let (cached, snapshot, hash) = state
        .with_database(|db| {
            crate::ai::brief::prepare(
                db,
                date,
                period_start,
                period_end,
                today_start,
                today_end,
                locale,
                force,
            )
        })
        .ok_or_else(|| "数据库未初始化".to_string())?
        .map_err(|e| e.to_string())?;
    if let Some(brief) = cached {
        let source_preview = brief
            .source_snapshot_json
            .as_deref()
            .and_then(|json| serde_json::from_str::<crate::ai::brief::BriefSnapshot>(json).ok())
            .map(|cached_snapshot| {
                let mut all = cached_snapshot.works;
                all.extend(cached_snapshot.tasks_open);
                all.extend(cached_snapshot.waiting);
                all.extend(cached_snapshot.calendar);
                all.into_iter().take(30).collect()
            })
            .unwrap_or_default();
        let source_counts = brief
            .source_snapshot_json
            .as_deref()
            .and_then(|json| serde_json::from_str::<crate::ai::brief::BriefSnapshot>(json).ok())
            .map(|s| s.source_counts)
            .unwrap_or_else(|| snapshot.source_counts.clone());
        return Ok(crate::ai::brief::BriefResult {
            content: brief.content.clone(),
            ai_used: brief.ai_used,
            warning: brief.warning.clone(),
            period_start,
            period_end,
            locale: locale.into(),
            source_counts,
            source_preview,
            brief,
        });
    }

    let routed_model = state
        .with_database(|db| {
            let repo = crate::db::provider::ProviderCatalogRepo::new(db.conn());
            crate::ai::router::resolve(
                &repo,
                &crate::ai::router::KeyringCredentialSource,
                "daily_brief",
            )
        })
        .ok_or_else(|| "数据库未初始化".to_string())?;
    let mut warning = None;
    let mut ai_used = false;
    let mut content = crate::ai::brief::render_local(&snapshot);
    let provider_name = if let Ok(resolved) = routed_model {
        match crate::ai::provider::get_api_key(&resolved.connection.credential_ref) {
            Ok(Some(key)) => {
                match crate::ai::brief::call(&resolved.connection, &resolved.model, &key, &snapshot)
                    .await
                {
                    Ok(value) if !value.trim().is_empty() => {
                        ai_used = true;
                        content = value;
                        resolved.connection.display_name
                    }
                    Ok(_) => {
                        warning = Some(if locale == "en-US" {
                            "AI returned empty content; local summary used".into()
                        } else {
                            "AI 返回为空，已使用本地摘要".into()
                        });
                        resolved.connection.display_name
                    }
                    Err(err) => {
                        warning = Some(if locale == "en-US" {
                            format!("AI unavailable; local summary used ({err})")
                        } else {
                            format!("AI 不可用，已使用本地摘要（{err}）")
                        });
                        resolved.connection.display_name
                    }
                }
            }
            Ok(None) => {
                warning = Some(if locale == "en-US" {
                    "No API key; local summary used".into()
                } else {
                    "未配置 API Key，已使用本地摘要".into()
                });
                "local".into()
            }
            Err(err) => {
                warning = Some(if locale == "en-US" {
                    format!("Credential unavailable; local summary used ({err})")
                } else {
                    format!("凭据不可用，已使用本地摘要（{err}）")
                });
                "local".into()
            }
        }
    } else {
        warning = Some(if locale == "en-US" {
            "No daily brief model is selected. The complete local summary is shown; choose a model in Settings for AI-enhanced analysis.".into()
        } else {
            "尚未选择每日简报模型，当前展示完整本地摘要；可在“设置”中选择模型以启用 AI 增强分析。"
                .into()
        });
        "local".into()
    };
    let brief = state
        .with_database(|db| {
            crate::ai::brief::save_with_meta(
                db,
                date,
                &provider_name,
                &content,
                &hash,
                &snapshot,
                ai_used,
                warning.as_deref(),
            )
        })
        .ok_or_else(|| "数据库未初始化".to_string())?
        .map_err(|e| e.to_string())?;
    let mut source_preview = snapshot.works.clone();
    source_preview.extend(snapshot.tasks_open.clone());
    source_preview.extend(snapshot.waiting.clone());
    source_preview.extend(snapshot.calendar.clone());
    source_preview.truncate(30);
    Ok(crate::ai::brief::BriefResult {
        brief,
        content,
        source_counts: snapshot.source_counts.clone(),
        source_preview,
        ai_used,
        warning,
        period_start,
        period_end,
        locale: locale.into(),
    })
}

/// 预览结构化事实，不调用 AI，也不写入 daily_briefs。
#[tauri::command]
pub fn preview_brief_snapshot(
    state: State<AppState>,
    date: String,
    period_start: i64,
    period_end: i64,
    today_start: i64,
    today_end: i64,
    locale: Option<String>,
) -> Result<crate::ai::brief::BriefSnapshot, String> {
    validate_brief_range(period_start, period_end)?;
    with_db(&state, |db| {
        crate::ai::brief::build_snapshot(
            db,
            &date,
            period_start,
            period_end,
            today_start,
            today_end,
            locale.as_deref().unwrap_or("zh-CN"),
        )
    })
}

/// 新 Brief API：任何 Provider 配置状态下都返回本地或 AI 增强摘要。
#[tauri::command]
pub async fn generate_brief(
    state: State<'_, AppState>,
    date: String,
    period_start: i64,
    period_end: i64,
    today_start: i64,
    today_end: i64,
    locale: Option<String>,
    force: Option<bool>,
) -> Result<crate::ai::brief::BriefResult, String> {
    generate_brief_inner(
        &state,
        &date,
        period_start,
        period_end,
        today_start,
        today_end,
        locale.as_deref().unwrap_or("zh-CN"),
        force.unwrap_or(false),
    )
    .await
}

/// 兼容旧前端的今日 Brief wrapper，内部使用新 API。
#[tauri::command]
pub async fn generate_morning_brief(
    state: State<'_, AppState>,
    date: String,
    yesterday_start: i64,
    yesterday_end: i64,
    day_start: i64,
    day_end: i64,
    locale: Option<String>,
    force: Option<bool>,
) -> Result<crate::db::brief::DailyBrief, String> {
    Ok(generate_brief_inner(
        &state,
        &date,
        yesterday_start,
        yesterday_end,
        day_start,
        day_end,
        locale.as_deref().unwrap_or("zh-CN"),
        force.unwrap_or(false),
    )
    .await?
    .brief)
}

#[tauri::command]
pub fn list_briefs(
    state: State<AppState>,
    limit: Option<u32>,
) -> Result<Vec<crate::db::brief::DailyBrief>, String> {
    let limit = limit.unwrap_or(20).clamp(1, 50);
    with_db(&state, |db| {
        crate::db::brief::BriefRepo::new(db.conn()).list(limit)
    })
}

#[tauri::command]
pub fn get_brief(
    state: State<AppState>,
    id: i64,
) -> Result<Option<crate::db::brief::DailyBrief>, String> {
    with_db(&state, |db| {
        crate::db::brief::BriefRepo::new(db.conn()).get(id)
    })
}

/// 读取今日已缓存 Brief（无则 None）。
#[tauri::command]
pub fn get_morning_brief(
    state: State<AppState>,
    date: String,
) -> Result<Option<crate::db::brief::DailyBrief>, String> {
    with_db(&state, |db| {
        crate::db::brief::BriefRepo::new(db.conn()).latest_for_date(&date)
    })
}

/// 搜索结果分组项。
#[derive(Debug, Clone, serde::Serialize)]
pub struct SearchResults {
    pub works: Vec<crate::db::work::Work>,
    pub files: Vec<FileHit>,
    pub tasks: Vec<crate::db::task::Task>,
    pub waiting: Vec<crate::db::task::WaitingItem>,
    pub calendar: Vec<crate::db::calendar::CalendarEvent>,
    pub inbox: Vec<crate::db::inbox::InboxItem>,
    pub resume_points: Vec<crate::db::work::ResumePoint>,
    pub activity: Vec<crate::db::activity::ActivityEvent>,
}

/// 文件命中：来自 work_file_refs 与 activity 中出现的路径（已索引，不遍历磁盘）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct FileHit {
    pub path: String,
    pub label: Option<String>,
    pub work_id: Option<i64>,
}

/// LIKE 转义：`%`、`_` 与反斜杠。
fn like_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn like_param(q: &str) -> String {
    format!("%{}%", like_escape(q))
}

const SEARCH_LIMIT: usize = 8;

/// 轻量搜索（SQLite LIKE，不建向量索引，不遍历磁盘）。
#[tauri::command]
pub fn search(state: State<AppState>, query: String) -> Result<SearchResults, String> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(SearchResults {
            works: Vec::new(),
            files: Vec::new(),
            tasks: Vec::new(),
            waiting: Vec::new(),
            calendar: Vec::new(),
            inbox: Vec::new(),
            resume_points: Vec::new(),
            activity: Vec::new(),
        });
    }
    with_db(&state, |db| {
        let conn = db.conn();
        let like = like_param(q);

        // 简单 LIMIT 查询（各表独立 LIKE）
        let works = crate::db::work::WorkRepo::new(conn).search(&like, SEARCH_LIMIT)?;
        let tasks = crate::db::task::TaskRepo::new(conn).search(&like, SEARCH_LIMIT)?;
        let waiting = crate::db::task::WaitingRepo::new(conn).search(&like, SEARCH_LIMIT)?;
        let calendar = crate::db::calendar::CalendarRepo::new(conn).search(&like, SEARCH_LIMIT)?;
        let inbox = crate::db::inbox::InboxRepo::new(conn).search(&like, SEARCH_LIMIT)?;
        let resume_points =
            crate::db::work::ResumePointRepo::new(conn).search(&like, SEARCH_LIMIT)?;
        let activity = crate::db::activity::ActivityRepo::new(conn).search(&like, SEARCH_LIMIT)?;

        // 文件：work_file_refs（label/path）+ activity 中的 path（去重）
        let mut files = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for f in crate::db::workspace::WorkFileRefRepo::new(conn).search(&like, SEARCH_LIMIT)? {
            if seen.insert(f.path.clone()) {
                files.push(FileHit {
                    path: f.path,
                    label: f.label,
                    work_id: Some(f.work_id),
                });
            }
        }
        for a in &activity {
            if let Some(p) = &a.path {
                if p.to_lowercase().contains(&q.to_lowercase()) && seen.insert(p.clone()) {
                    files.push(FileHit {
                        path: p.clone(),
                        label: None,
                        work_id: a.work_id,
                    });
                }
            }
        }

        Ok(SearchResults {
            works,
            files,
            tasks,
            waiting,
            calendar,
            inbox,
            resume_points,
            activity,
        })
    })
}

/// Continue 列表项：Work + 最新 Resume Point + 最近活动时间 + 相关文件。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContinueWork {
    pub work: crate::db::work::Work,
    pub latest_resume: Option<crate::db::work::ResumePoint>,
    pub last_activity_at: Option<i64>,
    pub files: Vec<crate::db::workspace::WorkFileRef>,
}

/// Today 页面聚合数据（指南 §20 排序规则）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct TodayData {
    /// Continue：未 done 的 Works，按最近活动/更新时间排序。
    pub continue_works: Vec<ContinueWork>,
    /// 今日 tasks（含 overdue）。
    pub today_tasks: Vec<crate::db::task::Task>,
    /// 今日 calendar 事件。
    pub today_calendar: Vec<crate::db::calendar::CalendarEvent>,
    /// 需要跟进/已到期的 waiting。
    pub waiting_followups: Vec<crate::db::task::WaitingItem>,
    /// 未处理 Inbox。
    pub inbox_pending: Vec<crate::db::inbox::InboxItem>,
}

/// 获取 Today 页面数据。
/// `day_start` / `day_end` 为"今天"的起止（Unix 秒，由前端按本地时区计算）。
#[tauri::command]
pub fn get_today(
    state: State<AppState>,
    day_start: i64,
    day_end: i64,
) -> Result<TodayData, String> {
    with_db(&state, |db| {
        let work_repo = crate::db::work::WorkRepo::new(db.conn());
        let resume_repo = crate::db::work::ResumePointRepo::new(db.conn());
        let activity_repo = crate::db::activity::ActivityRepo::new(db.conn());
        let file_repo = crate::db::workspace::WorkFileRefRepo::new(db.conn());

        // Continue：未 done 的 Works（active/paused/waiting）
        let mut continue_works: Vec<ContinueWork> = work_repo
            .list(None)?
            .into_iter()
            .filter(|w| w.status != "done" && w.status != "archived")
            .map(|work| {
                let latest_resume = resume_repo.latest_for_work(work.id).unwrap_or(None);
                let last_activity_at = activity_repo
                    .query(None, None, Some(work.id), None, None, Some(1))
                    .ok()
                    .and_then(|v| v.first().map(|a| a.timestamp))
                    .or(Some(work.updated_at));
                let files = file_repo.list_by_work(work.id).unwrap_or_default();
                ContinueWork {
                    work,
                    latest_resume,
                    last_activity_at,
                    files,
                }
            })
            .collect();

        // 排序（指南 §20.4）：最近有 Activity 且未 done 的 Work 在前
        continue_works.sort_by_key(|c| std::cmp::Reverse(c.last_activity_at.unwrap_or(0)));

        // 今日 tasks：due 在今天 + overdue（due ≤ 今天结束，未完成）
        let all_tasks = crate::db::task::TaskRepo::new(db.conn()).list(None, None)?;
        let today_tasks: Vec<_> = all_tasks
            .into_iter()
            .filter(|t| t.status != "done" && t.due_at.is_some_and(|d| d <= day_end))
            .collect();

        // 今日 calendar
        let today_calendar =
            crate::db::calendar::CalendarRepo::new(db.conn()).list_between(day_start, day_end)?;

        // waiting follow-up：open 且 follow_up_at ≤ 今天结束（含到期）
        let all_waiting = crate::db::task::WaitingRepo::new(db.conn()).list(None, None)?;
        let waiting_followups: Vec<_> = all_waiting
            .into_iter()
            .filter(|w| w.status == "open" && w.follow_up_at.is_some_and(|f| f <= day_end))
            .collect();

        // 未处理 Inbox
        let inbox_pending: Vec<_> = crate::db::inbox::InboxRepo::new(db.conn())
            .list()?
            .into_iter()
            .filter(|i| i.processed_at.is_none())
            .collect();

        Ok(TodayData {
            continue_works,
            today_tasks,
            today_calendar,
            waiting_followups,
            inbox_pending,
        })
    })
}

// ---------- 生命周期 / 通知设置（指南 §23） ----------

/// 启用/禁用 Windows 自启动（background 模式）。
#[tauri::command]
pub fn set_autostart(enabled: bool) -> Result<bool, String> {
    crate::lifecycle::set_autostart(enabled).map_err(|e| e.to_string())?;
    Ok(crate::lifecycle::is_autostart_enabled())
}

/// 查询自启动状态。
#[tauri::command]
pub fn autostart_status() -> Result<bool, String> {
    Ok(crate::lifecycle::is_autostart_enabled())
}

/// 开关通知提醒。
#[tauri::command]
pub fn set_notifications_enabled(state: State<AppState>, enabled: bool) -> Result<bool, String> {
    with_db(&state, |db| {
        crate::db::provider::AppSettingsRepo::new(db.conn()).set(
            "notifications_enabled",
            if enabled { "true" } else { "false" },
        )
    })?;
    Ok(enabled)
}

/// 设置提醒提前量（分钟）。
#[tauri::command]
pub fn set_reminder_lead_minutes(state: State<AppState>, minutes: i64) -> Result<i64, String> {
    let clamped = minutes.clamp(1, 1440);
    with_db(&state, |db| {
        crate::db::provider::AppSettingsRepo::new(db.conn())
            .set("reminder_lead_minutes", &clamped.to_string())
    })?;
    Ok(clamped)
}

/// 立即执行一次提醒检查（调试/验收用）。
#[tauri::command]
pub fn check_reminders_now(
    app: tauri::AppHandle,
) -> Result<Vec<crate::notifications::Reminder>, String> {
    Ok(crate::notifications::check_once(&app))
}

/// 读取应用设置项。
#[tauri::command]
pub fn app_settings_get(state: State<AppState>, key: String) -> Result<Option<String>, String> {
    with_db(&state, |db| {
        crate::db::provider::AppSettingsRepo::new(db.conn()).get(&key)
    })
}

/// 写入应用设置项。
#[tauri::command]
pub fn app_settings_set(state: State<AppState>, key: String, value: String) -> Result<(), String> {
    with_db(&state, |db| {
        crate::db::provider::AppSettingsRepo::new(db.conn()).set(&key, &value)
    })
}

#[cfg(test)]
mod validation_tests {
    use super::{allowed, required, valid_time_range, with_background_database};
    use crate::app_state::AppState;
    use crate::db::Database;

    #[test]
    fn required_rejects_blank_and_trims() {
        assert!(required("   ".into(), "标题").is_err());
        assert_eq!(required("  方案  ".into(), "标题").unwrap(), "方案");
    }

    #[test]
    fn enums_and_time_ranges_are_checked() {
        assert!(allowed("high", "优先级", &["low", "normal", "high"]).is_ok());
        assert!(allowed("urgent", "优先级", &["low", "normal", "high"]).is_err());
        assert!(valid_time_range(10, Some(10)).is_ok());
        assert!(valid_time_range(10, Some(9)).is_err());
    }

    #[test]
    fn background_database_work_does_not_hold_the_app_state_lock() {
        let root = std::env::temp_dir().join(format!("msl-background-db-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("test.db");
        let state = std::sync::Arc::new(AppState::default());
        state.set_database(Database::open(&path).unwrap());
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let worker_path = path.clone();

        let worker = std::thread::spawn(move || {
            with_background_database(&worker_path, |_db| {
                started_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                Ok(())
            })
            .unwrap();
        });

        started_rx.recv().unwrap();
        let version = state
            .with_database(|db| {
                db.conn()
                    .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                        row.get::<_, i64>(0)
                    })
                    .unwrap()
            })
            .unwrap();
        assert_eq!(version, 23);
        release_tx.send(()).unwrap();
        worker.join().unwrap();
        let _ = std::fs::remove_dir_all(root);
    }
}
