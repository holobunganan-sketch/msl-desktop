# Execution Report

## Status

FAILED

Allowed final values: `COMPLETED`, `PARTIALLY_COMPLETED`, `BLOCKED`, `FAILED`.

## Reporting Rule

每完成一个步骤立即追加一节，必须包含：

- Action：实际执行的命令与修改。
- Expected：该步骤规定的预期状态。
- Observed：实际结果，包括失败和重试。
- Evidence：路径、退出码、计数、hash、截图或最小输出。
- Verdict：PASS 或 STOP。

不得只写“已完成”“看起来正常”。不得记录真实 Key、Authorization、x-api-key、真实工作文件正文或完整 Provider 响应。

## Completed Steps

- STEP 01：STOP（检测到正式 MSL Desktop 进程，未进入基线测试）。

## Evidence

### STEP 01

- Action：按执行包顺序执行只读 `Get-Location`、`git rev-parse --is-inside-work-tree`、`git status --short`、Node/pnpm/Rust/Python 版本检查、`Get-Process -Name msl-desktop`、正式数据库三文件元数据检查和上一阶段关键路径存在性检查。发现运行中的 MSL Desktop 后立即停止；未结束进程、未运行 `pnpm check`、未运行 `cargo test`、未修改源码或正式数据。
- Expected：目录精确为项目根；工具可用；识别并保护既有 Git 改动；无运行中的 MSL Desktop；正式 DB 仅读取元数据；上一阶段关键文件存在；随后前端 0 errors/0 warnings、Rust 34 个或更多测试通过。
- Observed：目录为 `C:\Myfolder\MSL cowork\msl-desktop`；Git 仓库为 `true`，已有 56 条工作树改动；Node `v26.7.0`、pnpm `10.34.5`、rustc `1.97.1`、cargo `1.97.1`、Python `3.14.7` 可用；检测到正式进程 PID `24988`，路径为 `C:\Myfolder\MSL cowork\msl-desktop\src-tauri\target\release\msl-desktop.exe`，启动时间 `2026/8/15 7:49:47`；正式 DB/WAL/SHM 只读元数据显示 DB `131072` bytes、WAL `2233072` bytes、SHM `32768` bytes；`src/lib/i18n`、`src/lib/components`、`src-tauri/src/workspace/inventory.rs`、`scripts/run-isolated-audit.ps1` 存在，但执行包列出的 `scripts/ui-smoke.mjs` 不存在；因进程 STOP，未执行后续静态和 Rust 基线测试。
- Evidence：工作目录、版本、Git 状态、进程和 DB 元数据均来自只读 PowerShell 输出；未读取 DB 表内容、Key、工作文件正文或 Provider 响应；未调用终止进程命令；执行包七份文件已按顺序完整阅读，`STEPS.md` 36 步顺序与 STOP 规则已复核。
- Verdict：STOP（`E21-USER-AUTHORIZATION-REQUIRED`）。

后续步骤在执行时按相同五字段结构依次追加，禁止预填 PASS。

## Files / Artifacts Changed

- `docs/luna-ai-secretary-iteration/EXECUTION_REPORT.md`：仅记录 STEP 01 的只读基线证据和 STOP 状态；未修改产品源码、数据库、缓存或正式工作文件。

## Commands / External Actions

- 只读环境、Git、进程、文件元数据和路径检查（退出码 0）。
- 未执行 `pnpm check` 或 `cargo test`，原因是 STEP 01 规定检测到正式应用进程后立即 STOP。
- 未结束 PID 24988，未启动任何测试实例，未写入正式 APPDATA/LOCALAPPDATA/TEMP/TMP。

## Deviations

NONE

只有执行包明确允许的等价语法实现才可记录为 deviation。任何改变产品范围、schema 语义、Provider 协议、安全边界、缓存保护、确认机制或验收标准的偏离必须 STOP。

## Stop Codes

- `E21-USER-AUTHORIZATION-REQUIRED`：STEP 01 检测到正式 MSL Desktop 进程，必须由用户先退出后才能继续。

停止时必须使用：

```text
## STOPPED
Code: E21-USER-AUTHORIZATION-REQUIRED
Step: STEP 01
Expected: 不存在任何运行中的 MSL Desktop 进程后才可继续基线和后续实现。
Observed: PID 24988 正在运行正式 release exe `C:\Myfolder\MSL cowork\msl-desktop\src-tauri\target\release\msl-desktop.exe`。
Evidence: 只读 `Get-Process -Name msl-desktop` 输出已记录于 STEP 01；未结束进程。
Why execution cannot continue: 继续运行测试或迁移可能与正式实例竞争数据库/缓存并违反执行包安全边界。
Last safe completed step: STEP 00（执行包读取与环境审查；STEP 01 未通过）。
No further action taken: Confirmed
```

允许代码含义见 `STEPS.md` 与编译技能 stop-code 规则；不得自创模糊代码。

## Deliverables

- PENDING：Provider/model/task routing。
- PENDING：document intelligence。
- PENDING：AI analysis/proposal/review/scheduler。
- PENDING：top Brief/no-scroll Dashboard/translation。
- PENDING：storage governance。
- PENDING：tests、release、screenshots、documents。

## Unresolved Issues

- 最终视觉与产品体验门必须由用户或更高能力模型审阅；机械验收通过前不进入该门。

## Current Run — AI 秘书与存储治理第二阶段

### STEP 01

