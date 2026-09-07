# Execution Report

## Status

PARTIALLY_COMPLETED

允许的最终值：`COMPLETED`、`PARTIALLY_COMPLETED`、`BLOCKED`、`FAILED`。

## Completed Steps

- STEP 01 — PASS
- STEP 02 — PASS
- STEP 03 — PASS
- STEP 04 — PASS
- STEP 05 — PASS
- STEP 06 — PASS
- STEP 07 — PASS
- STEP 08 — PASS
- STEP 09 — PASS
- STEP 10 — PASS
- STEP 11 — PASS
- STEP 12 — PASS
- STEP 13 — PASS
- STEP 14 — PASS
- STEP 15 — PASS
- STEP 16 — PASS
- STEP 17 — PASS
- STEP 18 — PASS
- STEP 19 — PASS
- STEP 20 — PASS
- STEP 21 — PASS
- STEP 22 — PASS
- STEP 23 — PASS
- STEP 24 — PASS
- STEP 25 — PARTIALLY_COMPLETED（机械验收通过，等待视觉审阅）

## Evidence

每个步骤使用以下结构追加，不得只写“已完成”：

### STEP NN

- Action：实际执行的动作或命令。
- Expected：该步骤规定的结果。
- Observed：实际观察结果。
- Evidence：退出码、关键输出、文件路径、计数或截图。
- Verdict：PASS 或 STOP。

### STEP 01

- Action：运行 `project_cognition.py prepare --project "C:\\Myfolder\\MSL cowork"`；检查 Node/pnpm/Rust/Cargo 版本、`package.json` scripts、`src-tauri/Cargo.toml`、进程状态和正式数据库文件元数据；在项目根运行 `pnpm check`，在 `src-tauri` 运行 `cargo test`。
- Expected：工具和项目结构存在；没有运行中的 `msl-desktop.exe`；正式数据库只做只读元数据记录；前端与后端基线检查退出码为 0。
- Observed：cognition snapshot 2，state clean，indexed_files 144，errors 0；Node `v26.7.0`、pnpm `10.34.5`、rustc `1.97.1`、cargo `1.97.1`；`build`、`check`、`tauri` scripts 存在，`src-tauri/Cargo.toml` 存在；未发现 `msl-desktop` 进程；`pnpm check` 为 0 errors/0 warnings、退出码 0；`cargo test` 为 24 passed/0 failed、退出码 0。
- Evidence：正式数据库只读元数据：`msl-desktop.db` 存在、131072 bytes、LastWriteTimeUtc `2026-08-14T12:49:14.5601947Z`；`-wal` 存在、0 bytes、`2026-08-14T12:52:56.5736908Z`；`-shm` 存在、32768 bytes、`2026-08-14T12:55:00.1792935Z`。`project_cognition.py` 输出 `snapshot=2/state=clean`。实际目录已有现成 Git worktree（`git rev-parse --show-toplevel` 指向项目根），与交接文档旧事实不一致；本步骤未初始化、提交、reset、checkout、clean 或其他 Git 写操作。
- Verdict：PASS。

### STEP 02

- Action：在项目根创建 `.test-runtime\\luna-workbench\\appdata`、`workspace`、`artifacts`；新增 `scripts/run-isolated-audit.ps1` 和 `scripts/stop-isolated-audit.ps1`；在 `.gitignore` 追加 `.test-runtime/` 与 `output/playwright/`；不启动应用，仅执行启动脚本 dry-run 和停止脚本帮助检查。
- Expected：所有运行测试的 APPDATA、工作目录和产物目录均位于项目根内；启动脚本做路径校验并设置进程级隔离 APPDATA；停止脚本只能按传入 PID 且验证可执行路径位于 `src-tauri\\target`；脚本自身可安全验证。
- Observed：四个隔离目录均解析为 `C:\\Myfolder\\MSL cowork\\msl-desktop\\.test-runtime\\luna-workbench\\...`；启动 dry-run 退出码 0，输出 `WouldStart=true`、隔离 APPDATA 与 debug exe 路径；停止脚本 `-?` 帮助退出码 0；未启动应用、未创建数据库、未触碰正式 APPDATA。
- Evidence：`scripts/run-isolated-audit.ps1` 校验 project root、runtime paths、target executable，设置 `$env:APPDATA` 后 `Start-Process`；`scripts/stop-isolated-audit.ps1` 仅接受 `ProcessId`，按 PID 获取进程并验证完整路径位于项目 `src-tauri\\target` 且文件名为 `msl-desktop.exe`；`.gitignore` 新增 `.test-runtime/`、`output/playwright/`。
- Verdict：PASS。

### STEP 03

- Action：新增 `src/lib/styles/app.css` 全局 reset/design tokens；将 `src/app.html` 改为 `lang="zh-CN"` 和 `MSL 工作台`；重构 `src/routes/+page.svelte` 为侧栏、顶部栏和内部滚动内容区，保留现有 View 与业务 invoke 不变；运行 `pnpm check`、`pnpm build` 并用 `rg` 检查布局标记。
- Expected：全局背景、字体、box-sizing、按钮输入字体和指定 token 生效；侧栏 232px、顶部栏 64px；body 不滚动，内容区独立滚动；现有导航仍可编译。
- Observed：`pnpm check` 退出码 0，0 errors/0 warnings；`pnpm build` 退出码 0；构建静态资源包含新全局 CSS；`rg` 确认 `--sidebar-width: 232px`、`--topbar-height: 64px`、`html/body overflow: hidden`、`.content-scroll`、`lang="zh-CN"` 和 `MSL 工作台` 均存在。未修改业务 command 或数据调用。
- Evidence：新增 `src/lib/styles/app.css`；修改 `src/app.html`、`src/routes/+page.svelte`；build 生成 `build/`，仅有既有未使用 import 的构建提示，不影响退出码；静态布局检查无失败项。
- Verdict：PASS。

### STEP 04

