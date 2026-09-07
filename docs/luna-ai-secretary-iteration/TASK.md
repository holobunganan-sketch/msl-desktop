# Task

## Goal

在保留上一阶段已经完成的 Dashboard、CRUD、双语、工作目录元数据、Brief、本地 fallback 和隔离测试能力的基础上，把 MSL Desktop 增量升级为一个可以长期使用的 AI 工作秘书：固定提供 DeepSeek 与 OpenCode Go 接入模板，允许自定义其他模型接口；不同 AI 任务可以选择不同模型；绑定工作目录后可以安全读取 Word、PDF 和文本类文件正文；按默认每 3 小时与每天本地时间 06:00 综合分析工作目录和整个工作台；所有拟议变更先进入可编辑确认队列，用户确认后才分配到 Work、Task、Waiting、Calendar、Inbox 或 Resume Point；首页无页面级滚动并把每日 Brief 放在顶部，同时提供中英双向 AI 翻译；系统具有不会误删正式工作的精细缓存治理。

## Source Request

用户已经明确作出以下决定：

- 首页是每次打开首先看到的页面，在最小桌面窗口中尽量一眼看到全部摘要，不需要页面滚动。
- 固定 Provider 模板只有 DeepSeek 和 OpenCode Go；其他接口允许接入，但不提供固定模板。
- 不同任务可以绑定不同 Provider 模型。
- 绑定工作目录后，受支持的 Word、PDF、文本等文件正文对 AI 完全开放，不再询问隐私授权。
- AI 应综合管理工作、工作目录、任务计划、等待事项、日历和收件箱，但不能未经确认直接改正式数据。
- 绑定或导入工作后，AI 生成结构化工作模板，用户可直接修改并确认，确认后系统自动保存和分配。
- 常规分析默认每 3 小时一次，时间可设置；每天本地时间 06:00 进行一次日分析并生成首页顶部 Brief。
- 每次分析后出现逐项确认目录；项目可编辑、确认或拒绝；确认后写入对应模块。
- 首页提供 AI 翻译，自动识别中文或英文，并支持书面、口语两种风格。
- 系统必须提供精细缓存清理，长期使用不能无限增长，且清理不能影响正式工作。

## Deliverables

- Provider 连接、模型目录、任务路由三层架构，以及 DeepSeek、OpenCode Go 两个固定模板和一个无预设的自定义入口。
- OpenAI Chat Completions、OpenAI Responses、Anthropic Messages 三种文本协议适配器及本地 mock 覆盖。
- DeepSeek 当前 V4 Pro/Flash 预置；OpenCode Go 当前模型与协议映射预置，并通过 `/models` 刷新可用性。
- `workspace_analysis`、`work_draft`、`global_analysis`、`daily_brief`、`translation`、`general` 六类任务路由。
- `.docx`、`.pdf` 和明确文本类文件的增量提取、内容指纹、分段、状态、跳过原因和扫描版 PDF 识别。
- 工作目录与 Work 的关联；绑定新目录后可生成可编辑的 Work 草稿与后续事项。
- 分析运行、AI 建议、确认/拒绝/编辑、去重、来源引用和原子应用机制。
- 默认 180 分钟周期分析、每天本地时间 06:00 日分析、手动分析、持久化防重复和 Resident Core 调度。
- 每日 Brief 综合工作台事实、文件变化、正文派生摘要和待确认建议；首页顶部显示最新 Brief。
- 首页 AI 翻译模块，自动中英方向、书面/口语风格、复制结果和明确错误。
- 1024×640 与 1440×900 下无 Dashboard 页面级滚动的紧凑布局。
- 正式数据、运行历史、可重建缓存和源文件四层存储治理；自动清理、智能清理、高级清理、容量上限、活动压缩与清理审计。
- 新 migration、repository、commands、Rust tests、前端类型/i18n、UI CDP 冒烟、持久化与 release 构建。
- 更新 README、architecture、smoke checklist，并完成本执行包的执行报告。

## Scope

### In scope

