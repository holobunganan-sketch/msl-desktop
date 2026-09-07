# Steps

> 执行器必须严格按编号执行。每一步完成后先把 Action、Expected、Observed、Evidence、Verdict 写入本目录的 `EXECUTION_REPORT.md`，再进入下一步。默认工作目录为 `C:\Myfolder\MSL cowork\msl-desktop`。

## Phase A — 冻结现状并隔离所有新副作用

### STEP 01 — 验证环境、运行进程、工作树与基线
Delegation: CONDITIONAL

#### Purpose
确认执行现场仍与编译包一致，保护现有未提交成果，并建立本轮测试基线。

#### Input
项目根、`package.json`、`src-tauri/Cargo.toml`、Git 状态、`msl-desktop.exe` 进程、正式 APPDATA 文件元数据。

#### Action
1. 运行 `Get-Location`，确认目录精确为项目根。
2. 运行 `git rev-parse --is-inside-work-tree` 与 `git status --short`，只读记录状态；不得修改索引或工作树。
3. 运行 `node --version`、`pnpm --version`、`rustc --version`、`cargo --version`、`python --version`。
4. 运行 `Get-Process -Name msl-desktop -ErrorAction SilentlyContinue | Select-Object Id,Path,StartTime`。
5. 如果存在任意 MSL Desktop 进程，记录实际 PID 与路径并立即 STOP；不得结束它。
6. 只读记录 `%APPDATA%\MSLDesktop\msl-desktop.db`、`-wal`、`-shm` 的存在性、长度和 LastWriteTimeUtc，不查询表内容。
7. 运行 `pnpm check`。
8. 在 `src-tauri` 运行 `cargo test`，记录通过、失败和 warning 数。
9. 确认上一阶段新增的 i18n、shared UI、Workspace inventory、UI smoke 文件仍存在。

#### Expected Result
工具可用，无运行中的应用，Git 现有改动被识别为保护对象，前端 0 errors/0 warnings，Rust 34 个或更多测试全部通过，正式数据库只被读取文件元数据。

#### Evidence
记录版本、Git short status 摘要、进程检查、正式 DB 文件元数据、`pnpm check` 与 `cargo test` 的退出码和计数。

#### Verdict
- PASS：全部条件满足，进入 STEP 02。
- STOP：存在应用进程、工具缺失、源码结构重大变化或基线失败。

#### Exception Handling
- 应用仍在运行：`E21-USER-AUTHORIZATION-REQUIRED`。
- 工具缺失：`E05-TOOL-UNAVAILABLE`。
- 结构或 schema 已发生实质变化：`E02-ENVIRONMENT-MISMATCH`。
- 基线检查失败：`E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 02 — 扩展隔离运行环境
Delegation: SAFE_TO_DELEGATE

#### Purpose
保证新正文缓存、临时文件、WebView 数据、调度状态和数据库全部落入项目内测试区。

#### Input
`scripts/run-isolated-audit.ps1`、`scripts/stop-isolated-audit.ps1`、`.test-runtime` 规则。

#### Action
1. 将本轮隔离根固定为 `.test-runtime\luna-ai-secretary`。
2. 在该根下使用 `appdata`、`localappdata`、`temp`、`workspace`、`artifacts`、`mock-provider` 子目录。
3. 修改 `run-isolated-audit.ps1`，启动子进程前同时设置进程级 `APPDATA`、`LOCALAPPDATA`、`TEMP`、`TMP`，并在输出中只打印隔离路径和 PID。
4. 保留原有 PID+exe 路径停止校验；停止脚本不得按进程名批量结束。
5. 给启动脚本加入 dry-run，输出四个隔离环境变量与预计数据库、cache root，但不启动应用。
6. 解析所有隔离路径为绝对路径并验证位于项目根；验证失败不得创建或清理目录。
7. 将 `.test-runtime/` 和生成截图继续保持在 ignore 规则中，不改变其他 ignore。
8. 运行 dry-run 并记录输出。

#### Expected Result
以后任何测试启动都同时隔离 roaming data、local cache 和系统 temp，路径检查可机械证明不会落入正式目录。

#### Evidence
记录脚本路径、四个环境变量、路径前缀验证代码、dry-run 输出和未启动进程的证明。

#### Verdict
- PASS：四类目录全部隔离并通过绝对路径 gate，进入 STEP 03。
- STOP：任一测试副作用仍可能写入正式 APPDATA、LOCALAPPDATA 或系统 temp。

#### Exception Handling
- 隔离路径不在项目根：`E19-SECURITY-BOUNDARY`。
- 无法创建测试目录：`E04-PERMISSION-DENIED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

## Phase B — Provider 连接、模型目录与任务路由

### STEP 03 — 追加 migration 0003 AI Provider Catalog
Delegation: CONDITIONAL

#### Purpose
无损把当前单 Provider 单模型结构升级为连接、模型目录与任务路由三层结构。

#### Input
`0001_init.sql`、`0002_workbench_reliability.sql`、`db/migrations.rs`、`db/provider.rs`。

#### Action
1. Gate：在临时 v2 数据库确认最高 migration version 为 2，`provider_models` 与 `ai_task_routes` 不存在。
2. 新建 `src-tauri/migrations/0003_ai_provider_catalog.sql`，不得修改 0001/0002。
3. 严格按 `CONTEXT.md` 追加 `provider_settings` 四列，创建 `provider_models` 与 `ai_task_routes` 及 provider、enabled、task kind 索引。
4. 将旧 Provider 非空 `model` 迁入一条 `provider_models`，protocol=`chat_completions`、endpoint_path=`/chat/completions`、source=`legacy`、enabled=1、available=1。
5. 旧 Provider 若 `base_url` 规范化后等于 `https://api.deepseek.com`，设置 template_kind=`deepseek`；其他旧行设置 custom。
6. 在 `MIGRATIONS` 按升序登记 version 3。
7. 添加 migration 测试：全新库到 v3、v2 到 v3 保留所有旧行、旧 model 成功迁入、重复 open 幂等、外键与 unique 生效。
8. 运行 `cargo test db::tests`；失败不得进入下一步。

#### Expected Result
临时数据库无损升级到 v3，现有 Provider 与 key reference 保持，新模型目录和路由表可用。

#### Evidence
记录 gate、migration version、PRAGMA table_info、旧行迁移前后计数、测试名与退出码，不记录 Key。

#### Verdict
- PASS：v2→v3 与全新库测试全部通过，进入 STEP 04。
- STOP：现有 schema 不是 v2、旧 Provider 无法无损表示、需要删表或清数据。

#### Exception Handling
- Gate 不符：`E02-ENVIRONMENT-MISMATCH`。
- 需要破坏性 migration：`E13-DESTRUCTIVE-ACTION`。
- migration 测试失败：`E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 04 — 实现 Provider Catalog repository 与稳定类型
Delegation: SAFE_TO_DELEGATE

#### Purpose
让后端可靠管理 Provider connection、多个 model 与 task route，并保留旧 command 兼容。

#### Input
`db/provider.rs`、`commands/mod.rs`、前端 domain types、migration 0003。

#### Action
1. 在 `db/provider.rs` 定义 `ProviderConnection`、`ProviderModel`、`AiTaskRoute`，现有 `ProviderSetting` 可保留别名或兼容 struct。
2. repository 实现 connection CRUD、model upsert/list/get/enable/availability、route upsert/list/resolve。
3. 对 template_kind、auth_mode、protocol、source、task_kind 做固定枚举校验；非法值不得写库。
4. Provider 删除前检查路由引用；依靠 FK SET NULL 后返回受影响 task kinds，让 UI 显示需重新配置。
5. 保留 `credential_ref` 和 keyring 读写方式；模型行不得持有 Key。
6. 旧 `list_providers`、`save_provider` command 保留兼容 wrapper，但新 UI 使用新的 connection/model commands。
7. 新 commands 放入 `src-tauri/src/commands/provider_catalog.rs`，由 `commands/mod.rs` 声明和 re-export，不继续扩张单体文件业务逻辑。
8. 新 command 固定为：`list_provider_connections`、`save_provider_connection`、`list_provider_models`、`save_provider_model`、`set_provider_model_enabled`、`list_ai_task_routes`、`save_ai_task_route`。
9. 添加 repository/command 测试：同 Provider model_id unique、不同 Provider 可同名、route 删除后置 null、Key ref 不变。

#### Expected Result
连接、模型和任务路由能独立管理，旧 Provider 数据仍可读，Key 不进入模型表或返回日志。

#### Evidence
记录新增类型、commands、repository 测试和 key reference 保持断言。

#### Verdict
- PASS：repository 与 command tests 通过，进入 STEP 05。
- STOP：需要移动或重写旧凭据、旧 Provider 无法读取、路由完整性不成立。

#### Exception Handling
- 凭据需要读取明文才能迁移：`E19-SECURITY-BOUNDARY`。
- 旧 API 无法兼容：`E11-CONFLICTING-EVIDENCE`。
- 测试失败：`E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 05 — 重构三协议 AI adapter
Delegation: SAFE_TO_DELEGATE

#### Purpose
正确调用 Chat Completions、Responses 和 Anthropic Messages，不让业务代码关心协议差异。

#### Input
`src-tauri/src/ai/provider.rs`、reqwest、ProviderModel protocol/endpoint。