- Action：新增 `src/lib/i18n/zh-CN.ts`、`en-US.ts`、`index.ts`；定义类型化 `Locale`、`t`、`setLocale`、`initializeLocale`、日期格式化、状态与日程类型翻译；从 `app_settings_get/set("locale")` 读取和持久化；在应用顶部加入语言切换；迁移导航、工作目录、Today、Works、Plan、Waiting、Calendar、Inbox、Settings、Search、Quick Capture 的主要标题/按钮/占位文案；运行 `pnpm check`。
- Expected：首次中文、可切换英文、语言设置持久化且同步 document lang/title；主要页面文案和状态使用字典，数据库枚举保持英文值。
- Observed：中文和英文各有 153 个同键字典项；10 个 Svelte 页面已接入 i18n；顶部切换按钮根据 `locale` store 即时更新，`setLocale` 调用 `app_settings_set`，订阅同步 `document.documentElement.lang` 与标题；`pnpm check` 退出码 0，0 errors/0 warnings。
- Evidence：`src/lib/i18n/index.ts` 的 `Locale = "zh-CN" | "en-US"`、非法值回退中文、`Intl.DateTimeFormat`、状态/日程翻译函数；`src/routes/+page.svelte` 的导航、顶部语言按钮和持久化入口；数据库调用仍传 `active/next/...` 等英文枚举。后续 STEP 18 将继续扫描并迁移剩余页面特有文案。
- Verdict：PASS。

### STEP 05

- Action：新增共享组件 `AppButton`、`AppCard`、`Modal`、`FormField`、`EmptyState`、`StatusBadge`、`ConfirmDialog`；新增 `src/lib/stores/toast.ts` 与 `ToastHost.svelte`；在根页面挂载单一 ToastHost 和 ConfirmDialog 入口；运行 `pnpm check` 并检查无障碍标记。
- Expected：组件具备一致的视觉/交互 API；按钮 loading 时禁用；Modal 有遮罩、标题、关闭按钮、Esc、遮罩关闭、dialog/aria-modal 和初始焦点；FormField 关联 label/error；Toast 有成功/错误/信息、自动关闭和错误手动关闭；静态检查无 warning。
- Observed：8 个共享 UI 文件与 toast store 已创建；Modal 使用 `role="dialog"`、`aria-modal="true"`、标题关联、Esc、遮罩点击和 `bind:this` 初始焦点；ToastHost 使用 `aria-live="polite"`，错误项可关闭，默认 4.5 秒自动关闭；`pnpm check` 退出码 0，0 errors/0 warnings。
- Evidence：组件目录 `src/lib/components/ui/`；根页面 `ToastHost`/`ConfirmDialog` 挂载；`src/lib/stores/toast.ts` 提供 `addToast`、`dismissToast`、三种类型；本步骤仅挂载基础入口，未改变业务数据调用。
- Verdict：PASS。

### STEP 06

- Action：新增 `src/lib/types/domain.ts`、`src/lib/services/api.ts`、`src/lib/stores/dataRevision.ts`；集中定义核心实体类型、`command` invoke 包装和 `normalizeError`；将 Quick Capture 改为等待 `createInboxItem` 完成，成功后清空/Toast/invalidate，失败保留输入并 Toast error；Today 数据加载迁移到 typed `getToday` 并订阅 global revision；运行 `pnpm check`。
- Expected：invoke 错误格式统一；revision 支持 global/works/tasks/waiting/calendar/inbox/workspace/brief；Quick Capture 不再 fire-and-forget 或失败丢输入；Today 在 mutation 后可通过 revision 重新加载，且无 effect 循环。
- Observed：8 个 revision 域已建立；`normalizeError` 覆盖 Error、String、rejection object 和 fallback；Quick Capture `async save` 成功才清空，成功 invalidate inbox/brief，失败显示错误 Toast；Today 使用 typed API 和 `$dataRevision.global` 触发加载；`pnpm check` 退出码 0，0 errors/0 warnings。
- Evidence：`src/lib/services/api.ts`、`src/lib/stores/dataRevision.ts`、`src/lib/types/domain.ts`；`src/lib/components/QuickCapture.svelte` 的 await/Toast/invalidate；`src/lib/components/TodayView.svelte` 的 typed `getToday` 与 revision 订阅。
- Verdict：PASS。

### STEP 07

- Action：在 `.test-runtime\\luna-workbench\\artifacts\\step07-gate-v1.db` 用 0001 schema 建立临时 v1 库并确认 gate；新增 `src-tauri/migrations/0002_workbench_reliability.sql`，登记 migration version 2；加入 `uuid` v4；更新 Provider repository、keyring adapter、commands；新增 migration/credential_ref 测试；运行 `cargo test`。
- Expected：v1→v2 追加迁移无损、重复 open 幂等；现有 Provider 使用 `provider-{id}`，新 Provider 使用随机 UUID 引用；新增 workspace_file_state 与 Brief v2 元数据列；测试不读取真实 Key。
- Observed：gate `max_version=1`、`credential_ref_present=False`、Brief v2 columns=False、workspace_file_state=False；迁移后版本 2，旧 Provider 行保留且引用 `provider-1`；新表和列存在；`cargo test` 退出码 0，27 passed/0 failed；两个独立内存库的 Provider ID 都为 1 但 credential_ref 不同；生成引用测试通过。
- Evidence：`0002_workbench_reliability.sql` 只使用 ADD COLUMN/CREATE TABLE/CREATE INDEX/UPDATE 兼容操作；`src-tauri/src/db/migrations.rs` 登记 version 2；`src-tauri/src/db/mod.rs` 的 `migration_v1_to_v2_preserves_provider_and_adds_schema`、幂等/version 断言；`src-tauri/src/db/provider.rs` 的跨库隔离测试；`src-tauri/src/ai/provider.rs` 的纯引用随机性测试。未输出或调用任何真实 API Key。
- Verdict：PASS。

### STEP 08

- Action：在 `src-tauri/src/commands/mod.rs` 增加 command 层 `required`、`optional_trim`、枚举、时间范围和活动 Work 校验；应用到 Work、Task、Waiting、Calendar、Inbox、转换、Resume Point、文件引用和 Provider create/update；补充纯校验单元测试；运行 `cargo test`。
- Expected：空白必填、非法枚举、无效/归档 work_id、结束早于开始和无效 Provider 输入在写库前失败；正常路径保持；错误返回到前端而不是吞掉。
- Observed：校验均在 repository 调用前执行；Resume Point 要求 current_state 或 next_step 至少一项；`ensure_work_active` 拒绝不存在/已归档 Work；Calendar end>=start；Provider display/base/model trim-required；`cargo test` 退出码 0，29 passed/0 failed（其中包含新增校验测试）。
- Evidence：`commands/mod.rs` 的 `required`/`allowed`/`valid_time_range`/`ensure_work_active`；`validation_tests::required_rejects_blank_and_trims` 与 `enums_and_time_ranges_are_checked`；Inbox 空输入返回 `收件箱内容不能为空`；各 create/update 命令传入已 trim 值和合法枚举。
- Verdict：PASS。

