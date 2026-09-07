# MSL Desktop C1 UI Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在不改变现有后端与数据安全规则的前提下，将 MSL Desktop 重建为 C1 雾蓝银灰的编辑式 AI 秘书工作台。

**Architecture:** 使用全局设计变量、可复用 SVG 图标和统一应用外壳建立视觉基础；首页与工作项目页按已确认结构重排；其他业务页面通过共享页面样式和局部组件调整统一。保留现有 Svelte 状态、Tauri invoke 调用和 `data-testid`。

**Tech Stack:** Svelte 5、SvelteKit 2、TypeScript 5.6、Tauri 2、原生 CSS。

**Spec:** `docs/superpowers/specs/2026-08-20-msl-desktop-c1-ui-redesign-design.md`

## Global Constraints

- 中文为默认语言，英文覆盖新增固定文案。
- 不新增运行时依赖。
- 保留现有 Tauri command 接口和数据安全规则。
- 保留既有 `data-testid`。
- 不读取真实 API Key，不访问正式工作文件正文。
- 所有写入测试使用隔离 `APPDATA`、`LOCALAPPDATA`、`TEMP`、`TMP`。
- 当前工作树含大量已有修改；不得执行 reset、checkout、clean、stash、rebase、commit。

---

### Task 1: 建立 C1 设计系统与图标组件

**Files:**
- Modify: `src/lib/styles/app.css`
- Create: `src/lib/components/ui/Icon.svelte`
- Modify: `src/lib/components/ui/AppButton.svelte`
- Modify: `src/lib/components/ui/AppCard.svelte`
- Modify: `src/lib/components/ui/Modal.svelte`
- Modify: `src/lib/components/ui/EmptyState.svelte`
- Modify: `src/lib/components/ui/StatusBadge.svelte`

**Interfaces:**
- Produces: `Icon` 组件，属性为 `name: IconName`、`size?: number`、`strokeWidth?: number`。
- Produces: 全局颜色、尺寸、焦点、表单和页面基础类。

- [ ] **Step 1:** 将 `app.css` 的高饱和蓝色变量替换为规范中的 C1 色值，增加字体、焦点、滚动条、表单和选择文本样式。
- [ ] **Step 2:** 创建本地 SVG `Icon.svelte`，覆盖 dashboard、folder、briefcase、check、clock、calendar、inbox、sparkles、review、settings、search、languages、plus、chevron、file、activity 图标。
- [ ] **Step 3:** 调整五个 UI 基础组件的圆角、边框、悬停、加载、危险色与空状态。
- [ ] **Step 4:** 运行 `pnpm check`，预期 0 errors、0 warnings。

### Task 2: 重建应用外壳

**Files:**
- Modify: `src/routes/+page.svelte`
- Modify: `src/lib/i18n/zh-CN.ts`
- Modify: `src/lib/i18n/en-US.ts`

**Interfaces:**
- Consumes: Task 1 的 `Icon` 和全局 C1 变量。
- Produces: 分组导航、紧凑顶栏、响应式折叠侧栏和秘书状态区。

- [ ] **Step 1:** 为导航项建立包含视图、翻译键、图标和可选分组的本地配置。
- [ ] **Step 2:** 保留所有导航 `data-testid` 和 `dashboard:navigate` 事件，改写侧栏与顶栏标记。
- [ ] **Step 3:** 新增中英文文案：导航分组、快速记录辅助文案、秘书状态与下次分析占位状态。
- [ ] **Step 4:** 添加 1100px 折叠规则和 760px 紧凑顶栏规则。
- [ ] **Step 5:** 运行 `pnpm check`，预期 0 errors、0 warnings。

### Task 3: 重排首页信息架构

**Files:**
- Modify: `src/lib/components/TodayView.svelte`
- Modify: `src/lib/components/TranslationCard.svelte`
- Modify: `src/lib/i18n/zh-CN.ts`
- Modify: `src/lib/i18n/en-US.ts`

