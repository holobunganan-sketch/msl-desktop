# GPT-5.6 Luna 执行入口：AI 秘书与存储治理迭代

## 目标执行器

GPT-5.6 Luna，在 Codex Desktop 的当前工作区中直接修改 MSL Desktop。

这是一项实施任务，不是重新分析、重新设计或再次委派任务。产品范围、数据边界、架构选择和验收方式已经写入本执行包。Luna 只允许在不改变范围和语义的前提下做局部语法选择。

## 项目目录

`C:\Myfolder\MSL cowork\msl-desktop`

## 开始前必须完整阅读

按顺序读取，任何文件都不得只读摘要：

1. `docs/luna-ai-secretary-iteration/START_HERE.md`
2. `docs/luna-ai-secretary-iteration/TASK.md`
3. `docs/luna-ai-secretary-iteration/CONTEXT.md`
4. `docs/luna-ai-secretary-iteration/STEPS.md`
5. `docs/luna-ai-secretary-iteration/ACCEPTANCE.md`
6. `docs/luna-ai-secretary-iteration/EXECUTION_REPORT.md`
7. `docs/luna-ai-secretary-iteration/HANDOFF.md`

然后读取上一阶段现状，不得重新执行上一阶段步骤：

1. `docs/luna-workbench-rebuild/EXECUTION_REPORT.md`
2. `docs/workbench-rebuild-report.md`
3. `README.md`
4. `docs/architecture.md`
5. `package.json`
6. `src-tauri/Cargo.toml`
7. `src-tauri/src/db/migrations.rs`
8. `src-tauri/src/ai/provider.rs`
9. `src-tauri/src/ai/brief.rs`
10. `src-tauri/src/notifications/mod.rs`
11. `src-tauri/src/app_state.rs`
12. `src-tauri/src/commands/mod.rs`
13. `src-tauri/src/workspace/inventory.rs`
14. `src/lib/components/TodayView.svelte`
15. `src/lib/components/SettingsView.svelte`
16. `src/lib/components/WorkspaceView.svelte`
17. `src/lib/services/api.ts`
18. `src/lib/types/domain.ts`
19. `scripts/run-isolated-audit.ps1`
20. `scripts/ui-smoke-cdp.py`

## 绝对执行规则

- 不得创建子代理，不得再次委派，不得自行重新定义产品。
- 从 `STEPS.md` 的 STEP 01 开始，严格按编号顺序执行。
- 不得跳步，不得把多个尚未验证的阶段合并为一次大改。
- 每一步结束后，立即把 Action、Expected、Observed、Evidence、Verdict 写入本目录的 `EXECUTION_REPORT.md`，然后才能继续。
- PASS 后自动继续；STOP 后立即停止，不得靠猜测绕过。
- 使用 `rg` 搜索，使用 `apply_patch` 修改文件。
- 当前项目是存在大量未提交成果的 Git 工作树。不得 reset、checkout、clean、stash、rebase、commit、初始化新仓库或覆盖用户改动。
- 不得修改上一阶段执行包中的历史 Observed、Evidence 和 Verdict。
- 不得删除、移动、重建或批量修改正式 `%APPDATA%\MSLDesktop`。
- 所有应用启动、目录解析、模型调用、调度、持久化、缓存清理和 UI 测试必须使用隔离的 `APPDATA`、`LOCALAPPDATA`、`TEMP`、`TMP`。
- 不得读取、打印、复制或测试真实 API Key。
- 不得调用真实 DeepSeek、OpenCode Go 或自定义 AI 服务；模型测试必须使用本地 mock HTTP server。
- 产品运行时允许读取并向用户配置的 AI Provider 发送绑定工作目录中的受支持文件正文；测试和执行报告仍不得输出真实工作文件正文。
- 不得自动把 AI 建议直接写入 Work、Task、Waiting、Calendar、Inbox 或 Resume Point；必须先进入可编辑确认队列。
- 不得把源工作目录文件当作缓存，不得由清理器删除、移动或修改源文件。
- 不得只修改界面、只修改 CSS 或只创建数据库表后宣布完成。
- 不得省略 `pnpm check`、`cargo fmt --check`、`cargo test`、`pnpm build`、隔离 UI CDP 冒烟、隔离持久化、隔离缓存安全测试和 `pnpm tauri build`。

## Luna 的完成定义

只有当 `ACCEPTANCE.md` 的全部机械项目都有证据并通过，且 release 截图已经提交时，才能进入最终视觉门。最终首页是否真正做到成熟、紧凑、无需滚动，由用户或更高能力模型审阅；该门明确通过前，`EXECUTION_REPORT.md` 状态最多为 `PARTIALLY_COMPLETED`。

## 可直接发送给 Luna 的启动指令

你现在负责直接实施 MSL Desktop 的 AI 秘书与存储治理迭代。项目目录是 `C:\Myfolder\MSL cowork\msl-desktop`。不要创建子代理，不要重新规划。按顺序完整阅读 `docs/luna-ai-secretary-iteration` 下的七个执行文件，再从 `STEPS.md` 的 STEP 01 顺序执行。每一步完成后先写执行报告；PASS 自动继续；触发 STOP 代码立即停止。所有运行应用和写入测试使用隔离 APPDATA、LOCALAPPDATA、TEMP、TMP；不得使用真实 API Key 或真实 AI Provider；不得触碰正式数据库和真实工作目录；不得破坏当前未提交改动。