- Action：按执行包要求只读检查项目位置、Git 工作树、Node/pnpm/Rust/Python 工具链、正式 MSL Desktop 进程、正式 APPDATA 数据库三文件元数据和上一阶段关键路径；在项目根运行 `pnpm check`，在 `src-tauri` 运行 `cargo test`。
- Expected：目录精确为项目根；无运行中的正式 MSL Desktop；工具可用；既有 Git 改动被识别并保留；正式数据库仅读取文件元数据；前端 0 errors/0 warnings；Rust 34 个或更多测试全部通过。
- Observed：目录为 `C:\Myfolder\MSL cowork\msl-desktop`；Git worktree 为 true，存在 56 条既有短状态改动；Node `v26.7.0`、pnpm `10.34.5`、rustc `1.97.1`、cargo `1.97.1`、Python `3.14.7` 可用；`Get-Process -Name msl-desktop` 无输出；正式 `msl-desktop.db` 只读元数据为 233472 bytes，WAL/SHM 不存在；i18n、组件、inventory、隔离启动脚本和 UI CDP 脚本均存在；`pnpm check` 退出码 0、0 errors/0 warnings；`cargo test` 退出码 0，34 passed/0 failed（0 ignored）。
- Evidence：只读 PowerShell 输出与命令退出码；未读取数据库表内容、API Key、工作文件正文或 Provider 响应；未终止任何进程、未执行 Git 写操作。
- Verdict：PASS。

### STEP 35

- Action：在未发现 `msl-desktop` 进程后，按要求只读检查正式 `%APPDATA%\\MSLDesktop` 的 DB/WAL/SHM 元数据，准备进入正式数据库副本迁移 gate。
- Expected：正式 DB/WAL/SHM 的名称、大小、时间戳与 STEP 01 完全一致；只有 gate 通过后才复制到 `.test-runtime\\luna-ai-secretary\\artifacts\\production-db-copy` 并用 release `--background` 迁移。
- Observed：正式主 DB 仍为 233,472 bytes，但当前出现 `msl-desktop.db-shm` 32,768 bytes 与 `msl-desktop.db-wal` 0 bytes；STEP 01 记录为 WAL/SHM 不存在。因此 gate 不通过。未复制、未启动 release `--background`、未触碰 Key、未读取正式表内容，也未执行任何删除/重建来掩盖变化。
- Evidence：只读 `Get-Process -Name msl-desktop` 无输出；只读 `Get-ChildItem $env:APPDATA\\MSLDesktop -Filter msl-desktop.db*` 输出主 DB/WAL/SHM 元数据；STEP 01 baseline 已记录于本报告上方。
- Verdict：STOP（`E19-SECURITY-BOUNDARY`）。

## STOPPED

Code: E19-SECURITY-BOUNDARY
Step: STEP 35
Expected: 正式 DB/WAL/SHM 元数据与 STEP 01 完全一致后，才可复制并迁移副本。
Observed: 当前存在 `msl-desktop.db-shm`（32768 bytes）和 `msl-desktop.db-wal`（0 bytes），而 STEP 01 记录两者不存在。
Evidence: STEP 35 gate 的只读文件元数据输出与 STEP 01 记录。
Why execution cannot continue: 继续复制/启动迁移会违反正式数据边界；执行包禁止删除、重建或批量修改 `%APPDATA%\\MSLDesktop`，因此不能自行恢复或猜测原始状态。
Last safe completed step: STEP 34（release 构建、隔离 smoke、截图、文档和最终静态/Rust 检查）。
No further action taken: Confirmed

### STEP 34

- Action：运行 `pnpm tauri build`；记录 release exe/NSIS 的大小、UTC 时间和 SHA-256；用 release exe 通过隔离四目录脚本启动，运行 Dashboard layout、基础 UI smoke 与 `scripts/release-functional-cdp.py`；生成 Dashboard/Review/Workspace/Settings 截图；更新 README、`docs/architecture.md`、`docs/smoke-checklist.md` 和本阶段报告；重跑 `pnpm check`、`pnpm build`、`cargo fmt --check`、`cargo test`。
- Expected：release exe 与 NSIS 存在且可启动；不调用真实 Provider；release UI 功能和截图完整；文档与实际行为一致；最终静态、格式、构建和 Rust 测试通过。
- Observed：`pnpm tauri build` 退出码 0，release profile 通过并生成 exe/NSIS；release layout 4/4、基础 smoke 首次因隔离持久化语言仍为英文而出现 2 项预检失败，切换语言并等待初始化后重跑为 17/17；release functional smoke 11/11，runtime errors=0；最终 `pnpm check` 0 errors/0 warnings、`pnpm build` 退出码 0、`cargo fmt --check` 退出码 0、`cargo test` 66 passed/0 failed；根目录误运行一次 `cargo fmt --check`（无 Cargo.toml）后已在 `src-tauri` 正确重跑通过。
- Evidence：exe `src-tauri\\target\\release\\msl-desktop.exe` 19,119,616 bytes，SHA-256 `1F2D89C38580C7A6671C75292282B716C92768D880FA9CEAFBABE19C46EC1B9D`；NSIS `src-tauri\\target\\release\\bundle\\nsis\\msl-desktop_0.1.0_x64-setup.exe` 5,018,631 bytes，SHA-256 `9D16C93F0833DA188A6EFA00D951D86BB73AD3E84534216F29D02004F3472080`；截图位于 `.test-runtime\\luna-ai-secretary\\artifacts\\{dashboard-*,release-*.png}`；脚本输出 `UI_SMOKE_PASS=17 UI_SMOKE_FAIL=0`、`RELEASE_FUNCTIONAL_PASS=11 RELEASE_FUNCTIONAL_FAIL=0`。
- Verdict：PASS。

### STEP 33

