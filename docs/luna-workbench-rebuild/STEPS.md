# Steps

> 执行器必须按顺序执行。每一步完成后先更新 `EXECUTION_REPORT.md`，再进入下一步。命令默认工作目录为 `C:\Myfolder\MSL cowork\msl-desktop`；另有说明时才切换目录。

## Phase A — 建立可回退的工程基线

### STEP 01 — 验证环境、源码和正式数据边界
Delegation: SAFE_TO_DELEGATE

#### Purpose
确认执行环境与任务包假设一致，并记录正式数据库只读元数据，防止后续测试误写用户数据。

#### Input
`package.json`、`src-tauri/Cargo.toml`、`.project-cognition/START_HERE.md`、环境变量 `%APPDATA%`。

#### Action
1. 运行项目 cognition `prepare`，使用已安装 skill 中的 `project_cognition.py`，项目参数为 `C:\Myfolder\MSL cowork`。
2. 检查 `node --version`、`pnpm --version`、`rustc --version`、`cargo --version`，把版本写入执行报告。
3. 确认 `package.json` 中存在 `check`、`build`、`tauri` scripts，确认 `src-tauri/Cargo.toml` 存在。
4. 确认当前目录不是 Git worktree；只记录事实，不初始化 Git。
5. 确认没有 `msl-desktop.exe` 进程。若存在，记录 PID 和路径后停止，不得强制结束用户进程。
6. 只读记录 `%APPDATA%\MSLDesktop\msl-desktop.db`、`-wal`、`-shm` 的存在性、长度和 LastWriteTime；不要查询或输出用户正文。
7. 运行基线 `pnpm check` 和在 `src-tauri` 中运行 `cargo test`，保存退出码和测试计数。

#### Expected Result
工具可用，项目结构与 CONTEXT 一致，无运行中的真实应用，基线检查通过，正式数据库只有只读元数据记录。

#### Evidence
记录版本、脚本名、cognition state、进程检查结果、正式数据库文件元数据、两个基线命令的退出码和测试计数。

#### Verdict
- PASS：全部工具和文件存在，基线检查退出码为 0；进入 STEP 02。
- STOP：工具缺失、结构重大变化、应用仍在运行或基线本身失败。

#### Exception Handling
- 工具缺失：`E05-TOOL-UNAVAILABLE`。
- 项目结构与任务包不一致：`E02-ENVIRONMENT-MISMATCH`。
- 用户应用仍在运行且不能安全退出：`E21-USER-AUTHORIZATION-REQUIRED`。
- 基线检查失败：`E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 02 — 建立隔离运行和测试目录
Delegation: SAFE_TO_DELEGATE

#### Purpose
确保后续所有应用启动、数据库 migration、文件监听和 UI 测试与正式数据隔离。

#### Input
项目根和 Windows 环境变量 `APPDATA`。

#### Action
1. 创建 `C:\Myfolder\MSL cowork\msl-desktop\.test-runtime\luna-workbench\appdata`、`workspace`、`artifacts`。
2. 创建测试启动脚本 `scripts/run-isolated-audit.ps1`，脚本必须：解析项目根；验证测试路径位于项目根内；设置进程级 `APPDATA`；可选设置 WebView2 调试端口；启动指定 debug/release exe；输出 PID 与隔离数据库路径。
3. 创建测试停止脚本 `scripts/stop-isolated-audit.ps1`，脚本只能停止调用方传入且可执行路径位于本项目 `src-tauri\target` 下的 PID。
4. 在 `.gitignore` 中加入 `.test-runtime/` 和 `output/playwright/`，不得忽略源码或 docs。
5. 不启动应用；只验证两个脚本的参数帮助或 dry-run 分支。

#### Expected Result
存在一个不会接触正式 `%APPDATA%` 的、路径校验明确的启动/停止机制。

#### Evidence
记录新脚本路径、路径验证逻辑摘录、dry-run 输出和 `.gitignore` 新增规则。

#### Verdict
- PASS：脚本明确设置隔离 `APPDATA` 且停止逻辑按 PID+路径双重验证；进入 STEP 03。
- STOP：脚本仍可能打开正式数据库或广泛停止进程。

#### Exception Handling
- 无法创建隔离目录：`E04-PERMISSION-DENIED`。
- 安全边界无法机械验证：`E19-SECURITY-BOUNDARY`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

## Phase B — 统一界面、双语和刷新基础

### STEP 03 — 建立全局视觉基础和应用壳层
Delegation: SAFE_TO_DELEGATE

#### Purpose
消除默认浏览器样式、重复局部 CSS 和页面级滚动问题，建立 Dashboard 统一视觉基础。

#### Input
`src/app.html`、`src/routes/+page.svelte`、现有组件 style blocks、审计截图。

#### Action
1. 新建 `src/lib/styles/app.css` 并在根页面导入。
2. 定义至少以下 token：背景、surface、surface-muted、文字、muted、primary、primary-soft、success、warning、danger、border、shadow-sm、shadow-md、radius-sm/md/lg、sidebar-width、topbar-height。
3. 固定色值：背景 `#f4f7fb`、主文字 `#172033`、次文字 `#64748b`、主色 `#2563eb`、浅主色 `#dbeafe`、边框 `#e2e8f0`、危险 `#b91c1c`、成功 `#15803d`、警告 `#b45309`、侧栏 `#0f172a`。
4. 添加 `html, body` 的 margin 0、width/height 100%、overflow hidden、字体 `Segoe UI, Microsoft YaHei, system-ui, sans-serif`；全局 `box-sizing: border-box`；按钮和输入继承字体。
5. 把 `src/app.html` 改为默认 `lang="zh-CN"`，标题改为 `MSL 工作台`。
6. 将 `+page.svelte` 重构为固定侧栏、顶部栏、内部滚动内容区；body 不滚动，只有内容区滚动。
7. 侧栏宽 232px，收缩宽 76px；顶部栏 64px；内容区 padding 在宽屏 24px、窄屏 16px。
8. 暂时保留现有 View 切换，不在本步骤改业务数据调用。

#### Expected Result
应用壳层拥有统一 token，无 body 默认边距、无页面双滚动，现有页面仍可导航。

#### Evidence
记录新增 CSS 路径、`app.html` lang/title、1024×640 静态构建截图或 DOM 尺寸检查、业务文件未在本步骤修改的说明。

#### Verdict
- PASS：全局样式生效且 `pnpm check` 通过；进入 STEP 04。
- STOP：导航失效、内容不可滚动或 1024×640 出现水平滚动。