#### Action
1. 将 `provider.rs` 拆为 `ai/provider.rs` 公共 facade 与 `ai/adapters.rs` 协议实现；保持 `save/get/delete/has_api_key` 行为。
2. 定义统一 `AiTextRequest`：model_id、system、messages、temperature、max_output_tokens；定义统一 `AiTextResponse`：content、model、usage 可选、request_id 可选。
3. Chat Completions：POST `base_url + endpoint_path`，Bearer，解析 `choices[0].message.content`。
4. Responses：POST endpoint，使用 `model` 与 message input；优先解析顶层 `output_text`，否则扫描 `output[].content[].text`；两者都没有则返回明确 parse error。
5. Anthropic Messages：system 与 messages 分离，POST endpoint，发送 Bearer、`x-api-key`、`anthropic-version: 2023-06-01`，解析 `content[]` 中 type=text 的 text。
6. URL 通过 `reqwest::Url` 解析；只允许 http/https；禁止 URL 中包含 username/password；endpoint_path 必须以 `/` 开头且不得含 `..`。
7. 错误只返回 status、Provider/model 安全标识和最多 200 characters 的脱敏 message；删除 Authorization、x-api-key、响应 header 和请求正文。
8. 添加 `tests/mock_ai_server.rs` 或等价项目内 helper，用 `TcpListener` 返回三种固定响应并捕获请求路径、协议字段和 header 名，不保存 secret value。
9. 添加三协议成功、错误状态、坏 JSON、空内容、超时和 secret redaction 测试。

#### Expected Result
三个协议通过同一 facade 返回文本，错误安全，业务层不再拼接 `/chat/completions`。

#### Evidence
记录三种 mock 请求路径、解析结果、脱敏断言与测试退出码。

#### Verdict
- PASS：全部 adapter mock tests 通过，进入 STEP 06。
- STOP：任何协议必须用真实 Provider 验证、secret 出现在错误或无法统一返回文本。

#### Exception Handling
- 测试要求真实 Key：`E14-CREDENTIALS-REQUIRED`。
- 外网成为测试依赖：`E20-NETWORK-FAILURE`。
- 协议与官方快照冲突：`E11-CONFLICTING-EVIDENCE`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 06 — 实现 DeepSeek、OpenCode Go 与 Custom 模板
Delegation: SAFE_TO_DELEGATE

#### Purpose
提供用户只填 Key 即可使用的两个固定模板，同时允许没有品牌预设的自定义接口。

#### Input
`CONTEXT.md` 官方快照、Provider repository、adapter、Settings 当前 DeepSeek preset。

#### Action
1. 新建 `ai/catalog.rs`，把 DeepSeek 与 OpenCode Go 固定 connection defaults 和模型协议映射写成 typed constants。
2. DeepSeek 创建模板时预填 base URL、models endpoint、Bearer、V4 Pro/Flash；用户只需 Key 即可保存并测试。
3. OpenCode Go 创建模板时预填 base URL、models endpoint、Bearer，并 seed `CONTEXT.md` 列出的全部已知 model mapping。
4. 实现 `refresh_provider_models(provider_id)`：GET models endpoint，用 key 鉴权，读取 data[].id；已知模型更新 available；远端缺失只标 unavailable；远端未知写 disabled/manual-review row，不猜 protocol。
5. 模型刷新失败时保留现有 catalog 并返回可见 warning，不清空模型。
6. Custom 创建表单为空：用户显式填写 display name、base URL、auth mode；每个模型显式填写 model_id、protocol、endpoint_path；不显示 DeepSeek/OpenCode 默认值。
7. 固定模板的 base URL 在普通模式只读；高级编辑必须显示会失去模板保证的警告，但保存后 template_kind 改为 custom。
8. 连接测试必须让用户选择具体模型，走该模型 protocol；禁止继续使用 connection 的 legacy `model` 字段决定测试协议。
9. 用 mock `/models` 和三协议响应测试创建、刷新、未知模型、离线保留和 connection test。

#### Expected Result
DeepSeek/OpenCode Go 能一键建立正确结构，自定义接口不冒充模板，模型刷新不会破坏已有配置。

#### Evidence
记录两个模板字段、OpenCode 模型协议计数、unknown model 状态、mock refresh/test 结果。

#### Verdict
- PASS：固定模板、custom、刷新和连接测试全部通过，进入 STEP 07。
- STOP：OpenCode Go 模型被统一错误协议、刷新会删除配置或需要真实网络。

#### Exception Handling
- 官方模型快照无法按 schema 表达：`E16-DECISION-REQUIRED`。
- Mock 无法覆盖鉴权：`E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 07 — 实现按任务模型路由器
Delegation: SAFE_TO_DELEGATE

#### Purpose
让不同 AI 能力使用用户指定的 Provider model，并保证失败不会静默换模型。

#### Input
`ai_task_routes`、ProviderModel、keyring facade、六个固定 task kinds。

#### Action
1. 新建 `ai/router.rs`，唯一公开入口为 `resolve(task_kind)` 与 `complete(task_kind, request)`。
2. 解析顺序固定：精确 task_kind -> 用户显式配置的 general -> 配置错误。
3. route 指向 disabled/unavailable model、disabled connection 或缺 Key 时返回具名错误，包含 task/provider/model，不包含 Key。
4. 不自动选择 list 中第一个 enabled Provider，不根据价格或模型名称猜测。
5. Daily Brief 调用路由失败时由 Brief 层使用现有 local renderer；其他 AI 功能显示配置错误，不伪造结果。
6. 保存 route 时验证 model 属于启用 connection，protocol 已知，model enabled；available=false 允许保存但必须显示 warning。
7. 添加测试：精确 route、general fallback、无 route、disabled、unavailable、缺 Key 的纯解析分支；keyring IO 使用注入的 fake credential source。

#### Expected Result
六类任务路由可预测，模型不可用时错误明确，只有用户配置的 general 才能作为 fallback。

#### Evidence
记录路由决策测试矩阵和错误中不含 credential 的断言。

#### Verdict
- PASS：路由矩阵全部通过，进入 STEP 08。
- STOP：仍存在 first-enabled 隐式选择或错误泄露 credential。

#### Exception Handling
- 路由语义与旧 Brief 冲突：保留 local renderer，无法兼容则 `E11-CONFLICTING-EVIDENCE`。
- 测试失败：`E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 08 — 重建 Provider 与 AI 分工设置界面
Delegation: SAFE_TO_DELEGATE

#### Purpose
让非技术用户可以完成模板接入、模型选择与任务分工，而不编辑 JSON。

#### Input
`SettingsView.svelte`、shared UI、i18n、Provider/route commands。

#### Action
1. 将 Provider 区域抽为 `ProviderSettings.svelte`，路由区域抽为 `AiRoutingSettings.svelte`。
2. 顶部显示三个入口：DeepSeek、OpenCode Go、自定义接口；固定模板卡明确“只需填写 API Key”。
3. Connection card 显示模板类型、连接状态、Key 是否配置、模型数、上次刷新、测试按钮、编辑和删除。
4. OpenCode Go/DeepSeek 卡提供“刷新模型”；Custom 卡提供“添加模型”。
5. 模型表显示 display name、model_id、protocol、available、enabled；未知模型必须显示“需选择协议”。
6. 任务分工显示六行固定任务，每行选择 Provider + Model；未配置、不可用、使用 general fallback 都有不同文字状态。
7. API Key 输入默认为空，不回显；保存空 Key 表示保留旧 Key，显式“删除凭据”单独二次确认。
8. 所有保存、测试、刷新、删除有 loading、disabled、Toast 和 inline error；错误不显示响应正文。
9. 中英文词条全部进入 typed dictionary；protocol/model/base URL 不翻译。
10. mutation 后 invalidate provider/settings/global，不要求用户切页刷新。

#### Expected Result
用户不需要理解数据库即可配置两个模板、自定义模型和任务分工；凭据与错误显示安全。

#### Evidence
记录中文/英文设置截图、DeepSeek/OpenCode/custom 表单、六任务路由状态、空 Key 保留行为和 `pnpm check`。

#### Verdict
- PASS：设置流程完整且静态检查通过，进入 STEP 09。
- STOP：仍只有单 Provider model 输入、任务不能独立选择或 Key 被回显。

#### Exception Handling
- 现有 Settings 无法拆分而不破坏功能：`E11-CONFLICTING-EVIDENCE`。
- UI 需要大型组件库：`E12-SCOPE-CHANGE`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 09 — 完成 Provider 模块隔离回归
Delegation: SAFE_TO_DELEGATE

#### Purpose
在进入正文和 AI 秘书实现前冻结 Provider 基础，防止后续错误难以定位。

#### Input
migration 0003、Provider repository、adapters、catalog、router、Settings UI、mock server。

#### Action
1. 运行与 Provider 相关的全部 Rust tests。
2. 运行 `pnpm check`。
3. 构建 debug exe，使用 STEP 02 隔离环境和 mock server 启动。
4. 通过 UI 创建 DeepSeek 与 OpenCode Go connection，使用 synthetic key，刷新 mock models，配置六任务路由。
5. 退出并重启隔离实例，确认 connection/model/routes 持久化，synthetic key 不在 SQLite 或日志。
6. 删除一个被 route 使用的模型，确认 route 变为未配置且 UI 明确显示。
7. 停止隔离进程和 mock server，比较正式 DB 元数据与 STEP 01 完全相同。

#### Expected Result
Provider 三层结构通过单元、UI、持久化与凭据隔离验证。

#### Evidence
记录测试退出码、UI 操作、隔离 DB 聚合计数、重启结果、日志 secret scan 和正式 DB 对比。

#### Verdict
- PASS：Provider 模块全部通过，进入 STEP 10。
- STOP：持久化丢失、Key 落库、route 静默切换或正式数据变化。

#### Exception Handling
- UI/mock flow 失败：`E09-VALIDATION-FAILED`。
- synthetic secret 泄露：`E19-SECURITY-BOUNDARY`。
- 正式 DB 变化：`E19-SECURITY-BOUNDARY`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