- Action：在最新 debug 构建上启动隔离实例（APPDATA/LOCALAPPDATA/TEMP/TMP 均位于 `.test-runtime\\luna-ai-secretary`），运行 `scripts/dashboard-layout-cdp.py`、`scripts/capture-dashboard-cdp.py` 和 `scripts/ui-smoke-cdp.py`；先将 smoke 会话语言置为中文，再执行完整 DOM/键盘流程；停止实例后仅检查隔离数据库完整性与正式数据库文件元数据。
- Expected：Dashboard 在中文/英文、1024×640 与 1440×900 下无 body 滚动；Brief、指标、翻译、AI 审阅入口存在；Work/Task/Waiting/Calendar/Inbox/Quick Capture 可提交；语言可切换；隔离数据库可重开读取且完整性通过；正式数据库不被测试实例修改。
- Observed：布局 CDP 四个组合全部 PASS，均报告 `bodyH=clientH`、`bodyW=clientW`，Brief/metrics/translation/review 全部存在；截图生成 4 张（中英文各 1024×640、1440×900）；UI smoke `UI_SMOKE_PASS=17 UI_SMOKE_FAIL=0`；隔离实例已由 PID+路径校验停止；隔离 DB 只读检查 version=6、integrity=`ok`、foreign-key violations=0，未输出正文或 credential 值；正式 DB 文件仍为原始元数据，未启动正式实例。
- Evidence：`scripts/dashboard-layout-cdp.py` 输出 4×`[PASS]`；`scripts/ui-smoke-cdp.py` 输出 17/17；截图位于 `.test-runtime\\luna-ai-secretary\\artifacts\\dashboard-{zh-CN,en-US}-{1024x640,1440x900}.png`；隔离 SQLite 元数据检查 JSON；停止脚本输出 `Stopped=true`。
- Verdict：PASS。

### STEP 32

- Action：运行全量 `pnpm check`、`pnpm build`、`cargo fmt --check`、`cargo test`；修复 Rust 格式后重跑格式门；扫描本轮新增源码未调用真实 Provider/Key。
- Expected：前端 0 errors/0 warnings；构建成功；Rust 格式与全量单元测试通过；migration/AI/document/storage 回归完整。
- Observed：`pnpm check` 退出码 0、0 errors/0 warnings；`pnpm build` 退出码 0（仅既有 QuickCapture 外部导入 tree-shaking 提示）；`cargo fmt --check` 退出码 0；`cargo test` 退出码 0，66 passed/0 failed/0 ignored；Tauri linker 仅输出标准 warning。
- Evidence：本轮命令输出；`src-tauri` 全量测试计数 `66 passed; 0 failed`；未打印真实 Key、工作文件正文或正式路径。
- Verdict：PASS。

### STEP 31

- Action：收口新增 proposal/storage/schedule/translation typed API 与 domain；dataRevision 增加 providers/analysis/proposals/documents/storage 域（translation 不持久化）；Workspace draft/Today pending 改走服务层；新增 UI 输入 label/aria-label、异步 loading/disabled、错误保留输入；补扫中英文词条。
- Expected：新增能力不在组件重复拼 command；状态刷新有独立域；translation 不触发数据库/全局持久化；新增界面无静默按钮或明显硬编码。
- Observed：`pnpm check` 退出码 0，0 errors/0 warnings；`api.ts` 提供 proposal/draft/confirm/reject/work-draft 封装，i18n typed dictionary 包含 AI review/translation/storage/schedule；新组件异步按钮均有 loading/disabled。
- Evidence：`src/lib/services/api.ts`、`src/lib/types/domain.ts`、`src/lib/stores/dataRevision.ts`、新 Svelte 组件与双语 dictionaries；检查输出 `0 errors and 0 warnings`。
- Verdict：PASS。

### STEP 30

- Action：新增 `StorageSettings.svelte`，展示 cache/DB/category bytes/count、limit/high-water/target、Smart preview→protected/warnings→confirm、Activity compact、高级 WebView API 操作；执行期间 loading/disabled，部分失败显示 deleted/skipped/failed；中英文齐全。
- Expected：不存在“一键清空全部数据”；正式数据库、源目录、Key、confirmed/pending proposal、kept Brief 不成为可选删除项；用户可在执行前查看预计释放和保护计数。
- Observed：`pnpm check` 与 `cargo check` 均退出码 0；UI 通过 `get_storage_usage`/`preview_storage_cleanup`/`execute_storage_cleanup`/`compact_storage_history`/`clear_webview_data` commands；清理计划必须显式确认。
- Evidence：`src/lib/components/StorageSettings.svelte`、Settings/i18n、`src-tauri/src/storage/{usage,cleanup}.rs`；静态检查 0/0。
- Verdict：PASS。

### STEP 29

- Action：实现 `compact_activity_history` 与 `compact_history`：按本地日期/workspace/work/event_type 聚合 rollup，sample 限 200 字符，事务删除旧 raw；回收 30 天 superseded draft/rejected proposal，保留 kept/confirmed/pending；确认当前 Tauri 2.11.5 存在 `WebviewWindow::clear_all_browsing_data`，新增官方 API 清理 command。
- Expected：活动明细压缩后总量守恒且幂等；正式 Brief/Proposal 保护；WebView 不手工删除目录，仅调用官方 API。
- Observed：`cargo test storage::cleanup::tests` 退出码 0，3 passed/0 failed；rollup event_count=1、重复 compact 删除 0；WebView API gate 在本地 Tauri 源码中存在；`clear_webview_data` 已注册。
- Evidence：`src-tauri/src/storage/cleanup.rs`、`src-tauri/src/commands/ai_secretary.rs`、Tauri `WebviewWindow::clear_all_browsing_data` API gate；未执行真实 WebView 清理。
- Verdict：PASS。

### STEP 28

- Action：实现 `execute_storage_cleanup`：验证未过期 plan 与 safe cache path，逐候选重新检查 regular file/rebuildable，删除成功后才移除 manifest；document_index 指向被删 cache 时改 pending；写 storage_cleanup_runs 并仅保留最近 50 条审计；新增 execute command。
- Expected：只删除可重建 cache，源工作文件/正式实体/Brief/Proposal/Key 不受影响；部分失败/manifest stale 可见，删除后可重建。
- Observed：`cargo test storage::cleanup::tests` 退出码 0，2 passed/0 failed；synthetic cache 删除计为 1，文件不存在，cleanup result 返回 before/after/deleted/skipped/failed；未执行 VACUUM 或正式路径操作。
- Evidence：`src-tauri/src/storage/cleanup.rs`、`src-tauri/src/commands/ai_secretary.rs`；测试输出 `2 passed; 0 failed`。
- Verdict：PASS。

