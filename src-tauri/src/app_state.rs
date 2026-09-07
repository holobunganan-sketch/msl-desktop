//! 常驻核心的应用状态（指南 §3.1 Resident Core）。
//!
//! Stage 1：窗口生命周期状态；
//! Stage 2+：数据库连接；
//! Stage 3：文件 watcher。
//! 骨架方法在后续 Stage 才会被调用（IPC 命令 / 性能观测），当前允许 dead_code。

#![allow(dead_code)]

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use crate::db::Database;
use crate::workspace::watcher::FileWatcher;

pub struct AppState {
    /// Core 启动时刻。
    started_at: Option<Instant>,
    /// 主窗口累计创建次数（用于 10 次 open/close 泄漏观察）。
    window_create_count: AtomicU32,
    /// 主窗口当前是否已创建。
    window_open: AtomicBool,
    /// 用户是否通过托盘"退出"请求真正退出。
    quit_requested: AtomicBool,
    /// SQLite 连接（Stage 2+）。
    database: Mutex<Option<Database>>,
    /// 文件监听器（Stage 3+）。
    watcher: Mutex<Option<FileWatcher>>,
    /// 已通知提醒的去重集合（进程内，Stage 9）。
    notified: Mutex<std::collections::HashSet<String>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            started_at: Some(Instant::now()),
            window_create_count: AtomicU32::new(0),
            window_open: AtomicBool::new(false),
            quit_requested: AtomicBool::new(false),
            database: Mutex::new(None),
            watcher: Mutex::new(None),
            notified: Mutex::new(std::collections::HashSet::new()),
        }
    }
}

impl AppState {
    // ---------- 窗口生命周期 ----------

    /// 记录一次主窗口创建。
    pub fn record_window_created(&self) -> u32 {
        self.window_open.store(true, Ordering::Relaxed);
        self.window_create_count.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// 标记主窗口已销毁。
    pub fn record_window_destroyed(&self) {
        self.window_open.store(false, Ordering::Relaxed);
    }

    /// 主窗口累计创建次数。
    pub fn window_create_count(&self) -> u32 {
        self.window_create_count.load(Ordering::Relaxed)
    }

    /// 主窗口当前是否存在。
    pub fn window_open(&self) -> bool {
        self.window_open.load(Ordering::Relaxed)
    }

    /// 请求真正退出（托盘"退出"菜单调用）。
    pub fn request_quit(&self) {
        self.quit_requested.store(true, Ordering::Relaxed);
    }

    /// 是否已请求真正退出。
    pub fn quit_requested(&self) -> bool {
        self.quit_requested.load(Ordering::Relaxed)
    }

    // ---------- 数据库 ----------

    /// 注入数据库连接（启动时调用）。
    pub fn set_database(&self, db: Database) {
        *self.database.lock().unwrap() = Some(db);
    }

    /// 取数据库引用执行操作；未初始化时返回 None。
    pub fn with_database<T>(&self, f: impl FnOnce(&Database) -> T) -> Option<T> {
        self.database.lock().unwrap().as_ref().map(f)
    }

    /// 取出数据库（退出时用于 close/flush）；未初始化返回 None。
    pub fn take_database(&self) -> Option<Database> {
        self.database.lock().unwrap().take()
    }

    // ---------- 文件 watcher ----------

    pub fn set_watcher(&self, w: FileWatcher) {
        *self.watcher.lock().unwrap() = Some(w);
    }

    pub fn with_watcher<T>(&self, f: impl FnOnce(&FileWatcher) -> T) -> Option<T> {
        self.watcher.lock().unwrap().as_ref().map(f)
    }

    pub fn clear_watcher_for(&self, path: &str) {
        let mut watcher = self.watcher.lock().unwrap();
        if watcher
            .as_ref()
            .is_some_and(|w| w.root().to_string_lossy() == path)
        {
            // Drop outside the mutex: a pending callback may be finishing.
            let old = watcher.take();
            drop(watcher);
            drop(old);
        }
    }

    /// 已通知提醒去重集合。
    pub fn notified_set(&self) -> &Mutex<std::collections::HashSet<String>> {
        &self.notified
    }

    /// 自 Core 启动至今的秒数。
    pub fn uptime_secs(&self) -> u64 {
        self.started_at.map(|t| t.elapsed().as_secs()).unwrap_or(0)
    }
}