## Phase C — 工作目录正文与增量文档智能

### STEP 10 — 追加 migration 0004 Document Intelligence
Delegation: CONDITIONAL

#### Purpose
建立 Work/Workspace 关联、文档索引和可追踪缓存表，不把正文写进 SQLite。

#### Input
migration version 3、`CONTEXT.md` 0004 schema、workspace/work repositories。

#### Action
1. Gate：临时数据库最高版本为 3，三个目标表不存在。
2. 新建 `0004_document_intelligence.sql`，创建 `work_workspace_links`、`document_index`、`cache_entries` 和必要索引。
3. FK 删除规则：workspace 删除时 link/document_index cascade；work 删除时 link cascade；summary_model_id 删除时 SET NULL。
4. `cache_entries.relative_path` 建唯一约束，禁止绝对路径语义。
5. 在 migration runner 登记 version 4。
6. 添加全新 v4、v3→v4、幂等、link FK、document unique、cache path 字段测试。
7. 测试 SQL 明确断言 SQLite 不存在 full_content、raw_body、prompt_body 列。

#### Expected Result
文档与缓存元数据 schema 可用，旧业务行全部保留，数据库没有长期正文列。

#### Evidence
记录 gate、version、表/索引、FK、旧表行数和正文列排除断言。

#### Verdict
- PASS：migration tests 全部通过，进入 STEP 11。
- STOP：需要复制或重写旧 Work/Workspace、schema 会存完整正文或需要破坏性操作。

#### Exception Handling
- Gate 不符：`E02-ENVIRONMENT-MISMATCH`。
- 破坏性 migration：`E13-DESTRUCTIVE-ACTION`。
- 测试失败：`E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 11 — 建立专用 cache root 与文档依赖
Delegation: CONDITIONAL

#### Purpose
让正文提取物进入可控、可清理、与正式数据分离的位置，并安装限定依赖。

#### Input
`Cargo.toml`、`db/mod.rs`、Windows 环境变量、migration 0004。

#### Action
1. 在 `Cargo.toml` 添加 `sha2`、`zip`、`quick-xml`、`pdf-extract`、`encoding_rs`、`chrono`；只在实际使用 `tokio::sync` 时给 tokio 增加 sync feature。
2. 更新 lockfile，运行 `cargo check`；依赖不可获得时只允许一次普通重试，然后 STOP。
3. 新建 `storage/paths.rs`，定义 cache、tmp、logs root；优先 `%LOCALAPPDATA%\MSLDesktop`，无 LOCALAPPDATA 时回退到 `default_app_data_dir()/local`。
4. cache 子目录固定为 `extracted`、`chunks`、`model-catalog`、`previews`。
5. 实现 `safe_cache_path(relative)`：拒绝 absolute、prefix、parent component、空路径；join 后规范化父目录，必须仍在 canonical cache root。
6. 创建缓存文件使用临时文件 + flush + rename；失败不得留下被登记为 ready 的半文件。
7. 实现 cache entry register/touch/remove repository；relative_path 使用 `/` 统一分隔。
8. 添加 isolated path、traversal、absolute、rename failure、APPDATA/LOCALAPPDATA 分离测试。

#### Expected Result
正文缓存只能创建在专用 local cache root，路径穿越和半写入被阻止，依赖编译通过。

#### Evidence
记录依赖版本、cache root、路径拒绝测试、原子写测试和 `cargo check`。

#### Verdict
- PASS：依赖、路径与原子写测试通过，进入 STEP 12。
- STOP：依赖无法获得、cache root 无法与正式数据分离或路径边界不可证明。

#### Exception Handling
- 依赖缺失：`E06-DEPENDENCY-MISSING`。
- cache root 边界不明：`E19-SECURITY-BOUNDARY`。
- 编译失败：`E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 12 — 实现 DOCX、PDF 与文本提取器
Delegation: SAFE_TO_DELEGATE

#### Purpose
按照固定白名单本地读取正文，并对不支持、扫描版、超大或编码失败给出可解释状态。

#### Input
`CONTEXT.md` Supported Documents、tests fixtures、新 Rust dependencies。

#### Action
1. 新建 `documents/mod.rs`、`docx.rs`、`pdf.rs`、`text.rs`、`chunk.rs`。
2. 定义 `ExtractedDocument`：text、char_count、truncated、content_hash、language_hint、warnings；定义稳定 status 枚举。
3. 读取前用 symlink_metadata；只处理普通文件，不跟随 symlink/reparse target；检查 50 MiB 上限。
4. DOCX 作为 ZIP 打开，只读取规定的 Word XML entry；用 quick-xml 收集 `w:t`，段落结束加换行，tab/break 转为可读空白；禁止外部关系和宏。
5. PDF 用 pdf-extract 提取；trim 后为空返回 `needs_ocr`，解析损坏返回 `failed_parse`。
6. 文本按规定编码顺序解码；NUL 或替换字符比例超限返回对应失败状态。
7. 计算 SHA-256；提取超过 2,000,000 characters 时保留前 1,500,000 与后 500,000 并标 truncated。
8. 按 6,000/300 规则分段；每段带 index、start/end character 和 source hash。
9. 新增 synthetic fixtures：含段落/表格的 DOCX、文字 PDF、无文字 PDF、UTF-8/UTF-16/GBK 文本、超大 stub、二进制和 unsupported extension。
10. 测试不得把 fixture 全文打印到 stdout，只断言 hash、片段标记、计数和状态。

#### Expected Result
支持文件可稳定提取，不支持与失败状态准确，全文不进入日志。

#### Evidence
记录 fixture 名、每类 extract_status、char/hash 断言、chunk count 和敏感输出扫描。

#### Verdict
- PASS：全部 extractor tests 通过，进入 STEP 13。
- STOP：DOCX/PDF 需要外部应用、扫描 PDF 被误报 ready、二进制被当文本或正文进入日志。

#### Exception Handling
- crate API 与版本不符且无法按同一语义实现：`E02-ENVIRONMENT-MISMATCH`。
- 测试正文泄露：`E19-SECURITY-BOUNDARY`。
- 解析测试失败：`E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 13 — 实现增量文档索引、缓存与 dirty 标记
Delegation: SAFE_TO_DELEGATE

#### Purpose
只重新处理真正变化的文件，并为 AI 分析提供有限、可追溯的正文来源。

#### Input
workspace inventory/watcher、document_index、cache_entries、extractors。

#### Action
1. 新建 `documents/indexer.rs`，扫描复用 inventory 的目录排除和 temp file 规则。
2. 对新增/mtime+size 变化文件计算 hash；hash 未变化只更新时间和 touch cache，不重新提取。
3. 支持文件提取成功后将正文与 chunks 写 cache，再事务更新 document_index ready；写失败时不得提交 ready。
4. unsupported、too_large、needs_ocr、failed 状态写 document_index，error_message 限长 300 且不含正文。
5. 删除源文件时删除 document_index，并把关联 cache entry 标记 orphan；不在 watcher 回调中直接递归清理。
6. watcher 事件只标记 document dirty 并更新 metadata，不调用 AI；reconcile 后调度 blocking index task。
7. 同一 workspace 同时只有一个 index job；AppState 保存 atomic guard 与最近状态。
8. 新 commands：`workspace_document_status`、`list_workspace_documents`、`reindex_workspace_documents`；返回计数和状态，不返回全文。
9. 添加首次索引、无变化复用、mtime 变但 hash 相同、正文变化、删除、失败重试、并发 guard 测试。

#### Expected Result
文档索引增量、可追踪、不会实时调用 AI，缓存与 DB 状态保持一致。

#### Evidence
记录 extract 调用次数、hash 变化、cache entry 数、dirty/index 状态和并发测试。

#### Verdict
- PASS：增量与一致性测试通过，进入 STEP 14。
- STOP：每次扫描都重提取、watcher 直接调用 AI、源删除导致清理器访问源目录或 ready 指向缺失缓存。

#### Exception Handling
- cache/DB 原子性失败：`E09-VALIDATION-FAILED`。
- 文件权限错误：单文件记录 failed 并继续；根目录不可读使用 `E04-PERMISSION-DENIED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 14 — 建立 Work 与 Workspace 关联和文档状态 UI
Delegation: SAFE_TO_DELEGATE

#### Purpose
让目录可以属于一个或多个 Work，并让用户看到 AI 实际能读取哪些文件。

#### Input
work_workspace_links、Workspace/Works 页面、document commands。

#### Action
1. repository/commands 实现 link、unlink、list by work、list by workspace；同一 workspace 至多一个 primary link。
2. Workspace 卡显示关联 Work、文档 ready/unsupported/failed/needs OCR/too large 数、最近索引时间和“重新索引”。
3. 文档列表只显示相对路径、类型、大小、状态、摘要是否存在、最后分析时间和跳过原因，不显示全文。
4. Works detail 增加“工作目录”区域，可选择已有 workspace 或绑定新目录；解绑关系不删除 workspace 或源文件。
5. 新绑定目录完成 metadata baseline 后自动排队本地正文索引，但不立即调用 AI。
6. 提供“分析并生成工作草稿”按钮；此时只发出后续 STEP 21 将实现的 command，占位按钮必须 disabled 并显示“AI 引擎尚未配置”，不得成为无动作按钮。
7. 所有文案中英文齐全，列表长时在 Workspace 页内部正常滚动，不影响 Dashboard 无滚动要求。
8. 添加 link/unlink/primary、解绑不删源/不删 work、UI status tests。

#### Expected Result
用户明确知道目录与 Work 的关系以及哪些文件可被读取，绑定不会自动写业务建议。

