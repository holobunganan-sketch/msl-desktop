//! 常驻核心的应用状态骨架（指南 §3.1 Resident Core 的一部分）。
//!
//! Stage 1 只保留极简的运行状态，用于验证 UI 生命周期与后续性能观察。
//! 后续 Stage 再按需扩充（SQLite 连接、workspace registry、watcher 等）。
//!
//! 骨架方法在后续 Stage 才会被调用（IPC 命令 / 性能观测），
//! 当前阶段允许 dead_code。

#![allow(dead_code)]

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Instant;

pub struct AppState {
    /// Core 启动时刻。
    started_at: Option<Instant>,
    /// 主窗口累计创建次数（用于 10 次 open/close 泄漏观察）。
    window_create_count: AtomicU32,
    /// 主窗口当前是否已创建。
    window_open: AtomicBool,
    /// 用户是否通过托盘"退出"请求真正退出（否则窗口销毁导致的
    /// ExitRequested 一律阻止，保持 Core 常驻）。
    quit_requested: AtomicBool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            started_at: Some(Instant::now()),
            window_create_count: AtomicU32::new(0),
            window_open: AtomicBool::new(false),
            quit_requested: AtomicBool::new(false),
        }
    }
}

impl AppState {
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

    /// 自 Core 启动至今的秒数。
    pub fn uptime_secs(&self) -> u64 {
        self.started_at
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0)
    }
}
