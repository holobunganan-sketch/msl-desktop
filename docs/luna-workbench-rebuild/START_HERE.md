# GPT-5.6 Luna 执行入口

## 目标执行器

GPT-5.6 Luna，在 Codex Desktop 的同一工作区中直接修改当前项目。

## 开始前必须做的事

1. 将工作目录固定为 `C:\Myfolder\MSL cowork\msl-desktop`。
2. 按顺序完整阅读：
   - `docs/luna-workbench-rebuild/TASK.md`
   - `docs/luna-workbench-rebuild/CONTEXT.md`
   - `docs/luna-workbench-rebuild/STEPS.md`
   - `docs/luna-workbench-rebuild/ACCEPTANCE.md`
   - `docs/luna-workbench-rebuild/EXECUTION_REPORT.md`
3. 再读取 `README.md`、`docs/architecture.md`、`src/routes/+page.svelte`、全部 `src/lib/components/*.svelte`、`src-tauri/src/commands/mod.rs`、`src-tauri/src/ai/brief.rs`、`src-tauri/src/workspace/watcher.rs`、`src-tauri/src/db/migrations.rs` 和将要修改的 repository 文件。
4. 在 `EXECUTION_REPORT.md` 中把状态保持为 `PENDING`，从 STEP 01 开始顺序执行。

## 执行规则

- 不要创建子代理，不要把任务再次委派。
- 不要跳过步骤，不要把多个未验证阶段合并成一次大改。
- 每完成一步，立即把命令、观察结果、证据和 PASS/STOP 写入 `EXECUTION_REPORT.md`。
- 前一步 PASS 后自动继续；遇到 STOP 条件立即停止，使用指定停止代码，不得猜测。
- 使用 `rg` 搜索；使用 `apply_patch` 修改文件；不得用覆盖整文件的临时脚本绕过已有内容。
- 工作区不是 Git 仓库：不得初始化 Git、不得创建提交、不得执行 reset/checkout/clean。
- 所有运行应用的测试必须使用隔离的 `APPDATA`。不得让开发版或测试版打开正式数据库。
- 不得读取、复制、输出或调用真实 API Key；不得在测试中调用真实 AI 服务。
- 不得删除、重建或批量修改 `%APPDATA%\MSLDesktop`。
- 不得顺手扩展为云同步、团队协作、CRM、医学内容生成或文件全文索引。

## Luna 的完成定义

只有在 `pnpm check`、`cargo test`、隔离环境 UI 冒烟、release 构建和 `ACCEPTANCE.md` 全部通过后，才能把执行报告状态改为 `COMPLETED`。仅仅“代码写完”不算完成。

