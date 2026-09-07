# Acceptance

## Acceptance Criteria

- `pnpm check` 退出码为 0，输出 0 errors / 0 warnings。
- `cargo test` 退出码为 0，原有测试与新增测试全部通过。
- `pnpm build` 退出码为 0。
- `pnpm tauri build` 退出码为 0，并生成 release exe 与 NSIS installer。
- 所有测试运行使用隔离 `APPDATA`；正式数据库的大小、修改时间和内容未因测试改变。
- 中文是首次启动默认语言；切换英文后导航、按钮、表单、状态、空状态、Dashboard 和 Brief UI 全部变为英文；重启隔离实例后保持选择。
- `src/app.html` 默认 `lang="zh-CN"`，标题不再是模板标题；语言切换同步更新 `document.documentElement.lang`。
- 空标题提交 Work、Task、Waiting、Calendar、Quick Capture 时显示可见错误，输入不被清空，数据库计数不增加。
- 有效提交时按钮进入 disabled/loading；成功后显示 Toast、关闭或重置表单、列表更新。
- Quick Capture 在用户停留 Today 或 Inbox 时提交，相关卡片/列表无需切换页面即可刷新。
- Task、Waiting、Calendar 和 Inbox 转换均能选择 Work；保存后 `work_id` 正确，Work 详情立即出现相关记录。
- Calendar 至少提供周视图时间轴和事件列表；新建、编辑、删除、全天、地点、备注、类型与 Work 关联可用。
- 首次绑定目录建立 `workspace_file_state` 基线，不产生每文件一条“刚创建”噪音；产生一条基线汇总事实。
- 应用停止期间对隔离工作目录进行新增、修改、删除，重启后 reconcile 正确写入三类变化且更新基线。
- 实时 watcher 继续记录运行期间变化；临时文件过滤和 debounce 仍通过测试。
- Brief snapshot 包含所选范围活动、Work/Resume、未完成及近期完成任务、Waiting、Calendar、未处理 Inbox、文件变化。
- 文件变化查询在 SQL/Repo 层先过滤 `file.%` 再限量，不能被最近非文件活动挤掉。
- Brief 支持昨天、最近 7 天和自定义范围；范围边界使用本地时间转 Unix 秒并在 UI 中明确显示。
- 无 Provider、无 Key 或 mock AI 失败时仍返回本地结构化 Brief，并显示警告而不是空白或崩溃。
- AI 路径只接收结构化 snapshot，不接收文件正文；Prompt 根据 locale 输出中文或英文。
- Brief 页面显示来源覆盖计数，并能查看精简来源列表；内容中的建议不得超出 snapshot。
- 新 Provider 使用随机 `credential_ref`；两个独立测试数据库的 Provider ID 即使相同，也不会共享凭据引用。
- 现有 Provider 的旧凭据引用兼容，迁移不删除旧 Key，除非新引用保存成功且已有自动化验证。
- Dashboard 在 1024×640 和 1440×900 下无页面级水平滚动条、无 body 默认边距、无双重纵向滚动条。
- Dashboard 包含顶部栏、导航、四个关键指标、继续推进、今日时间线、Waiting、Inbox、最近文件变化、Brief 与来源覆盖。
- 所有用户可见字符串均来自 i18n 字典；允许品牌名、文件路径和用户数据保持原样。
- UI 级 CDP 冒烟通过：Work、Task、Waiting、Calendar、Quick Capture、Work 关联、语言切换、Dashboard 刷新和本地 Brief。
- `README.md`、`docs/architecture.md`、`docs/smoke-checklist.md` 与实际实现一致。

## Validation Procedure

1. 按 STEP 21 执行静态检查与全部单元测试，保存退出码和测试计数。
2. 按 STEP 20/22 在隔离环境执行 UI CDP 冒烟与持久化复查。
3. 按 STEP 23 完成 release 构建，记录 exe 和 installer 路径、大小、修改时间。
4. 对正式数据库记录构建前后的只读文件元数据，确认测试未修改它。
5. 对 1024×640 与 1440×900 各截取中文 Dashboard、英文 Dashboard、Work 创建 Modal、Calendar 周视图、Brief 来源面板。
6. 使用 `rg` 搜索 Svelte 文件中的直接用户文案，列出并处理未进入字典的文本；用户数据、路径、aria 技术值除外。
7. 按本文件逐项标记 PASS/FAIL，把证据写入 `EXECUTION_REPORT.md`。
8. 运行 delegation package validator，确保执行包自身仍为 VALID。

## Required Evidence

- `pnpm check`、`cargo test`、`pnpm build`、`pnpm tauri build` 的命令、退出码和关键输出。
- 新旧 migration 在内存库/临时库上的版本号、表/列存在性、重复运行幂等结果。
- UI 冒烟逐步骤 PASS 列表和隔离数据库计数。
- 离线文件 reconcile 测试的新增/修改/删除计数与对应 activity event_type。
- Brief snapshot 单元测试中各来源类别的非零计数，以及 fallback 输出断言。
- 凭据引用隔离测试，不包含任何 Key 内容。
- 五类截图的绝对路径。
- 修改文件清单和每个文件的用途。
- 正式数据库未修改的文件元数据对比。

## Failure Conditions

- 任一必需命令退出码非 0：`E10-ACCEPTANCE-FAILED`。
- 测试触碰正式数据库或真实 Provider：`E19-SECURITY-BOUNDARY`。
- 新增/编辑仍存在静默失败路径：`E10-ACCEPTANCE-FAILED`。
- 任一主要实体无法关联 Work：`E10-ACCEPTANCE-FAILED`。
- Brief 漏掉 Inbox、文件变化或无截止任务：`E10-ACCEPTANCE-FAILED`。
- 无 AI 时 Brief 仍完全不可用：`E10-ACCEPTANCE-FAILED`。
- 中文/英文切换不完整或不能持久化：`E10-ACCEPTANCE-FAILED`。
- 1024×640 出现关键操作不可达或页面级水平滚动：`E10-ACCEPTANCE-FAILED`。
- migration 需要删表、清库或无法保留旧数据：`E13-DESTRUCTIVE-ACTION`。
- 无证据仅凭口头宣称完成：`E10-ACCEPTANCE-FAILED`。

## High-Model Review Gates

- 最终视觉层级、中文自然度、信息密度和“像成熟 Dashboard”程度由用户或更高能力模型审阅五类截图。Luna 必须提供截图和说明，但不得用主观描述替代此审阅。
- 如果实现过程中发现必须改变产品边界、数据库语义或 Brief 隐私策略，停止并使用 `E16-DECISION-REQUIRED`，不得自行决定。