### STEP 27

- Action：新增 `storage/usage.rs` 与 `storage/cleanup.rs`；盘点 cache categories、bytes/count、DB 元数据、limit/high-water/target；preview 生成 10 分钟 plan_id、候选、预计释放、protected counts 与 warnings；缓存只从安全 cache root/manifest 统计，不计工作目录。
- Expected：清理前用户能看到范围与估算；pending/confirmed proposal、kept brief、正式 DB、凭据和源目录进入 protected 计数而非 delete candidates；plan 可过期。
- Observed：`cargo test storage::cleanup::tests` 退出码 0，1 passed/0 failed；synthetic extracted manifest 计入 1 个候选，protected_counts 含 pending/confirmed/kept keys，plan 可取出；commands `get_storage_usage`/`preview_storage_cleanup` 已注册。
- Evidence：`src-tauri/src/storage/usage.rs`、`src-tauri/src/storage/cleanup.rs`、`src-tauri/src/commands/ai_secretary.rs`、`src-tauri/src/db/documents.rs`；测试未访问正式路径或输出正文。
- Verdict：PASS。

### STEP 26

- Action：追加 `0006_storage_governance.sql` 并登记 version 6；创建 daily_activity_rollups、storage_cleanup_runs、COALESCE unique/index；INSERT OR IGNORE 写入五个默认 cache setting；补充 v5→v6 与设置不覆盖测试并更新 migration gate。
- Expected：存储治理 schema 无损加入；用户已有 cache_limit 等设置不被默认值覆盖；重复迁移幂等。
- Observed：`cargo test db::tests` 退出码 0，10 passed/0 failed；fresh DB version=6、业务表 23 张；synthetic `cache_limit_bytes=123` 迁移后仍为 123，rollup 表存在，旧表/Brief/Provider 保持。
- Evidence：`src-tauri/migrations/0006_storage_governance.sql`、`src-tauri/src/db/migrations.rs`、`src-tauri/src/db/mod.rs`；测试输出 `10 passed; 0 failed`。
- Verdict：PASS。

### STEP 25

- Action：将 Dashboard 改为固定高度网格：Brief 置顶 132px、欢迎/状态条、五项指标（含 pending AI）、主体固定区域；Quick Capture 移入顶部栏；1024–1100px 侧栏收窄；Brief 默认内容截断且详情入口保留；主体列表和翻译卡按网格排列，页面根与 body 继续禁止滚动。
- Expected：主页在常用窗口高度内优先一眼呈现 Brief、指标、核心工作与翻译入口，详情不通过页面滚动承载。
- Observed：`pnpm check` 0 errors/0 warnings；TodayView 使用显式 grid rows/overflow 边界，Quick Capture 只在 Dashboard 顶栏渲染，五项指标均有真实导航；CSS 未以隐藏按钮代替功能，长内容在详情/局部入口截断。
- Evidence：`src/routes/+page.svelte`、`src/lib/components/TodayView.svelte`、`src/lib/components/TranslationCard.svelte`；静态检查通过。隔离 CDP 四尺寸机械断言留待 STEP 33 统一执行。
- Verdict：PASS（结构门通过；最终视觉/滚动机械门在 STEP 33 复核）。

### STEP 24

- Action：新增 `ai/translation.rs` 方向/风格校验与 `translate_text` command；使用 translation task route、Keyring facade 和三协议 router，输入限制 20,000 字符，前端新增 `TranslationCard.svelte`，支持自动方向、书面/口语、Ctrl+Enter、复制/清空，内容仅保留在组件内存。
- Expected：中文→英文、英文→中文方向可预测；无 route/Key/网络时显示配置错误，不写数据库/cache/Activity；失败保留源文本。
- Observed：`cargo test ai::translation::tests` 退出码 0，1 passed/0 failed；`pnpm check` 0 errors/0 warnings；TranslationCard 已放入 Dashboard，command 未添加任何 translation history 持久化。
- Evidence：`src-tauri/src/ai/translation.rs`、`src-tauri/src/commands/ai_secretary.rs`、`src/lib/components/TranslationCard.svelte`、两套 i18n；未调用真实 Provider。
- Verdict：PASS。

### STEP 23

- Action：扩展 DailyBrief repository 支持 retention_state/analysis_run_id/superseded_at；新增 insert_with_retention（同日 draft 自动 supersede）、keep、latest_visible_for_date 与 `keep_daily_brief` command；首页现有 local renderer 继续作为无 Provider fallback。
- Expected：自动生成 Brief 可标记 draft，同日新 draft 不覆盖 kept；用户保留后永不被自动 supersede/清理；首页只读取非 superseded 记录。
- Observed：`cargo test db::brief::tests` 退出码 0，2 passed/0 failed；测试验证 draft1→superseded、draft2→kept 后 latest_visible 返回 kept；旧 insert/list/get 兼容；migration 默认旧 Brief kept。
- Evidence：`src-tauri/src/db/brief.rs`、`src-tauri/src/commands/ai_secretary.rs`；测试输出 `2 passed; 0 failed`；local brief 不调用真实 AI。
- Verdict：PASS。

### STEP 22

- Action：新增 `scheduler` 纯逻辑模块与分析调度 commands；使用本地时间、30–1440 分钟校验、每日 06:00 判定、daily/interval 同 tick 合并、失败后依赖下一周期；Settings 新增 `AnalysisScheduleSettings.svelte`，支持启用、周期、每日时间、立即分析和最近运行。
- Expected：文件 watcher 不直接调用 AI；调度跨重启读取 last state，避免同日重复；用户可设置 3 小时/06:00 默认节奏。
- Observed：`cargo test scheduler::tests` 退出码 0，2 passed/0 failed；测试覆盖 disabled、首次 interval、daily coalesce、跨日防重复；`pnpm check` 0 errors/0 warnings；命令和设置面板已注册。
- Evidence：`src-tauri/src/scheduler/mod.rs`、`src-tauri/src/commands/ai_secretary.rs`、`src/lib/components/AnalysisScheduleSettings.svelte`、Settings/i18n；未等待真实三小时、未调用真实 Provider。
- Verdict：PASS。

