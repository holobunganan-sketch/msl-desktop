# Stage 1 完成报告

> 日期：2026-08-13
> 项目：msl-desktop
> 依据：`DEEPSEEK_V4_FLASH_MSL_DESKTOP_DEVELOPMENT_GUIDE.md` §15

## 1. 本阶段目标

证明最关键架构成立：**UI 可以被销毁，而 Core 继续常驻**。
实现 System Tray、主窗口销毁/重建、Tray Exit、单实例、AppState 骨架，
并建立 `scripts/measure-memory.ps1` 性能脚本与 `docs/performance.md`。

## 2. 实际完成

- **System Tray**：托盘图标（32x32 PNG）+ 菜单（打开 MSL Desktop / Quick Capture 占位 / 分隔线 / 退出）+ tooltip；
- **窗口生命周期**（指南 §3.2 流程）：
  - 关闭按钮 → 拦截 `CloseRequested` → `prevent_close` → `window.destroy()`（WebView 销毁）→ Core + Tray 常驻；
  - 托盘左键点击 / 菜单"打开" → 主窗口重建（统一 `create_main_window`，1240x720 / min 1024x640）；
  - 托盘"退出" → `request_quit()` 置位 → `app.exit(0)` → 进程结束；
- **常驻保活**：`app.run` 拦截 `RunEvent::ExitRequested`，仅当用户明确请求退出时放行，窗口销毁导致的退出请求一律 `prevent_exit()`；
- **单实例**：Win32 named mutex（`Local\MSLDesktop_SingleInstance_v1`），第二实例静默退出，无额外插件依赖；
- **AppState 骨架**：started_at / window_create_count / window_open / quit_requested；
- **scripts/measure-memory.ps1**：递归统计子进程（ParentProcessId）、Working Set/Private Memory（主/子/合计）、window-open/tray 状态标记、多次测量追加 CSV；
- **docs/performance.md**：测量方法、子进程归属限制、验收口径（Working Set、Release）、10 次 open/close 测试步骤。

## 3. 修改/新增文件

- `src-tauri/src/lib.rs` — 重写：Tray / 窗口生命周期 / 常驻保活 / 单实例调度
- `src-tauri/src/app_state.rs` — 新增：AppState 骨架
- `src-tauri/src/single_instance.rs` — 新增：Win32 mutex 单实例
- `src-tauri/Cargo.toml` — tauri 增加 `tray-icon`、`image-png`；新增 `windows-sys`（Foundation/System_Threading/Security）
- `src-tauri/tauri.conf.json` — 移除 `app.windows`（窗口改由代码统一创建）
- `scripts/measure-memory.ps1` — 新增性能脚本
- `docs/performance.md` — 新增测量说明 + Stage 1 实测记录
- `docs/stage-1-report.md` — 本报告
- `.gitignore` — 追加 `scripts/measure-results.csv`（测量运行数据）

## 4. 执行的验证

验收流程（dev build，自动化为 UIA + Win32 消息）：

- command: `pnpm tauri dev`（后台）+ PowerShell `CloseMainWindow()`
  result: 窗口关闭后进程存活（pid 保持），`FindWindow("MSL Desktop")=0`，WebView2 子进程数=0
- command: UIA 定位托盘按钮 + 鼠标左键点击
  result: 主窗口重建，标题恢复 "MSL Desktop"（pid 不变）
- command: 右键托盘 + 向菜单窗口投递 `WM_KEYDOWN(Down×3+Enter)`
  result: 选中"退出"，进程结束（PROCESS EXITED）
- command: 启动第二实例 `target/debug/msl-desktop.exe`
  result: 第二实例立即退出，第一实例保持运行（单实例 OK）
- command: `scripts/measure-memory.ps1`（10 次 open/close 循环）
  result: 托盘 Working Set 39.32 → 40.95 MB（+1.63 MB < 15 MB 限值）✓；每次关闭后子进程归零

## 5. 测试结果

- `cargo check`：通过，无警告；
- 10 次 open/close：全部成功，无窗口创建失败；
- 内存泄漏检查：达标（+1.63 MB / 10 轮）。

## 6. 性能（dev build，参考值；正式验收以 Stage 10 Release 为准）

- 托盘常驻：Total Working Set ≈ 37.7–41.0 MB，无子进程（WebView 全量释放）；
- 窗口打开：Total ≈ 460–476 MB（主进程 ~32–48 MB + 6 个 WebView2 子进程 ~420–434 MB，dev 模式含 devtools 开销）；
- CPU：无后台轮询，idle 正常；
- 观察：托盘 WS 存在约 0.2 MB/轮的缓慢上升（10 轮 +1.63 MB），绝对值远低于限值，来源待 Stage 10 profiling 确认。

## 7. 与指南的偏差

无架构偏差。实现细节：
1. 托盘图标在 Windows 中被放入溢出区（隐藏图标），非代码问题，属系统行为；
2. Quick Capture 菜单为占位（恢复主窗口），真实现属 Stage 4；
3. `measure-memory.ps1` 状态判定基于 `MainWindowTitle`，在快速开关循环下偶发误判（Get-Process 缓存），建议后续改用 EnumWindows 判定（已记入 performance.md 已知限制）。

## 8. 已知问题

- 托盘常驻内存存在轻微递增趋势（+1.63 MB/10 轮），未达阈值，Stage 10 需 profiling 确认来源；
- `measure-memory.ps1` 的 State 判定在窗口快速重建时可能误标（不影响功能）。

## 9. 当前本地 Git 状态

- branch: master
- HEAD: ff561d6（Stage 0 提交）→ 本 Stage 提交后更新
- working tree: 待提交
- remote: 无

## 10. 下一阶段

Stage 2 — SQLite / Migration / Core Data：
- 建立 workspaces / works / resume_points / work_file_refs / tasks / waiting_items /
  inbox_items / calendar_events / activity_events / daily_briefs / provider_settings / app_settings；
- migration、repository/service 最小 CRUD、unit tests、WAL、shutdown flush。

停止，不自动进入下一阶段。
