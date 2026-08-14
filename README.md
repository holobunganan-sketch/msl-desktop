# MSL Desktop

面向 MSL（医学联络官）日常工作的**本地工作管理桌面**（Windows）。

帮助用户在工作被打断后快速恢复上下文：知道自己正在做什么、上次做到哪里、
下一步是什么、有什么正在等待、今天有什么不能忘。真实工作文件保留在原
Windows 路径，软件只管理结构化数据与文件引用。

> 本地 Alpha 版（v0.1.0）。Local-first，无云端、无远程仓库。

## 主要功能

- **Today**：Continue（最近进行中的 Work + Resume Point）、今日任务/日历/
  截止、需跟进 Waiting、未处理 Inbox、Morning Brief（可选 AI）
- **Workspace**：绑定真实 Windows 工作目录、惰性文件浏览、打开/Reveal、
  文件监听（事件驱动，含 Office 临时文件过滤与 debounce）→ Activity
- **Works**：工作上下文容器 + Resume Point（上次做到 / 下一步 / 需要记住，
  无百分比）、相关文件（置顶/打开）、任务/等待/日历/最近活动聚合
- **Plan / Waiting / Inbox / Calendar**：任务与排程、等待跟进、快速捕获
  （Quick Capture，含全局热键 Ctrl+Shift+Space）、本地日历（Day/Week）
- **Search**：Ctrl+K 跨 Works/Files/Tasks/Calendar/Inbox/Resume/Activity 分组搜索
- **AI（可选）**：DeepSeek / OpenAI-compatible Provider（API Key 存
  Windows 凭据管理器）、Morning Brief（快照驱动、每日缓存）；未配置时
  App 完整可用
- **常驻体验**：托盘常驻（窗口关闭不退出）、提醒通知、可选开机自启动
  （后台模式）、窗口状态恢复、单实例

## 技术栈

| 层 | 技术 |
| --- | --- |
| 桌面框架 | Tauri 2（Rust 常驻核心，Windows 11 第一目标） |
| 前端 | Svelte 5 + TypeScript + Vite (SvelteKit, adapter-static) |
| 数据 | SQLite（rusqlite，WAL，显式 migration） |
| 文件监听 | notify（ReadDirectoryChangesW） |
| AI | reqwest（OpenAI-compatible /chat/completions）+ keyring |

## 开发

环境要求：Rust（MSVC toolchain）、Node.js、pnpm、WebView2 Runtime。

```bash
pnpm install
pnpm tauri dev        # 开发模式
```

质量检查：

```bash
pnpm check            # 前端 TypeScript/Svelte 检查
cargo test            # Rust 单元测试（24 个）
```

## 构建安装包

```bash
pnpm tauri build      # 生成 NSIS setup.exe
```

产物：

- `src-tauri\target\release\msl-desktop.exe`
- `src-tauri\target\release\bundle\nsis\msl-desktop_0.1.0_x64-setup.exe`

安装为当前用户（`%LOCALAPPDATA%\msl-desktop`），卸载保留用户数据库
（`%APPDATA%\MSLDesktop`）。

## 冒烟与性能

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\smoke-test.ps1
powershell -ExecutionPolicy Bypass -File .\scripts\measure-memory.ps1
powershell -ExecutionPolicy Bypass -File .\scripts\defender-scan.ps1   # 需管理员
```

详细记录见 `docs/`（架构、性能、12 个 Stage 报告、人工冒烟清单）。

## 已知限制

- 未签名（SmartScreen"未知发布者"提示，属本地构建预期）；
- 真实 DeepSeek 成功调用需用户提供 API Key（协议已由 mock + 401 验证）；
- 文件系统事件的 Work 归因（watcher → work_id）为后续增强项。