#### Exception Handling
- Svelte/CSS 检查失败：`E09-VALIDATION-FAILED`。
- 需要引入大型 UI 框架：`E16-DECISION-REQUIRED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 04 — 实现类型化中英文系统
Delegation: SAFE_TO_DELEGATE

#### Purpose
让中文成为默认界面，并提供完整、持久化的英文切换，消除散落硬编码文案。

#### Input
全部 `src/**/*.svelte`、`app_settings_get/set` commands、状态与 kind 枚举。

#### Action
1. 新建 `src/lib/i18n/zh-CN.ts`、`en-US.ts`、`index.ts`。
2. 字典键按域命名：`nav.*`、`common.*`、`dashboard.*`、`work.*`、`task.*`、`waiting.*`、`calendar.*`、`inbox.*`、`settings.*`、`brief.*`、`validation.*`、`status.*`、`kind.*`。
3. 在 `index.ts` 定义 `Locale = "zh-CN" | "en-US"`、locale store、`t(key, params?)`、日期时间格式器、状态/优先级/kind 翻译函数。
4. 默认 locale 为 `zh-CN`；初始化时读取 `app_settings_get("locale")`；无值或非法值回退中文。
5. 切换时调用 `app_settings_set("locale", locale)`，同步 `document.documentElement.lang` 与 `document.title`。
6. 在顶部栏加入“中文 / EN”切换；按钮提供可访问 label 和 active 状态。
7. 把所有用户可见静态文案移入字典。品牌名、用户输入、文件路径、API model/base URL 不翻译。
8. 为 `active/paused/waiting/done/archived`、`next/scheduled/resolved`、priority 和 Calendar kind 提供双语显示，数据库仍保存原英文枚举。
9. 使用 `Intl.DateTimeFormat(locale)` 统一日期时间，不再在每个组件复制 `pad/fmtTime`。

#### Expected Result
首次加载全中文；切换英文后全部主要页面即时切换；重启隔离实例后语言保持；数据库枚举不变。

#### Evidence
记录字典键数量、语言设置持久化命令、中文/英文导航和一个表单的截图、`rg` 用户硬编码扫描结果。

#### Verdict
- PASS：两种语言完整、即时切换、持久化且 `pnpm check` 通过；进入 STEP 05。
- STOP：主要页面仍混用硬编码文案、语言不能持久化或数据库值被翻译。

#### Exception Handling
- 字典类型错误或缺键：`E09-VALIDATION-FAILED`。
- 必须引入第三方 i18n 包才能继续：`E12-SCOPE-CHANGE`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 05 — 建立共享 UI 组件和反馈系统
Delegation: SAFE_TO_DELEGATE

#### Purpose
让所有新增、编辑、删除操作拥有一致的可见状态，避免静默失败和重复提交。

#### Input
现有各页面的 button/input/card/form CSS 与交互。

#### Action
1. 新建 `src/lib/components/ui/AppButton.svelte`，支持 `variant=primary|secondary|ghost|danger`、loading、disabled、type。
2. 新建 `AppCard.svelte`、`Modal.svelte`、`FormField.svelte`、`EmptyState.svelte`、`StatusBadge.svelte`、`ConfirmDialog.svelte`。
3. 新建 `src/lib/stores/toast.ts` 与 `ToastHost.svelte`；支持 success/error/info，自动关闭不少于 3 秒，错误可手动关闭。
4. Modal 必须具有遮罩、标题、关闭按钮、Esc 关闭、遮罩点击关闭、`role="dialog"`、`aria-modal="true"` 和初始焦点。
5. FormField 支持 label、required、hint、error，并通过 id 关联 label/input/error。
6. AppButton loading 时保留宽度、禁用点击并显示双语 loading 文案或 spinner。
7. 在根页面只挂载一个 ToastHost 和一个 ConfirmDialog 管理入口。
8. 先用组件替换一个非业务关键示例区域，运行检查；其余页面在后续步骤迁移。

#### Expected Result
共享组件可复用、可访问且不会改变业务数据；成功/失败可以在全局显示。

#### Evidence
记录组件清单、props、键盘交互验证和 `pnpm check` 结果。

#### Verdict
- PASS：共享组件检查通过且示例交互正常；进入 STEP 06。
- STOP：Modal 键盘不可用、loading 仍可重复提交或 Toast 覆盖关键内容。

#### Exception Handling
- 组件 API 与 Svelte 5 不兼容：`E09-VALIDATION-FAILED`。
- 需要引入大型组件库：`E12-SCOPE-CHANGE`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 06 — 建立类型化 API 与跨页面刷新机制
Delegation: SAFE_TO_DELEGATE

#### Purpose
统一 Tauri invoke、错误转换和数据失效通知，让 Quick Capture 与各页面修改后 Dashboard 自动刷新。

#### Input
全部前端 `invoke(...)` 调用及重复 TypeScript 类型。

#### Action
1. 新建 `src/lib/types/domain.ts`，集中 Workspace、Work、ResumePoint、Task、WaitingItem、CalendarEvent、InboxItem、ActivityEvent、Provider、Brief 相关类型。
2. 新建 `src/lib/services/api.ts`，为现有 command 提供类型化函数；统一捕获 unknown error 并返回可显示消息。
3. 新建 `src/lib/stores/dataRevision.ts`，至少维护 `global`、`works`、`tasks`、`waiting`、`calendar`、`inbox`、`workspace`、`brief` revision。
4. 提供 `invalidate(...domains)`；实体修改成功后递增对应域和 `global`。
5. 页面加载 effect 只订阅需要的 revision；防止 effect 因自身赋值形成循环。
6. 将 Quick Capture 改为 `async`：提交前保留输入；成功后清空、Toast、invalidate inbox/global/brief；失败时保留输入并 Toast error。
7. 逐步把后续页面改用 `api.ts`；本步骤至少迁移 Quick Capture 和 Today/Dashboard 数据加载。
8. 添加一个前端可测试的 error normalization 函数，覆盖 String、Error 和 Tauri rejection object。

#### Expected Result
Quick Capture 成功后当前 Today/Inbox 可通过 revision 刷新；失败不会丢输入；invoke 错误格式一致。

#### Evidence
记录 revision 域、Quick Capture 成功/失败行为、前端检查结果和一次不切页刷新演示。

#### Verdict
- PASS：刷新机制无循环、Quick Capture 不再 fire-and-forget；进入 STEP 07。
- STOP：出现重复请求循环、失败仍清空输入或刷新需要切页。

#### Exception Handling
- Svelte effect 循环或性能异常：`E09-VALIDATION-FAILED`。
- API 类型与实际 command 返回冲突：`E11-CONFLICTING-EVIDENCE`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

## Phase C — 数据安全、校验与实体关联

### STEP 07 — 追加可靠性 migration 并修复 Provider 凭据引用
Delegation: CONDITIONAL

#### Purpose
无损增加文件基线、Brief 元数据和稳定凭据引用，为后续实现提供 schema。

#### Input
`src-tauri/migrations/0001_init.sql`、`src-tauri/src/db/migrations.rs`、`db/provider.rs`、`ai/provider.rs`、`commands/mod.rs`、`Cargo.toml`。

#### Action
1. Gate：在临时数据库确认最高 migration version 为 1，且新列/表不存在。若不是该状态，停止并报告，不得改写方案。
2. 新建 `src-tauri/migrations/0002_workbench_reliability.sql`，不得修改 0001。
3. migration 添加 `provider_settings.credential_ref TEXT`；现有行设置为 `'provider-' || id`；创建 `credential_ref` 非空唯一索引。
4. migration 创建 `workspace_file_state(workspace_id, path, modified_at, size, seen_at, PRIMARY KEY(workspace_id,path))` 及 workspace/modified 索引。
5. migration 为 `daily_briefs` 添加 `period_start INTEGER`、`period_end INTEGER`、`locale TEXT`、`source_snapshot_json TEXT`、`ai_used INTEGER NOT NULL DEFAULT 0`、`warning TEXT`。
6. 在 `MIGRATIONS` 登记 version 2，保持升序。
7. 在 `Cargo.toml` 添加 `uuid` v4 特性；更新 lockfile。
8. `ProviderSetting` 增加 `credential_ref`；新 insert 生成 `provider-{uuid-v4}`，所有 select/row mapping 同步。
9. `ai/provider.rs` 的 save/get/delete/has 函数改为接收 `credential_ref`，不再接收整数 id；commands 必须先读取 Provider 再操作凭据。
10. 现有 Provider 继续使用 migration 写入的旧引用；新 Provider 使用 UUID 引用。不得自动删除旧凭据。
11. 添加测试：migration v1→v2 保留行；重复 open 幂等；新 Provider 引用唯一；两个独立数据库同为 id 1 时 credential_ref 不同；测试不读取真实 keyring，可将纯引用生成与 keyring IO 分离测试。

#### Expected Result
临时数据库无损升级到 version 2；现有 Provider 仍指向旧凭据名；新 Provider 不会因整数 ID 碰撞。

#### Evidence
记录 gate 查询、migration version、PRAGMA table_info、测试名称和结果；不得记录 Key 内容。

#### Verdict
- PASS：gate 符合、migration 与 Provider 测试全部通过；进入 STEP 08。
- STOP：现有 schema 不符合 version 1、migration 需要删表/清数据、凭据兼容无法保证。

#### Exception Handling
- Gate 不符合：`E02-ENVIRONMENT-MISMATCH`。
- 需要破坏性迁移：`E13-DESTRUCTIVE-ACTION`。
- uuid 依赖无法获得且缓存中不存在：`E06-DEPENDENCY-MISSING`。
- 凭据迁移需要读取或输出真实 Key：`E19-SECURITY-BOUNDARY`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 08 — 在后端统一校验和明确错误
Delegation: SAFE_TO_DELEGATE

#### Purpose
让后端成为最终数据边界，阻止空标题、非法枚举和无效时间，即使前端出错也不会写入垃圾数据。

#### Input
`commands/mod.rs`、task/work/calendar/inbox/provider repositories 与现有 error 类型。

#### Action
1. 新建或在 commands 内集中定义 trim-required、枚举校验、时间范围校验函数。
2. Work title、Task title、Waiting title、Calendar title、Inbox content、Provider display name/base URL/model 必须 trim 后非空。
3. 校验 Work status、Task priority/status、Calendar kind 仅允许 schema 注释中的枚举。
4. Calendar 若 `end_at` 存在，必须 `end_at >= start_at`；全天事件允许同一天边界但不得负数。
5. `work_id` 存在时确认对应 Work 存在且未 archived；不存在返回可识别的中文边界错误代码/消息。
6. 保持 repository 负责 SQL，command/service 负责输入校验；不得把前端文案硬编码进 repository。
7. 所有 create/update 添加成功和失败测试：空白、非法 enum、无效 work_id、结束早于开始、正常路径。
8. 错误不得只 `eprintln` 后返回成功；主操作失败必须传到前端。

#### Expected Result
非法输入不会写入数据库；错误可被前端显示；正常 command 行为保持。

#### Evidence
记录新增校验测试名、写入前后计数断言和 `cargo test` 结果。

#### Verdict
- PASS：非法输入测试和原测试全部通过；进入 STEP 09。
- STOP：校验改变已有合法数据语义或需要清理正式数据。

#### Exception Handling
- 合法枚举与现有数据冲突：`E11-CONFLICTING-EVIDENCE`。
- 需要修改用户旧数据：`E13-DESTRUCTIVE-ACTION`。
- 测试失败：`E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 09 — 重建 Work 创建与详情交互
Delegation: SAFE_TO_DELEGATE

