# OpenCode Go、外观与 AI 工作流 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复 OpenCode Go 协议并完成 Muse、外观、独立翻译页、首页重排、系统指令和 Windows 图标迭代。

**Architecture:** Provider 使用单连接多模型共享凭据；外观设置以 root dataset 和 CSS 变量实现；翻译拥有独立路由视图；提示词集中在 Rust prompt registry 中并由各 AI 调用点消费。

**Tech Stack:** Svelte 5、TypeScript、Tauri 2、Rust、SQLite、CDP Python smoke。

**Spec:** `docs/superpowers/specs/2026-08-20-opencode-appearance-workflow-design.md`

## Global Constraints

- 不读取、打印或调用真实 API Key。
- 不调用真实 Provider；网络协议测试使用本地 Mock HTTP server。
- 应用测试同时隔离 APPDATA、LOCALAPPDATA、TEMP、TMP。
- 不 reset、checkout、clean、stash、rebase、commit；保留已有未提交成果。
- AI 建议确认前不得写入正式实体。

---

### Task 1: OpenCode Go 与 Muse 协议

**Files:**
- Modify: `src-tauri/src/ai/catalog.rs`
- Modify: `src-tauri/src/ai/adapters.rs`
- Modify: `src/lib/components/ProviderSettings.svelte`
- Modify: `src/lib/i18n/zh-CN.ts`
- Modify: `src/lib/i18n/en-US.ts`

**Interfaces:**
- Produces: `known_model("opencode_go", "muse-spark-1.2-contributor") -> responses /responses`。
- Produces: OpenCode Go 连接下 Muse 模型复用连接 `credential_ref`。

- [ ] 写 Rust 失败测试，断言 Muse ID、协议、端点和 Responses 请求路径。
- [ ] 运行 `cargo test ai::catalog::tests::fixed_templates_match_locked_snapshot`，确认因缺少 Muse 失败。
- [ ] 在固定模型表增加 Muse；未知模型使用安全占位 `responses`、标记 `needs_protocol` 且保持禁用；固定模板刷新修复协议。
- [ ] 运行 catalog/adapters 测试，确认通过。
- [ ] 在 Provider UI 标识 Muse 专用模型和 Contributor 数据使用提示。

### Task 2: 外观设置

**Files:**
- Create: `src/lib/stores/appearance.ts`
- Create: `src/lib/components/AppearanceSettings.svelte`
- Modify: `src/lib/styles/app.css`
- Modify: `src/lib/components/SettingsView.svelte`
- Modify: `src/routes/+page.svelte`
- Modify: `src/lib/i18n/zh-CN.ts`
- Modify: `src/lib/i18n/en-US.ts`

**Interfaces:**
- Produces: `initializeAppearance(): Promise<void>`、`setFontSize(FontSize)`、`setTheme(AppTheme)`。
- Persists: `appearance_font_scale`, `appearance_theme`。

- [ ] 扩展 CDP 测试，断言外观控件和 root dataset；运行确认失败。
- [ ] 实现四档字号、四套主题和持久化 store。
- [ ] 在根页面初始化，在设置页加入实时预览控件。
- [ ] 运行 `pnpm check` 和隔离 CDP 外观测试，确认通过。

### Task 3: 独立 AI 翻译页面

**Files:**
- Create: `src/lib/components/TranslationView.svelte`
- Modify: `src/lib/components/TranslationCard.svelte`
- Modify: `src/lib/components/TodayView.svelte`
- Modify: `src/routes/+page.svelte`
- Modify: `src/lib/i18n/zh-CN.ts`
- Modify: `src/lib/i18n/en-US.ts`

**Interfaces:**
- Produces: `View = "translation"` 和 `data-testid="nav-translation"`。
- Consumes: `translate_text(input, style)`。

- [ ] 扩展 CDP 测试，断言首页不含翻译输入且侧栏翻译页包含大输入和输出区；运行确认失败。
- [ ] 新建翻译页面并接入侧栏导航。
- [ ] 从首页删除翻译卡片并清理相应布局 CSS。
- [ ] 运行 `pnpm check` 与 CDP 测试，确认通过。

### Task 4: 首页布局

**Files:**
- Modify: `src/lib/components/TodayView.svelte`
- Modify: `scripts/dashboard-layout-cdp.py`

**Interfaces:**
- Produces: 三栏桌面布局、两栏窄桌面布局、单栏移动布局。

- [ ] 增加布局断言，验证首页无横向滚动、关键板块可见、无翻译输入；运行确认失败。
- [ ] 重排桌面工作区板块并调整间距、对齐、卡片高度。
- [ ] 在中英文 1024×640、1440×900 下运行布局测试。

### Task 5: 任务系统指令

**Files:**
- Create: `src-tauri/src/ai/prompts.rs`
- Modify: `src-tauri/src/ai/mod.rs`
- Modify: `src-tauri/src/ai/brief.rs`
- Modify: `src-tauri/src/commands/ai_secretary.rs`

**Interfaces:**
- Produces: `prompt_for(task_kind: &str, locale: &str, context: PromptContext<'_>) -> Result<String, String>`。
- Consumes: `workspace_analysis`, `work_draft`, `global_analysis`, `daily_brief`, `translation`。

- [ ] 写失败测试，断言每类提示词含来源、事实边界、注入防护和确认队列约束。
- [ ] 实现版本化 prompt registry。
- [ ] 替换 Brief 与翻译的内联 system 文本；为结构化分析提供统一 builder。
- [ ] 运行 prompt、brief、translation、schema 测试。

### Task 6: 图标、完整验证与发布

**Files:**
- Modify: `src-tauri/icons/*`
- Modify: `scripts/release-functional-cdp.py`

**Interfaces:**
- Produces: 带新版任务栏/标题栏/安装程序图标的 `msl-desktop.exe` 与 NSIS 安装包。

- [ ] 用 Tauri icon 生成全尺寸资源，核对 `tauri.conf.json` 引用。
- [ ] 运行 `pnpm check`、`pnpm build`、`cargo fmt --check`、`cargo test`。
- [ ] 构建 debug，运行隔离 UI smoke、Provider、外观、翻译和布局测试。
- [ ] 运行 `pnpm tauri build`。
- [ ] 用 release exe 再跑隔离功能 smoke，验证数据库完整性并生成截图。
