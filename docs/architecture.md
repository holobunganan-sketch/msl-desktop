# MSL Desktop — Architecture

> 版本：2.1（AI 秘书与存储治理第二阶段）
> 对应开发指南：`DEEPSEEK_V4_FLASH_MSL_DESKTOP_DEVELOPMENT_GUIDE.md`
> 状态：Dashboard/CRUD/Workspace 文档索引/AI 审阅/调度/翻译/存储治理已实现；release NSIS setup.exe 已构建，视觉审阅仍由用户确认。

## 1. 产品定位

MSL（医学联络官）本地工作管理桌面。

- 帮助用户知道"正在做什么、上次做到哪里、下一步是什么"；
- 管理工作，不承担重型专业工作本身；
- Local-first：真实文件保留在原 Windows 路径，软件只保存引用与结构化数据；
- 长期常驻，RAM 受严格约束。

## 2. 技术栈

| 层 | 技术 | 说明 |
| --- | --- | --- |
| 桌面框架 | Tauri 2（`custom-protocol` + `tray-icon` + `image-png`） | Rust 常驻核心，Windows 11 第一目标 |
| UI 渲染 | Windows WebView2 | 单一主窗口，可销毁/重建；`--disable-gpu --renderer-process-limit=1` 优化内存 |
| 前端 | Svelte 5 + TypeScript + Vite (SvelteKit, adapter-static, SPA) | 无大型 UI 框架，单页内视图切换 |
| 包管理 | pnpm | |
| 数据库 | SQLite（rusqlite bundled，WAL） | `%APPDATA%\MSLDesktop\msl-desktop.db`，显式 migration |
| 文件监听 | notify（ReadDirectoryChangesW） | 事件驱动，2s debounce，Office 临时文件过滤 |
| AI | reqwest（三协议 adapter）+ keyring | DeepSeek/OpenCode Go preset；API Key 存 Windows 凭据管理器 |
| 通知/快捷键 | tauri-plugin-notification / global-shortcut | Windows toast、Ctrl+Shift+Space 热键 |
| 托盘 | tauri tray | 打开 / Quick Capture / 退出；左键单击重建窗口 |

## 3. 运行时三层结构

```text
MSL Desktop
│
├── Resident Core（常驻）
│   ├── SQLite（Database：WAL / migration / graceful close）
│   ├── Workspace Registry + File Watcher（notify，UI 销毁后继续）
│   ├── Activity Recorder（命令操作 + 文件事件 → activity_events）
│   ├── Reminder Scheduler（30s 低频轮询：waiting/task/calendar 到期通知）
│   ├── Tray（打开 / Quick Capture / 退出）
│   └── lifecycle（autostart / --background / window state / graceful shutdown）
│
├── Desktop UI（可销毁重建）
│   ├── Dashboard（指标 / Continue / 时间线 / Waiting / Inbox / 文件变化 / Brief）
│   ├── Workspace（文件浏览 / 绑定 / 打开 / Reveal）
│   ├── Works（Work 详情：Resume Point 置顶 + 各区块聚合）
│   ├── Plan / Waiting / Inbox / Calendar
│   ├── Search（Ctrl+K 覆盖层）
│   └── Settings（语言外观 / 同步健康 / AI Provider / Notifications / Autostart / 快捷键）
│
└── On-demand（按需，任务结束即释放）
    ├── AI 请求（Chat Completions / Responses / Anthropic Messages，request-scoped client）
    ├── Analysis Scheduler（周期/每日 06:00；调度状态跨重启持久化）
    ├── Document Intelligence（DOCX/PDF/text → LOCALAPPDATA cache → bounded snapshot）
    ├── Proposal Review（结构化建议 → 可编辑队列 → 用户确认事务）
    ├── Morning Brief（snapshot 构造 → optional model/local fallback → draft/kept）
    ├── Translation（中英方向检测；仅组件内存，不落库）
    ├── Storage Governance（usage/preview/cleanup/rollup；受保护对象永不删除）
    ├── 文件元数据刷新 / list_dir（惰性单层）
    └── 提醒检查（进程内去重）
```