#### Purpose
把 Work 从一行输入框改为可靠的工作上下文入口，并完整使用共享反馈组件。

#### Input
`WorksView.svelte`、Work commands、共享 UI/i18n/api/revision。

#### Action
1. 页面标题区放主按钮“新建工作 / New work”，打开 Modal。
2. Modal 字段：标题必填、摘要可选、初始状态默认 active；标题为空时显示 validation 错误并保持 Modal。
3. 提交期间 primary button loading/disabled；成功后 Toast、关闭 Modal、刷新 works、自动选中新 Work。
4. 失败时保留用户输入、显示表单级错误，不清空。
5. 列表项显示翻译后的状态、更新时间、摘要首行；提供空状态 CTA。
6. Work 详情顶部提供编辑标题/摘要/状态 Modal；归档使用 ConfirmDialog。
7. Resume Point 必须至少 current_state 或 next_step 有一项非空；空提交显示错误。
8. 文件关联使用系统文件选择器优先，保留手动路径作为次要方式；不存在路径显示错误，不写库。
9. 所有成功 mutation 调用对应 invalidate；错误成功后清除旧错误。

#### Expected Result
Work 创建、编辑、归档、Resume Point 和文件关联具有明确反馈且详情即时更新。

#### Evidence
记录空标题、有效创建、失败保留输入、自动选中、Resume 空校验的 UI 结果和数据库计数。

#### Verdict
- PASS：所有 Work 关键流程可用且 `pnpm check` 通过；进入 STEP 10。
- STOP：任一操作仍静默失败或页面必须刷新才能看到结果。

#### Exception Handling
- 系统文件选择器不可用：保留手动路径并记录，`E05-TOOL-UNAVAILABLE` 仅在两种方式都不可用时使用。
- 前后端字段冲突：`E11-CONFLICTING-EVIDENCE`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 10 — 重建 Task 与 Waiting 表单并关联 Work
Delegation: SAFE_TO_DELEGATE