### STEP 21

- Action：新增 `start_workspace_work_draft` command；绑定目录后的 CTA 从无动作 disabled 改为真实可追踪分析入口：检查 `work_draft` route 的启用 Provider/model/availability，创建 `workspace_import` analysis run，配置缺失时返回安全错误；Workspace UI 显示 loading/Toast。
- Expected：目录索引与 AI 分析分离；用户点击后才生成工作草稿运行，未经审阅不创建 Work；已有关联流程不绕过 proposal confirmation。
- Observed：`cargo check` 与 `pnpm check` 均退出码 0；CTA 已调用 command，配置缺失会显示“AI work draft route 尚未配置”，配置满足时创建 analysis_run 而不写 works/tasks；无真实 Provider 调用。
- Evidence：`src-tauri/src/commands/ai_secretary.rs`、`src-tauri/src/lib.rs`、`src/lib/components/WorkspaceView.svelte`、两套 workspace i18n；静态检查 0/0。
- Verdict：PASS。

### STEP 20

- Action：新增 `AiReviewCenter.svelte` 与 typed proposal API/domain；加入顶部导航入口、待确认队列、上一项/下一项、kind/operation/confidence/run 元信息、可编辑标题/reason/payload、保存草稿、确认写入、拒绝与 stale 错误处理；中英文词条齐全。
- Expected：所有 AI 建议先进入可编辑确认队列，确认前不写 Work/Task/Waiting/Calendar/Inbox/Resume；用户修改后可持久化，重开仍可见 pending。
- Observed：`pnpm check` 退出码 0，0 errors/0 warnings；组件通过 `list_ai_proposals`、`update_ai_proposal_draft`、`confirm_ai_proposal`、`reject_ai_proposal` 真实 commands，不含 raw 自动执行按钮；导航入口已渲染。
- Evidence：`src/lib/components/AiReviewCenter.svelte`、`src/lib/services/api.ts`、`src/lib/types/domain.ts`、`src/routes/+page.svelte`、两套 i18n；`pnpm check` 输出 `0 errors and 0 warnings`。
- Verdict：PASS。

### STEP 19

- Action：新增 `ai/apply.rs`，将确认作为唯一 AI 写入口；支持 Work/Task/Waiting/Calendar/Inbox/Resume Point create 与受限 Work/Task update；确认前检查 pending+updated_at，edited payload 重新校验，实体写入、proposal confirmed、Activity 在同一 SQLite transaction；新增 confirm/reject/update-draft commands。
- Expected：未经确认不写业务表；确认成功原子落库并可追溯；非法字段、stale、目标不存在时全部回滚；拒绝只改建议状态与 Activity。
- Observed：`cargo test ai::apply::tests` 退出码 0，1 passed/0 failed；synthetic Work confirm 后只增加 1 条 Work，reject proposal 不新增 Work；`cargo check` 退出码 0；commands 已注册。
- Evidence：`src-tauri/src/ai/apply.rs`、`src-tauri/src/commands/ai_secretary.rs`、`src-tauri/src/commands/mod.rs`、`src-tauri/src/lib.rs`；测试输出 `1 passed; 0 failed`；未调用真实 Provider。
- Verdict：PASS。

### STEP 18

- Action：新增 `db/ai.rs` 的 AnalysisRunRepo、ProposalRepo、ScheduleRepo；实现 pending dedupe、user_edited 保护、乐观 updated_at 草稿更新与状态迁移；错误摘要限长；加入分析运行/建议/调度持久化测试。
- Expected：重复分析更新未编辑 pending 而不无限新增；用户编辑内容不会被后台覆盖；stale UI 更新被拒绝；proposal status 仅允许 pending/confirmed/rejected/superseded。
- Observed：`cargo test db::ai::tests` 退出码 0，1 passed/0 failed；测试验证同 dedupe key 保留用户编辑标题，过期 updated_at 返回 stale error；analysis_runs 创建/读取与 migration 外键可用。
- Evidence：`src-tauri/src/db/ai.rs`、`src-tauri/src/db/mod.rs`；测试输出 `1 passed; 0 failed`；未返回任何正文或凭据。
- Verdict：PASS。

### STEP 17

- Action：新增 `ai/schema.rs` 严格 AI Output Contract 与 `ai/analysis.rs` 运行状态/结构化 proposal 写入原语；实现单层 code fence、JSON parse、kind/operation/title/confidence/source_ref 校验；禁止 delete/archive/complete/resolve/send 等破坏性操作；analysis run 失败时显式落 failed。
- Expected：模型只输出可验证 JSON；未知操作、伪造 source、缺少 update target 或超界字段在入队前拒绝；失败释放运行状态并不写业务实体。
- Observed：`cargo test ai::schema::tests` 与 `cargo test ai::analysis::tests` 均退出码 0，各 1 passed/0 failed；合法 fenced JSON 通过，unsafe operation/fake source 被拒绝；坏 JSON 运行记录由 running 更新为 failed，错误信息限长。
- Evidence：`src-tauri/src/ai/schema.rs`、`src-tauri/src/ai/analysis.rs`；两组测试输出均 `1 passed; 0 failed`；未调用真实 Provider，未输出正文或凭据。
- Verdict：PASS。

### STEP 16

- Action：新增 `ai/analysis_snapshot.rs`，在既有 BriefSnapshot 基础上汇总工作台事实、Workspace 文档状态、相对路径 source_ref 和可选缓存正文；实现 20 文件、单文件 40,000 字符、总计 120,000 字符预算、排序、截断计数及 SHA-256 snapshot hash；绝不序列化 cache 绝对路径。
- Expected：AI 输入可覆盖全局事实和绑定目录正文，同时有固定预算、来源追溯和 hash 变化；旧 Brief renderer 字段保持兼容。
- Observed：`cargo test ai::analysis_snapshot::tests` 退出码 0，1 passed/0 failed；synthetic snapshot 只返回相对路径与 document id，未包含 `C:/synthetic` 绝对工作目录；无 cache_rel_path 时不读取正文，hash 非空。
- Evidence：`src-tauri/src/ai/analysis_snapshot.rs`、`src-tauri/src/ai/mod.rs`；测试输出 `1 passed; 0 failed`；测试未打印 synthetic 正文。
- Verdict：PASS。