生命周期：主窗口关闭 → 保存窗口状态 → 销毁 WebView → Core + Tray 常驻；
托盘点击 → 重建窗口（恢复位置大小）；托盘"退出" → WAL checkpoint 后退出；
`--background` 启动（自启动）不创建主窗口。

## 4. 项目结构

```text
msl-desktop/
├── src/                        # SvelteKit 前端（SPA）
│   ├── routes/+page.svelte     # 应用壳：导航 + 视图切换
│   └── lib/components/         # Today/Workspace/Works/Plan/Waiting/
│                               #   Inbox/Calendar/Search/Settings/QuickCapture
├── src-tauri/
│   ├── migrations/0001_init.sql + 0002_workbench_reliability.sql
│   ├── capabilities/default.json
│   └── src/
│       ├── lib.rs              # Builder/setup/生命周期/命令注册
│       ├── app_state.rs        # DB + watcher + 窗口/退出状态
│       ├── commands/           # IPC 边界层（~50 命令）
│       ├── db/                 # Database + 8 个域 repository
│       ├── workspace/          # 目录浏览 + watcher + metadata inventory/reconcile
│       ├── ai/                 # adapters/catalog/router/analysis/proposals/translation/Brief snapshot
│       ├── documents/          # DOCX/PDF/text extraction, chunks, incremental indexer
│       ├── storage/            # safe cache paths, usage, cleanup and activity compaction
│       ├── scheduler/          # interval/daily due logic with injected-clock tests
│       ├── notifications/      # reminder 调度
│       └── lifecycle.rs        # autostart/window state/background
├── scripts/
│   ├── measure-memory.ps1      # 内存测量（子进程归属/状态标记/CSV）
│   ├── smoke-test.ps1 + smoke-cdp.py
│   ├── defender-scan.ps1
│   └── generate-large-dir.ps1  # 10,000 文件大目录
├── tests/fixtures/workspace-small
└── docs/                       # architecture/performance/stage-0~12/smoke-checklist
```

原则：单文件单职责；IPC command 只做边界转换；前端不直接接触数据库。

## 5. 数据模型（v7；正式数据库仍位于 APPDATA）

workspaces / works / resume_points / work_file_refs / tasks / waiting_items /
inbox_items / calendar_events / activity_events / daily_briefs /
provider_settings / app_settings / provider_connections / provider_models /
ai_task_routes / work_workspace_links / document_index / cache_entries /
analysis_schedule_state / analysis_runs / ai_proposals / daily_activity_rollups /
storage_cleanup_runs / reports / report_schedule_state（+ schema_migrations）。

关键约束：work_file_refs 只存路径引用；workspace_file_state 只存文件元数据；
API Key 不落 SQLite（keyring，数据库仅存 credential_ref）；Brief source_snapshot_json
只保存结构化事实，不保存工作文件正文、Provider header 或密钥；
activity_events 是工作事实时间线核心（事件驱动、dedupe_key、多条件查询）。

迁移 0003–0007 均保留已有业务数据：Provider 目录与任务路由、文档智能元数据、AI 运行/建议与
保留状态、活动 rollup/清理审计、周报/月报及其调度分别加入；长正文不进入 SQLite，凭据只保存 credential_ref。

## 6.1 Brief source pipeline

`workspace inventory / command mutations → activity_events + document_index →
AnalysisSnapshot（预算/来源 hash） → optional router/adapter → ai_proposals + daily_briefs`

Brief snapshot 明确记录 period、locale、source_counts、truncated 和有限来源预览。
本地 renderer 固定建议排序（逾期高优任务、今日安排、Waiting、Resume next_step、Inbox），
AI 不得改变事实范围；Provider 缺失、Key 缺失、HTTP 错误或空响应都回退到本地摘要。

## 7. AI 秘书与存储边界

- watcher/reconcile 只更新 dirty/index 状态，不直接调用 AI；调度按用户设置触发分析。
- AI 输出只允许 create/update 的 Work、Task、Waiting、Calendar、Inbox、Resume Point；
  所有结果先入 `ai_proposals`，用户可编辑并确认，确认事务才写业务表。