#### Purpose
使任务和等待事项真正进入 Work 上下文，消除 `workId: null` 固定值和静默提交。

#### Input
`PlanView.svelte`、`WaitingView.svelte`、Works list API、Task/Waiting commands。

#### Action
1. 两个页面均使用主按钮打开 Modal，不再把所有字段挤在单行。
2. Task 字段：标题必填、Work 可选下拉、优先级、截止时间、备注；编辑时回填。
3. Waiting 字段：标题必填、Work 可选、等待谁、开始时间默认现在、跟进时间、备注；编辑时回填。
4. 如后端缺少 update waiting 或所需字段 command，按现有 repository 添加明确 update command 并注册 handler。
5. Work 下拉只列未归档 Work，显示标题和翻译状态；空时提供跳转到 Works 的提示。
6. 创建/编辑/完成/解决/删除全部使用 loading、Toast、ConfirmDialog 和 invalidate。
7. 列表提供 All/Open/Done 或对应双语过滤，状态显示翻译但传值保持英文。
8. 成功后从 Work 详情确认 task/waiting 聚合出现；不得只验证独立页面。

#### Expected Result
Task/Waiting 可创建、编辑和关联 Work，独立页面与 Work 详情同步。

#### Evidence
记录带/不带 work_id 的创建结果、Work 详情聚合、空标题错误、编辑与完成/解决流程。

#### Verdict
- PASS：关联和刷新全部通过；进入 STEP 11。
- STOP：仍有固定 `workId:null` 创建路径或 Work 详情不更新。

#### Exception Handling
- 缺少后端 update command：在本步骤范围内添加；实现仍需 schema 破坏性变化则 `E13-DESTRUCTIVE-ACTION`。
- 数据关联与现有外键冲突：`E11-CONFLICTING-EVIDENCE`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 11 — 把 Calendar 改成可用日历
Delegation: SAFE_TO_DELEGATE

#### Purpose
将当前简单列表改成可识别的周视图/日程时间轴，并补全事件字段与 Work 关联。

#### Input
`CalendarView.svelte`、Calendar commands/repository、共享 UI/i18n。

#### Action
1. 保留 Day/Week 切换，但默认 Week；顶部显示当前范围、上一周期、今天、下一周期和“新建日程”。
2. Week 视图为 7 列；每列显示日期、全天区和按时间排序的事件卡。无需拖拽和缩放。
3. Day 视图显示当日时间轴与事件；在 1024×640 可纵向滚动。
4. 事件 Modal 字段：标题必填、Work 可选、类型、全天、开始、结束、地点、备注。
5. 全天切换时使用日期输入；非全天使用 datetime-local；前端与后端均校验结束不早于开始。
6. 编辑回填全部字段；删除使用 ConfirmDialog；成功后 invalidate calendar/global/brief/works。
7. 事件卡使用 kind 对应的有限颜色 token，保持文字对比度；显示 Work 标题、地点和时间摘要。
8. 当列表为空时显示双语 EmptyState 和新建 CTA。

#### Expected Result
用户可直观看到一周安排并完整创建/编辑/删除事件，Work 详情同步。

#### Evidence
记录周视图截图、全天和非全天事件创建、无效时间错误、Work 关联和删除结果。

#### Verdict
- PASS：视图和 CRUD/关联通过；进入 STEP 12。
- STOP：Calendar 仍只是单列列表、字段丢失或无效时间可入库。

#### Exception Handling
- 需要第三方日历库才能继续：`E12-SCOPE-CHANGE`；本任务要求用现有 Svelte/CSS 实现有限周视图。
- 时间转换出现时区冲突：`E11-CONFLICTING-EVIDENCE`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 12 — 重建 Inbox 转换和 Quick Capture 闭环
Delegation: SAFE_TO_DELEGATE

#### Purpose
让工作台接收到的信息立即可见，并能无损转换为关联 Work 的实体。

#### Input
`QuickCapture.svelte`、`InboxView.svelte`、Inbox conversion commands、共享 UI。

#### Action
1. Quick Capture 提交等待后端完成；空输入显示 inline error；成功才清空并 Toast。
2. 当前停留 Dashboard 或 Inbox 时，成功后列表立即刷新。
3. Inbox 页区分未处理/已处理，默认未处理在前；提供计数和双语空状态。
4. 替换全部 `prompt()` 为 Task/Waiting/Calendar 转换 Modal。
5. 每个转换 Modal 都有 Work 可选下拉和目标实体必需字段；Inbox 原内容作为标题初值且可编辑。
6. 转换成功后 Inbox 标记 processed，目标实体出现，Toast 提供“查看”动作或清晰提示。
7. 取消转换不得改变 Inbox；失败保留输入并保持未处理。
8. 删除未处理 Inbox 使用 ConfirmDialog；已处理项保留转换类型和目标 id 展示。

#### Expected Result
Quick Capture 与 Inbox 转换不会丢数据，目标实体可关联 Work，并即时反映到 Dashboard。

#### Evidence
记录同页刷新、取消不变、转换成功、失败保留、work_id 和 Dashboard 计数变化。

#### Verdict
- PASS：Quick Capture 和三种转换闭环通过；进入 STEP 13。
- STOP：任何失败路径清空输入、转换后目标不可见或仍使用 prompt。

#### Exception Handling
- 后端 conversion command 缺字段：在本步骤更新签名与测试；需要破坏性 schema 变化则 `E13-DESTRUCTIVE-ACTION`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

## Phase D — 工作目录事实与 Brief 引擎

### STEP 13 — 实现工作目录基线、离线差异和可见状态
Delegation: SAFE_TO_DELEGATE

#### Purpose
让绑定前已有文件成为基线，让应用停止期间的变化在下次启动被发现，同时保留实时 watcher。

#### Input
`workspace/watcher.rs`、`workspace/mod.rs`、`db/workspace.rs`、migration v2、`lib.rs` setup、Workspace UI。

