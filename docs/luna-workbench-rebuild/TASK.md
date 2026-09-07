# Task

## Goal

把当前 MSL Desktop 从功能原型改造成可日常使用的本地 MSL 工作台：中文为默认语言、英文可切换；首页是信息密度合理的 Dashboard；Work、任务、Waiting、Calendar、Inbox 与文件活动能够可靠创建、互相关联、即时刷新；每日 Brief 能基于可追溯的工作台事实与工作目录变化总结昨天或指定时期，并提出今天应推进的事项。

## Source Request

用户确认当前程序基本无法正常使用，具体指出界面不像 Dashboard、多个页面的新增按钮像无用按钮、每日 Brief 没有整理出有效内容、希望 Brief 综合工作目录变化和工作台全部信息、希望全中文并可切换英文。用户要求形成适合 GPT-5.6 Luna 执行的、结合当前项目现状的手把手修改指令。

## Deliverables

- 可靠的新增、编辑、删除、完成、归档与转换交互，具有必填校验、提交状态、成功反馈、错误反馈和跨页面刷新。
- Work 关联能力：任务、Waiting、Calendar、Inbox 转换和文件引用均可选择或保留 `work_id`。
- 中文默认、英文可切换的轻量 i18n 系统，覆盖全部用户可见文本、状态、日期和空状态。
- 统一的 Dashboard 视觉系统和可复用 UI 组件，不再由各页面重复定义零散样式。
- 真正的 Dashboard 首页：关键指标、继续推进、今日任务与日程、需跟进 Waiting、Inbox、最近文件变化、Brief 和来源覆盖情况。
- 文件事实管线：首次绑定建立元数据基线；启动或恢复时对比离线变化；运行时 watcher 继续增量记录；不读取文件正文。
- Brief 管线：支持昨天、最近 7 天和自定义范围；包含 Work、Resume Point、任务、完成事项、Waiting、Calendar、Inbox 和文件变化；支持无 AI 的本地结构化摘要；AI 可用时生成有来源约束的叙述。
- Provider 凭据引用隔离修复，避免不同数据库中相同整数 ID 误用同一 Windows Credential Manager 凭据。
- 后端单元测试、前端静态检查、隔离 UI 冒烟、release 构建和人工验收证据。
- 更新 `README.md`、`docs/architecture.md`、`docs/smoke-checklist.md`，并完成 `docs/luna-workbench-rebuild/EXECUTION_REPORT.md`。

## Scope

### In scope

- `src/` 下 Svelte 5 前端、状态刷新、i18n、Dashboard、表单与页面组件。
- `src-tauri/` 下 Tauri command、SQLite migration/repository、文件元数据基线与差异检测、Brief snapshot、Provider 凭据引用。
- `scripts/` 下现有 CDP 冒烟修复与新的 UI 级冒烟覆盖。
- 与上述实现直接相关的文档和测试。

### Out of scope

- 云同步、多人协作、远程数据库、账号体系、CRM 集成。
- Word/PDF/PPT/邮件正文解析、向量数据库、全文索引。
- 医学结论生成、医学内容审核、外部消息发送。
- 替换 Tauri、Svelte、SQLite 或现有本地优先架构。
- 自动发布、代码签名、GitHub、PR、Tag、Release 流程。
- 对正式用户数据库进行清空、重建或测试写入。

## Inputs

- 项目根：`C:\Myfolder\MSL cowork\msl-desktop`。
- 产品约束：`C:\Myfolder\MSL cowork\DEEPSEEK_V4_FLASH_MSL_DESKTOP_DEVELOPMENT_GUIDE.md`。
- 当前架构与阶段文档：`docs/architecture.md`、`docs/stage-*.md`。
- 当前实现：`src/`、`src-tauri/src/`、`src-tauri/migrations/`、`scripts/`。
- 审计截图：`output/playwright/audit-today.png`、`output/playwright/audit-works.png`。

## Constraints

- Windows 11 第一目标，PowerShell 环境。
- 保持 Tauri 2、Svelte 5、TypeScript、SQLite、Rust、pnpm。
- 保持 local-first、托盘常驻、单实例、关闭窗口后 Core 常驻的生命周期。
- 默认中文 `zh-CN`，可切换 `en-US`；语言选择必须持久化。
- 文件管线只读取路径、相对路径、大小、修改时间和事件类型；禁止读取或发送正文。
- 所有 AI 输出必须可回溯到 snapshot；没有事实时不得编造。
- UI 必须在最小窗口 1024×640 下可用，不产生页面级双重滚动条。
- 新增依赖必须轻量且确有必要；不得引入 Tailwind、Bootstrap、Material UI 或大型状态管理框架。
- 现有用户数据必须通过追加 migration 保留，禁止修改 `0001_init.sql` 来假装升级。
- 所有测试应用启动必须使用隔离 `APPDATA` 和隔离工作目录。

## Prohibited Actions

- 不得初始化 Git 或执行破坏性 Git 命令。
- 不得删除 `%APPDATA%\MSLDesktop`、用户工作目录或用户凭据。
- 不得打印 API Key、Authorization header、Credential Manager 内容。
- 不得把真实用户工作路径或 Brief 正文写入测试日志和交付文档。
- 不得用真实 Provider 做连接测试或 Brief 测试。
- 不得以“视觉优化”为理由改变业务数据含义或绕过后端校验。
- 不得把失败吞掉后继续报告成功。
- 不得只修改 CSS 就宣布完成。

## Compiler Decisions

- 先修可靠性和数据关联，再改 Brief，最后完成 Dashboard 与双语；视觉层不得先于数据闭环。
- 不引入第三方 i18n 包；使用项目内类型化字典和 Svelte store，降低依赖与运行成本。
- 不引入大型 UI 库；建立 CSS design tokens 和少量共享组件。
- 使用一个全局 `dataRevision`/domain invalidation store 触发当前页面和 Dashboard 重新加载，解决 Quick Capture 与跨页面数据不刷新。
- 所有新增/编辑操作使用一致的 Modal 或清晰的内联表单；必填失败必须显示中文/英文错误，不允许静默 `return`。
- 文件目录首次绑定只建立基线并记录一条汇总活动，避免把既有文件误报为用户刚创建；后续启动比较基线，记录离线新增、修改、删除。
- Brief 默认只发送结构化管理事实和文件元数据变化，不发送文件正文。
- Brief 在无 Provider、无 Key或 AI 调用失败时仍生成本地结构化摘要，并显示非阻塞警告。
- Provider 新增稳定 `credential_ref`；现有记录兼容旧 `provider-{id}`，新记录不得再仅用整数 ID 作为凭据名称。
- Calendar 必须呈现可识别的周视图/日程时间轴，而不是只显示一列记录；无需实现复杂拖拽。
- 不追求像素级复刻第三方产品；按本任务给出的 design tokens 和布局验收。

## Delegation Summary

- SAFE_TO_DELEGATE：STEP 01–06 的基线、共享基础设施、i18n 与视觉骨架；STEP 08–19 的表单、关联、文件事实、Brief、Dashboard、测试与文档实现；STEP 21–23 的机械验证与报告。
- CONDITIONAL：STEP 07 数据迁移、STEP 20 隔离 UI 冒烟、STEP 24 正式数据库首次迁移准备；必须先满足各自机械门槛。
- HIGH_MODEL_REQUIRED：STEP 25 最终视觉与产品一致性审阅。Luna只负责生成证据，不得自行宣布主观视觉目标完全达标；该门由用户或更高能力模型审阅。