- 受支持文件正文仅在用户绑定目录并发起索引/分析时读取；snapshot 限制 20 文件、单文件
  40,000 字符、总计 120,000 字符，source_ref 为相对路径/文档 id。
- cleanup 只处理安全 cache root 下可重建条目和明确过期历史；pending/confirmed proposal、
  kept Brief、保留报告、正式实体、凭据和源目录列为 protected；WebView 只能通过 Tauri API 清理。

## 7.1 决策工作流与周期报告

- 首页“需要您决定”仅加载最近一次已完成分析的待确认建议；每条建议可调整为 Work、Task、Waiting、Calendar 或 Inbox。
- Work 表示长期项目；Task 与 Waiting 可关联 Work，也可标记为临时事务。建议可暂缓，确认事务完成前不写业务表。
- AI 审阅加载最近 7 天记录，可按状态和分析批次筛选；待确认与已暂缓记录可深入编辑，已处理记录保持只读。
- 周报默认覆盖生成日前 7 天，正文使用序号列表；月报默认覆盖上一个自然月，并引用周期重叠的周报作为综合分析证据。
- 报告生成必须配置 `weekly_report` 或 `monthly_report` 路由；运行、完成和失败状态均持久化，失败记录可重试。
- 调度器每分钟判断分析、周报和月报是否到期，并利用周期键去重。默认周报为星期日 17:00，月报为每月 1 日 09:00。
- 报告快照汇总 Work、Task、Waiting、Calendar、Inbox、Resume Point、活动时间线与工作目录变化，排除 AI 翻译记录。
- AI 审阅默认筛选待确认建议，并展示最近一次分析的运行状态；失败时显示安全截断的错误原因和重新分析入口。
- 首页只读取最近一次已完成分析的建议。最新分析没有建议时返回空列表，不回流旧批次建议。
- Provider HTTP 客户端启用 Windows 系统代理；localhost、127.0.0.0/8 与 IPv6 loopback 保持直连，供本地 Mock 和离线服务使用。

## 8. 关键工程约束

- "精巧"：常驻只保留必须工作的核心；AI/文档解析按需调用；
- 文件监控事件驱动，禁止周期遍历；2s debounce 合并同路径同事件族；
- Office 临时文件（`~$*`、`*.tmp`、`*.swp`、Thumbs.db、desktop.ini）过滤；
- 无向量数据库 / Embedding / RAG / Electron；
- 性能（Release）：托盘 Working Set 28–36 MB（预算 ≤80），窗口 Private
  141–155 MB（预算 ≤220），idle CPU 0.017%，10× reopen +7.56 MB；
- 文件安全：当前仅"打开/定位"，无删除操作（后续须回收站 + 二次确认）。

## 9. 开发阶段（已完成）

Stage 0 初始化 → 1 Resident Core/Tray → 2 SQLite → 3 Workspace/Watcher →
4 Plan/Waiting/Inbox/Calendar/Quick Capture → 5 Works/Resume Point →
6 Today → 7 Search → 8 AI/Morning Brief → 9 Notification/Autostart →
10 性能加固 → 11 回归/Smoke → 12 本地 NSIS setup.exe。

关键缺陷修复：Stage 10 发现 Release 需 `custom-protocol` feature
（否则误按 dev 加载 devUrl 导致 IPC Origin 校验失败）。

## 10. 交付状态

- 安装包：`src-tauri\target\release\bundle\nsis\msl-desktop_0.1.0_x64-setup.exe`
- 安装验证：release exe/NSIS 已生成；隔离 release layout 4/4、基础 UI smoke 17/17、
  AI Review/Workspace/Provider/Schedule/Storage 功能 smoke 11/11 ✅
- Defender：实时保护无检测（自定义扫描脚本需管理员运行）
- Git：本次执行未初始化、提交、reset、checkout、clean 或修改 Git 状态；项目现有 worktree 事实已记录在执行报告。