**Interfaces:**
- Consumes: 现有 `getToday`、`listAiProposals`、`generate_brief`、`get_morning_brief`、`workspace_sync_status`、`recent_files`。
- Produces: 单屏 `dashboard-c1` 布局，不改变命令参数和返回类型。

- [ ] **Step 1:** 保留所有数据加载和操作函数，将页面标记改为简报、指标条和三列工作区。
- [ ] **Step 2:** 将简报详情、范围和来源放入可展开区域；首屏显示摘要和 3 条优先事项。
- [ ] **Step 3:** 组合今日日程与任务时间线，保留完成任务、解决等待和页面跳转动作。
- [ ] **Step 4:** 将翻译卡改为紧凑常驻形式，保留书面/口语、自动检测、复制和错误提示。
- [ ] **Step 5:** 在 1024×640 和 1440×900 CSS 断点下限制首页外层滚动。
- [ ] **Step 6:** 运行 `pnpm check` 和 `pnpm build`，两项均成功。

### Task 4: 重建工作项目内页

**Files:**
- Modify: `src/lib/components/WorksView.svelte`
- Modify: `src/lib/components/AiReviewCenter.svelte`

**Interfaces:**
- Consumes: 现有 work、resume point、file、task、waiting、calendar 和 activity commands。
- Produces: 左侧工作列表、中间详情标签区、右侧 AI 确认入口；所有写入仍调用现有 commands。

- [ ] **Step 1:** 将工作列表改为固定宽度面板，显示状态、更新时间和选择态。
- [ ] **Step 2:** 将工作详情改为标题摘要、状态操作、继续推进和分类内容区。
- [ ] **Step 3:** 将关联任务、等待、日程、文件和活动整理为紧凑卡片；保留全部按钮处理函数。
- [ ] **Step 4:** 将 AI 审阅中心调整为可编辑确认工作区，保留保存草稿、拒绝、上一项、下一项和确认写入动作。
- [ ] **Step 5:** 运行 `pnpm check`，预期 0 errors、0 warnings。

### Task 5: 统一任务、等待、日历、收件箱和工作目录

**Files:**
- Modify: `src/lib/components/PlanView.svelte`
- Modify: `src/lib/components/WaitingView.svelte`
- Modify: `src/lib/components/CalendarView.svelte`
- Modify: `src/lib/components/InboxView.svelte`
- Modify: `src/lib/components/WorkspaceView.svelte`

**Interfaces:**
- Consumes: 各组件现有 Tauri commands、表单状态与测试定位符。
- Produces: 统一的 `page-head`、工具条、内容卡片、状态徽标、空状态和弹窗形式。

- [ ] **Step 1:** 统一五个页面的标题区、主要操作和说明文字层级。
- [ ] **Step 2:** 统一输入、筛选、列表、状态和行内操作视觉，删除重复的局部颜色常量。
- [ ] **Step 3:** 调整日历日/周视图、收件箱转换面板和工作目录索引状态，使窄窗口可用。
- [ ] **Step 4:** 保留新建任务、等待、日程、收件箱转换、绑定与扫描目录的全部现有函数和 `data-testid`。
- [ ] **Step 5:** 运行 `pnpm check`，预期 0 errors、0 warnings。

### Task 6: 重建设置与 Provider 页面

**Files:**
- Modify: `src/lib/components/SettingsView.svelte`
- Modify: `src/lib/components/ProviderSettings.svelte`
- Modify: `src/lib/components/AiRoutingSettings.svelte`
- Modify: `src/lib/components/AnalysisScheduleSettings.svelte`
- Modify: `src/lib/components/StorageSettings.svelte`

**Interfaces:**
- Consumes: 现有 DeepSeek、OpenCode Go、自定义 Provider、任务路由、调度和存储治理 commands。
- Produces: 设置分类导航与右侧设置内容；业务字段和保存逻辑保持原样。