### STEP 15

- Action：追加 `0005_ai_secretary.sql` 并在 migration runner 登记 version 5；创建单例分析调度、analysis_runs、ai_proposals、pending dedupe partial unique index，并给 daily_briefs 增加 retention_state/analysis_run_id/superseded_at；插入默认 3 小时与每日 06:00 调度；补充 v4→v5、默认值、旧 Brief kept、dedupe 与幂等测试。
- Expected：旧 Brief 保持 kept；分析运行和待确认建议独立持久化；默认 schedule 可重启读取；pending dedupe 生效且外键删除安全。
- Observed：`cargo test db::tests` 退出码 0，9 passed/0 failed；fresh DB version=5、业务表 21 张；v4→v5 synthetic migration 保留旧 Brief 为 kept，默认 `(enabled=1, interval=180, daily=1, 06:00)`，重复 pending dedupe 被拒绝，重复 migration 仍只有 5 条记录。
- Evidence：`src-tauri/migrations/0005_ai_secretary.sql`、`src-tauri/src/db/migrations.rs`、`src-tauri/src/db/mod.rs`；测试输出 `9 passed; 0 failed`；fixture 不含真实正文或凭据。
- Verdict：PASS。

### STEP 14

- Action：新增 `WorkWorkspaceLinkRepo` 的 link/unlink/list 操作与 primary 关系处理；注册四个关联 command；Workspace 页面接入文档状态、增量重索引、相对路径状态列表和“分析目录并生成工作草稿”配置占位 CTA；补充中英文文案与文档卡布局。
- Expected：目录可与多个 Work 关联且可解绑不删除源文件；同一 Work 的 primary 关系可替换；用户可看到 ready/unsupported/failed/needs OCR/too large 统计和相对路径；未配置 AI 时分析按钮明确禁用而非无动作。
- Observed：`cargo test db::documents::tests` 退出码 0，1 passed/0 failed；测试验证 primary link 替换、解绑不影响 Work/Workspace 实体；`pnpm check` 0 errors/0 warnings；Workspace 页面已调用文档状态/列表/重索引 commands，分析 CTA 在未配置时 disabled。
- Evidence：`src-tauri/src/db/documents.rs`、`src-tauri/src/commands/mod.rs`、`src-tauri/src/lib.rs`、`src/lib/components/WorkspaceView.svelte`、`src/lib/i18n/zh-CN.ts`、`src/lib/i18n/en-US.ts`；测试输出 `1 passed; 0 failed`。
- Verdict：PASS。

### STEP 02

- Action：将 `scripts/run-isolated-audit.ps1` 的隔离根固定为 `.test-runtime\\luna-ai-secretary`，增加 `appdata`、`localappdata`、`temp`/`tmp`、`workspace`、`artifacts`、`mock-provider`；启动子进程时设置进程级 `APPDATA`、`LOCALAPPDATA`、`TEMP`、`TMP`；dry-run 输出数据库和 cache root 预期路径；保留 `scripts/stop-isolated-audit.ps1` 的 PID+可执行路径校验。
- Expected：四个运行环境变量、工作目录、产物和 mock Provider 均解析为项目根下路径；dry-run 不启动应用；正式 APPDATA/LOCALAPPDATA/TEMP 不被使用。
- Observed：脚本修改完成；dry-run 退出码 0，输出四个隔离环境变量和 `ExpectedDatabase=.test-runtime\\luna-ai-secretary\\appdata\\MSLDesktop\\msl-desktop.db`、`ExpectedCacheRoot=.test-runtime\\luna-ai-secretary\\localappdata\\MSLDesktop\\cache`；逐项绝对路径 gate 通过，目录存在；无 MSL Desktop 测试进程被启动。
- Evidence：`scripts/run-isolated-audit.ps1`；dry-run JSON 与 `STEP02_PATH_GATE=PASS`；`.gitignore` 已保留 `.test-runtime/` 与 `output/playwright/`；停止脚本未修改。
- Verdict：PASS。

### STEP 03

- Action：先在内存 v2 schema gate 上确认 `schema_migrations` 最高版本为 2 且 `provider_models`/`ai_task_routes` 不存在；新增 `src-tauri/migrations/0003_ai_provider_catalog.sql`，在 `MIGRATIONS` 登记 version 3；仅追加 `provider_settings` 四列、Provider model catalog、六类 task route 和索引；更新数据库迁移测试。
- Expected：v2→v3 无损升级，旧 Provider 与 `credential_ref` 保持；非空 legacy model 迁移为 Chat Completions 模型；模型 unique、route 外键和重复迁移幂等；不修改 0001/0002。
- Observed：migration v1→v3、v2→v3、fresh DB、idempotent、外键/级联测试均通过；`cargo test db::tests` 退出码 0，7 passed/0 failed；DeepSeek base URL（含尾斜杠）被识别为 `template_kind=deepseek`；legacy model `deepseek-v4-flash` 迁移为 protocol=`chat_completions`、endpoint=`/chat/completions`、source=`legacy`；删除 Provider 后 model 被 cascade 删除、route 的 `provider_model_id` 置 NULL。
- Evidence：`src-tauri/migrations/0003_ai_provider_catalog.sql`、`src-tauri/src/db/migrations.rs`、`src-tauri/src/db/mod.rs`；测试输出 `7 passed; 0 failed`；未读取或输出任何 Key。
- Verdict：PASS。

### STEP 04

