# MSL Desktop

面向 MSL（医学联络官）日常工作的**本地工作管理桌面**（Windows）。

帮助用户在工作被打断后快速恢复上下文：知道自己正在做什么、上次做到哪里、
下一步是什么、有什么正在等待、今天有什么不能忘。真实工作文件保留在原
Windows 路径，软件只管理结构化数据与文件引用。

> 本地 Alpha 版（v0.1.0）。Local-first，无云端、无远程仓库。界面默认中文，可切换英文。

## 主要功能

- **Dashboard**：Continue（最近进行中的 Work + Resume Point）、今日任务/日历/
  截止、需跟进 Waiting、未处理 Inbox、最近文件变化和 Brief 汇总；指标卡可直接导航
- **Workspace**：绑定真实 Windows 工作目录、惰性文件浏览、打开/Reveal、
  文件监听（事件驱动，含 Office 临时文件过滤与 debounce）→ Activity
- **Works**：工作上下文容器 + Resume Point（上次做到 / 下一步 / 需要记住，
  无百分比）、相关文件（置顶/打开）、任务/等待/日历/最近活动聚合
- **Plan / Waiting / Inbox / Calendar**：可创建、编辑、完成/解决、删除的任务与排程、
  等待跟进、快速捕获（Quick Capture，含全局热键 Ctrl+Shift+Space）、本地日历（Day/Week）
- **Search**：Ctrl+K 跨 Works/Files/Tasks/Calendar/Inbox/Resume/Activity 分组搜索
- **Brief（本地优先）**：按昨天、过去 7 天或自定义范围汇总 Work、Resume、Task、
  Waiting、Calendar、Inbox、Activity 与文件元数据变化；无 Provider/Key 或请求失败时
  使用确定性的本地摘要，AI 只负责增强表达
- **AI（可选）**：DeepSeek / OpenAI-compatible Provider（API Key 只存
  Windows 凭据管理器，SQLite 仅保存 credential_ref）；未配置时 App 完整可用
- **AI 秘书**：DeepSeek 与 OpenCode Go 固定模板；自定义 Provider 可选择 Chat
  Completions、Responses 或 Anthropic Messages。六类任务可分别绑定模型，所有 AI
  建议先进入可编辑的 AI 审阅队列，确认后才写入 Work/Task/Waiting/Calendar/Inbox/Resume。
- **工作目录智能**：绑定目录后增量读取受支持的 DOCX、文本和 PDF 文本层文件；正文只进入
  LOCALAPPDATA 下可清理的提取缓存，不写入 SQLite。扫描版 PDF 会标记为需要 OCR。
- **调度、翻译与存储治理**：可设置每 30–1440 分钟周期分析及每日 06:00 分析；首页提供
  中英自动检测翻译（书面/口语）；设置中可预览并确认缓存清理、压缩活动历史和调用 Tauri
  WebView 清理 API，正式数据、源文件、凭据、已确认建议与保留简报始终受保护。
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
cargo fmt --check     # Rust 格式检查
  cargo test            # Rust 单元测试
pnpm build            # 前端生产构建
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
$env:MSL_CDP_PORT='9333'; python .\scripts\ui-smoke-cdp.py
$env:MSL_CDP_PORT='9342'; python .\scripts\release-functional-cdp.py
```

隔离冒烟同时设置 APPDATA、LOCALAPPDATA、TEMP、TMP 到
`.test-runtime\luna-ai-secretary`，不会修改正式数据库；完整执行证据见
`docs/luna-ai-secretary-iteration/EXECUTION_REPORT.md`，截图在该目录下的隔离 artifacts。

## 已知限制

- 未签名（SmartScreen"未知发布者"提示，属本地构建预期）；
- 真实 DeepSeek 成功调用需用户提供 API Key；无 Key 时使用本地 Brief fallback；
- 文件监听只读取路径、大小、修改时间等元数据；用户主动绑定目录并运行文档索引时，
  仅对 DOCX、文本和 PDF 文本层读取正文，提取物进入可治理缓存并受大小/数量预算约束；
- 未签名（SmartScreen“未知发布者”提示，属本地构建预期）。