### STEP 09

- Action：重建 `WorksView.svelte` 的 Work 入口：页面 header 主按钮打开共享 `Modal`；表单包含必填标题、摘要和可见错误；提交使用共享 `AppButton` loading/disabled；成功 Toast、关闭、刷新并自动打开新 Work；新增编辑 Modal；Resume Point 增加 current_state/next_step 至少一项校验；文件引用在后端校验路径存在；接入 works/brief revision 和 Toast；运行 `pnpm check`、`cargo test`。
- Expected：Work 创建/编辑/归档/进度记录/文件引用都有反馈，空输入不静默、不写库，成功后即时更新。
- Observed：Work 创建 Modal 空标题显示错误且保留输入；有效提交会 loading、保存后关闭并打开详情；编辑 Modal 回填标题/摘要/状态；Resume 空提交显示错误；后端不存在文件路径返回“文件路径不存在，未创建关联”；`pnpm check` 0 errors/0 warnings；`cargo test` 29 passed/0 failed。
- Evidence：`src/lib/components/WorksView.svelte` 的 `showCreate/createLoading/createError/showEdit`、Modal/AppButton/invalidate；`commands::add_work_file_ref` 的路径存在性边界；测试输出 29 passed/0 failed。归档仍保留既有确认流程，后续统一迁移到 ConfirmDialog。
- Verdict：PASS。

### STEP 10

- Action：为 Task/Waiting 增加共享 Modal 表单、必填校验、loading、编辑、删除/完成/解决反馈；新增未归档 Work 下拉并将 `workId` 真实传入；Task 更新支持修改 work_id；Waiting repository/command 新增 update；mutation 触发 domain revision；补充 repository 更新测试；运行 `pnpm check` 与 `cargo test`。
- Expected：Task/Waiting 创建、编辑、完成/解决、删除均可用；空标题可见报错且保留输入；关联 Work 进入实体和 Work 详情聚合；无固定 `workId:null` 写入路径。
- Observed：Plan/Waiting 页面主按钮打开 Modal，字段包含标题、Work、优先级/等待人、时间和备注；编辑回填并调用 `update_task`/`update_waiting`；Work 下拉过滤 archived；成功 Toast/invalidate；新增 `task_update_can_change_work_context`、`waiting_update_preserves_open_status_and_changes_context` 测试；`pnpm check` 0 errors/0 warnings，`cargo test` 退出码 0，31 passed/0 failed。
- Evidence：`PlanView.svelte`/`WaitingView.svelte` 的 `formWorkId`、Modal 和 `invoke(... workId ...)`；`src-tauri/src/db/task.rs` Waiting update 与 Task work_id update；`commands::update_waiting` 已注册到 `lib.rs`。
- Verdict：PASS。

### STEP 11

- Action：重建 `CalendarView.svelte`：默认 Week，支持 Day/Week、上一周期/今天/下一周期；实现 7 列周视图、全天区、事件卡与日视图时间线；事件 Modal 补齐标题、Work、类型、全天、开始/结束、地点、备注；编辑/删除带 loading、Toast、revision；Calendar backend update 支持 work_id 并校验时间；运行 `pnpm check`、`cargo test`。
- Expected：Calendar 不再是单列原型，创建/编辑/删除、全天/非全天、类型、地点、备注和 Work 关联可用，结束早于开始不能写入。
- Observed：Week 视图 7 列按日分组，Day 视图为事件时间线；Modal 字段完整并过滤归档 Work；创建和编辑都传 workId，成功 Toast/invalidate；后端 `update_calendar_event` 更新 work_id 并验证 end>=start；`pnpm check` 0 errors/0 warnings；`cargo test` 31 passed/0 failed。
- Evidence：`CalendarView.svelte` 的 `weekDays/eventsForDay`、`week-grid/day-timeline`、共享 Modal/AppButton；`db/calendar.rs` update work_id；`commands/mod.rs` 日程类型/时间/work 校验。
- Verdict：PASS。

### STEP 12

- Action：重写 `InboxView.svelte`，移除全部 `prompt()`；用共享 Modal 为 Task/Waiting/Calendar 转换提供可编辑标题、Work 下拉和目标字段；转换成功标记 Inbox processed、Toast、invalidate 并刷新；取消不改 Inbox，失败保留 Modal 输入；后端 conversion commands 增加可选 title，保留兼容默认 Inbox 原内容；Quick Capture 已在 STEP 06 完成 await/失败保留；运行 `pnpm check`、`cargo test`。
- Expected：Quick Capture 与 Inbox 转换闭环无数据丢失；三种转换都能关联 Work，标题可编辑，取消/失败保持未处理；目标实体即时出现。
- Observed：Inbox 页面只使用 Modal，不再出现 `prompt`；转换表单包含标题、Work、任务优先级/截止、Waiting 跟进、Calendar 类型/开始/结束；成功标记 processed 并刷新，空标题/反向时间可见报错；`pnpm check` 0 errors/0 warnings；`cargo test` 31 passed/0 failed。
- Evidence：`InboxView.svelte` 的 `openConversion/convert/closeConversion`、`dataRevision` 和 `Modal/AppButton`；`commands/mod.rs` conversion title 参数与 work/time validation；`rg -n prompt src/lib/components` 无 Inbox prompt。
- Verdict：PASS。

### STEP 13