- Action：在 `db/provider.rs` 增加 `ProviderConnection`、`ProviderModel`、`AiTaskRoute`、固定 task/protocol/template/auth 校验和独立 `ProviderCatalogRepo`；新增 `commands/provider_catalog.rs` 与 7 个固定 command；在 `commands/mod.rs` 暴露数据库边界，在 `lib.rs` 注册子模块 command；补充 catalog/repository 测试。
- Expected：Provider connection、model、route 可独立持久化；同一 Provider 的 model_id 唯一、不同 Provider 可同名；route 保存校验启用连接/模型且允许 unavailable；旧 `list_providers`/`save_provider` 兼容保留；credential_ref 不变且不进入模型表/返回。
- Observed：`cargo test db::provider::tests` 退出码 0，5 passed/0 failed；测试覆盖枚举拒绝、同 Provider upsert、跨 Provider 同名、route 清除和 Provider 删除级联；新增 command 编译注册成功；Key 仍只经 keyring facade 处理，SQLite 只保存 credential_ref。
- Evidence：`src-tauri/src/db/provider.rs`、`src-tauri/src/commands/provider_catalog.rs`、`src-tauri/src/commands/mod.rs`、`src-tauri/src/lib.rs`；测试输出 `5 passed; 0 failed`；未输出 API Key。
- Verdict：PASS。

### STEP 05

- Action：新增 `src-tauri/src/ai/adapters.rs`，定义 Chat Completions、Responses、Anthropic Messages 三种协议适配器；在 `ai/provider.rs` 增加统一 `AiTextRequest`/`AiTextResponse` 与 `complete_model` facade，并保留旧 `complete`/`test_connection` 兼容入口；加入 URL、endpoint、协议错误和 secret redaction；使用本地 `TcpListener` mock 覆盖成功/错误/坏内容。
- Expected：业务层只调用统一文本接口；三协议路径和必要 headers 正确；Responses 支持顶层 `output_text` 与嵌套文本；Anthropic 同时发送 Bearer、`x-api-key` 和版本 header；错误最多 200 字且不含 key/header/body 敏感值；不依赖外网。
- Observed：`cargo test ai::provider::adapters::tests` 退出码 0，3 passed/0 failed；mock 验证 `/chat/completions`、`/responses`、`/messages` 和 Anthropic headers；401 响应中的 synthetic key 被脱敏；endpoint credentials/traversal 被拒绝；旧 Brief 入口仍可编译。
- Evidence：`src-tauri/src/ai/provider.rs`、`src-tauri/src/ai/adapters.rs`；测试输出 `3 passed; 0 failed`；Mock 仅监听 `127.0.0.1`，未调用真实 Provider，未输出 key。
- Verdict：PASS。

### STEP 06

- Action：新增 `src-tauri/src/ai/catalog.rs` 固定模板常量、模型协议映射、模型目录刷新/availability 应用逻辑；固定模板 command 支持 DeepSeek/OpenCode Go 创建与 seed；新增 `refresh_provider_models` 和 `test_provider_model` command，连接测试按具体 model protocol 调用；补充本地目录响应解析测试。
- Expected：DeepSeek 预置官方 V4 Pro/Flash；OpenCode Go 预置 Responses 2、Chat Completions 11、Anthropic Messages 7；刷新只更新 available、不删除保存模型/路由；未知模型 disabled 且需要显式协议；Custom 不带品牌默认；测试不依赖真实网络。
- Observed：`cargo test ai::catalog::tests` 退出码 0，2 passed/0 failed；模板 URL、models endpoint 和协议计数与 `CONTEXT.md` 一致；mock 目录应用逻辑验证未知模型 `enabled=false`、远端缺失模型 `available=false`；新增 command 通过编译，Key 仍不返回 UI。
- Evidence：`src-tauri/src/ai/catalog.rs`、`src-tauri/src/commands/provider_catalog.rs`、`src-tauri/src/lib.rs`；测试输出 `2 passed; 0 failed`；无真实网络/Provider 调用。
- Verdict：PASS。

### STEP 07

- Action：新增 `src-tauri/src/ai/router.rs`，提供唯一任务路由 facade `resolve`/`complete`；实现精确 task kind、显式 `general` fallback、Provider/model enabled/available/protocol/credential 检查；通过注入 `CredentialSource` 测试 fake key，不让业务层自行挑选 Provider。
- Expected：六类 task kind 路由可预测；无 route、disabled、unavailable、disabled connection、缺 Key 返回具名安全错误；不选择第一个启用 Provider，不静默切换模型。
- Observed：`cargo test ai::router::tests` 退出码 0，2 passed/0 failed；测试验证精确 route 优先于 general、缺 route、缺 Key、模型禁用分支；错误包含 task/provider/model 安全标识，不含 synthetic credential。
- Evidence：`src-tauri/src/ai/router.rs`、`src-tauri/src/ai/mod.rs`；测试输出 `2 passed; 0 failed`；fake credential source 仅返回 synthetic 值，未调用真实 Provider。
- Verdict：PASS。

### STEP 08

- Action：新增 `ProviderSettings.svelte` 与 `AiRoutingSettings.svelte`；重建 `SettingsView.svelte` 的 Provider 区域；增加 DeepSeek/OpenCode Go/custom 入口、模型目录状态、刷新、选择模型测试、Key 状态、六类任务路由；新增中英文 typed dictionary 词条；所有 mutation 使用 loading、disabled、Toast/inline error。
- Expected：用户无需编辑 JSON 即可创建固定模板、添加自定义接口、查看协议/availability/enabled、刷新模型和为六类任务选择模型；API Key 输入默认空且不回显；错误不显示响应正文；旧 Settings 通知/自启动功能保留。
- Observed：中文/英文设置组件完成；DeepSeek/OpenCode Go 卡片与 custom 表单均可触发真实 command；模型行显示 protocol/available/enabled，未知目录行显示“需选择协议”；六个 task kind 均有独立 select；`pnpm check` 退出码 0、0 errors/0 warnings。
- Evidence：`src/lib/components/ProviderSettings.svelte`、`src/lib/components/AiRoutingSettings.svelte`、`src/lib/components/SettingsView.svelte`、`src/lib/i18n/zh-CN.ts`、`src/lib/i18n/en-US.ts`；`pnpm check` 输出 `svelte-check found 0 errors and 0 warnings`。
- Verdict：PASS。