#### Evidence
记录关联测试、目录状态截图、解绑前后表计数和源 fixture 存在性。

#### Verdict
- PASS：关联、状态和安全边界通过，进入 STEP 15。
- STOP：解绑会删源文件、文档列表泄露正文或绑定后直接写 Work。

#### Exception Handling
- 现有 Workspace 语义冲突：`E11-CONFLICTING-EVIDENCE`。
- 源目录发生写操作：`E19-SECURITY-BOUNDARY`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

## Phase D — AI 分析、建议队列与确认应用

### STEP 15 — 追加 migration 0005 AI Secretary
Delegation: CONDITIONAL

#### Purpose
持久化分析调度、运行、建议和 Brief 保留状态，为确认式 AI 管理建立事实边界。

#### Input
migration version 4、`CONTEXT.md` 0005 schema、daily_briefs。

#### Action
1. Gate：临时 DB 最高版本为 4，目标表/列不存在。
2. 新建 `0005_ai_secretary.sql`，严格创建 schedule singleton、analysis_runs、ai_proposals，并追加 daily_briefs 三列。
3. 插入 id=1 默认 schedule：enabled=1、interval_minutes=180、daily_enabled=1、daily_hour=6、daily_minute=0。
4. 旧 daily_briefs retention_state 默认 kept；后续自动生成由代码显式写 draft。
5. 为 runs status/time、proposal status/dedupe/workspace/work 建索引；pending dedupe 用 partial unique index。
6. analysis_run_id 使用 nullable FK SET NULL，避免清理 run 时删除确认建议；brief_id 使用 SET NULL。
7. 登记 migration version 5。
8. 添加 v4→v5、默认 schedule、old brief kept、partial dedupe、FK、幂等测试。

#### Expected Result
AI 运行与建议有独立持久化，旧 Brief 自动受到保护，默认调度准确。

#### Evidence
记录 gate、schema、default row、旧 Brief migration 和测试结果。

#### Verdict
- PASS：migration tests 通过，进入 STEP 16。
- STOP：旧 Brief 需要删除/重写、建议无法与业务表隔离或 schema 不可无损升级。

#### Exception Handling
- Gate 不符：`E02-ENVIRONMENT-MISMATCH`。
- 破坏性迁移：`E13-DESTRUCTIVE-ACTION`。
- 测试失败：`E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 16 — 构建全局 AI 分析快照
Delegation: SAFE_TO_DELEGATE

#### Purpose
把整个工作台和文档派生事实组织成有限、可追溯的输入，不把数据库或目录无界倾倒给模型。

#### Input
现有 BriefSnapshot、Work/Task/Waiting/Calendar/Inbox/Activity、document_index 与 chunks。

#### Action
1. 新建 `ai/analysis_snapshot.rs`，定义 typed `AnalysisSnapshot`。
2. 包含：未归档 Work、最新 Resume、open/完成 Task、open Waiting、相关 Calendar、未处理 Inbox、最近 Activity/file changes、workspace links、document status/summary、选中正文 chunks。
3. 默认周期范围为上次成功 run 到当前；首次 run 最多回看 7 天；daily run 使用昨天 00:00 到今天 06:00，并附今天已安排事项。
4. 正文选择顺序：本期 changed ready files -> 无 summary 的新 ready files -> 与 active Work 关联且最近使用的文档；单次最多 20 个文件。
5. 每个文件最多发送 40,000 characters；全请求正文总预算 120,000 characters；超出时记录 truncated counts，不静默丢失。
6. snapshot source_ref 包含类型、实体 id 或相对路径、workspace id、content hash、时间；不包含 cache absolute path。
7. snapshot hash 覆盖结构化事实、文档 hash、范围、locale 和 task kind。
8. 实现用于审计的 `source_counts`，但日志只写计数/hash，不写正文。
9. 扩展现有 BriefSnapshot 时保留旧 local renderer 所需字段和测试。
10. 添加全来源、预算、排序、截断、hash 变化、正文不进入 Debug/log 测试。

#### Expected Result
AI 输入覆盖全局信息与相关正文，同时有明确预算、来源和 hash，不产生无界 prompt。

#### Evidence
记录 synthetic snapshot 各来源计数、选中文件顺序、字符预算、truncated count、hash 与日志排除断言。

#### Verdict
- PASS：snapshot tests 全部通过，进入 STEP 17。
- STOP：事实类别缺失、正文无预算、cache absolute path/Key 进入 snapshot 或旧 Brief 回归。

#### Exception Handling
- 预算无法满足单个最小输入：返回 partial snapshot 并 warning；不得提高无限上限。
- 敏感凭据进入 snapshot：`E19-SECURITY-BOUNDARY`。
- 测试失败：`E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 17 — 实现文档摘要与全局分析引擎
Delegation: SAFE_TO_DELEGATE

#### Purpose
按任务路由调用 AI，把文档变化和工作台事实转为结构化 summary 与 proposals。

#### Input
router、AnalysisSnapshot、document_index、analysis_runs、AI output contract。

#### Action
1. 新建 `ai/analysis.rs` 与 `ai/schema.rs`；所有后台入口统一走 `run_analysis(trigger, scope)`。
2. 运行开始先原子检查 AppState analysis_running；已运行则返回 busy，不启动第二个实例。
3. 创建 analysis_run status=running，再构建 snapshot；任何失败更新 run=failed/partial 并释放 guard。
4. 对缺 summary 或 hash 改变的 selected document，使用 `workspace_analysis` route 生成有限摘要；成功写 document_index summary/hash/model/time，失败记录文件 warning 并继续全局分析。
5. 全局建议调用 `global_analysis`；workspace import 调用 `work_draft`；prompt 明确只输出 AI Output Contract JSON，禁止 delete/archive/send/execute。
6. 解析流程：trim -> 移除单层 code fence -> JSON parse -> typed validation；禁止从任意自然语言中猜测字段。
7. 每条 source_ref 必须能在 snapshot 中找到；update target 必须存在；时间、枚举、work link 和字段白名单必须通过应用层校验。
8. 无模型/Key/网络时 run=failed 或 partial，保留已有文档摘要，不创建伪建议；daily brief 可在 STEP 23 使用 local fallback。
9. 成功后 run=awaiting_review；没有 proposal 时 run=completed，summary 仍保存。
10. Mock tests 覆盖文档摘要成功/失败、合法 JSON、code fence、坏 JSON、未知操作、伪 source、并发 guard 和错误释放。

#### Expected Result
AI 分析运行可追踪、结构化、受来源约束；失败不会写业务表或卡住调度器。

#### Evidence
记录 mock run statuses、document summary 更新、proposal count、非法输出拒绝和 guard 释放测试。

#### Verdict
- PASS：分析引擎所有 mock tests 通过，进入 STEP 18。
- STOP：解析器需要猜测自然语言、非法建议可进入队列、失败仍写业务表或 guard 不释放。

#### Exception Handling
- 模型配置缺失：记录 run failed，单元测试继续；产品验收中按配置错误显示。
- 真实 Provider 被请求：`E14-CREDENTIALS-REQUIRED`。
- schema 验证失败：`E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 18 — 实现 Proposal repository、去重与版本保护
Delegation: SAFE_TO_DELEGATE

#### Purpose
保证反复分析不会制造无限重复建议，也不会覆盖用户已经编辑的草稿。

#### Input
ai_proposals、validated proposal、analysis_runs。

#### Action
1. 新建 `db/ai.rs`，实现 AnalysisRunRepo、ProposalRepo、ScheduleRepo。
2. dedupe key 固定由 kind、operation、target id 或 workspace、规范化 title、关键日期和 source hash 组成。
3. 新建议命中 pending 且 user_edited=0 时更新 payload/reason/source/confidence/run id；命中 user_edited=1 时保留原 proposal，只把新证据追加到受限 source_refs。
4. 命中 confirmed/rejected 时，新建议只有 source hash 或关键字段实质变化才创建；否则忽略。
5. 同一批分析中重复建议先内存合并，再写库。
6. status 只允许 pending/confirmed/rejected/superseded；更新使用乐观 `updated_at` 检查防止 UI 覆盖后台变化。
7. list command 支持 status/kind/workspace/run filter，limit 1–200；detail 返回 editable payload，不返回 document body。
8. 添加 create/update/edit/reject/dedupe/user-edited protection/concurrent timestamp tests。

#### Expected Result
建议队列有界、可编辑、可追溯，后台不会覆盖用户判断。

#### Evidence
记录每个 dedupe 分支的前后计数、payload、user_edited 和 source_refs 断言。

#### Verdict
- PASS：Proposal repository tests 通过，进入 STEP 19。
- STOP：同一建议会无限新增、用户编辑被覆盖或全文进入 proposal payload/source_refs。

#### Exception Handling
- dedupe 无法稳定计算：`E16-DECISION-REQUIRED`。
- 并发一致性失败：`E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 19 — 实现 Proposal 确认事务与业务实体应用器
Delegation: SAFE_TO_DELEGATE

#### Purpose
把用户确认的建议原子、安全地写入正确模块，并保证拒绝或失败不会产生半完成状态。

#### Input
Proposal payload、现有 Work/Task/Waiting/Calendar/Inbox/Resume repositories、command 校验函数。