- Action：新增 `src-tauri/src/workspace/inventory.rs`，以 metadata-only 方式递归扫描工作目录，复用临时文件过滤并设置 50,000 文件硬上限；新增 `WorkspaceFileStateRepo` 的快照 CRUD；实现基线、离线 reconcile、删除差异的事务更新和 dedupe；实时 watcher 成功写入活动后同步 `workspace_file_state`；启动与首次绑定均使用后台线程 reconcile；新增 `workspace_sync_status` 与 `workspace_rescan` command；工作目录页和设置页展示监听状态、快照文件数、上次扫描、警告和“立即扫描”；运行 `pnpm check`、`cargo test`。
- Expected：首次绑定已有文件只形成一条 `workspace.baseline`，不制造 created 噪音；停止期间新增/修改/删除在下次扫描分别形成 reconcile 活动；临时文件忽略；重复 reconcile 不重复生成差异；扫描不读取正文、不阻塞 UI；状态可见且可手动扫描。
- Observed：`inventory::scan` 只调用 `read_dir`/`metadata`，跳过 `~$`、`tmp`、`swp`、Thumbs.db、desktop.ini；达到上限时返回中文 warning；`reconcile` 使用 SQLite transaction 写快照与活动，metadata 标记 `source=reconcile`，首次活动文本只含文件数/跳过数；实时 watcher 写入 `workspace_id` 并调用 `sync_live_state`；`lib.rs` setup 和 `bind_workspace` 均启动后台 reconcile；`workspace_sync_status` 返回 paused/root/baseline_count/last_scan/last_warning；UI 已接入 Workspace 与 Settings。新增临时目录测试覆盖基线无 created、离线新增/删除、临时文件忽略、重复 reconcile 无重复活动；`cargo test` 退出码 0，32 passed/0 failed；`pnpm check` 退出码 0，0 errors/0 warnings。
- Evidence：`src-tauri/src/workspace/inventory.rs`、`src-tauri/src/db/workspace.rs`、`src-tauri/src/workspace/watcher.rs`、`src-tauri/src/commands/mod.rs`、`src-tauri/src/lib.rs`；测试 `workspace::inventory::tests::baseline_and_offline_reconcile_are_quiet_and_repeatable`；运行记录：`cargo test` 32 passed/0 failed，`pnpm check` 0 errors/0 warnings；未读取或打印工作文件正文。
- Verdict：PASS。

### STEP 14

- Action：重写 `src-tauri/src/ai/brief.rs` 的松散 JSON snapshot 为 `BriefFact`、`SourceCounts`、`BriefSnapshot` typed serializable structs；参数改为 period_start/period_end/today_start/today_end/locale；补齐期间 Activity、未归档 Work/summary、带 Work 上下文的最新 Resume Point、逾期/今日/无截止未完成 Task、期间完成 Task、open Waiting、今日 Calendar、未处理 Inbox、期间 `file.*` 变化；加入每类上限与 `truncated`；在 ActivityRepo 增加 SQL 层 file.* 时间过滤，在 InboxRepo 增加 `list_unprocessed`；Brief 保存时写入 `source_snapshot_json`、周期、locale、ai_used；更新生成命令和 Today 调用传 locale；运行 `pnpm check`、`cargo test`。
- Expected：所有指定来源均进入 snapshot，来源计数可见，文件变化不会被“最近 10 条非文件活动”错误截断；Resume 带 Work id/title；无截止任务最多 100（并记录截断）；snapshot 不包含工作文件正文、API Key 或 Provider header；daily_briefs 本地保存完整来源快照。
- Observed：`BriefFact` 字段固定且无 `serde_json::Value`；`source_counts` 覆盖 works/resume_points/tasks_open/tasks_completed/waiting/calendar/inbox/file_changes/activity；任务按 high→normal→low、updated_at 倒序并限制 100；Inbox 使用 SQL `processed_at IS NULL`；file changes 使用 `event_type LIKE 'file.%'` 且时间过滤在 SQL 后 LIMIT；`BriefRepo::insert_with_snapshot` 写入 period/locale/source_snapshot_json/ai_used/warning；测试构造 Work、Resume、open/no-due Task、completed Task、Waiting、Calendar、Inbox、file activity 并断言计数和正文排除；`cargo test` 33 passed/0 failed；`pnpm check` 0 errors/0 warnings。
- Evidence：`src-tauri/src/ai/brief.rs`、`src-tauri/src/db/activity.rs`、`src-tauri/src/db/inbox.rs`、`src-tauri/src/db/brief.rs`、`src-tauri/src/commands/mod.rs`、`src/lib/components/TodayView.svelte`；测试 `ai::brief::tests::snapshot_contains_all_sources_and_excludes_file_body`；未读取工作文件内容，测试中的文件正文仅用于确认不会进入序列化快照。
- Verdict：PASS。

### STEP 15

- Action：在 `ai/brief.rs` 增加 `BriefResult`、本地 deterministic renderer 和范围校验；建议排序固定为高优未完成任务、今日 Calendar、需跟进 Waiting、Resume next_step、Inbox；无事实时返回中英文明确提示；AI 路径复用同一 typed snapshot，空响应/Key 缺失/Provider 或 HTTP 错误均 fallback 到本地摘要并保留 warning；生成保存 v2 period/locale/snapshot metadata；范围最大 90 天；新增中英文、本地非空、优先级排序和无记录测试；运行 `cargo test`。
- Expected：没有 AI 也能得到有内容 Brief；AI 只增强表达，不决定事实；中文/英文可切换；范围有上限；同快照 hash、period、locale 可缓存；错误不会让 Brief command 失败。
- Observed：`render_local` 在有事实时输出期间进展、停滞工作、今日硬安排、waiting、文件变化、建议推进；空事实输出“当前范围没有记录。请检查工作目录基线或录入事项。”或英文对应文本；`generate_brief_inner` 在无 Provider、无 Key、AI 空响应、AI 错误时均保存 `ai_used=false` 的 local content 与双语 warning；`validate_brief_range` 拒绝 start>=end 和超过 90 天；snapshot hash 包含 period/locale 字段，force=false 复用相同组合；`cargo test` 34 passed/0 failed。
- Evidence：`src-tauri/src/ai/brief.rs` 的 `BriefResult`/`render_local`/`save_with_meta`；`src-tauri/src/commands/mod.rs` 的 `generate_brief_inner` 与 range 校验；测试 `local_renderer_is_non_empty_bilingual_and_deterministically_prioritized`、`snapshot_contains_all_sources_and_excludes_file_body`；未使用真实 Provider、Key 或网络测试。
- Verdict：PASS。

### STEP 16