### STEP 09

- Action：运行全量 `cargo test`、`pnpm check`，构建 debug exe；使用 `.test-runtime\\luna-ai-secretary` 四目录脚本以 9339 端口启动并按 PID+路径安全停止。
- Expected：Provider/adapter/catalog/router 与既有 CRUD 测试通过；前端 0 errors/0 warnings；debug 实例只使用隔离 APPDATA/LOCALAPPDATA/TEMP/TMP；停止后无残留进程，正式数据库元数据不变。
- Observed：`cargo test` 退出码 0，44 passed/0 failed；`pnpm check` 退出码 0，0 errors/0 warnings；`cargo build` 退出码 0；隔离 debug PID 10932 启动输出同时包含四个隔离环境变量和 cache root，运行 3 秒后由停止脚本安全结束，进程不存在；未发现正式 DB 写入。
- Evidence：`scripts/run-isolated-audit.ps1` 输出 `IsolatedAppData/IsolatedLocalAppData/IsolatedTemp/IsolatedTmp`；`scripts/stop-isolated-audit.ps1` 输出 `Stopped=true`；测试/构建退出码记录；未调用真实 Provider 或 Key。
- Verdict：PASS。

### STEP 10

- Action：在 v3 gate 基础上新增 `0004_document_intelligence.sql`，登记 version 4；创建 `work_workspace_links`、`document_index`、`cache_entries`、FK/索引/primary link 约束；补充 v3→v4、幂等、唯一字段和正文列排除测试。
- Expected：旧业务行不变；Work/Workspace 可多对多关联；workspace 删除级联文档索引，work 删除级联 link，summary model SET NULL；数据库不含 `full_content/raw_body/prompt_body`。
- Observed：`cargo test db::tests` 退出码 0，8 passed/0 failed；fresh DB version=4、业务表 18 张；document unique、link 表和 cache manifest 可写；正文列排除断言通过。
- Evidence：`src-tauri/migrations/0004_document_intelligence.sql`、`src-tauri/src/db/migrations.rs`、`src-tauri/src/db/mod.rs`；测试输出 `8 passed; 0 failed`；未写入任何正文。
- Verdict：PASS。

### STEP 11

- Action：在 `Cargo.toml` 添加 `sha2`、`zip`、`quick-xml`、`pdf-extract`、`encoding_rs`、`chrono`；新增 `storage/paths.rs` 与 `storage/mod.rs`，定义 LOCALAPPDATA cache/tmp/logs root、安全相对路径校验和临时文件 flush+rename 原子写。
- Expected：依赖可获取；cache 与正式 APPDATA 分离；绝对路径、`..`、prefix/symlink 边界无法穿越；原子写失败不留下 ready 半文件。
- Observed：`cargo check` 退出码 0，依赖成功锁定与编译；`cargo test storage::paths::tests` 退出码 0，2 passed/0 failed；路径 traversal/absolute 拒绝与 cache root 归属通过；正式数据库路径仍由 `APPDATA` 控制，cache root 由 `LOCALAPPDATA` 控制。
- Evidence：`src-tauri/Cargo.toml`、`src-tauri/src/storage/paths.rs`、`src-tauri/src/storage/mod.rs`；Cargo lock 更新由 Cargo 生成；测试输出 `2 passed; 0 failed`，未触碰正式路径。
- Verdict：PASS。

### STEP 12

- Action：新增 `documents/mod.rs`、`docx.rs`、`pdf.rs`、`text.rs`、`chunk.rs`；实现 symlink/普通文件与 50 MiB gate、DOCX OpenXML 文本层、PDF text-layer/needs_ocr、UTF-8/UTF-16/GBK 解码、NUL/替换字符失败、2,000,000 字符截断、SHA-256 和 6,000/300 分段；加入 synthetic text/unsupported/long-text/chunk tests。
- Expected：支持文件稳定返回正文和 hash；扫描版 PDF 不伪报 ready；二进制和不支持扩展明确跳过；正文不进入 stdout/日志。
- Observed：`cargo test documents::` 退出码 0，3 passed/0 failed；文本 ready、unsupported、固定截断和 hash 长度、chunk overlap 全部通过；实现未打印正文。
- Evidence：`src-tauri/src/documents/*.rs`；测试输出 `3 passed; 0 failed`；fixture 仅在系统 temp 下创建并删除，未使用真实工作文件。
- Verdict：PASS。

### STEP 13

- Action：新增 `db/documents.rs` 的 document/cache repository；新增 `documents/indexer.rs` 增量扫描，复用 inventory 排除规则，按 mtime+size 复用，变化文件提取并写 LOCALAPPDATA cache，事务更新 document_index；新增 `list_workspace_documents`、`workspace_document_status`、`reindex_workspace_documents` commands；绑定后的后台 reconcile 完成后排队索引，不调用 AI。
- Expected：首次/变化/无变化/删除状态可追踪；ready 只在缓存写成功后登记；删除源文件只删 index/orphan manifest，不遍历删除其他源文件；watcher/reconcile 不直接调用 AI。
- Observed：`cargo test documents::indexer::tests` 退出码 0，1 passed/0 failed；测试验证首次 ready、同一 mtime+size reused、源删除后 index 移除；commands 已注册，正文不会通过 command 返回。
- Evidence：`src-tauri/src/db/documents.rs`、`src-tauri/src/documents/indexer.rs`、`src-tauri/src/commands/mod.rs`、`src-tauri/src/lib.rs`；测试仅使用系统 temp synthetic 文件和隔离 local cache，无真实工作目录/正文日志。
- Verdict：PASS。
