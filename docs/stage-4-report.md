# Stage 4 完成报告

> 日期：2026-08-13
> 项目：msl-desktop
> 依据：`DEEPSEEK_V4_FLASH_MSL_DESKTOP_DEVELOPMENT_GUIDE.md` §18

## 1. 本阶段目标

完成不依赖 Work 的基本个人工作管理：Plan（Task）、Waiting、Inbox、
Calendar、Quick Capture，并跑通两条真实流程验收。

## 2. 实际完成

### Plan（Task）
- create / edit / complete / due / priority / notes；
- 筛选（全部 + next/scheduled/waiting/paused/done）；
- 前端：任务列表（逾期标记、优先级色标、完成/删除）。

### Waiting
- create（waiting_for / follow_up_at / notes）、resolve、delete；
- 前端：已等天数、"今天需跟进"标记（open 且 follow_up_at ≤ now）、解决置灰。

### Inbox
- Quick Capture 写入、列出（未处理在前）、删除；
- 转换：Inbox → Task / Waiting / Calendar（转换后自动 mark_processed，
  记录 converted_to_type/id）。

### Calendar
- create / edit / delete、list_between（[start, end]）；
- 前端：Day / Week 视图（周一为一周起点）、翻页、6 种 kind 标签。

### Quick Capture（指南 §7.8）
- 主窗口顶部输入框：Enter 保存 Inbox、Esc 清空；
- 托盘"Quick Capture"菜单：显示主窗口 + emit "quick-capture" 事件 →
  前端聚焦输入框。

## 3. 修改/新增文件

- `src-tauri/src/commands/mod.rs` — 新增 19 个命令（task/waiting/inbox/calendar）
- `src-tauri/src/lib.rs` — 注册命令；托盘 Quick Capture 接线（Emitter）
- `src/lib/components/QuickCapture.svelte` / `PlanView.svelte` /
  `WaitingView.svelte` / `InboxView.svelte` / `CalendarView.svelte` — 新增
- `src/routes/+page.svelte` — 集成全部视图与导航
- `src-tauri/src/lib.rs` — `pub mod app_state`（命令测试用，后随 mock 测试移除而保留 pub）
- `docs/stage-4-report.md` — 本报告

## 4. 执行的验证

真实应用 + WebView2 CDP 驱动前端 invoke（等价 UI 操作）完成验收：

| 验收项 | 结果 |
| --- | --- |
| Quick Capture → Inbox | ✅ create_inbox_item 成功，未处理 |
| Inbox → Task → Schedule → Complete | ✅ convert（priority=high, due_at 排程, status=next）→ complete → done |
| Inbox 转换标记 | ✅ processed_at + converted_to_type=task/id 正确 |
| Inbox → Waiting → follow-up → resolve | ✅ waiting_for=王老师、follow_up_at 设置 → resolve → resolved |
| Inbox → Calendar | ✅ kind=kol_visit，list_between 命中 |
| 重启后数据保持 | ✅ 重启后 tasks/inbox/waiting/calendar 全部保留 |

## 5. 测试结果

- `cargo test`：21 passed / 0 failed（数据层与 watcher 测试维持）；
- `cargo check`：无警告；
- `pnpm check`：0 errors / 0 warnings。

注：曾尝试用 `tauri::test::mock_app` 编写命令层集成测试，但测试进程在
Windows 上以 `STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139)` 崩溃（mock 运行时的
DLL 链接问题），已删除该测试文件，改用真实应用 CDP 验收（更贴近指南要求的
"完成一条真实流程"）。

## 6. 性能

- 命令均为单次 SQLite 操作（短事务），无额外开销；
- 前端视图按需 invoke 加载，无轮询。

## 7. 与指南的偏差

无。实现说明：
1. Quick Capture 键盘快捷键（全局热键）按指南 §18 说明可延后至 Stage 8 前完成，
   本阶段实现应用内 + 托盘入口；
2. Task "schedule"（scheduled_start/end 字段）已存在于数据层，
   前端当前通过 due_at 提供排程输入，scheduled 字段的可视化编辑留待 Stage 6 Today
   排程需求时完善；
3. Calendar 先完成 Day + Week（指南 §7.6 允许 Month 后置）。

## 8. 已知问题

- tauri mock runtime 集成测试在 Windows 崩溃（0xc0000139），真实验收依赖
  应用运行 + CDP 自动化；如需单元级命令测试，后续可在 CI/其他平台评估；
- 前端 Inbox 转换使用浏览器 prompt() 收集附加字段，后续可替换为内联表单
  （视觉上更一致，属 polish 项）。

## 9. 当前本地 Git 状态

- branch: master
- HEAD: edf4bf8（Stage 3）→ 本 Stage 提交后更新
- working tree: 待提交
- remote: 无

## 10. 下一阶段

Stage 5 — Works / Resume Point / Work Context：
- create Work、active/paused/waiting/done、最新 Resume Point + 历史、
  Work 相关文件/tasks/waiting/calendar/activity、轻量 notes、pinned files。

停止，不自动进入下一阶段。