#### Action
1. 新建 `src-tauri/src/workspace/inventory.rs`，定义只包含 workspace_id、绝对/相对路径、modified_at、size 的文件元数据。
2. 扫描递归复用现有临时文件过滤；跳过不可读文件并累计 skipped 数，不读取文件内容。
3. 为单 workspace 设 50,000 文件硬上限；超过时停止扫描并返回可见警告，不静默截断。
4. repository 提供 upsert baseline、加载旧状态、删除消失路径和事务提交。
5. 首次绑定：写入完整基线，记录一条 `workspace.baseline` Activity，文本只包含文件数和跳过数，不把每个既有文件记为 created。
6. 后续 reconcile：比较 path+mtime+size，分别写 `file.created`、`file.modified`、`file.deleted`，metadata 标记 `source="reconcile"`，然后事务更新 baseline。
7. 实时 watcher 写入事件成功后同步更新对应 `workspace_file_state`，并填入 workspace_id；失败必须日志+状态，不得吞掉。
8. setup 中以后台 blocking task 扫描文件系统，扫描结束后短时间持有 DB lock 完成 compare/write；不得在 UI 主线程长时间扫描。
9. 增加 `workspace_sync_status` command，返回 watcher paused/root、baseline count、last scan、last warning；Workspace 和 Settings 显示状态及“立即扫描”。
10. 添加临时目录测试：首次基线无 created 噪音；停止期间新增/修改/删除后 reconcile 各一条；临时文件忽略；重复 reconcile 无重复事件。

#### Expected Result
目录事实从“只看绑定后实时事件”升级为“初始基线+离线差异+实时事件”，且用户可见健康状态。

#### Evidence
记录测试目录文件数、四类测试、Activity event_type/count、状态 command 返回和 `cargo test`。

#### Verdict
- PASS：基线、离线差异、实时同步和状态展示通过；进入 STEP 14。
- STOP：实现读取文件正文、首次基线制造大量 created、扫描阻塞 UI 或重复 reconcile 产生重复活动。

#### Exception Handling
- 目录超过上限：返回警告并在 UI 显示；若必须忽略上限才能继续则 `E16-DECISION-REQUIRED`。
- 权限文件不可读：累计 skipped 并继续；根目录不可读则 `E04-PERMISSION-DENIED`。
- DB 事务无法保持一致：`E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 14 — 重建 Brief 的结构化事实快照
Delegation: SAFE_TO_DELEGATE

#### Purpose
让 Brief 使用所有应纳入的工作台事实，并提供来源计数与可追溯列表。

#### Input
`ai/brief.rs`、Activity/Work/Task/Waiting/Calendar/Inbox repositories、migration v2。

#### Action
1. 用明确 serializable struct 替换 snapshot 中松散 `serde_json::Value`；每条来源包含 type、entity_id、work_id、title/display、timestamp、status 与必要的有限字段。
2. Snapshot 参数改为 `period_start`、`period_end`、`today_start`、`today_end`、locale。
3. 包含：所选期间 Activity、未归档 Work 与 summary、每个 Work 最新 Resume Point、open Waiting、逾期/今日/无截止的未完成 Task、期间完成 Task、今日 Calendar、未处理 Inbox、期间 file.* 变化。
4. Resume Point 来源必须带 Work 标题/id，避免失去上下文。
5. 无截止任务最多 20 条，优先 high、再按 updated_at；其余类别设明确上限并在 snapshot 记录 truncated count。
6. ActivityRepo 增加按时间且 `event_type LIKE 'file.%'` 查询，必须在 SQL 过滤后再 LIMIT。
7. InboxRepo 为未处理列表增加直接方法，不在调用层取全表后过滤。
8. 生成 `source_counts`：works、resume_points、tasks_open、tasks_completed、waiting、calendar、inbox、file_changes、activity。
9. Snapshot JSON 存入 `daily_briefs.source_snapshot_json`，只在本地；不得包含文件正文、API Key、provider header。
10. 添加测试，构造每类至少一条事实，断言全部进入 snapshot；特别断言 Inbox、无截止 Task、旧于最近 10 条非文件活动的 file event 仍进入。

#### Expected Result
Brief snapshot 完整、类型明确、范围可控、来源可计数且不会因错误 LIMIT 漏项。

#### Evidence
记录 snapshot 测试各来源计数、截断规则、敏感字段排除断言和 `cargo test`。

#### Verdict
- PASS：全部来源与边界测试通过；进入 STEP 15。
- STOP：仍缺 Inbox/无截止任务/文件变化，或 snapshot 包含正文/密钥。

#### Exception Handling
- 数据类型需改变 schema 才能表达：只允许使用 v2 已加 JSON/period 列；其他 schema 变化 `E16-DECISION-REQUIRED`。
- 敏感信息进入 snapshot：`E19-SECURITY-BOUNDARY`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 15 — 实现本地摘要、AI 增强和时间范围
Delegation: SAFE_TO_DELEGATE

#### Purpose
保证 Brief 不依赖 AI 也有用，并在 AI 可用时生成受事实约束的中文/英文叙述。

#### Input
新 BriefSnapshot、Provider adapter、daily_briefs v2 列、Today/Dashboard 需求。

#### Action
1. 定义 `BriefResult`：brief record、content、source_counts、source_preview、ai_used、warning、period_start/end、locale。
2. 实现 deterministic local renderer，中文/英文均输出：期间进展、停滞工作、今日硬安排、需跟进、Inbox、文件变化、建议推进 1–3 项。
3. 推进建议排序规则：逾期高优任务 > 今日 Calendar/截止 > 到期 Waiting > Resume next_step > 未处理 Inbox；不得由模型自行决定来源优先级。
4. 若没有事实，明确显示“当前范围没有记录”，同时提示用户检查工作目录基线或录入事项；不得生成空字符串。
5. AI 路径使用相同 snapshot，system prompt 明确 locale、范围、source id、不得编造；AI 返回空或解析失败时使用 local renderer 并 warning。
6. 无 enabled provider、无 key、HTTP/API 失败均使用 local renderer，`ai_used=false`，warning 为双语可显示信息；不得让整个 Brief command 返回错误。
7. 支持 range preset：yesterday、last_7_days、custom；前端负责本地时区范围，后端校验 start < end 且最大 90 天。
8. 缓存 key 至少包含 snapshot hash、period、locale；相同组合且 force=false 复用；force=true 新生成但不得无界显示重复历史。
9. 保存 v2 metadata 和 snapshot JSON；provider_id 继续只保存显示名或安全标识，不保存 Key。
10. 添加纯本地测试和 mock HTTP 测试：无 provider、有 provider 成功、失败 fallback、中文/英文、同快照缓存、范围变化导致 hash 变化。

#### Expected Result
任何配置状态下 Brief 都能返回有内容结果；AI 只增强表达，不决定事实；范围和语言可控。

#### Evidence
记录排序测试、fallback 测试、mock AI 测试、缓存 hash/period 断言和内容非空断言。

#### Verdict
- PASS：本地和 AI mock 路径全部通过；进入 STEP 16。
- STOP：测试需要真实 Key、AI 失败导致无 Brief、建议引用不存在事实或范围无上限。

#### Exception Handling
- 真实凭据被请求：`E14-CREDENTIALS-REQUIRED`。
- 外部网络成为必需条件：`E20-NETWORK-FAILURE`。
- 模型输出无法约束且无 fallback：`E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 16 — 暴露 Brief 预览、生成、读取与历史命令
Delegation: SAFE_TO_DELEGATE

