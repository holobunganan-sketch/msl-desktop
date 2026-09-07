//! 文件监听（指南 §2.4 / §11 / §17）。
//!
//! 基于 `notify`（Windows ReadDirectoryChangesW 后端）：
//! - 事件驱动，UI 销毁后仍继续运行（挂在 Resident Core）；
//! - 记录 create / modify / rename / move / delete；
//! - Office 临时文件过滤（指南 §7.7）；
//! - 同一路径同一事件族 2 秒窗口 debounce/coalesce（指南 §11）；
//! - 不读取文件正文，只写 activity_events。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Manager};

use crate::app_state::AppState;
use crate::db::activity::{ActivityEvent, ActivityRepo};
use crate::db::{now_unix, Database};

use notify::event::ModifyKind;

/// debounce 窗口（秒）：同一路径同一事件族在窗口内合并为一个 Activity。
pub const DEBOUNCE_SECS: i64 = 2;

/// 临时文件过滤规则（指南 §7.7）：只影响 Activity 记录，不影响真实文件系统。
pub fn is_temp_file(path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if name.starts_with("~$") {
        return true;
    }
    if name == "Thumbs.db" || name == "desktop.ini" {
        return true;
    }
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    matches!(ext.as_str(), "tmp" | "swp")
}

/// 事件族：用于 debounce 分组。
fn event_family(kind: &EventKind) -> &'static str {
    match kind {
        EventKind::Create(_) => "created",
        EventKind::Remove(_) => "deleted",
        EventKind::Modify(m) if matches!(m, ModifyKind::Name(_)) => "renamed",
        EventKind::Modify(_) => "modified",
        _ => "other",
    }
}

/// 将 notify 事件映射为 activity event_type（指南 §6.9）。
fn event_type_for(kind: &EventKind) -> &'static str {
    match kind {
        EventKind::Create(_) => "file.created",
        EventKind::Remove(_) => "file.deleted",
        EventKind::Modify(m) if matches!(m, ModifyKind::Name(_)) => "file.renamed",
        EventKind::Modify(_) => "file.modified",
        _ => "file.changed",
    }
}

fn display_text_for(event_type: &str, path: &Path, old_path: Option<&Path>) -> String {
    let p = path.display().to_string();
    match (event_type, old_path) {
        ("file.created", _) => format!("新建文件 {p}"),
        ("file.deleted", _) => format!("删除文件 {p}"),
        ("file.renamed", Some(old)) => format!("重命名 {} → {}", old.display(), p),
        ("file.renamed", None) => format!("重命名 {p}"),
        ("file.modified", _) => format!("修改文件 {p}"),
        _ => format!("文件变化 {p}"),
    }
}

/// 处理单个文件事件并写入 Activity（可测试的纯逻辑）。
/// 返回写入的 Activity；被临时过滤或 debounce 合并时返回空。
pub fn handle_file_event(
    db: &Database,
    event: &Event,
    debounce: &mut HashMap<String, i64>,
) -> Vec<ActivityEvent> {
    if event.paths.is_empty() {
        return Vec::new();
    }
    let kind_family = event_family(&event.kind);
    let event_type = event_type_for(&event.kind);

    // 旧路径（rename 事件可能携带两个路径）
    let old_path: Option<&Path> = if event_type == "file.renamed" && event.paths.len() >= 2 {
        Some(event.paths[0].as_path())
    } else {
        None
    };
    // 新路径：rename 取最后一个，其余取第一个
    let path: &Path = if event_type == "file.renamed" && event.paths.len() >= 2 {
        event.paths.last().unwrap()
    } else {
        &event.paths[0]
    };

    if is_temp_file(path) {
        return Vec::new();
    }

    // debounce：同一路径同一事件族在窗口内合并
    let now = now_unix();
    let key = format!("{kind_family}|{}", path.display());
    if let Some(&last) = debounce.get(&key) {
        if now - last < DEBOUNCE_SECS {
            return Vec::new();
        }
    }
    debounce.insert(key, now);

    let display_text = display_text_for(event_type, path, old_path);
    let metadata_json =
        old_path.map(|o| serde_json::json!({ "old_path": o.to_string_lossy() }).to_string());
    let repo = ActivityRepo::new(db.conn());
    let workspace_id = crate::db::workspace::WorkspaceFileStateRepo::new(db.conn())
        .workspace_for_path(&path.to_string_lossy())
        .ok()
        .flatten()
        .map(|ws| ws.id);
    match repo.insert(
        event_type,
        workspace_id,
        None,
        Some("file"),
        None,
        Some(&path.to_string_lossy()),
        &display_text,
        metadata_json.as_deref(),
        Some(&format!("file|{event_type}|{}", path.display())),
    ) {
        Ok(ev) => {
            if let Err(err) =
                crate::workspace::inventory::sync_live_state(db, event_type, path, old_path)
            {
                eprintln!("[workspace] live state update failed: {err}");
            }
            vec![ev]
        }
        Err(err) => {
            eprintln!("[workspace] activity write failed: {err}");
            Vec::new()
        }
    }
}

/// 监听器状态（IPC 查询用）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct WatcherStatus {
    pub paused: bool,
    pub root: Option<String>,
}

