# MSL Desktop — Architecture (draft)

> 版本：0.1（Stage 0 初稿）
> 对应开发指南：`DEEPSEEK_V4_FLASH_MSL_DESKTOP_DEVELOPMENT_GUIDE.md`
> 本文件随 Stage 推进持续更新。

## 1. 产品定位

MSL（医学联络官）本地工作管理桌面。

- 帮助用户知道"正在做什么、上次做到哪里、下一步是什么"；
- 管理工作，不承担重型专业工作本身；
- Local-first：真实文件保留在原 Windows 路径，软件只保存引用与结构化数据；
- 长期常驻，RAM 受严格约束。

## 2. 技术栈（指南 §2）

| 层 | 技术 | 说明 |
| --- | --- | --- |
| 桌面框架 | Tauri 2 | Rust 常驻核心，Windows 11 第一目标平台 |
| UI 渲染 | Windows WebView2 | 单一主 WebView 窗口，可销毁/重建 |
| 前端 | Svelte 5 + TypeScript + Vite (SvelteKit, adapter-static) | 轻量，原生 CSS，Lucide 级别轻图标 |
| 包管理 | pnpm | |
| 数据库 | SQLite (rusqlite) | `%APPDATA%\MSLDesktop\msl-desktop.db`，WAL |
| 文件监听 | notify crate | 事件驱动，debounce，临时文件过滤 |
| AI | Rust Provider Adapter | DeepSeek preset + Generic OpenAI-compatible，按需调用 |

## 3. 运行时三层结构（指南 §3）

```text
MSL Desktop
│
├── Resident Core          ← 常驻：SQLite / Workspace Registry / File Watcher /
│                              Activity Recorder / scheduler / Tray / settings
│
├── Desktop UI             ← 可销毁重建：Today / Workspace / Works / Plan /
│                              Calendar / Activity / Settings
│
└── On-demand Work         ← 按需：AI 请求 / Morning Brief / 元数据刷新 /
                              文档检视 / 搜索扩展
```

生命周期：主窗口关闭 → 持久化 UI 状态 → 销毁 WebView → Core + Tray 继续；
点击托盘 → 重建主窗口并恢复路由；托盘"退出"才真正结束进程。

## 4. 项目结构（指南 §5，基于 Tauri 模板微调）

```text
msl-desktop/
├── package.json / vite.config.* / svelte.config.* / tsconfig.json
├── src/                      # SvelteKit 前端（adapter-static）
│   ├── routes/               # 页面路由（+layout.ts / +page.svelte）
│   ├── lib/                  # ipc / types / stores
│   └── styles/               # tokens.css / global.css
├── src-tauri/
│   ├── Cargo.toml / tauri.conf.json
│   ├── capabilities/         # IPC 权限
│   ├── migrations/           # SQL migration（Stage 2 引入）
│   └── src/
│       ├── main.rs / lib.rs / app_state.rs
│       ├── commands/         # IPC 边界层
│       ├── db/               # SQLite / repositories
│       ├── workspace/ activity/ works/ plan/ calendar/ inbox/
│       ├── search/ ai/ lifecycle/ notifications/
├── scripts/                  # measure-memory.ps1 / smoke-test.ps1 / defender-scan.ps1
├── tests/fixtures/           # 测试夹具（workspace-small 等）
└── docs/                     # architecture.md / performance.md
```

原则：单文件单职责；IPC command 只做边界转换；前端不直接接触数据库。

## 5. 数据模型（指南 §6，Stage 2 落地）

- workspaces / works / resume_points / work_file_refs
- tasks / waiting_items / inbox_items / calendar_events
- activity_events / daily_briefs / provider_settings / app_settings

关键约束：work_file_refs 只存路径引用，不复制真实文件；API Key 不落 SQLite
明文（keyring reference）；activity_events 是工作事实时间线核心。

## 6. 关键工程约束（指南 §1.2 / §4）

- "精巧"是硬约束：常驻只保留必须工作的核心；
- 文件监控事件驱动，禁止周期性全目录遍历；
- AI 按需调用，默认关闭，未配置时 App 完整可用；
- 禁止向量数据库 / Embedding / RAG / 大型 UI 框架 / Electron；
- 性能预算（Release）：托盘 ≤ 80 MB（优秀 ≤ 60 MB），窗口打开 ≤ 220 MB；
- 文件删除类高风险操作必须二次确认，优先 Recycle Bin。

## 7. 开发阶段（指南 §12–§26）

Stage 0 环境验证与项目初始化 → Stage 1 Resident Core/Tray/生命周期 →
Stage 2 SQLite/迁移 → Stage 3 Workspace/Watcher → Stage 4 Plan/Waiting/Inbox/
Calendar → Stage 5 Works/Resume Point → Stage 6 Today → Stage 7 Search →
Stage 8 AI/Morning Brief → Stage 9 Notification/Autostart → Stage 10 性能加固 →
Stage 11 回归/Smoke → Stage 12 本地 NSIS setup.exe。

每个 Stage 完成后停止并报告，等待用户允许进入下一 Stage。

## 8. Stage 0 现状

- Tauri 2 + Svelte 5 + TS 模板已初始化（create-tauri-app svelte-ts / pnpm）；
- `pnpm check` 通过（0 errors / 0 warnings）；
- `cargo check` 待验证；
- 纯本地 Git（无 remote），符合指南 §13。
