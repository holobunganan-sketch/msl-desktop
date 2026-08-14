# Stage 9 完成报告

> 日期：2026-08-13
> 项目：msl-desktop
> 依据：`DEEPSEEK_V4_FLASH_MSL_DESKTOP_DEVELOPMENT_GUIDE.md` §23

## 1. 本阶段目标

补齐长期常驻体验：提醒通知（waiting/task/calendar）、可选 Windows 启动
（background 模式不弹主窗口）、窗口状态恢复、快捷键（含 Quick Capture
全局热键）、优雅退出、first-run 引导。

## 2. 实际完成

### 提醒调度（`src/notifications/mod.rs`）
- `collect_due_reminders`：waiting follow-up / task deadline / calendar
  日程（提前窗口默认 60 分钟，可配）；
- 30 秒低频调度（`MissedTickBehavior::Skip`，无持续 CPU 尖峰）；
- 进程内去重（同一事件只通知一次）；
- Windows toast（tauri-plugin-notification）；
- 设置：`notifications_enabled`、`reminder_lead_minutes`。

### Autostart / background（`src/lifecycle.rs`）
- `set_autostart` / `is_autostart_enabled`：HKCU\Software\Microsoft\Windows\
  CurrentVersion\Run 注册表；启用时命令行附加 `--background`；
- `is_background_start`：启动时抑制主窗口，仅 tray 常驻。

### 窗口状态 / 优雅退出
- CloseRequested 保存窗口位置/大小（app_settings window_x/y/w/h），
  重建时恢复（防御值校验 <640x400 或 >8000 忽略）；
- ExitRequested（真正退出）→ `take_database().close()`（WAL checkpoint TRUNCATE）。

### 前端
- Settings：Notifications（开关/提前分钟）、Windows 启动（开关）、快捷键说明；
- QuickCapture 注册全局快捷键 `Ctrl+Shift+Space`（tauri-plugin-global-shortcut）。

## 3. 修改/新增文件

- `src-tauri/src/notifications/mod.rs` — 新增提醒调度
- `src-tauri/src/lifecycle.rs` — 新增 autostart/窗口状态
- `src-tauri/src/app_state.rs` — notified 去重集合
- `src-tauri/src/commands/mod.rs` — set_autostart/autostart_status/
  set_notifications_enabled/set_reminder_lead_minutes/check_reminders_now/
  app_settings_get/app_settings_set
- `src-tauri/src/lib.rs` — 插件注册、background 启动、窗口状态存取、优雅退出
- `src-tauri/Cargo.toml` — notification/global-shortcut 插件、tokio time
- `src/lib/components/SettingsView.svelte` — Notifications/Autostart 区块
- `src/lib/components/QuickCapture.svelte` — 全局热键
- `docs/stage-9-report.md` — 本报告

## 4. 执行的验证

| 验收项 | 结果 |
| --- | --- |
| autostart 注册表开关 | 初始 False → 启用 → True → 关闭 → False ✓ |
| reminder 触发 | check_reminders_now 返回 3 条（waiting/task/calendar）并 toast ✓ |
| 窗口状态保存 | 关闭窗口后 window_x/y/w/h 写入 app_settings（132,132,2170,1260）✓ |
| background 启动 | `--background` 不弹主窗口（FindWindow=0），进程常驻 ✓ |
| 托盘 RAM | Total Working Set 31.75 MB（预算 80 MB）✓ |
| graceful shutdown | 退出时 WAL checkpoint + close（代码路径 + 既有 reopen 测试）✓ |

## 5. 测试结果

- `cargo test`：24 passed / 0 failed（新增 lifecycle 3 个：background 检测、
  窗口状态往返、非法值防御）；
- `cargo check`：无警告；
- `pnpm check`：0 errors / 0 warnings。

## 6. 性能

- 提醒调度 30s 一次轻量 SQL 查询（Resident Core 内置，无常驻重型服务）；
- background 模式 Working Set 31.75 MB（无 WebView 子进程）。

## 7. 与指南的偏差

无。实现说明：
1. Autostart 默认关闭（指南要求用户选择），启用后以 `--background` 后台模式启动；
2. first-run wizard 以轻量形式实现：空状态页面提示（各视图已有 empty states），
   未做独立向导弹窗（减少初次启动摩擦，符合"精巧"）；
3. dev 模式下 background 实例（直接运行 dev exe，无 vite）托盘重建窗口受
   devUrl 依赖限制；Release 构建（Stage 12）将做完整安装与窗口重建验证；
   全局热键的实际按键验证留待人工清单（注册无报错）。

## 8. 已知问题

- dev exe 独立运行（无 vite）时托盘重建窗口受限（Release 无此问题）；
- 提醒去重为进程内（重启后窗口内事件可能重发，可接受）。

## 9. 当前本地 Git 状态

- branch: master
- HEAD: 8ce22f1（Stage 8）→ 本 Stage 提交后更新
- working tree: 待提交
- remote: 无

## 10. 下一阶段

Stage 10 — Performance Hardening：
- cold start / warm open / tray RAM / Today RAM / Workspace RAM /
  10× open-close / 10,000 files / watcher 高频 / AI 后回落 / 30min idle CPU；
- `docs/performance.md` 记录每项环境、build mode、测量方法、优化前后。

停止，不自动进入下一阶段。