#### Action
1. 新建 `ai/apply.rs`，唯一写入口为 `confirm_proposal(id, expected_updated_at, edited_payload)`。
2. 确认前重新读取 proposal，要求 status=pending 且 updated_at 与 UI 一致；否则返回 stale error。
3. edited_payload 再走与人工表单相同的 required、enum、time、work_id、target 存在性校验。
4. create work 只写 title/summary/status；create task/waiting/calendar/inbox/resume 使用现有 repository 字段，不扩展业务语义。
5. update 仅允许修改目标已有的非破坏字段；禁止 delete、archive、complete、resolve、send。
6. proposal status 更新、实体写入和 `ai.proposal.confirmed` Activity 必须在同一 SQLite transaction；任一步失败全部回滚。
7. Work draft 确认后，如果同 run 的其他 pending proposal work_id 为空且 workspace 相同，将其绑定到新 work_id；不自动确认它们。
8. Reject 只更新 proposal status/reason/time 和 activity，不修改目标实体。
9. 提供 `confirm_ai_proposal`、`reject_ai_proposal`、`update_ai_proposal_draft` commands。
10. 添加每种 kind 成功、非法字段、stale、回滚、拒绝不写、Work 后续绑定测试。

#### Expected Result
确认建议准确进入对应模块，所有失败原子回滚，用户仍逐项掌控。

#### Evidence
记录六类 proposal 的表计数、transaction rollback、Activity 与 status 断言。

#### Verdict
- PASS：全部应用器 tests 通过，进入 STEP 20。
- STOP：存在绕过确认的写路径、失败留下半条数据或建议能执行破坏性操作。

#### Exception Handling
- 现有 repository 无法共享事务：在本步骤增加接受 `&Connection`/transaction 的内部方法；需要改变业务语义则 `E16-DECISION-REQUIRED`。
- 一致性测试失败：`E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 20 — 建立 AI 建议审阅中心
Delegation: SAFE_TO_DELEGATE

#### Purpose
实现用户要求的逐个查看、直接修改、确认或拒绝的秘书式交互。

#### Input
Proposal commands、shared Modal/ConfirmDialog、i18n、domain revisions。

#### Action
1. 新建 `AiReviewCenter.svelte` 与 `ProposalEditor.svelte`，可从顶部栏、Dashboard pending 指标和分析完成通知打开。
2. 顶部显示待确认总数、当前第几项、来源 run、模型、生成时间；提供上一项/下一项。
3. 根据 kind 显示普通表单字段，不显示 raw JSON；Work/Task/Waiting/Calendar/Inbox/Resume 字段与人工表单一致。
4. 显示 AI reason、confidence、最多 5 个 source reference；文件只显示 workspace 相对路径。
5. 用户修改任一字段即调用 draft update 并标 user_edited；离开再回来仍保留。
6. “确认并写入”必须显示目标模块，提交 loading；成功后 Toast、invalidate 对应 domain，并前进到下一项。
7. “拒绝”需要可选短原因；“稍后处理”只关闭，不改变 status。
8. 提供“确认本批全部”但默认折叠，点击后列出数量与目标类型并二次确认；批量确认逐条事务，任一失败停止并报告已完成 id，不谎报全部成功。
9. Window 收到 `ai-analysis-completed` event 时显示非阻塞 banner；窗口关闭时使用系统通知，重开后 pending 仍可见。
10. 中文/英文、键盘焦点、Esc、错误保留输入、stale refresh 全部覆盖。

#### Expected Result
每条 AI 建议可读、可改、可确认、可拒绝，确认后立即进入正确页面，没有技术 JSON 暴露。

#### Evidence
记录六类 editor 截图、编辑持久化、确认/拒绝、stale 和批量部分失败 UI 结果。

#### Verdict
- PASS：审阅闭环和 `pnpm check` 通过，进入 STEP 21。
- STOP：用户无法编辑、确认后目标不可见、关闭窗口会丢 pending 或存在自动接受。

#### Exception Handling
- 某实体字段无法用现有表单表达：`E11-CONFLICTING-EVIDENCE`。
- 需要引入 JSON editor/大型表单库：`E12-SCOPE-CHANGE`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 21 — 实现绑定目录后的 AI Work 草稿流程
Delegation: SAFE_TO_DELEGATE

#### Purpose
让用户绑定或导入一份工作后，由 AI 生成可编辑模板，避免手工填写大量项目。

#### Input
Workspace binding/indexing、work_draft route、analysis engine、review center、work_workspace_links。

#### Action
1. Workspace 绑定成功后仅自动执行 metadata baseline 和 document index，不自动调用 AI。
2. 索引完成后显示 CTA“分析目录并生成工作草稿”；如果 work_draft route/Key 缺失，CTA 保留可见并给配置入口。
3. 点击 CTA 创建 trigger=`workspace_import` 的 analysis run，scope 只含该 workspace、其文档摘要/正文和未处理关联信息。
4. Prompt 要求最多生成：1 个 Work、1 个 Resume Point、10 个 Task、5 个 Waiting、10 个 Calendar、10 个 Inbox；不得为了凑数生成空泛事项。
5. Work proposal 必须排在本 run 第一项；未确认 Work 前，其他 proposal 显示“确认工作后继续”，但仍可编辑。
6. 用户确认 Work 后建立 primary work_workspace_link，并把本 run 后续建议绑定新 work_id。
7. 用户拒绝 Work 时，后续 proposal 保持 pending 但 work_id 为空，允许选择已有 Work 或逐项拒绝，不自动删除。
8. 已关联现有 Work 的 workspace 再分析时不生成重复 Work，生成 update/resume/task 等建议。
9. 添加 mock end-to-end tests：新目录草稿、编辑标题、确认 Work、后续绑定、拒绝 Work、已有 Work 不重复。

#### Expected Result
目录导入从“手填全部字段”变为“AI 草拟—用户修改—确认入库”，且不会擅自创建实体。

#### Evidence
记录 analysis run、proposal 顺序、确认前后各表计数、workspace link 和 UI 截图。

#### Verdict
- PASS：新旧 Work 两条流程全部通过，进入 STEP 22。
- STOP：绑定后自动写 Work、Work 未确认就写后续实体或重复 Work 无法抑制。

#### Exception Handling
- 工作目录没有可读文件：仍允许用目录名/metadata 生成最小草稿并显示证据不足；不得编造细节。
- 模型配置缺失：可见错误与设置入口；核心 CRUD 不受影响。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

## Phase E — 定时秘书、Brief 与翻译

### STEP 22 — 实现默认 3 小时与每日 06:00 AI 调度
Delegation: SAFE_TO_DELEGATE

#### Purpose
让 Resident Core 在用户可设置的节奏综合分析，而不是文件一变化就调用模型。

#### Input
analysis_schedule_state、notifications loop、AppState、analysis engine、chrono Local。

#### Action
1. 新建 `scheduler/mod.rs`，与 reminders 调度分离；tick 固定每 60 秒，MissedTickBehavior=Skip。
2. 使用 `chrono::Local` 计算本地日期和 06:00，不用当前 notifications 的 UTC 粗略格式。
3. 周期配置范围 30–1440 分钟，默认 180；daily hour 0–23、minute 0–59，默认 06:00。
4. due 规则：enabled 且 last_interval_run_at 为空或已超过 interval；daily_enabled 且当前时间已过目标、last_daily_local_date 不是今天。
5. daily 与 interval 同 tick 到期时只启动 trigger=daily，并同时更新两个 last state，避免双调用。
6. 启动前检查 AppState analysis_running；busy 时本 tick skip，不更新 last state。
7. 文件 watcher/reconcile 只更新 dirty 状态；不得直接启动 scheduler run。
8. daily run 先生成/刷新 Brief，再运行 global proposals；interval run 生成全局 proposals，不强制生成 Brief。
9. 成功、partial、failed 都记录 run；网络失败不在一分钟内无限重试，last attempt 记录后等下一周期或手动 retry。
10. 新 commands：`get_analysis_schedule`、`save_analysis_schedule`、`run_analysis_now`、`list_analysis_runs`、`retry_analysis_run`。
11. Settings 新建 `AnalysisScheduleSettings.svelte`，提供启用、间隔、每日时间、立即分析、最近运行与下次预计时间。
12. 使用注入 clock 测试 180 分钟、06:00、跨日、重启防重复、合并、busy、失败不狂重试；不得用 sleep 等待三小时。

#### Expected Result
定时分析节奏可预测、可配置、跨重启防重复，文件变化不会实时花费 API。

#### Evidence
记录 fake clock 时间矩阵、run count、last state、Settings 截图和无 watcher AI 调用断言。

#### Verdict
- PASS：全部 scheduler tests 和设置 UI 通过，进入 STEP 23。
- STOP：同一到期点重复运行、时区错误、失败每分钟重试或 watcher 直接调用 AI。

#### Exception Handling
- 本地时区无法从系统读取：`E02-ENVIRONMENT-MISMATCH`。
- 调度持久化失败：`E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 23 — 将 Daily Brief 升级为 AI 秘书顶部简报
Delegation: SAFE_TO_DELEGATE

#### Purpose
综合昨天/所选期间的工作台与文档信息，给出今天推进重点，并正确管理草稿与正式保留。

#### Input
现有 Brief engine、AnalysisSnapshot、daily route、analysis run、daily_briefs retention columns。

#### Action
1. Brief 输入增加 document summaries、changed document source refs、analysis summary 和 pending proposal counts；不把整个 cache path 写入 snapshot。
2. Daily AI route 成功时输出固定四段：昨日/期间进展、风险与等待、今日硬安排、建议推进 1–5 项。
3. AI route 缺失或失败继续使用现有 bilingual local renderer，内容非空且显示 warning。
4. 自动 06:00 生成的 Brief 写 retention_state=draft；同日新 draft 将旧 draft 标 superseded，不改变 kept。
5. 用户点击“保留简报”把当前 draft 改 kept；kept 永不被自动覆盖或清理。
6. 首页只显示最新未 superseded draft，若没有则显示今天最新 kept；历史页面区分 draft/kept/superseded。
7. Brief 顶部摘要只显示最多 3 条推进建议和 2 条风险；全文与来源进入 Modal/Drawer。
8. 生成 Brief 后 emit `brief-updated` 与 `ai-analysis-completed`，Dashboard 不切页刷新。
9. 测试正文 summary 进入事实、local fallback、同日 supersede、keep 保护、daily schedule、来源计数和 hash。