/// 文件监听器：持有 notify watcher，挂载到 Resident Core。
/// `watcher` 字段必须保持存活——drop 会停止监听（"精巧"：不监听时直接销毁）。
pub struct FileWatcher {
    #[allow(dead_code)] // 仅用于持有，防止 drop
    watcher: RecommendedWatcher,
    paused: Arc<AtomicBool>,
    root: PathBuf,
}

impl FileWatcher {
    /// 启动监听 `root` 目录（递归），事件写入 AppState 中的数据库。
    /// 失败返回 notify 错误。
    pub fn start(app: AppHandle, root: PathBuf) -> notify::Result<Self> {
        let watched_root = root.to_string_lossy().into_owned();
        let paused = Arc::new(AtomicBool::new(false));
        let paused_clone = paused.clone();
        let debounce = Arc::new(Mutex::new(HashMap::<String, i64>::new()));

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if paused_clone.load(Ordering::Relaxed) {
                return;
            }
            let Ok(event) = res else { return };
            // 只处理文件事件，忽略目录级噪音
            if event.paths.iter().any(|p| p.is_dir()) {
                // 目录事件本身仍可能携带文件路径；仅跳过纯目录事件
                if event.paths.iter().all(|p| p.is_dir()) {
                    return;
                }
            }
            let mut map = debounce.lock().unwrap();
            if let Some(state) = app.try_state::<AppState>() {
                state.with_database(|db| {
                    if !crate::db::workspace::WorkspaceRepo::new(db.conn())
                        .list()
                        .is_ok_and(|items| {
                            items
                                .iter()
                                .any(|w| w.enabled && w.root_path == watched_root)
                        })
                    {
                        return;
                    }
                    handle_file_event(db, &event, &mut map);
                });
            }
        })?;

        watcher.watch(&root, RecursiveMode::Recursive)?;

        Ok(Self {
            watcher,
            paused,
            root,
        })
    }

    /// 暂停/恢复监控（托盘菜单"暂停文件监控"）。
    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Relaxed);
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    /// 当前监听的根目录。
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 当前状态（暂停标志 + 监听根目录）。
    pub fn status(&self) -> WatcherStatus {
        WatcherStatus {
            paused: self.is_paused(),
            root: Some(self.root.to_string_lossy().into_owned()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use notify::event::{DataChange, EventAttributes, ModifyKind};

    fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "msl-watch-test-{tag}-{}-{}",
            std::process::id(),
            now_unix()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn temp_file_filter_rules() {
        for name in [
            "~$方案V3.docx",
            "~$数据.xlsx",
            "~$报告.pptx",
            "foo.tmp",
            "x.swp",
            "Thumbs.db",
            "desktop.ini",
        ] {
            assert!(is_temp_file(Path::new(name)), "{name} 应被过滤");
        }
        for name in [
            "方案V3.docx",
            "数据.xlsx",
            "统计.sas7bdat",
            "README.md",
            "报告.pdf",
        ] {
            assert!(!is_temp_file(Path::new(name)), "{name} 不应被过滤");
        }
    }

    #[test]
    fn watcher_records_file_events() {
        let db = Database::open_in_memory().unwrap();
        let root = temp_dir("events");
        let file = root.join("note.md");
        std::fs::write(&file, "v1").unwrap();

        // 用真实 notify watcher + 与 FileWatcher 相同的处理逻辑
        let debounce = Arc::new(Mutex::new(HashMap::<String, i64>::new()));
        let db_arc = Arc::new(Mutex::new(db));
        let debounce_c = debounce.clone();
        let db_c = db_arc.clone();
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            let Ok(event) = res else { return };
            let mut map = debounce_c.lock().unwrap();
            let db = db_c.lock().unwrap();
            handle_file_event(&db, &event, &mut map);
        })
        .unwrap();
        watcher.watch(&root, RecursiveMode::Recursive).unwrap();

        // 修改文件 → 等待 notify 事件
        std::thread::sleep(std::time::Duration::from_millis(300));
        std::fs::write(&file, "v2").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(800));

        let events = ActivityRepo::new(db_arc.lock().unwrap().conn())
            .query(None, None, None, Some("file.modified"), None, None)
            .unwrap();
        assert!(!events.is_empty(), "应记录 file.modified 事件");
        assert!(events[0].display_text.contains("note.md"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn debounce_coalesces_repeated_events() {
        let mut debounce = HashMap::new();
        let db = Database::open_in_memory().unwrap();
        let root = temp_dir("debounce");
        let file = root.join("a.txt");
        std::fs::write(&file, "x").unwrap();

        // 构造两个同路径 modify 事件
        let e1 = Event {
            kind: EventKind::Modify(ModifyKind::Data(DataChange::Any)),
            paths: vec![file.clone()],
            attrs: EventAttributes::new(),
        };
        let e2 = e1.clone();

        let n1 = handle_file_event(&db, &e1, &mut debounce).len();
        let n2 = handle_file_event(&db, &e2, &mut debounce).len();
        assert_eq!(n1, 1);
        assert_eq!(n2, 0, "2 秒窗口内的重复事件应被合并");

        // 模拟 2 秒后：同 key 新事件
        std::thread::sleep(std::time::Duration::from_millis(2100));
        let mut debounce2 = HashMap::new();
        // 直接用新 debounce 表模拟窗口过期
        let n3 = handle_file_event(&db, &e1, &mut debounce2).len();
        assert_eq!(n3, 1);

        let _ = std::fs::remove_dir_all(&root);
    }
}
