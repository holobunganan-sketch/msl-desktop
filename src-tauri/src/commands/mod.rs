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