- Action：新增 `preview_brief_snapshot`（只读）、`generate_brief`（返回 `BriefResult`）、旧 `generate_morning_brief` 兼容 wrapper、`list_briefs`、`get_brief`；将全部 commands 注册到 `lib.rs`；扩展 `BriefRepo` 的 v2 row mapping、`list(limit)` 和 snapshot 字段；生成命令统一使用 `generate_brief_inner`，错误不返回 snapshot 全文、路径或 Provider header；运行 `pnpm check`、`cargo test`。
- Expected：前端拥有稳定的预览/生成/历史 API；preview 不写库；local/AI 生成均写入并可缓存；limit 约束在 1–50；旧 Brief v1 行的 v2 null 字段安全可读；camelCase IPC 参数可匹配。
- Observed：handler 已注册 `preview_brief_snapshot`、`generate_brief`、`generate_morning_brief`、`list_briefs`、`get_brief`；preview 只调用 typed `build_snapshot`；新生成返回 `BriefResult { brief, content, source_counts, source_preview, ai_used, warning, period_start/end, locale }`；旧 wrapper 保持 Today 兼容；`list_briefs` 将 limit clamp 到 1–50；旧插入行读取 `ai_used=false`、snapshot null 无错误；`cargo test` 34 passed/0 failed；`pnpm check` 0 errors/0 warnings。
- Evidence：`src-tauri/src/commands/mod.rs` 的 Brief commands 和 `generate_brief_inner`；`src-tauri/src/lib.rs` handler 清单；`src-tauri/src/db/brief.rs` 的 v2 select/row/list；`db::brief::tests::brief_latest_for_date` 断言旧行兼容及 list；无真实 Key/网络调用。
- Verdict：PASS。

### STEP 17

- Action：重写 `src/lib/components/TodayView.svelte` 为 Dashboard：首页欢迎区显示本地日期、工作目录同步状态与快速记录入口；增加四个可点击指标卡；主内容使用 2:1（约 8/4 列）grid 展示继续推进 Work、今日时间线；右侧展示 Waiting、Inbox、最近文件变化；Brief 支持昨天/过去 7 天/自定义范围、生成/重新生成、本地/AI 状态、来源计数 chips 和展开来源列表；订阅 global/brief/workspace revision；根页面增加 Dashboard 卡片导航事件；加载采用 skeleton，模块错误提供重试；增加 1024px 单列和移动端响应式布局；运行 `pnpm check`。
- Expected：首页从稀疏原型变为可操作 Dashboard，指标与数据即时刷新；Quick Capture/Brief/Work/Waiting/Calendar/Inbox 能从首页进入；单模块错误不致整页空白；1024 宽度可用。
- Observed：Dashboard 使用 `.metrics` 四卡、`.dashboard-grid` 主/侧布局、`.brief-panel` 跨宽度；指标点击发出 `dashboard:navigate` 并由 `+page.svelte` 切换 View；`get_today`、`workspace_sync_status`、`recent_files`、Brief cache 并行加载；Work 卡显示 Resume、Next step、活动时间、最多 3 文件；时间线合并 Task/Calendar；右侧三块均有计数/空状态/查看全部；skeleton、局部错误重试和响应式 CSS 已存在；`pnpm check` 0 errors/0 warnings。
- Evidence：`src/lib/components/TodayView.svelte` 的 Dashboard grid/metrics/timeline/brief range；`src/routes/+page.svelte` 的 `dashboard:navigate` listener；静态检查确认 `@media (max-width: 1024px)` 单列与 `@media (max-width: 720px)` 移动布局；`pnpm check` 退出码 0。
- Verdict：PASS。

### STEP 18

- Action：将根页面内联 Workspace 抽为 `src/lib/components/WorkspaceView.svelte`，使用 AppButton/AppCard、统一 header、同步状态、目录浏览、空状态和 Toast/invalidate；Settings 增加语言与外观卡片、同步健康卡片、Provider 必填校验、Toast 和 ConfirmDialog 删除流程，Key 状态只调用 `provider_has_key`；Search 选中文件无 work_id 时明确进入 Workspace，其他实体继续导航；补充中英文 Provider/Workspace/设置提示字典；删除根页面重复 Workspace CSS；运行 `pnpm check`。
- Expected：Workspace/Settings/Search/导航不再是半成品；Provider 未配置 Key 不显示已配置；删除有二次确认；界面默认中文、可切换英文；基础视觉由共享组件和 tokens 提供。
- Observed：`WorkspaceView.svelte` 已独立维护目录、绑定、扫描、状态和文件操作，绑定/扫描成功显示 Toast 并 invalidate workspace；Settings Provider 表单空白必填可见错误，删除使用 ConfirmDialog，测试连接/保存/删除均有 Toast，Key 仍只读取布尔状态；Settings 增加语言切换和同步卡片；Search 文件命中无精确 Work 时进入 Workspace 而非假装定位；根页面 Workspace 状态和旧 CSS 已移除；`pnpm check` 退出码 0，0 errors/0 warnings。
- Evidence：`src/lib/components/WorkspaceView.svelte`、`src/lib/components/SettingsView.svelte`、`src/routes/+page.svelte`；中英文 `settings.*`、`workspace.*`、`feedback.*` 字典；静态 `rg` 确认主要导航均来自 i18n，Provider Key 内容没有进入 UI 或日志。
- Verdict：PASS。

### STEP 19

- Action：进行全页面可访问性/响应式/交互一致性收口：为顶部语言、Calendar 前后切换、Dashboard 操作和已有 icon-only 控件补充双语 aria-label/title；为 Work 新建/编辑、Resume、文件关联表单补充 label/aria-label；保留共享 Modal 的焦点进入、Esc、遮罩关闭和 `role=dialog/aria-modal`；检查 loading/disabled/error/success 文本状态、颜色 token、页面级滚动与 1024/720 响应式规则；运行 `pnpm check`。
- Expected：关键动作键盘可达，Modal/ConfirmDialog 可关闭且不误提交，图标按钮可读，输入有 label，状态不依赖颜色，目标宽度无页面级横向滚动，静态检查无 warning/error。
- Observed：Root language toggle 使用 `common.toggleLanguage` aria-label/title；Calendar ‹/› 使用双语 aria-label/title；Dashboard Waiting ✓ 使用 resolve aria-label；Toast/Modal close 已有 aria-label；Work 表单标题/摘要/Resume/文件路径等新增显式 label；全局 `html/body overflow:hidden`，内容区内部滚动，Dashboard 在 1024px 单列、720px 移动布局；`pnpm check` 退出码 0，0 errors/0 warnings。其余输入均保留可见 placeholder 或现有 label，后续 UI CDP 将复核焦点与窗口宽度。
- Evidence：`src/lib/components/ui/Modal.svelte`、`ConfirmDialog.svelte`、`src/lib/components/WorksView.svelte`、`TodayView.svelte`、`CalendarView.svelte`、`+page.svelte`；`pnpm check` 0/0；静态 `rg` 未发现无文本且无 aria/title 的关键 icon-only button。
- Verdict：PASS。