#### Expected Result
Brief 真正综合目录与工作台，自动生成但可保留，首页顶部始终有紧凑而有用的摘要。

#### Evidence
记录 mock/local 内容段落、source counts、draft/kept 状态转换、同日记录计数和 UI 截图。

#### Verdict
- PASS：Brief 数据与 UI tests 通过，进入 STEP 24。
- STOP：无 AI 时 Brief 为空、kept 被覆盖、正文来源不可追溯或 Brief 仍在首页底部。

#### Exception Handling
- 模型输出格式不符：使用 local renderer 并记录 warning，不创建伪 AI 内容。
- kept 数据会被清理：`E13-DESTRUCTIVE-ACTION`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 24 — 实现首页 AI 中英翻译
Delegation: SAFE_TO_DELEGATE

#### Purpose
提供轻量、随手可用的中文到英文和英文到中文翻译，支持书面与口语风格。

#### Input
translation route、router、Dashboard shared UI、clipboard API。

#### Action
1. 新建 `TranslationCard.svelte` 与 `translate_text` command。
2. 输入为空前端与后端都拒绝；输入最大 20,000 characters，超过显示明确限制。
3. 本地检测方向：Han characters 占非空字母数字的 20% 及以上 -> zh-to-en，否则 en-to-zh；把检测结果作为提示，模型仍只执行该方向。
4. 风格固定 `written` 与 `spoken`；prompt 要求只返回译文，不加解释，不改变数字、专有名词和换行结构。
5. 使用 translation task route；无 route/Key/网络显示错误和设置入口，不使用伪本地翻译。
6. UI 包含源文本框、检测方向、风格切换、翻译按钮、结果框、复制、清空；Ctrl+Enter 触发。
7. 输入与输出只保存在 Svelte component memory；不写数据库、cache、Activity 或日志。
8. 页面卸载或点击清空时释放内容；错误仍保留源文本。
9. Mock tests 覆盖中文、英文、混合、两风格、空、超长、模型失败、结果复制与不持久化。

#### Expected Result
首页翻译快速、明确、不增长数据库，不影响其他 AI 任务模型选择。

#### Evidence
记录方向/风格 mock request、UI 两语言截图、数据库前后计数和日志正文排除。

#### Verdict
- PASS：翻译功能与不持久化验证通过，进入 STEP 25。
- STOP：翻译历史落库、方向明显错误、错误清空输入或使用错误 task route。

#### Exception Handling
- Clipboard 权限不可用：保留可选择结果文本并显示 warning；翻译本身继续。
- Mock 调用失败：`E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

## Phase F — 无滚动 Dashboard 收口

### STEP 25 — 将 Dashboard 改为无页面滚动的固定高度工作总览
Delegation: SAFE_TO_DELEGATE

#### Purpose
让 1024×640 和 1440×900 的首页摘要一眼可见，把详情移出默认页面流。

#### Input
当前 `TodayView.svelte`、截图、Brief、Review Center、TranslationCard、app shell 尺寸。

#### Action
1. 保持主窗口 min size 1024×640；在 <=1100px 自动使用 76px 折叠侧栏，保证内容宽度。
2. Dashboard root 使用可用内容区 100% height、overflow hidden、三行 grid：顶部 Brief 132–148px、指标条 64–72px、主体 minmax(0,1fr)。
3. Quick Capture 合并进顶部栏的单行输入，不再单独占 Dashboard 纵向区域。
4. 顶部 Brief 横跨全宽，显示日期、生成状态、最多 3 条今日推进、最多 2 条风险、查看全文、保留/重新生成和待确认建议入口。
5. 指标固定五项：active Work、今日/逾期 Task、今日 Calendar、需跟进 Waiting、pending AI proposals；点击进入对应页面/Review Center。
6. 主体使用三列：左列“今天安排+继续推进”，中列“Waiting+Inbox+文件变化”，右列“AI 秘书状态+TranslationCard”。
7. 每个默认列表最多显示 3 项，超出显示“还有 N 项”；不得在 Dashboard 默认卡片中出现内部滚动条。
8. Brief 全文、来源、全部列表和 Proposal 进入可滚动 Modal/Drawer；隐藏内容必须有可见入口和键盘访问。
9. 单模块错误在自己的卡片显示 retry，不扩大整体高度；loading skeleton 使用固定高度。
10. CSS 不得仅用 overflow hidden 截掉按钮；CDP 必须断言所有规定模块和 CTA 的 bounding rect 在 viewport 内。
11. 1024×640 和 1440×900 下断言 document/body/dashboard `scrollHeight <= clientHeight + 1`、`scrollWidth <= clientWidth + 1`，且垂直 scrollbar 不出现。
12. 中英文均验证长文本截断、tooltip/详情入口与无滚动。

#### Expected Result
首页 Brief 在最上方，核心摘要与翻译全部可见，无页面或卡片滚动条，详情仍可达。

#### Evidence
记录两种语言、两种尺寸截图，DOM scroll metrics，全部模块 bounding rect 和 Modal 可达性。

#### Verdict
- PASS：四组尺寸/语言机械断言全部通过，进入 STEP 26。
- STOP：存在页面滚动、关键模块不可见、被 CSS 裁掉或 1024 宽布局不可读。

#### Exception Handling
- 无法在规定高度放下全部摘要：进一步减少默认行数与 padding，禁止移除需求模块；仍失败则 `E09-VALIDATION-FAILED`。
- 需要大型 Dashboard 框架：`E12-SCOPE-CHANGE`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

## Phase G — 精细缓存与长期存储治理

### STEP 26 — 追加 migration 0006 Storage Governance
Delegation: CONDITIONAL

#### Purpose
建立活动压缩与清理运行审计，使长期增长可控且可解释。

#### Input
migration version 5、`CONTEXT.md` 0006 schema、activity_events、app_settings。

#### Action
1. Gate：临时 DB 最高版本为 5，rollup/cleanup 表不存在。
2. 新建 `0006_storage_governance.sql`，创建两个表、表达式 unique index 与时间索引。
3. 通过 INSERT OR IGNORE 写入五个默认 cache setting，仅在 key 不存在时写，不覆盖用户值。
4. 登记 migration version 6。
5. 添加 v5→v6、全新 v6、默认 setting 不覆盖、rollup unique、cleanup audit、幂等测试。

#### Expected Result
存储治理 schema 无损加入，默认策略可持久化，用户已有设置不被覆盖。

#### Evidence
记录 gate、version、默认 values、重复 migration 和旧表计数。

#### Verdict
- PASS：migration tests 全部通过，进入 STEP 27。
- STOP：需要删除 Activity/Brief、默认设置覆盖已有值或 migration 非幂等。

#### Exception Handling
- Gate 不符：`E02-ENVIRONMENT-MISMATCH`。
- 破坏性 migration：`E13-DESTRUCTIVE-ACTION`。
- 测试失败：`E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 27 — 实现存储盘点与清理计划预览
Delegation: SAFE_TO_DELEGATE

#### Purpose
在删除前准确告诉用户正式数据、缓存、临时文件和可释放空间分别是多少。

#### Input
cache_entries、storage paths、document_index、analysis/proposal/brief/activity tables。

#### Action
1. 新建 `storage/usage.rs` 与 `storage/cleanup.rs`，定义 category：extracted、chunks、model_catalog、previews、temp、logs、webview、analysis_history、superseded_briefs、rejected_proposals、activity_raw。
2. `get_storage_usage` 返回每类 bytes/count、正式 DB 文件大小、cache 总量、上限、高水位、最后清理时间；工作目录大小不计算进应用占用。
3. cache size 只统计 safe cache root 下 manifest entry 和 app 自有子目录；symlink/reparse entry 标 skipped。
4. `preview_cleanup(mode,categories)` 生成 plan_id、cutoff、estimated bytes/count、protected counts、warnings，不执行删除。
5. Smart mode 固定候选：temp>24h、failed artifacts>7d、model catalog>7d、extracted/chunks 30天未访问、logs>30天或总量>100MiB、superseded brief drafts>30天、rejected/superseded proposal>30天。
6. Pending proposal、confirmed proposal、kept brief、正式实体、Key、DB 文件、source workspace 永远列入 protected，不出现在 delete candidates。
7. 容量高于 80% 时，把 rebuildable cache 按 last_accessed_at LRU 加入计划，直到预计降到 60%；未超高水位不做容量 LRU。
8. plan 只保存内存并 10 分钟过期；执行时使用 plan cutoff，之后新建或被 touch 的 entry 不删除。
9. 添加大小统计、TTL、LRU、protected、symlink、plan expiry 和 80→60 测试。

#### Expected Result
用户可在清理前看到精确范围，保护对象永远不会出现在删除计划。

#### Evidence
记录 synthetic storage 各 category、预计 bytes、protected counts、LRU 顺序和路径安全测试。

#### Verdict
- PASS：盘点与计划 tests 通过，进入 STEP 28。
- STOP：工作目录被计入删除候选、kept/confirmed 被列入、路径边界或估算不可验证。

#### Exception Handling
- 文件在盘点中消失：标记 changed 并在执行时重算，不视为错误。
- 路径越界：`E19-SECURITY-BOUNDARY`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 28 — 实现原子、安全、可恢复的缓存清理器
Delegation: SAFE_TO_DELEGATE

