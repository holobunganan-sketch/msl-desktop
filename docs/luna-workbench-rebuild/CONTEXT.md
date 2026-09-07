# Context

## Known Facts

- 当前项目不是 Git worktree；`.project-cognition/START_HERE.md` 已建立项目结构索引。
- `package.json` 使用 Svelte 5、SvelteKit 2、Tauri 2、TypeScript 5.6、Vite 6；包管理器为 pnpm。
- `pnpm check` 在审计时为 0 errors / 0 warnings。
- `cargo test` 在审计时为 24 passed / 0 failed。
- 以上测试只证明静态类型和 repository/watcher 单元逻辑，不证明真实按钮、反馈和跨页面刷新。
- 隔离数据库中的真实页面点击可以创建 Work、Task、Waiting、Calendar、Inbox；因此当前主要问题不是所有 command 都缺失，而是交互反馈、关联、刷新和信息管线不闭环。
- 正式数据库审计时完整性为 `ok`，但 Work、Task、Waiting、Calendar、Activity 均为 0；Inbox 为 1；Daily Brief 为 3。
- 三份 Brief 的 `source_snapshot_hash` 相同，内容均表示“无记录”。
- 已绑定目录在绑定时已有文件，但绑定后没有新修改；当前 watcher 不做首次基线或离线差异扫描，所以 Activity 为 0 是当前设计的直接结果。
- `src-tauri/src/ai/brief.rs` 的 snapshot 不包含 Inbox；任务只选 `due_at <= day_end` 的未完成项；最近文件变化先对全部活动限量再过滤 `file.*`。
- `src/lib/components/WorksView.svelte`、`PlanView.svelte`、`WaitingView.svelte` 在必填标题为空时直接返回，没有错误反馈。
- `src/lib/components/QuickCapture.svelte` 不等待写入成功就清空输入，错误只进入 console。
- Plan、Waiting、Calendar 和 Inbox 转换界面将 `workId` 固定为 `null`，导致数据无法进入 Work 聚合上下文。
- 页面文案硬编码且中英文混用；`src/app.html` 仍为 `lang="en"` 和模板标题。
- 当前样式由各组件重复定义；没有全局 reset、design tokens 或统一按钮/卡片/Modal/Toast。
- `src-tauri/src/ai/provider.rs` 使用 `provider-{整数 id}` 作为凭据用户名；不同数据库可出现 ID 碰撞。
- `scripts/smoke-cdp.py` 在当前 WebView2 上会因 Origin 安全要求得到 WebSocket 403；连接需要使用客户端的 `suppress_origin=True` 或等价安全做法。

## Environment

- Platform: Windows，PowerShell。
- Working location: `C:\Myfolder\MSL cowork\msl-desktop`。
- Source root: `src/`；Rust root: `src-tauri/`。
- Existing commands:
  - `pnpm check`
  - `pnpm build`
  - `pnpm tauri build`
  - 在 `src-tauri` 中运行 `cargo test`
- Existing executable: `src-tauri\target\release\msl-desktop.exe`，但执行前必须重新构建并使用隔离 `APPDATA`。
- Permissions: 可读写工作区；不得假定有管理员权限；不得请求真实凭据。
- Network: 实现与核心验证不应依赖网络；如果新增 Rust crate 或 pnpm package 必须下载且失败，按停止条件处理。

## Assumptions

- `pnpm`、Node、Rust MSVC toolchain 仍可用 -> 通过 STEP 01 的版本与命令检查验证；失败 -> `E05-TOOL-UNAVAILABLE`。
- 当前源码与本执行包生成时一致 -> 通过 cognition prepare、文件存在性和基线检查验证；结构重大变化 -> `E02-ENVIRONMENT-MISMATCH`。
- 用户希望保留已有数据 -> 所有 schema 变化使用追加 migration；发现 migration 不能无损完成 -> `E13-DESTRUCTIVE-ACTION`。
- 默认不读取文件正文符合当前产品边界 -> 如果实现要求正文解析才能继续 -> `E12-SCOPE-CHANGE`。
- Dashboard 可用 CSS 和现有 Svelte 能力完成 -> 如果必须引入大型 UI 框架才能继续 -> `E16-DECISION-REQUIRED`。
- AI 测试可以使用本地 deterministic fallback 和 mock HTTP，不需要真实 Key -> 如果测试被真实凭据阻塞 -> `E14-CREDENTIALS-REQUIRED`。

## Boundaries

- 可以修改 `msl-desktop` 内的源码、migration、测试、脚本和文档。
- 不得修改 `C:\Myfolder\MSL cowork` 下其他项目内容；`.project-cognition` 只由其脚本刷新，不手工编辑 generated/cache。
- 不得修改正式数据库或工作目录来制造验收数据。
- 不得改动用户真实 Provider Key；凭据 migration 只在应用运行时按兼容规则引用，不读取到日志。
- 不得移除托盘、单实例、窗口销毁后 Core 常驻、通知和开机启动能力。
- 不得降低现有数据库外键、WAL、busy timeout 或 migration 事务保障。

## Dependencies

- 前端共享状态使用 Svelte 自带 `svelte/store`，无需新包。
- i18n 使用项目内 TypeScript 字典，必须同时提供 `zh-CN` 与 `en-US`。
- Provider `credential_ref` 如需生成 UUID，优先使用 Rust `uuid` crate 的 v4 功能；只有该依赖安装失败时才停止，不得改用可预测的整数 ID。
- UI 自动化继续复用 Python `websocket-client`；连接时禁止 Origin header。
- 隔离测试目录统一位于项目的 `.test-runtime\luna-workbench`；创建前验证绝对路径位于项目根内，清理时同样验证。

## Current State Snapshot

- 主前端入口：`src/routes/+page.svelte`。
- 现有页面：`TodayView.svelte`、`WorksView.svelte`、`PlanView.svelte`、`WaitingView.svelte`、`CalendarView.svelte`、`InboxView.svelte`、`SettingsView.svelte`。
- Quick Capture：`src/lib/components/QuickCapture.svelte`。
- 后端 command：`src-tauri/src/commands/mod.rs`。
- Brief：`src-tauri/src/ai/brief.rs`、`src-tauri/src/db/brief.rs`。
- 文件监听：`src-tauri/src/workspace/watcher.rs`。
- migration runner：`src-tauri/src/db/migrations.rs`，当前只登记 version 1。
- Provider：`src-tauri/src/ai/provider.rs`、`src-tauri/src/db/provider.rs`。
- 当前截图：`output/playwright/audit-today.png`、`output/playwright/audit-works.png`。
- 正式数据库路径仅作为禁止写入边界：`%APPDATA%\MSLDesktop\msl-desktop.db`。