### STEP 20

- Action：确认 Python `websocket-client` 可 import；为 `scripts/run-isolated-audit.ps1` 增加 DebugPort 对应的进程级 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS`，不使用 `remote-allow-origins=*`；新增 `scripts/ui-smoke-cdp.py`，CDP 使用 `suppress_origin=True`，通过 data-testid、DOM 输入、点击和键盘覆盖中文默认、空 Work 错误、Work/Task/Waiting/Calendar 创建、Quick Capture、Inbox 转 Task、语言切换、Dashboard 计数、本地 Brief、错误捕获；为关键导航/表单按钮添加 data-testid；重建前端和 debug exe 后在隔离 APPDATA/9333 端口运行；重启实例验证语言持久化；统计隔离数据库各表计数；只读比较正式数据库元数据。
- Expected：真实 DOM/UI 流程全部通过；脚本捕获 window exception/console error 和页面错误；测试数据只写隔离 APPDATA；语言切换重启后保持；正式数据库元数据不变。
- Observed：`python -c import websocket` 退出码 0；首次旧 exe 流程出现 command not found，定位为 debug exe 未重建，随后执行 `pnpm build` + `cargo build` 并重跑；最终 `ui-smoke-cdp.py` 输出 `UI_SMOKE_PASS=17 UI_SMOKE_FAIL=0`，包含 `[PASS]`：Chinese default、empty Work validation、valid Work、Task/Waiting/Calendar、Quick Capture same-page refresh、Inbox conversion entry、English/Chinese toggle、Dashboard counters、local Brief、Brief content、no unexpected window errors；重启后 CDP 输出 `RESTART_LANG=en-US`，随后已恢复中文；隔离 DB 表计数为 `activity_events=12, app_settings=1, calendar_events=4, daily_briefs=2, inbox_items=2, provider_settings=0, resume_points=0, schema_migrations=2, tasks=4, waiting_items=4, work_file_refs=0, workspace_file_state=0, workspaces=0, works=4`；正式 DB 只读元数据仍为 db 131072 bytes/LastWriteTimeUtc `2026-08-14T12:49:14.5601947Z`、wal 0 bytes、shm 32768 bytes，与 STEP 01 相同。
- Evidence：`scripts/ui-smoke-cdp.py`、`scripts/run-isolated-audit.ps1`、data-testid 位于 `+page.svelte`/`QuickCapture.svelte`/Works/Plan/Waiting/Calendar/Inbox；隔离运行 PID 21624/26368、端口 9333；最终 17/0 UI 流程与重启语言记录；未读取工作文件正文、未调用真实 API Key。
- Verdict：PASS。

### STEP 21

- Action：在项目根运行 `pnpm check`；在 `src-tauri` 运行 `cargo fmt --check`、`cargo test`（无过滤器）；运行 `pnpm build`；检查输出中的路径与敏感字段。
- Expected：前端、Rust 格式、Rust 单元测试和生产前端构建全部退出码为 0；无 error/warning；无真实路径、Key 或 Authorization header 泄露。
- Observed：`pnpm check` 退出码 0，`svelte-check found 0 errors and 0 warnings`；`cargo fmt --check` 退出码 0；`cargo test` 退出码 0，34 passed/0 failed（另有 0 main tests、0 doc-tests）；`pnpm build` 退出码 0，Vite/SvelteKit 两阶段均显示 `built`，最终写出 static site；构建日志未发现 API Key、Authorization header 或用户工作文件正文。STEP 20 期间曾发现 Rust 格式差异并已按规则运行 `cargo fmt`，本步复核已通过。
- Evidence：`.test-runtime/luna-workbench/artifacts/step21-pnpm-check.log`、`step21-cargo-fmt.log`、`step21-cargo-test.log`、`step21-pnpm-build.log`；工作目录为项目根或 `src-tauri`，四项命令最终退出码均为 0。
- Verdict：PASS。

### STEP 22

- Action：验证并解析 `.test-runtime\\luna-workbench` 位于项目根后，将旧隔离 DB 文件移入隔离 artifacts 归档（未触碰正式数据），创建 3 个 synthetic seed 文件；启动三次 debug exe，第一次绑定隔离工作目录并创建 Work/Task/Waiting/Calendar/Inbox/Resume，切换英文并生成 local Brief；停止后在隔离目录执行新增、修改、删除；第二次启动核对英文、实体和 reconcile；切换中文后第三次启动核对中文；最后停止所有隔离进程并只读比较正式 DB 元数据。
- Expected：实体、语言和 Brief 跨重启保持；停止期间的 created/modified/deleted 均进入 Activity/Brief；隔离进程全部退出；正式数据库元数据与 STEP 01 完全一致。
- Observed：三次启动 PID 分别为 `27264`、`25624`、`25880`，每次结束后 PID 均已不存在。第一次绑定返回 workspace id 1，启动 reconcile 后 baseline_count=3；首次创建后 `works=1,tasks=1,waiting=1,calendar=1,inbox=1,resume_points=1`，切换后 `LANG_AFTER_TOGGLE=en-US`，local Brief `ai_used=false`、内容非空，source_counts 为 works=1/resume_points=1/tasks_open=1/waiting=1/calendar=1/inbox=1/file_changes=0/activity=5。停止期间新增 `offline-created.txt`、修改 `seed-modify.txt`、删除 `seed-delete.txt`；第二次启动语言仍为 en-US，reconcile 后 baseline_count=3，Brief preview `file_changes=3`，Activity 类型计数明确包含 `file.created=1,file.modified=1,file.deleted=1`，且实体仍为各 1；切换中文后第三次启动 `THIRD_LANG=zh-CN`，实体和历史 Brief 仍存在（briefs=1）。正式 DB 只读元数据仍为 db 131072 bytes、wal 0 bytes、shm 32768 bytes，时间与 STEP 01 相同。
- Evidence：隔离启动输出中的 PID/APPDATA；CDP 记录 `LANG_AFTER_TOGGLE=en-US`、`LANG_AFTER_RESTART=en-US`、`THIRD_LANG=zh-CN`；`workspace_sync_status` baseline_count=3；Brief source_counts/file_changes；隔离 SQLite 仅读取聚合计数：`activity_events` 类型包含 file.created/file.modified/file.deleted 各 1；`TABLE_COUNTS` works/tasks/waiting_items/calendar_events/inbox_items/daily_briefs/workspace_file_state 均符合预期；正式 DB 通过 `Get-Item` 只读比较，未启动正式应用、未读取工作文件正文。
- Verdict：PASS。

### STEP 23

- Action：运行 `pnpm tauri build`；核验 release exe 与 NSIS；使用隔离 APPDATA 启动 release exe 在 9334 端口运行 UI smoke；修复 Work Modal 重复摘要标签后停止锁定 exe 的隔离进程并重新构建；生成中文/英文 Dashboard、Work Modal、Calendar Week、Brief source panel 的 1024×640 与 1440×900 截图；更新 README、architecture、smoke checklist 和本次 workbench rebuild report。
- Expected：release 构建与短 smoke 通过；两个 release 产物存在；截图完整；文档与实现一致，不触碰正式数据库。
- Observed：第一次 `pnpm tauri build` 退出码 0，生成 release 与 NSIS；截图审阅发现 Work Modal 的“摘要”重复标签，已删除重复渲染。修复后第一次重建因 release exe 仍被隔离进程 PID 28404 锁定而退出码 1（拒绝访问），按安全流程停止该隔离进程后重建，最终 `pnpm tauri build` 退出码 0；最终 release smoke PID 11480 初次因 WebView 尚未完成 hydration 出现 3 个定位失败，脚本增加 1 秒启动等待后重跑 `UI_SMOKE_PASS=17 UI_SMOKE_FAIL=0`。随后又将页面硬编码文案迁入 i18n，并将 Calendar 周网格改为可压缩列以消除 1024px 横向滚动；最终 `pnpm check/build/tauri build` 和 release smoke 均再次通过。最终截图脚本退出码 0，生成 10 张 PNG（两种语言/四类页面/两种尺寸）；Brief source panel 截图已滚动到来源 chips 和来源列表。最终产物：`src-tauri/target/release/msl-desktop.exe` 16,307,712 bytes，SHA256 `03B122AE0E40C3C438FA41E554A072285761B758E0280DDE3E2A6EB1C2D4DC9E`；NSIS 4,223,056 bytes，SHA256 `7D6F885868E622B6FA690B5BAD6A8C9F0934E357DBB3AD708A9F46322A429D8B`。构建/截图日志未包含 Key、Authorization header 或工作文件正文。
- Evidence：`.test-runtime/luna-workbench/artifacts/step23-tauri-build-final.log`、`output/playwright/final-*.png`、`scripts/ui-smoke-cdp.py`、`scripts/capture-final-screenshots.py`；文档 `README.md`、`docs/architecture.md`、`docs/smoke-checklist.md`、`docs/workbench-rebuild-report.md`；最终 release smoke 17/17，所有 release 进程已停止。
- Final closeout：STEP 25 前又完成一次硬编码用户文案扫描，将 Inbox/Calendar 时间错误、Waiting/Work 详情、归档确认和语言按钮全部迁入双语字典；同时移除 Calendar 周网格固定最小宽度，消除 1024px 横向滚动，并收紧旧 `smoke-test.ps1/smoke-cdp.py` 的 CDP 连接（移除宽泛 Origin override，统一 suppress_origin）。最终 `pnpm check` 0 errors/0 warnings、`pnpm build` 0、`cargo fmt --check` 0、`cargo test` 34/34；重新 `pnpm tauri build` 退出码 0，重新执行 release UI smoke 17/17，并重新生成全部 10 张截图。最终截图仍位于 `output/playwright/final-*.png`。
- Verdict：PASS。

### STEP 24

- Action：Gate 检查确认没有 `msl-desktop` 进程，正式 DB 元数据与 STEP 01 一致；将正式 `msl-desktop.db`、`-wal`、`-shm` 复制到 `.test-runtime\\luna-workbench\\artifacts\\production-db-copy\\MSLDesktop`；仅以该副本作为 APPDATA 启动 release exe `--background` 完成应用 migration，然后停止副本进程；对副本执行 `PRAGMA integrity_check`、schema version、migration、表计数和 credential_ref 检查；再次比较正式 DB 元数据。
- Expected：副本从 schema version 1 无损升级到 2，完整性通过，原表行数不减少，Provider credential_ref 为 `provider-{id}`，正式 DB 原文件完全不变。
- Observed：Gate 通过，正式 DB 无进程占用；副本启动 PID 13980，migration 后已停止。副本 `COPY_AFTER_VERSION=2`，`PRAGMA integrity_check=ok`，schema_migrations 为 `[[1,"init"],[2,"workbench_reliability"]]`；原表计数未减少（workspaces=1、works=0、tasks=0、waiting_items=0、calendar_events=0、inbox_items=1、daily_briefs=3、provider_settings=1、activity_events=1），新增 `workspace_file_state=186` 为副本启动时对副本所绑定目录的 metadata reconcile；Provider `credential_ref` 为 `provider-1`，未读取 keyring。正式 DB 再次为 db 131072 bytes、wal 0 bytes、shm 32768 bytes，时间戳与 STEP 01 完全相同。
- Evidence：`.test-runtime/luna-workbench/artifacts/production-db-copy/MSLDesktop/`；副本 `COPY_AFTER_INTEGRITY=ok`、version=2、schema 列清单和 table counts；正式 DB `Get-Item` 前后元数据；未启动正式 APPDATA 下的应用，未读取或打印工作文件正文。
- Verdict：PASS。

### STEP 25

- Action：按 `ACCEPTANCE.md` 逐条核对机械验收；运行 delegation validator；汇总步骤、文件、命令、偏差、交付物和未解决事项；将状态设置为 `PARTIALLY_COMPLETED`，保留视觉审阅门，不替用户作主观通过决定。
- Expected：所有机械标准有证据；执行包 validator 为 VALID；提供五类截图和明确的视觉审阅请求；未通过视觉门前不得写 `COMPLETED`。
- Observed：`validate_execution_pack.py docs/luna-workbench-rebuild` 输出 `VALID`。机械验收逐项结果如下：
  1. `pnpm check`：PASS，0 errors/0 warnings。
  2. `cargo test`：PASS，34 passed/0 failed。
  3. `pnpm build`：PASS。
  4. `pnpm tauri build`：PASS，release exe 与 NSIS 均存在。
  5. 测试隔离 APPDATA、正式 DB 元数据/内容未改变：PASS。
  6. 中文默认、英文切换、重启持久化：PASS。
  7. `src/app.html` lang/title：PASS。
  8. Work/Task/Waiting/Calendar/Quick Capture 空输入校验且不增加计数：PASS（UI CDP + command validation）。
  9. 有效提交 loading/Toast/列表刷新：PASS。
  10. Quick Capture 同页刷新：PASS。
  11. Task/Waiting/Calendar/Inbox conversion Work 关联：PASS。
  12. Calendar Day/Week、编辑、删除、全天、地点、备注、类型：PASS。
  13. Workspace quiet baseline：PASS。
  14. 离线 created/modified/deleted reconcile：PASS，各 1。
  15. watcher 临时文件过滤/debounce：PASS（Rust tests）。
  16. Brief snapshot 全部来源：PASS。
  17. file.* 先 SQL 过滤再限量：PASS（Repo test/evidence）。
  18. Brief range yesterday/7d/custom：PASS。
  19. 无 Provider/Key/mock failure local fallback：PASS，内容非空。
  20. AI 仅接收 typed snapshot、不接正文：PASS。
  21. Brief 来源计数与精简来源列表：PASS，截图已展示。
  22. credential_ref 隔离与旧引用迁移：PASS（`provider-1` 副本证据）。
  23. 1024×640/1440×900 Dashboard 无页面级水平滚动：PASS（截图与 CSS gate）。
  24. Dashboard 顶栏、导航、四指标、Continue、时间线、Waiting、Inbox、文件变化、Brief：PASS。
  25. 用户文案 i18n 扫描：PASS；组件内剩余中文仅注释/品牌/用户数据。
  26. UI CDP 冒烟：PASS，debug 17/17，release 17/17。
  27. README/architecture/smoke checklist 一致：PASS，新增 `docs/workbench-rebuild-report.md`。
  28. 最终视觉成熟度：PENDING，需用户或更高能力模型审阅截图；这是预期门槛，不是工程失败。
- Evidence：`VALID` validator 输出；`output/playwright/final-*.png` 10 张截图；最终日志 `final-pnpm-check.log`、`final-pnpm-build.log`、`final-cargo-fmt.log`、`final-cargo-test.log`、`final-tauri-build.log`；完整步骤证据 STEP 01–24；release 产物与 SHA256 已在 STEP 23 记录。
- Verdict：PARTIALLY_COMPLETED（机械标准全部 PASS；视觉门等待审阅）。

## Files / Artifacts Changed

- 前端壳与视觉：`src/app.html`、`src/routes/+page.svelte`、`src/lib/styles/app.css`、`src/lib/i18n/{zh-CN,en-US,index}.ts`。
- 共享 UI：`src/lib/components/ui/*.svelte`、`src/lib/stores/{toast,dataRevision}.ts`、`src/lib/services/api.ts`、`src/lib/types/domain.ts`。
- 页面：`TodayView.svelte`（Dashboard）、`WorkspaceView.svelte`、`WorksView.svelte`、`PlanView.svelte`、`WaitingView.svelte`、`CalendarView.svelte`、`InboxView.svelte`、`SettingsView.svelte`、`SearchOverlay.svelte`、`QuickCapture.svelte`。
- Rust：`src-tauri/migrations/0002_workbench_reliability.sql`、`src-tauri/src/{commands,db,workspace,ai,lib.rs}` 相关文件、credential_ref/keyring 和 Brief v2。
- 测试/脚本：`scripts/run-isolated-audit.ps1`、`scripts/ui-smoke-cdp.py`、`scripts/capture-final-screenshots.py`、`.gitignore`。
- 文档/证据：`README.md`、`docs/architecture.md`、`docs/smoke-checklist.md`、`docs/workbench-rebuild-report.md`、`output/playwright/final-*.png`、`.test-runtime/luna-workbench/artifacts/*`。

## Commands / External Actions

- 项目根：`pnpm check`、`pnpm build`、`pnpm tauri build`；最终退出码均为 0。
- `src-tauri`：`cargo fmt --check`、`cargo test`；最终退出码均为 0，34 passed/0 failed。
- 隔离 UI：`scripts/run-isolated-audit.ps1` + `python scripts/ui-smoke-cdp.py`（debug/release，9333/9334）；均 17/17。
- 隔离持久化：三次 release/debug 启动、离线文件变化、Brief preview、聚合 SQLite 计数；未读取工作文件正文。
- Migration：正式 DB 只读复制到 `.test-runtime/luna-workbench/artifacts/production-db-copy`，副本 version 1→2、integrity ok；正式 appdata 未启动。
- Validator：`python C:\Users\ZhouNan\.codex\skills\compiling-tasks-for-delegation\scripts\validate_execution_pack.py docs/luna-workbench-rebuild` → `VALID`。

## Deviations

- 项目目录实际已有 Git worktree，与交接文档的旧事实不一致；未初始化 Git，未执行提交、reset、checkout、clean 或其他 Git 写操作。
- STEP 23 曾因隔离 release 进程锁定 exe 和 WebView hydration 时序出现可恢复失败；停止隔离进程、增加启动等待后最终构建与 smoke 均 PASS，不改变产品范围。

只有执行包明确允许的等价实现才可记录为偏差；物质性变化必须停止。

## Stop Codes

NONE

停止时必须使用 `STEPS.md` 中规定的代码和标准停止载荷。

## Deliverables

- `src-tauri/target/release/msl-desktop.exe`（16,307,712 bytes）。
- `src-tauri/target/release/bundle/nsis/msl-desktop_0.1.0_x64-setup.exe`（4,219,751 bytes）。
- `output/playwright/final-dashboard-*.png`、`final-work-modal-*.png`、`final-calendar-week-*.png`、`final-brief-sources-*.png`（1024×640 与 1440×900）。
- `docs/workbench-rebuild-report.md` 与本执行报告。

## Unresolved Issues

- 仅剩视觉审阅门：请用户或更高能力模型查看最终截图，确认视觉层级、中文自然度、信息密度和“成熟 Dashboard”程度；审阅前状态保持 `PARTIALLY_COMPLETED`。

执行过程中发现但不在范围内的问题应记录于此，不得顺手修改。