#### Purpose
按预览计划释放空间，同时保证分析、源文件和正式工作不受影响。

#### Input
CleanupPlan、AppState analysis/index guards、cache manifest、SQLite。

#### Action
1. `execute_cleanup(plan_id)` 先验证 plan 未过期、路径 root、analysis/index/另一个 cleanup 均未运行；busy 时返回明确错误。
2. 对每个 cache candidate 再检查 relative path、symlink_metadata、last_accessed<=cutoff、rebuildable=1；不满足则 skip。
3. 删除文件成功后才删除 cache_entries；文件不存在视为已清理并移除 stale manifest；删除失败保留 manifest 和错误。
4. document_index 指向被删 cache 时清空 cache_rel_path，extract_status 改 pending，但保留 summary/hash/正式来源。
5. 数据库历史清理在事务中执行；kept/confirmed/pending 通过 WHERE 条件明确排除。
6. Activity 原始记录的删除不在本步骤进行，留给 STEP 29 先 rollup。
7. 完成后执行 `wal_checkpoint(TRUNCATE)` 与 `PRAGMA optimize`；普通 Smart cleanup 不运行 VACUUM。
8. 写 `storage_cleanup_runs`，记录 before/after/count/error，不记录文件名或正文；只保留最近 50 条 cleanup audit。
9. 自动清理最多每 24 小时一次；启动时仅在 auto enabled 且 TTL/高水位命中时运行 Smart mode。
10. 添加中断、部分失败、cache in use、touch after preview、manifest stale、document pending、正式 DB/源文件 hash 不变测试。

#### Expected Result
清理只删除可重建缓存和明确过期历史，失败可见且不会造成正式数据损坏。

#### Evidence
记录 before/after bytes、deleted/skipped/failed counts、document status、cleanup audit 和 protected/source hash。

#### Verdict
- PASS：全部安全与失败测试通过，进入 STEP 29。
- STOP：任何正式对象或源文件变化、分析期间删除、失败后状态谎报成功。

#### Exception Handling
- cache 文件被占用：记录 skipped/busy，继续其他候选。
- 删除权限不足：记录 per-entry failure；全部失败则 `E04-PERMISSION-DENIED`。
- 正式或源数据变化：`E19-SECURITY-BOUNDARY`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 29 — 实现 Activity 压缩、Brief 草稿回收与 WebView 清理
Delegation: CONDITIONAL

#### Purpose
控制数据库与 WebView 长期增长，同时保留长期工作事实和用户正式记录。

#### Input
daily_activity_rollups、activity_events、daily_briefs、ai_proposals、Tauri WebviewWindow API。

#### Action
1. 实现 `compact_activity_history(cutoff=now-90d)`：按本地日期、workspace、work、event_type 聚合 count/first/last/sample，upsert rollup 后在同一事务删除对应 raw events。
2. sample_text 最多 200 characters，不含 metadata_json 或正文；重复 compact 幂等。
3. Brief 只删除 superseded 且 retention_state=draft 且 superseded_at 超过 30 天的记录；kept 永远排除。
4. Proposal 只删除 rejected/superseded 超过 30 天；pending/confirmed 永远排除。
5. Analysis run 超过 90 天时清除 error_message/source_counts 以外的可重建明细或删除无 proposal/brief 引用的 run；保留 compact summary。
6. 提供高级“压缩数据库空闲空间”：先 checkpoint，确认无 analysis/index/cleanup，记录 integrity_check=ok 后运行 SQLite VACUUM，再次 integrity_check；仅用户显式点击，自动清理不运行。
7. Gate：编译确认当前 Tauri `WebviewWindow::clear_all_browsing_data()` 存在。
8. 提供高级 WebView 清理 command，调用该 API；不得手工删除 WebView data dir；成功后提示需要 reload/reopen UI。
9. 添加 rollup 汇总一致、幂等、protected Brief/Proposal、VACUUM 副本完整性测试；WebView 用可用 mock dispatcher 或隔离 UI command 验证。

#### Expected Result
详细活动转为小型长期 rollup，正式记录受保护，WebView 通过官方 API 清理。

#### Evidence
记录 raw before/after、rollup totals、protected ids、integrity_check、DB size 和 WebView API 结果。

#### Verdict
- PASS：压缩、保护、VACUUM 副本与 WebView gate 通过，进入 STEP 30。
- STOP：Tauri API 不存在、rollup count 不守恒、kept/confirmed/pending 被删或需要手工删除 WebView 目录。

#### Exception Handling
- Tauri API 不存在：`E02-ENVIRONMENT-MISMATCH`。
- integrity_check 非 ok：`E10-ACCEPTANCE-FAILED`。
- 保护边界失败：`E19-SECURITY-BOUNDARY`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 30 — 建立“存储与清理”设置界面
Delegation: SAFE_TO_DELEGATE

#### Purpose
让用户看懂空间去向、调整自动策略并安全执行 Smart/Advanced cleanup。

#### Input
storage usage/preview/execute commands、Settings、i18n、ConfirmDialog。

#### Action
1. 新建 `StorageSettings.svelte`，显示总缓存、正式 DB、提取、chunks、模型目录、temp、logs、运行历史、上次清理。
2. 显示默认 2 GiB 上限、80% 高水位、60% 目标和 auto cleanup 开关；上限允许 256 MiB–20 GiB。
3. “智能清理”先 preview，Modal 展示每类预计释放、protected 数和 warning，确认后执行。
4. “高级清理”提供可重建类别复选框、Activity 压缩、旧草稿/拒绝记录、WebView 数据、数据库压缩；高风险动作分别二次确认。
5. 正式数据库、源工作目录、API Key、confirmed proposal、kept brief 不显示为可选删除项。
6. 执行期间按钮 loading/disabled；完成显示实际释放、skipped、failed；部分失败不显示全成功。
7. 显示最近 10 次 cleanup audit，不显示具体源文件名。
8. 中英文、键盘、错误状态、refresh usage 和设置持久化覆盖。

#### Expected Result
存储管理透明、可预览、可控制，用户不会误以为正式数据是缓存。

#### Evidence
记录中文/英文 usage、Smart preview、Advanced 保护项、部分失败和 audit 截图。

#### Verdict
- PASS：Storage UI 与 `pnpm check` 通过，进入 STEP 31。
- STOP：存在“一键清空全部数据”、正式对象可选删除、无 preview 或部分失败谎报成功。

#### Exception Handling
- 某 category 无法准确计量：显示 unknown 与 warning，不计入预计释放；核心类别 unknown 则 `E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

## Phase H — 前后端收口与全量验证

### STEP 31 — 收口 typed API、i18n、可访问性与错误边界
Delegation: SAFE_TO_DELEGATE

#### Purpose
确保新增能力和现有工作台一致，不留下硬编码、重复类型或静默按钮。

#### Input
全部新增 commands/components、`api.ts`、`domain.ts`、i18n dictionaries、dataRevision。

#### Action
1. 把新增 domain type 集中到 `src/lib/types/domain.ts`，不得在多个 component 复制 Provider/Proposal/Storage 类型。
2. 把新 invoke 封装到 `src/lib/services/api.ts`；统一 error normalization，禁止 component 直接拼 command 参数。
3. dataRevision 增加 providers、analysis、proposals、documents、storage、translation 必要域；translation 不持久化，不触发 global data reload。
4. 所有用户可见静态文本加入 zh-CN/en-US typed dictionary；运行 `rg` 扫描新组件硬编码中文/英文。
5. 所有 icon button 有 aria-label/title，输入有 label，Modal/Drawer focus 进入/返回，Esc 不提交。
6. 所有异步按钮 loading/disabled，失败保留输入，成功 Toast+invalidate；不允许空 onclick 或只改变本地假状态。
7. Provider/analysis/document/storage 错误在 UI 脱敏，最多显示安全摘要。
8. 运行 `pnpm check`，必须 0 errors/0 warnings。

#### Expected Result
新增功能类型统一、双语完整、键盘可用、错误安全，所有按钮都有真实闭环。

#### Evidence
记录类型/API 清单、hardcoded scan 例外、键盘流程和 `pnpm check`。

#### Verdict
- PASS：全部收口检查通过，进入 STEP 32。
- STOP：关键按钮无作用、硬编码混合语言、类型重复导致漂移或错误泄露正文/Key。

#### Exception Handling
- 旧组件存在不相关历史硬编码：只修本任务可见区域并记录；影响验收则 `E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 32 — 运行全量静态、格式、单元和前端构建
Delegation: SAFE_TO_DELEGATE

#### Purpose
在端到端测试前确认全部模块没有静态或单元回归。

#### Input
完成后的源码、migrations、lockfiles、tests。

#### Action
1. 项目根运行 `pnpm check`。
2. 项目根运行 `pnpm build`。
3. `src-tauri` 运行 `cargo fmt --check`；如失败，运行 `cargo fmt` 后重跑并记录格式化文件。
4. `src-tauri` 运行无过滤 `cargo test`。
5. 统计 Rust tests 总数/通过/失败/ignored，前端 errors/warnings。
6. 扫描日志中的 synthetic key、Authorization、x-api-key、fixture 正文和正式路径；synthetic secret 也不得出现。
7. 运行 execution pack validator，确认本目录结构仍 VALID。

#### Expected Result
四项工程检查全部退出 0，所有 tests 通过，日志无 secret/正文，执行包有效。

#### Evidence
记录命令、工作目录、退出码、计数、敏感扫描与 validator 输出。

#### Verdict
- PASS：全部通过，进入 STEP 33。
- STOP：任一检查失败、日志泄露或 validator 非 VALID。