- `src/` 下的 Svelte 5 Dashboard、Provider 设置、模型路由、AI 建议中心、翻译、存储设置和双语文案。
- `src-tauri/` 下的增量 migration、Provider 协议适配、文档提取、分析引擎、调度器、建议应用器、存储治理与 commands。
- `scripts/` 下隔离环境、mock Provider、UI CDP 和截图脚本。
- 受支持正文解析、内容派生摘要和发送给用户选择的 Provider。
- 与新增能力直接相关的测试、文档与视觉证据。

### Out of scope

- 云同步、多人协作、账号体系、远程数据库、CRM 或邮件服务集成。
- OCR、手写识别、音视频转写、老式 `.doc` 解析、宏文档执行。
- 自动修改或写回源工作文件。
- 未经用户确认自动写入正式工作台实体。
- 向外部联系人自动发送邮件、消息、会议邀请或文件。
- 向量数据库、远程检索服务或大型本地模型运行时。
- 为 DeepSeek 与 OpenCode Go 之外的供应商制作品牌化固定模板。
- 替换 Tauri、Svelte、SQLite、Windows Credential Manager 或 local-first 架构。
- 发布、代码签名、GitHub PR、提交、Tag 或 Release 操作。

## Inputs

- 项目根：`C:\Myfolder\MSL cowork\msl-desktop`。
- 上一阶段完成证据：`docs/luna-workbench-rebuild/EXECUTION_REPORT.md`。
- 当前架构：`docs/architecture.md`、`docs/workbench-rebuild-report.md`。
- 当前 Provider：`src-tauri/src/ai/provider.rs`、`src-tauri/src/db/provider.rs`、`src/lib/components/SettingsView.svelte`。
- 当前 Brief：`src-tauri/src/ai/brief.rs`、`src-tauri/src/db/brief.rs`、`src/lib/components/TodayView.svelte`。
- 当前目录事实：`src-tauri/src/workspace/inventory.rs`、`watcher.rs`。
- 当前调度：`src-tauri/src/notifications/mod.rs`、`app_state.rs`、`lib.rs`。
- 当前隔离测试：`scripts/run-isolated-audit.ps1`、`scripts/stop-isolated-audit.ps1`、`scripts/ui-smoke-cdp.py`。

## Constraints

- Windows 11、PowerShell、Tauri 2、Svelte 5、TypeScript、Rust、SQLite、pnpm 保持不变。
- migration 必须从现有 version 2 追加，禁止修改 `0001_init.sql` 和 `0002_workbench_reliability.sql`。
- 现有 `provider_settings.model` 字段保留兼容；新任务路由以 `provider_models` 为准。
- API Key 继续只存 Windows Credential Manager，SQLite 只保存 `credential_ref`。
- 所有模型协议都必须通过统一路由器调用；业务代码不得自行拼接 Provider URL。
- OpenCode Go 模型协议按模型记录，禁止把全部模型强制当作 `/chat/completions`。
- 自定义 Provider 不显示品牌预置值；用户必须显式填写名称、Base URL、鉴权方式、模型和协议。
- 文件变化只标记 dirty，不立即触发 AI；仅手动、周期或每日调度触发分析。
- 源文件正文允许发送给选定 Provider，但不得把完整正文写进日志、执行报告或长期数据库字段。
- 正文缓存放在应用专用 cache root，数据库只保留索引、摘要、来源和相对缓存路径。
- 单文件默认上限 50 MiB；超过则状态为 `too_large`，不得静默截断为成功。
- `.docx` 只读取 OpenXML 文本；`.pdf` 只读取文字层；无文字层时状态为 `needs_ocr`。
- AI 建议必须经过 schema 校验、来源校验和用户确认才能写正式数据。
- 每次分析最多只有一个运行实例；每日 06:00 与周期任务同时到期时合并为 daily run。
- 首页在 1024×640 和 1440×900 不产生页面级水平或垂直滚动；详情使用 Modal/Drawer。
- 清理器只能处理应用登记的缓存、过期运行历史和明确的压缩对象；不得遍历删除工作目录。
- 用户确认或主动保留的 Brief 永不自动删除；被替代的 Brief 草稿才可按策略清理。
- 新增依赖限于实现本任务所需的轻量 Rust crate；禁止大型前端 UI、状态或编辑器框架。

## Prohibited Actions

