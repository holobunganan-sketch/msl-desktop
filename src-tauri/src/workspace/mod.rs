//! Workspace 服务：真实 Windows 工作目录的窗口（指南 §7.2 / §17）。
//!
//! 职责：
//! - 目录列表（lazy：只读当前层，不递归、不全文扫描）；
//! - 文件 metadata（name / is_dir / modified / size）；
//! - 打开文件（系统默认程序）与 Reveal in Explorer；
//! - 最近修改文件；
//! - 文件 watcher 的启动/停止（见 watcher 子模块）。

pub mod inventory;
pub mod watcher;

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Serialize;

use crate::db::{Database, DbResult};

/// 单层目录项（lazy loading：一次只返回一个目录的内容）。
#[derive(Debug, Clone, Serialize)]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    /// Unix 秒；无法读取时为 0。
    pub modified: i64,
    /// 字节；目录为 0。
    pub size: u64,
}

/// 列出目录单层内容：目录在前、按名称排序。
/// 不递归、不读取文件正文。
pub fn list_dir(path: &Path) -> std::io::Result<Vec<DirEntry>> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue, // 跳过无法读取的项（权限/占位文件）
        };
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        entries.push(DirEntry {
            name,
            path: entry.path().to_string_lossy().into_owned(),
            is_dir: meta.is_dir(),
            modified,
            size: if meta.is_file() { meta.len() } else { 0 },
        });
    }
    // 目录在前，组内按名称（不区分大小写）排序
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
}

/// 用系统默认程序打开文件/目录。
pub fn open_path(path: &Path) -> std::io::Result<()> {
    let path_str = path.to_string_lossy().into_owned();
    tauri_plugin_opener::open_path(path_str, None::<&str>)
        .map_err(|e| std::io::Error::other(e.to_string()))
}

/// 在 Explorer 中定位文件（选中）。
pub fn reveal_in_explorer(path: &Path) -> std::io::Result<()> {
    tauri_plugin_opener::reveal_item_in_dir(path.to_string_lossy().as_ref())
        .map_err(|e| std::io::Error::other(e.to_string()))
}

/// 最近修改的文件（来自 activity_events 中 path 非空的 file.* 事件，
/// 按时间倒序，按 path 去重）。
pub fn recent_files(db: &Database, limit: u32) -> DbResult<Vec<RecentFile>> {
    use crate::db::activity::ActivityRepo;
    let events = ActivityRepo::new(db.conn()).query(
        None,
        None,
        None,
        None,
        None,
        Some(limit.saturating_mul(20)),
    )?;
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for ev in events {
        if !ev.event_type.starts_with("file.") {
            continue;
        }
        let Some(p) = ev.path.clone() else { continue };
        if !seen.insert(p.clone()) {
            continue;
        }
        out.push(RecentFile {
            path: p,
            event_type: ev.event_type,
            display_text: ev.display_text,
            timestamp: ev.timestamp,
        });
        if out.len() as u32 >= limit {
            break;
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize)]
pub struct RecentFile {
    pub path: String,
    pub event_type: String,
    pub display_text: String,
    pub timestamp: i64,
}

/// 判断路径是否在受管 workspace 之内（用于 watcher 事件归属）。
pub fn path_is_within(path: &Path, root: &Path) -> bool {
    path.starts_with(root)
}

/// 规范化路径（去掉末尾分隔符），用于一致性比较。
pub fn normalize(path: &Path) -> PathBuf {
    let mut p = path.to_path_buf();
    while p.as_os_str().len() > 3 && p.to_string_lossy().ends_with(['/', '\\']) {
        p.pop();
    }
    p
}