#### Purpose
给前端提供稳定、可测试的 Brief API，而不是只返回一段不可解释文本。

#### Input
`commands/mod.rs`、`lib.rs` invoke handler、`db/brief.rs`、BriefResult。

#### Action
1. 添加 `preview_brief_snapshot(periodStart, periodEnd, todayStart, todayEnd, locale)`，不调用 AI、不写数据库。
2. 添加或重构 `generate_brief(...)` 返回 BriefResult；保留旧 `generate_morning_brief` 兼容 wrapper，内部走新逻辑。
3. 添加 `list_briefs(limit)` 和 `get_brief(id)`；limit 限制在 1–50。
4. 更新 BriefRepo insert/select/row mapping，支持 v2 列；旧行列为 null 时提供安全默认值。
5. 所有 command 注册到 handler；参数命名按 Tauri JS camelCase 调用规则验证。
6. command tests 覆盖 preview 不写库、local generate 写库、cache、list limit、读取旧行。
7. 输出 error 不包含 snapshot 全文、路径全文或 Provider 响应 header。

#### Expected Result
前端可预览来源、生成 Brief、查看历史；旧数据仍可读。

#### Evidence
记录 command 清单、handler 注册、旧行兼容测试、preview 写入前后计数和 `cargo test`。

#### Verdict
- PASS：新旧命令和测试通过；进入 STEP 17。
- STOP：旧 Brief 无法读取、preview 写库或参数调用不匹配。

#### Exception Handling
- 旧记录兼容需要删除数据：`E13-DESTRUCTIVE-ACTION`。
- command 参数名与前端无法一致：`E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

## Phase E — Dashboard 与全部页面收口

### STEP 17 — 将 Today 重建为 Dashboard
Delegation: SAFE_TO_DELEGATE

#### Purpose
提供用户要求的成熟 Dashboard，并把已完成的数据闭环集中呈现。

#### Input
`TodayView.svelte`、`get_today`、Brief commands、recent file/activity API、design tokens、i18n。

#### Action
1. 可将 `TodayView.svelte` 重命名为 `DashboardView.svelte`，同时更新入口；不得留下失效 import。
2. 顶部欢迎区显示本地日期、工作目录同步状态、全局搜索、语言开关和主要“快速记录”入口。
3. 第一行四个指标卡：进行中 Work、今日/逾期任务、今日 Calendar、需跟进 Waiting；每卡可点击进入对应页。
4. 主区使用 12 列 grid：左侧 8 列，右侧 4 列；1024 宽时改为单列。
5. 左主区依次展示“继续推进”Work 卡和“今日时间线”；Work 卡显示最新 Resume、下一步、最近活动、最多 3 个相关文件。
6. 右侧展示 Waiting、Inbox 和最近文件变化；每区有计数、空状态和查看全部入口。
7. Brief 卡横跨主内容宽度，显示范围选择（昨天/7天/自定义）、生成/重新生成、AI/fallback 状态、来源覆盖 chips、可展开来源列表。
8. `get_today` 增强返回 dashboard counts 与 recent_file_changes，或通过并行 typed API 获取；不得重复发起相同查询。
9. Dashboard 订阅 global/brief/workspace revision；其他页面 mutation 后无需切页即可更新。
10. 加载使用 skeleton；错误按卡片局部显示并提供重试，单个模块失败不能让整个 Dashboard 空白。

#### Expected Result
首页成为信息密度合理、可操作、可解释的 Dashboard，内容随数据变化即时刷新。

#### Evidence
记录 1024×640 和 1440×900 中文截图、指标与数据库计数对照、Quick Capture 同页刷新、单模块错误重试。

#### Verdict
- PASS：所有规定区域、刷新和响应式行为通过；进入 STEP 18。
- STOP：首页仍是稀疏卡片堆叠、核心模块缺失或需要切页刷新。

#### Exception Handling
- get_today 扩展与现有调用冲突：保留兼容字段并添加新字段；无法兼容则 `E11-CONFLICTING-EVIDENCE`。
- 1024 宽布局不可用：`E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 18 — 收口 Workspace、Settings、Search 与导航
Delegation: SAFE_TO_DELEGATE

#### Purpose
让非核心页面也遵循统一设计、双语和健康状态，不留下明显原型区域。

#### Input
`+page.svelte`、Workspace 内联实现、`SettingsView.svelte`、`SearchOverlay.svelte`、workspace status 与 Provider 修复。

#### Action
1. 将 Workspace 内联页面抽为 `WorkspaceView.svelte`，使用统一 header/card/button/empty state。
2. Workspace 显示绑定目录、文件数、baseline 数、最近扫描、watcher 状态、警告和“立即扫描”；目录绑定成功 Toast+invalidate。
3. Settings 分为“语言与外观”“AI Provider”“通知”“Windows 启动”“快捷键”卡片。
4. Provider 表单必填校验；不得把未配置 Key 显示成已配置；保存/测试/删除使用 loading、Toast、ConfirmDialog。
5. SearchOverlay 全部双语；结果选中后能导航到目标页面，并在可能时传递 selected entity id。无法精确定位时至少显示明确提示，不得假装定位成功。
6. 侧栏使用中文默认标签、英文切换、active indicator、计数徽标；窄宽允许折叠但所有入口可达。
7. 所有页面删除重复 button/card/muted 基础 CSS，保留页面特有布局；基础视觉只来自 design tokens/共享组件。
8. 运行 `rg` 搜索 `Today|Workspace|Works|Plan|Waiting|Calendar|Inbox|Settings|Morning Brief` 等硬编码用户文案，逐项迁移或记录合理例外。

#### Expected Result
全部主要页面视觉和语言一致，Workspace/Provider 状态可信，导航和搜索不再是半成品。

#### Evidence
记录 Workspace 状态截图、Settings 中英文截图、搜索定位结果、硬编码扫描清单和例外说明。

#### Verdict
- PASS：页面收口和检查通过；进入 STEP 19。
- STOP：仍有主要页面使用原型单行表单、明显混合语言或 Provider Key 状态错误。

#### Exception Handling
- 搜索结果无法精确定位且需大改路由：显示明确提示并记录；若用户目标必须路由重构则 `E16-DECISION-REQUIRED`。
- Provider 状态需要读取 Key 内容才能判断：`E19-SECURITY-BOUNDARY`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 19 — 完成可访问性、响应式和交互一致性检查
Delegation: SAFE_TO_DELEGATE