- [ ] **Step 1:** 在设置页增加分类目录和锚点，将 Provider、模型分工、分析调度、存储、通知、启动和语言分组。
- [ ] **Step 2:** 将 Provider 模板改为轻量目录卡，突出 DeepSeek 和 OpenCode Go 固定模板，自定义接口保持次级入口。
- [ ] **Step 3:** 统一路由选择、调度表单、存储容量与清理确认的视觉层级。
- [ ] **Step 4:** 保留 API Key 掩码、安全说明、连接测试、清理预览和确认机制。
- [ ] **Step 5:** 运行 `pnpm check` 和 `pnpm build`，两项均成功。

### Task 7: 自动化界面验证

**Files:**
- Modify if selectors require adaptation: `scripts/ui-smoke-cdp.py`
- Modify if layout assertions require adaptation: `scripts/dashboard-layout-cdp.py`
- Use: `scripts/run-isolated-audit.ps1`
- Use: `scripts/capture-final-screenshots.py`

**Interfaces:**
- Consumes: 现有隔离运行脚本与 CDP 定位符。
- Produces: 首页、工作项目、AI 审阅、设置页截图和机械验收结果。

- [ ] **Step 1:** 运行 `pnpm check`，预期成功。
- [ ] **Step 2:** 运行 `pnpm build`，预期成功。
- [ ] **Step 3:** 使用隔离目录启动测试实例，不得接触正式数据库和正式缓存。
- [ ] **Step 4:** 运行 UI CDP 冒烟，验证导航、快速记录、搜索、语言切换、创建入口和设置入口。
- [ ] **Step 5:** 运行首页布局检查，验证 1024×640 与 1440×900 无外层滚动和遮挡。
- [ ] **Step 6:** 截取首页、工作项目、AI 审阅、设置页并逐张检查文字截断、溢出、对比度和层级。
- [ ] **Step 7:** 修复检查发现的问题，重复步骤 1–6，直到全部通过。

### Task 8: Release 构建与交付

**Files:**
- Modify: `docs/superpowers/plans/2026-08-20-msl-desktop-c1-ui-redesign.md`（勾选完成状态）

**Interfaces:**
- Consumes: Task 1–7 的界面成果和验证证据。
- Produces: 可安装 release 构建与最终截图。

- [ ] **Step 1:** 运行 `cargo fmt --check` 和 `cargo test`，确认 UI 修改未影响 Rust 项目。
- [ ] **Step 2:** 运行 `pnpm tauri build`，预期生成成功的 Windows release 包。
- [ ] **Step 3:** 使用隔离环境启动 release，重复关键导航和首页检查。
- [ ] **Step 4:** 在计划中勾选完成项，记录构建产物和截图路径。

## Self-review

- 规范中的颜色、外壳、首页、内页、AI 确认、设置、国际化、响应式和验收均对应具体任务。
- 计划未使用待定项或省略实现的占位描述。
- 组件接口沿用当前 Svelte 5 与 Tauri invoke 结构；新增接口仅为本地 `Icon` 组件。
- Git 步骤已按当前工作树保护要求删除。

## Completion Record · 2026-08-20

- Tasks 1–6：完成。C1 设计系统、应用外壳、首页、工作项目、AI 审阅、业务页与设置页已落地。
- Task 7：完成。UI smoke `17/17`；release functional `11/11`；首页布局中英文 × 两种分辨率 `4/4`。
- Task 8：完成。`pnpm check`、`pnpm build`、`cargo fmt --check`、`cargo test` 和 `pnpm tauri build` 通过。
- Rust 测试：`68 passed; 0 failed`。
- 安装包：`src-tauri/target/release/bundle/nsis/msl-desktop_0.1.0_x64-setup.exe`。
- 最终视觉证据：`.test-runtime/luna-ai-secretary/artifacts/ui-c1/` 与 `.test-runtime/luna-ai-secretary/artifacts/release-c1-final/`。
- 集成处理：按项目约束保留当前工作树，不执行提交、分支合并或清理。