#### Exception Handling
- 编译/测试失败：`E09-VALIDATION-FAILED`。
- secret/正文泄露：`E19-SECURITY-BOUNDARY`。
- validator 失败：`E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 33 — 扩展隔离 UI CDP、mock AI 与持久化验收
Delegation: SAFE_TO_DELEGATE

#### Purpose
用真实 UI 操作证明 Provider、正文、AI 建议、调度、Brief、翻译、Dashboard 和缓存清理闭环。

#### Input
隔离四环境变量、debug exe、mock server、UI CDP、synthetic workspace fixtures。

#### Action
1. 在 `.test-runtime\luna-ai-secretary\workspace` 创建 synthetic DOCX、文字 PDF、无文字 PDF、UTF-8/UTF-16/GBK 文本、unsupported binary；不得使用真实工作文件。
2. 启动 mock server，提供 `/models`、`/chat/completions`、`/responses`、`/messages` 和可切换失败响应；使用 synthetic key。
3. 构建并启动 debug exe，确认四环境变量指向隔离根。
4. UI flow 覆盖：创建两个模板与 custom；刷新模型；三协议 test；六任务 routes；绑定 workspace；文档状态；点击 work draft；编辑/确认 Work；确认 Task/Waiting/Calendar/Inbox/Resume；拒绝一项。
5. 覆盖 manual analysis、pending count、Review Center 重启保持、同建议 dedupe、用户编辑不被下一次分析覆盖。
6. 通过测试专用 command 或注入 clock 触发 interval/daily due；不得等待真实时间；验证 coalesce 和 Brief draft/keep。
7. 覆盖翻译两方向与两风格，断言数据库无 translation history。
8. 在 1024×640 与 1440×900、中文与英文断言 Dashboard 无滚动、所有模块 bounding rect 可见，并截图。
9. 制造超过 TTL/容量的隔离 cache；执行 Smart preview/cleanup，断言 source fixture、正式实体、kept brief、confirmed/pending proposal 和 synthetic credential 都保留。
10. 退出并重启两次，确认 schedule/routes/document summary/proposals/kept brief 持久化，已删 cache 自动变 pending 并可重建。
11. 捕获 window.error、unhandledrejection、console error、Tauri error；预期校验错误外任何错误使 smoke fail。
12. 停止 debug 与 mock PID；确认 PID 不存在；正式 DB 元数据与 STEP 01 相同。

#### Expected Result
所有新增功能通过真实 DOM、mock Provider、重启、清理和安全边界验证，正式数据不变。

#### Evidence
记录每个 flow PASS/FAIL、mock request protocol/path、隔离表计数、cache bytes、DOM metrics、截图、PID 和正式 DB 对比。

#### Verdict
- PASS：全部 UI flow 通过且无意外 runtime error，进入 STEP 34。
- STOP：任一闭环失败、正式数据变化、源 fixture 被清理、secret/正文进入日志或残留测试进程。

#### Exception Handling
- CDP 连接失败：`E05-TOOL-UNAVAILABLE`。
- UI flow 失败：`E09-VALIDATION-FAILED`。
- 安全边界变化：`E19-SECURITY-BOUNDARY`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 34 — 构建 release、生成最终截图并更新文档
Delegation: SAFE_TO_DELEGATE

#### Purpose
形成可交付构建、复核材料和与实际一致的说明。

#### Input
通过 STEP 33 的源码、README、architecture、smoke checklist、截图脚本。

#### Action
1. 运行 `pnpm tauri build`，记录退出码。
2. 验证 release exe 与 NSIS installer 存在，记录 path、size、LastWriteTimeUtc、SHA-256。
3. 用 release exe、隔离四环境变量和 mock server 运行精简 smoke：启动、Dashboard、pending proposal、translation、storage usage、退出。
4. 生成最终截图：中文/英文 Dashboard 1024×640 与 1440×900；Provider 模板与任务路由；Workspace 文档状态；AI Review Center；Brief 全文/来源；Storage preview。
5. 更新 README：Provider 模板、模型路由、支持文件、AI 建议确认、调度、Brief、翻译、缓存治理、限制与测试。
6. 更新 `docs/architecture.md`：v3–v6 schema、adapter/router、document pipeline、analysis/proposal/scheduler/storage data flow。
7. 更新 `docs/smoke-checklist.md`，加入本轮全部关键 flow 与正式数据保护。
8. 新建 `docs/ai-secretary-iteration-report.md`，只记录实际实现、测试、限制和截图，不伪造上一阶段历史。
9. 重新运行 `pnpm check` 与 `cargo test`，确保文档/截图调整没有带来代码回归。

#### Expected Result
release 可运行、证据完整、文档与实现一致、最终工程检查通过。

#### Evidence
记录构建输出、产物 hash、release smoke、截图绝对路径、文档路径和最终测试计数。

#### Verdict
- PASS：release、smoke、截图、文档和最终 tests 全部通过，进入 STEP 35。
- STOP：构建/smoke/测试失败、截图缺失或文档与实现矛盾。

#### Exception Handling
- Release exe 被隔离进程锁定：只停止已验证属于当前隔离启动的 PID 后重试一次；仍失败 -> `E09-VALIDATION-FAILED`。
- 代码签名或发布要求：范围外，不阻塞本地 installer；若环境强制要求 -> `E12-SCOPE-CHANGE`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 35 — 在正式数据库副本验证 v2→v6，不启动正式应用
Delegation: CONDITIONAL

#### Purpose
证明现有真实数据形状可以无损升级，同时保持正式数据库和凭据完全不变。

#### Input
正式 DB 只读文件、副本隔离目录、release exe、migration tests。

#### Action
1. Gate：确认没有 MSL Desktop 进程，正式 DB/WAL/SHM 元数据与 STEP 01 完全一致；否则 STOP。
2. 把正式 DB/WAL/SHM 复制到 `.test-runtime\luna-ai-secretary\artifacts\production-db-copy\MSLDesktop`；不得移动或删除原文件。
3. 设置副本专用 APPDATA、LOCALAPPDATA、TEMP、TMP，启动 release `--background` 完成 migration 后按已验证 PID 正常停止。
4. 对副本运行 `PRAGMA integrity_check`、migration version、schema_migrations、旧表计数、新表计数、foreign_key_check。
5. 旧 Provider 必须保留 credential_ref，legacy model 迁入 provider_models；不得读取 Keyring。
6. 旧 daily_briefs 必须 retention_state=kept；原 works/tasks/waiting/calendar/inbox/activity 行数不得减少。
7. 再次记录正式 DB/WAL/SHM 元数据，必须与 Gate 相同。
8. 执行报告明确写明只迁移副本，未启动正式 APPDATA。

#### Expected Result
真实 v2 副本无损升级到 v6、完整性通过、正式原文件不变。

#### Evidence
记录 Gate、副本路径、integrity/foreign key、version、前后表计数、legacy Provider/Brief 断言和原文件元数据。

#### Verdict
- PASS：副本无损升级且原文件不变，进入 STEP 36。
- STOP：进程存在、元数据变化、副本完整性失败、原表行数减少或 Key 需要读取。

#### Exception Handling
- 应用仍运行：`E21-USER-AUTHORIZATION-REQUIRED`。
- 复制权限不足：`E04-PERMISSION-DENIED`。
- migration 数据损失/完整性失败：`E10-ACCEPTANCE-FAILED`。
- 正式数据变化：`E19-SECURITY-BOUNDARY`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

### STEP 36 — 汇总机械验收并进入最终视觉门
Delegation: HIGH_MODEL_REQUIRED

#### Purpose
用累计证据逐条计算完成度，并把主观 Dashboard/秘书体验交给用户或更高能力模型审阅。

#### Input
`ACCEPTANCE.md`、全部步骤证据、release 产物、最终截图、执行包 validator。

#### Action
1. 按 `ACCEPTANCE.md` 顺序逐项标记 PASS/FAIL，禁止省略。
2. 完成 `EXECUTION_REPORT.md` 的步骤、证据、修改文件、命令、偏差、停止代码、交付物和未解决问题。
3. 重新运行 execution pack validator；必须输出 VALID。
4. 所有机械项 PASS 时状态改为 `PARTIALLY_COMPLETED`；任一机械项失败则 FAILED/BLOCKED 并使用对应 stop code。
5. 向用户提供 Dashboard 四张核心截图以及 Provider、Review Center、Workspace documents、Storage preview 的截图链接。
6. 请求用户审阅：首页是否真正无需滚动且信息一眼可见、Brief 是否最突出、翻译是否自然融入、AI 确认流程是否像秘书、设置是否不过度重型。
7. 只有用户或更高能力模型明确通过视觉/产品门，才把状态改为 `COMPLETED`。

#### Expected Result
机械证据完整且可复核，Luna 不自行越过主观视觉与产品体验门。

#### Evidence
最终 Acceptance 对照、执行报告、validator、截图路径、产物 hash 和审阅结论。

#### Verdict
- PASS：机械验收和视觉门均明确通过，状态 `COMPLETED`。
- STOP：机械全通过但等待审阅时状态 `PARTIALLY_COMPLETED`；机械失败按相应 stop code。

#### Exception Handling
- 等待视觉审阅：`E21-USER-AUTHORIZATION-REQUIRED`，这是预期门，不等同工程失败。
- Acceptance 机械项失败：`E10-ACCEPTANCE-FAILED`。
- Validator 失败：`E09-VALIDATION-FAILED`。
- 任何未覆盖状态：`E07-UNEXPECTED-STATE`。

## 标准停止载荷

任何 STOP 都必须在回复和 `EXECUTION_REPORT.md` 中写：Code、Step、Expected、Observed、Evidence、Why execution cannot continue、Last safe completed step、No further action taken。停止后不得执行后续步骤。