#### Purpose
消除视觉重构后常见的键盘、焦点、滚动、对比度和状态反馈问题。

#### Input
全部 Svelte 页面和共享组件、1024×640/1440×900 目标尺寸。

#### Action
1. 所有 icon-only button 添加双语 `aria-label` 和 title；所有输入具有 label。
2. 键盘可完成导航、打开/提交/关闭 Modal、ConfirmDialog、语言切换和搜索。
3. Modal 打开后焦点进入，关闭后回到触发按钮；Esc 关闭不提交。
4. loading/disabled/错误/成功状态不仅依赖颜色；StatusBadge 同时显示文本。
5. 检查文字与背景对比；禁止浅灰文字放在白色上低于可读程度。
6. 1024×640 不出现页面级水平滚动，侧栏/内容区/Modal 关键按钮可达；1440×900 利用空间但不过度拉伸。
7. 删除 body 默认 scrollbar 和嵌套双 scrollbar；长列表只在指定容器滚动。
8. 日期、数字和空状态在中英文下不溢出；英文长按钮允许合理换行或保持最小宽度。
9. 运行 `pnpm check`，修复全部 warning/error。

#### Expected Result
键盘、焦点、状态和两种窗口尺寸均可用，没有明显可访问性回退。

#### Evidence
记录键盘流程、焦点返回、两个尺寸的 scrollWidth/clientWidth、对比度抽查和 `pnpm check` 输出。

#### Verdict
- PASS：全部机械检查通过；进入 STEP 20。
- STOP：关键动作键盘不可达、滚动阻断或静态检查非零。

#### Exception Handling
- 无法在现有 WebView 自动检测某项：记录人工复查步骤；核心交互仍无法验证则 `E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

## Phase F — 端到端验证、构建与交付

### STEP 20 — 修复并扩展隔离 UI CDP 冒烟
Delegation: CONDITIONAL

#### Purpose
用真实页面点击证明按钮、反馈、关联、刷新、语言和 Brief 闭环，而不是只直接 invoke command。

#### Input
`scripts/smoke-cdp.py`、`scripts/smoke-test.ps1`、隔离启动脚本、页面 `data-testid`。

#### Action
1. Gate：确认 Python 可以 import `websocket`，release/debug WebView 可通过隔离启动脚本开启指定调试端口。任一不满足时按异常停止。
2. 修改 CDP 连接使用 `suppress_origin=True` 或等价的不发送 Origin 方式，不设置宽泛 `remote-allow-origins=*`。
3. 保留 command-level smoke 作为后端证据，但新建 `scripts/ui-smoke-cdp.py` 通过 DOM 输入、click、keyboard 真实操作。
4. 为稳定定位给关键控件添加 `data-testid`；禁止根据易变 CSS 序号定位。
5. UI flow 必须覆盖：中文默认；空 Work 错误；有效 Work；Task/Waiting/Calendar 关联该 Work；Quick Capture 同页刷新；Inbox 转 Task；语言切换并重启持久化；Dashboard 计数；本地 Brief 生成；Calendar 编辑删除。
6. 脚本捕获 `window.error`、`unhandledrejection` 和页面 `.status.error`；预期校验错误除外，其他错误使脚本失败。
7. 所有数据写入隔离 APPDATA；测试前记录正式 DB 元数据，测试后比较不变。
8. 输出每一步 PASS/FAIL、最终数据库各表计数和截图路径；不输出用户数据或 Key。

#### Expected Result
UI 级冒烟全部 PASS，当前 WebView2 可连接，正式 DB 未修改。

#### Evidence
记录 gate、脚本输出、PASS 数、runtime error 数、隔离 DB 计数和正式 DB 元数据对比。

#### Verdict
- PASS：所有 UI flow 通过；进入 STEP 21。
- STOP：依赖或调试连接缺失、任一 UI flow 失败、正式 DB 元数据变化。

#### Exception Handling
- websocket-client 缺失且不能使用现有环境安装：`E06-DEPENDENCY-MISSING`。
- 调试端口不可用：`E05-TOOL-UNAVAILABLE`。
- 正式数据库被触碰：`E19-SECURITY-BOUNDARY`。
- UI flow 失败：`E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 21 — 运行完整静态检查和单元测试
Delegation: SAFE_TO_DELEGATE

#### Purpose
确认全部改动在前端类型、Rust 单元和 migration 层没有回归。

#### Input
完成后的源码、lockfiles 和测试。

#### Action
1. 在项目根运行 `pnpm check`。
2. 在 `src-tauri` 运行 `cargo fmt --check`；失败时运行 `cargo fmt` 后重新检查，并记录格式化文件。
3. 在 `src-tauri` 运行 `cargo test`，不使用过滤器。
4. 运行 `pnpm build`。
5. 统计 Rust 测试总数、通过数、失败数；记录前端 error/warning 数。
6. 检查构建输出没有真实路径、Key 或 Authorization header。

#### Expected Result
所有命令退出码为 0，测试全通过，前端 0 errors/0 warnings。

#### Evidence
逐命令记录工作目录、退出码、关键输出和测试计数。

#### Verdict
- PASS：四项检查全部通过；进入 STEP 22。
- STOP：任一退出码非 0 或敏感信息出现在输出。

#### Exception Handling
- 编译/测试失败：`E09-VALIDATION-FAILED`。
- 敏感信息泄露：`E19-SECURITY-BOUNDARY`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 22 — 执行隔离持久化和离线文件变化验收
Delegation: SAFE_TO_DELEGATE

#### Purpose
证明重启后语言/数据保持，并验证应用离线期间文件变化能进入 Activity 和 Brief。

#### Input
隔离 APPDATA、隔离 workspace、构建后的 exe、UI smoke 脚本。

#### Action
1. 清理前先解析并验证 `.test-runtime\luna-workbench` 位于项目根；只清理该隔离目录的旧测试数据库和 workspace 内容。
2. 第一次启动：绑定隔离 workspace，创建 Work/Task/Waiting/Calendar/Inbox，切换英文，生成本地 Brief，记录计数后从托盘正常退出。
3. 应用停止时在隔离 workspace 新建一个文件、修改一个既有文件、删除一个既有文件；内容使用无敏感测试文本。
4. 第二次启动：确认语言仍为英文、所有实体仍存在、reconcile 产生 created/modified/deleted、Dashboard 文件变化和 Brief 来源计数更新。
5. 再切换中文并正常退出；第三次启动确认中文保持。
6. 完成后正常退出所有隔离进程；记录 PID 已不存在。
7. 再次比较正式数据库只读元数据与 STEP 01，必须完全相同。