- 不得破坏、丢弃、覆盖、提交或清理当前 Git 工作树中的已有改动。
- 不得修改上一阶段执行记录来伪造本轮完成。
- 不得删除、重建或运行测试写入正式 `%APPDATA%\MSLDesktop`。
- 不得把真实工作目录作为测试目录。
- 不得读取或输出真实 Key、Authorization、x-api-key 或 Provider 响应头。
- 不得在测试中调用真实 Provider 或互联网模型。
- 不得把整个 Provider 错误响应或文件正文直接显示在 Toast/日志中。
- 不得自动接受 AI 建议，不得提供绕过确认队列的后台写入路径。
- 不得让缓存清理删除正式数据库、Keyring 凭据、用户配置、确认建议、保留 Brief 或源文件。
- 不得在分析或正文提取正在使用某缓存时删除它。
- 不得因为某模型失败就静默切换到另一个未配置模型。
- 不得用 CSS 隐藏溢出而让关键首页内容不可达。

## Compiler Decisions

- 使用四个追加 migration：0003 Provider Catalog、0004 Document Intelligence、0005 AI Secretary、0006 Storage Governance；每个 migration 独立测试且按顺序登记。
- Provider 连接、模型目录与任务路由分离；模型 ID 只在所属 Provider 内唯一。
- 协议值固定为 `chat_completions`、`responses`、`anthropic_messages`；自定义 Provider 必须显式选择。
- 固定模板值与模型映射以 `CONTEXT.md` 的 2026-08-15 官方快照为准；刷新 `/models` 只改变 available 状态，未知模型默认禁用并要求用户选择协议。
- 任务路由先查精确 task kind，再查用户显式配置的 `general`；两者都没有则返回配置错误，不做隐式回退。Daily Brief 仍可使用现有本地 renderer。
- 新增文档模块使用 `zip` + `quick-xml` 读取 DOCX，`pdf-extract` 读取 PDF 文字层，`encoding_rs` 解码文本，`sha2` 计算内容指纹，`chrono` 处理本地 06:00 调度。
- 工作目录正文缓存只存 `%LOCALAPPDATA%\MSLDesktop\cache`；正式 SQLite 继续留在 `%APPDATA%\MSLDesktop`。
- 工作目录与 Work 使用多对多 `work_workspace_links`，允许一个 Work 引用多个目录。
- AI 输出采用严格 JSON envelope；解析失败只产生失败运行，不直接写业务表。
- 重复分析通过 proposal dedupe key 合并；用户编辑过的 pending proposal 不得被后续自动分析覆盖。
- Proposal 确认使用事务调用既有 repository，并记录 `ai.proposal.confirmed` Activity；拒绝不写业务表。
- 新目录导入先建立索引和摘要，再生成 Work/Task/Waiting/Calendar/Inbox/Resume Point 草稿；确认 Work 后，其他建议才能绑定该 Work。
- Dashboard 使用固定高度网格：顶部 Brief、指标条、三列主体；默认视图每个列表最多三项，无页面或卡片滚动条，详情进入 Modal/Drawer。
- 翻译不保存长期历史；源文和结果只保留在当前 UI 会话，避免形成无界数据。
- 缓存治理使用 TTL、总量上限和 LRU 的混合策略；默认 2 GiB，上限 80% 触发，清理到 60%。
- 原始 Activity 保留 90 天后先汇总到 daily rollup 再删除；确认记录和保留 Brief 不参与自动清理。
- WebView 缓存通过 Tauri `clear_all_browsing_data` 清理，不允许手工删除 WebView 数据目录。

## Delegation Summary

- SAFE_TO_DELEGATE：基线记录、隔离环境、追加 schema、repository、协议适配、文档解析、任务路由、AI schema、建议应用器、翻译、Dashboard、缓存治理、测试、文档和构建。
- CONDITIONAL：STEP 01 进程/环境 gate、各 migration gate、Rust 依赖获取、WebView 清理 API、正式数据库副本迁移；每个条件都有明确 PASS/STOP 分支。
- HIGH_MODEL_REQUIRED：最终视觉成熟度审阅，以及任何与本执行包官方协议快照发生实质冲突的外部变化。Luna 不得自行重新设计。
