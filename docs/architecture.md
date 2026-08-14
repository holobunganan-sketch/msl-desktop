# MSL Desktop — Architecture

> 版本：1.0（最终，本地 Alpha 完成后）
> 对应开发指南：`DEEPSEEK_V4_FLASH_MSL_DESKTOP_DEVELOPMENT_GUIDE.md`
> 状态：Stage 0–12 全部完成，NSIS setup.exe 交付。

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
| AI | reqwest（OpenAI-compatible）+ keyring | DeepSeek preset；API Key 存 Windows 凭据管理器 |
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
│   ├── Today（Continue / Today / Waiting / Inbox / Morning Brief）
│   ├── Workspace（文件浏览 / 绑定 / 打开 / Reveal）
│   ├── Works（Work 详情：Resume Point 置顶 + 各区块聚合）
│   ├── Plan / Waiting / Inbox / Calendar
│   ├── Search（Ctrl+K 覆盖层）
│   └── Settings（AI Providers / Notifications / Autostart / 快捷键）
│
└── On-demand（按需，任务结束即释放）
    ├── AI 请求（60s timeout，request-scoped reqwest client）
    ├── Morning Brief（snapshot 构造 → 模型调用 → 落库，daily cache）
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
│   ├── migrations/0001_init.sql
│   ├── capabilities/default.json
│   └── src/
│       ├── lib.rs              # Builder/setup/生命周期/命令注册
│       ├── app_state.rs        # DB + watcher + 窗口/退出状态
│       ├── commands/           # IPC 边界层（~50 命令）
│       ├── db/                 # Database + 8 个域 repository
│       ├── workspace/          # 目录浏览 + watcher（notify/debounce/filter）
│       ├── ai/                 # provider（OpenAI-compatible/keyring）+ brief
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

## 5. 数据模型（12 张表）

workspaces / works / resume_points / work_file_refs / tasks / waiting_items /
inbox_items / calendar_events / activity_events / daily_briefs /
provider_settings / app_settings（+ schema_migrations）。

关键约束：work_file_refs 只存路径引用；API Key 不落 SQLite（keyring）；
activity_events 是工作事实时间线核心（事件驱动、dedupe_key、多条件查询）。

## 6. 关键工程约束

- "精巧"：常驻只保留必须工作的核心；AI/文档解析按需调用；
- 文件监控事件驱动，禁止周期遍历；2s debounce 合并同路径同事件族；
- Office 临时文件（`~$*`、`*.tmp`、`*.swp`、Thumbs.db、desktop.ini）过滤；
- 无向量数据库 / Embedding / RAG / Electron；
- 性能（Release）：托盘 Working Set 28–36 MB（预算 ≤80），窗口 Private
  141–155 MB（预算 ≤220），idle CPU 0.017%，10× reopen +7.56 MB；
- 文件安全：当前仅"打开/定位"，无删除操作（后续须回收站 + 二次确认）。

## 7. 开发阶段（已完成）

Stage 0 初始化 → 1 Resident Core/Tray → 2 SQLite → 3 Workspace/Watcher →
4 Plan/Waiting/Inbox/Calendar/Quick Capture → 5 Works/Resume Point →
6 Today → 7 Search → 8 AI/Morning Brief → 9 Notification/Autostart →
10 性能加固 → 11 回归/Smoke → 12 本地 NSIS setup.exe。

关键缺陷修复：Stage 10 发现 Release 需 `custom-protocol` feature
（否则误按 dev 加载 devUrl 导致 IPC Origin 校验失败）。

## 8. 交付状态

- 安装包：`src-tauri\target\release\bundle\nsis\msl-desktop_0.1.0_x64-setup.exe`
- 安装验证：安装/开始菜单/启动/数据持久化/卸载/用户数据保留 ✅
- Defender：实时保护无检测（自定义扫描脚本需管理员运行）
- Git：本地 12 个 Stage 提交，无 remote，从未上传