#### Expected Result
实体、语言、文件基线和 Brief 跨重启保持；离线变化被准确发现；正式数据不变。

#### Evidence
记录三次启动 PID、每次语言/计数、三类文件事件、Brief source_counts、正式 DB 元数据对比。

#### Verdict
- PASS：全部持久化和离线变化通过；进入 STEP 23。
- STOP：数据丢失、语言不保持、文件差异漏记/重复、残留进程或正式 DB 变化。

#### Exception Handling
- 隔离目录解析不在项目根：`E13-DESTRUCTIVE-ACTION`。
- 持久化/差异失败：`E09-VALIDATION-FAILED`。
- 正式数据库变化：`E19-SECURITY-BOUNDARY`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 23 — 构建 release、更新文档和生成视觉证据
Delegation: SAFE_TO_DELEGATE

#### Purpose
形成可交付构建、与实现一致的文档和最终审阅材料。

#### Input
通过全部测试的源码、README/architecture/smoke checklist、截图目录。

#### Action
1. 运行 `pnpm tauri build`，记录退出码。
2. 验证 `src-tauri\target\release\msl-desktop.exe` 和 `bundle\nsis\*.exe` 存在，记录大小和修改时间。
3. 使用 release exe 和隔离 APPDATA 再运行一次短 UI smoke，不打开正式数据库。
4. 在 1024×640 和 1440×900 截取：中文 Dashboard、英文 Dashboard、Work Modal、Calendar Week、Brief 来源面板；保存到 `output/playwright/final-*`。
5. 更新 `README.md` 的功能、开发、测试、Brief fallback、双语和已知限制。
6. 更新 `docs/architecture.md` 的前端状态、workspace inventory、Brief source pipeline、migration v2、credential_ref 和测试架构。
7. 更新 `docs/smoke-checklist.md`，加入空表单、跨页面刷新、Work 关联、语言持久化、离线变化和 local Brief。
8. 不修改旧 stage 报告来伪造历史；新建 `docs/workbench-rebuild-report.md` 记录本次实际改动、测试证据和剩余限制。

#### Expected Result
release 构建可用、文档与实际一致、五类截图齐全。

#### Evidence
记录构建命令、产物路径/大小/时间、release smoke、文档文件和截图绝对路径。

#### Verdict
- PASS：构建、release smoke、文档和截图全部完成；进入 STEP 24。
- STOP：release 构建或 smoke 失败、文档与实现矛盾、截图缺失。

#### Exception Handling
- release 构建失败：`E09-VALIDATION-FAILED`。
- 需要代码签名/发布才能继续：记录为范围外；若被当成完成前提则 `E12-SCOPE-CHANGE`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 24 — 在正式数据库副本上验证 migration，不启动正式应用
Delegation: CONDITIONAL

#### Purpose
证明真实 v1 数据形状可安全升级，同时保持正式数据库完全不变。

#### Input
正式数据库只读路径、隔离 artifacts 目录、release exe 或 migration 测试工具。

#### Action
1. Gate：确认没有 MSL Desktop 进程，正式 DB 文件元数据与 STEP 01 一致；若不一致或进程存在则停止。
2. 将正式 `msl-desktop.db` 复制到 `.test-runtime\luna-workbench\artifacts\production-db-copy\`；如存在 WAL/SHM，在应用已确认退出后一起复制。不得移动或删除原文件。
3. 仅对副本运行 migration，可通过设置隔离 APPDATA 指向副本目录后启动 release 并正常退出。
4. 对副本运行 `PRAGMA integrity_check`、schema version、表计数对比；除 migration 新列/表和 schema_migrations 外，原表行数不得减少。
5. 验证现有 Provider `credential_ref` 为旧 `provider-{id}`；不得读取 keyring。
6. 再次记录正式 DB 元数据，必须与 Gate 完全一致。
7. 在执行报告中写明：未启动正式数据库，首次真实启动应由用户在已有备份后完成。

#### Expected Result
正式数据副本可无损升级，原数据库未改变。

#### Evidence
记录 Gate、复制路径、integrity_check、version、前后表计数和正式 DB 元数据对比；不记录用户正文。

#### Verdict
- PASS：副本完整升级且原文件未变；进入 STEP 25。
- STOP：进程存在、元数据变化、副本 integrity 失败或行数减少。

#### Exception Handling
- 应用仍运行：`E21-USER-AUTHORIZATION-REQUIRED`。
- 复制权限不足：`E04-PERMISSION-DENIED`。
- migration 导致行数减少/完整性失败：`E10-ACCEPTANCE-FAILED`。
- 正式数据库被改变：`E19-SECURITY-BOUNDARY`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 25 — 完成执行报告并进入视觉审阅门
Delegation: HIGH_MODEL_REQUIRED

#### Purpose
汇总全部可验证证据，并把主观视觉/产品审阅明确交给用户或更高能力模型。

#### Input
`ACCEPTANCE.md`、全部步骤证据、最终截图、构建产物、修改文件清单。

#### Action
1. 按 `ACCEPTANCE.md` 逐条标记 PASS/FAIL，不得省略。
2. 完成 `EXECUTION_REPORT.md`：步骤、证据、文件、命令、偏差、停止代码、交付物、未解决问题。
3. 只有所有机械标准 PASS 时，状态改为 `PARTIALLY_COMPLETED`，并说明等待视觉审阅；存在机械失败则使用 `FAILED` 或 `BLOCKED`。
4. 向用户提供五类截图和不超过 10 条的变更摘要，请求审阅：视觉层级、中文自然度、信息密度、Dashboard 成熟度。
5. 用户或更高能力模型明确通过视觉门后，才可将状态改为 `COMPLETED`。
6. 运行 execution pack validator，验证本目录为 VALID；记录结果。

#### Expected Result
机械证据完整，主观视觉门未被 Luna 自行越过，最终状态准确。

#### Evidence
完整执行报告、Acceptance 对照、截图路径、validator 输出和视觉审阅结论。

#### Verdict
- PASS：机械标准通过且视觉审阅明确通过，状态 `COMPLETED`。
- STOP：等待审阅时状态 `PARTIALLY_COMPLETED`；机械失败使用对应 stop code。

#### Exception Handling
- 视觉审阅需要用户决定：`E21-USER-AUTHORIZATION-REQUIRED`，这是预期门槛，不视为工程失败。
- Acceptance 任一机械项失败：`E10-ACCEPTANCE-FAILED`。
- 执行包 validator 失败：`E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

## 标准停止载荷

任何 STOP 必须在回复和 `EXECUTION_REPORT.md` 中使用以下字段：Code、Step、Expected、Observed、Evidence、Why execution cannot continue、Last safe completed step、No further action taken。停止后不得继续后续步骤。

