# MSL Desktop

**Local-first workspace for Medical Science Liaisons.**

面向医学联络官的桌面工作台。以长期项目为主线，连接专家交流、证据需求、任务、等待反馈和日历，在本机保存可追溯的工作记录。

> 不要让记录工作成为比工作本身更耗费时间的事。
>
> 把 AI 嵌入工作，让信息随工作的推进自然整理、关联和复用。

[官网](https://msl-desktop.pages.dev/) · [下载 Windows x64](https://msl-desktop.pages.dev/downloads/MSL-Desktop-Windows-x64.exe) · [GitHub Releases](https://github.com/holobunganan-sketch/msl-desktop/releases/latest) · [架构说明](docs/architecture.md)

当前交付 Windows 桌面版；macOS 版本开发中。支持中文、英文、主题色与字号设置。安装后的名称为 **MSL Desktop**。

## 工作模型

```text
随手记 / 专家交流 / 已绑定目录的变化
                  │
                  ▼
          原始记录与来源证据
                  │
        用户方向、已有项目与事项
                  │
                  ▼
        秘书整理 → 可编辑的建议
                  │ 用户确认
                  ▼
        项目 ─┬─ 任务与等待反馈
              ├─ 日历与推进记录
              └─ 专家、资料与洞察
                  │
                  ▼
          日常推进 / 问答 / 周月报
```

项目承载长期工作；临时任务、等待和日程可以独立存在。输入一段原话即可开始，分类与时间安排由秘书提出，用户保留最终决定权。用户修改与审阅结果参与后续整理，已关闭事项不会因再次读取同一份资料自动重新开放。

### 与 MSL 日常工作的对应

| 工作场景 | 工作台中的处理 |
| --- | --- |
| 专家交流后留下零散记录 | 保留原话、机构与科室，关联专家和项目，整理临床实践障碍、证据需求、研究合作机会 |
| 项目需要长期跟进 | 把任务、等待反馈、时间节点、进展和多个目录放在同一项目下 |
| 会前需要恢复上下文 | 检索历史交流、已确认方向、相关资料和未解决事项，通过问答继续追问 |
| 收到新资料或方向调整 | 从目录变化和用户输入整理建议，在确认队列中修改或采纳 |
| 阶段性汇报 | 根据周期内记录变化生成周报；月报结合工作记录及相关周报，保留来源与信息缺口 |

这是一款工作组织工具。医学判断、对外沟通与资料使用仍需遵守所在机构的审核和数据政策。

## 功能模块

- **今日工作台**：每日简报、继续推进、待决定建议、日程及资料动态；摘要与细节分层展示。
- **项目与事项**：项目、任务、等待、日历、收件箱之间保留归属关系；手动完成可从最近记录恢复，存在后续修改时阻止覆盖。
- **AI 秘书**：手动或定时整理，后台执行；已有建议等待处理期间保持安静，用户推进后再进入下一轮。
- **专家与洞察**：专家档案、交流记录、资料附件、专家整理及跨专家分析；按专家保留未提交草稿，支持来源追溯。
- **工作台问答**：连续对话、独立模型选择、`@项目` 与专家范围限定、证据引用与资料缺口说明。
- **周报与月报**：可配置周期和生成时间，保留结构化条目及来源；后续建议经编辑确认进入待整理，支持补充报告及历史报告管理。
- **目录与资料**：只读监听关联目录、提取支持格式、维护应用内项目认知与索引，按需打开原文件。
- **翻译**：中英自动识别，书面／口语两种风格，使用独立任务模型。
- **数据管理**：本地 SQLite、可恢复备份、多设备文件夹同步、可重建缓存清理。

## 架构

| 层 | 技术与职责 |
| --- | --- |
| 桌面容器 | Tauri 2、WebView2、托盘、单实例、窗口与快捷键生命周期 |
| 界面 | Svelte 5、TypeScript、SvelteKit、Vite；通过 Tauri IPC 调用业务命令 |
| 业务核心 | Rust；项目与事项、确认流程、来源校验、后台任务及调度 |
| 持久化 | SQLite / rusqlite、WAL、事务和追加式迁移 |
| 资料处理 | notify 文件监听，DOCX / PDF / 文本提取，分块索引、内容摘要与资料定位 |
| 模型连接 | reqwest；Chat Completions、Responses、Anthropic Messages 协议适配 |
| 备份与同步 | 快照包、SHA-256 校验、跨设备实体映射、字段合并与显式冲突 |

前端页面不持有长时模型调用。任务及结果由后台管理，页面切换不会取消已提交的分析。中断的调用不会在下次启动时无条件重放，避免重复请求及费用。

### 模型输入与输出约束

```text
任务路由 → 范围选择 → 结构化记录与相关证据 → 任务规格
                                               │
                                               ▼
用户确认 ← 建议队列 ← 结构、引用与状态校验 ← 模型输出
```

- Provider、模型能力、任务路由和凭据分开管理；秘书、问答、翻译、报告和专家分析可以使用不同模型。
- 提供 DeepSeek、OpenCode Go 模板及自定义连接。文件输入能力受所选接口和模型支持范围限制。
- 任务规格约束 JSON 等输出结构、引用方式与可执行动作，工作主题和内容保持开放。
- 事实、用户表达、推断和建议分开处理；资料不足时保留缺口，不能以生成内容补作证据。
- 文件正文与模型返回内容作为不可信材料处理，不执行其中的脚本、宏、任意 SQL 或操作指令。
- 使用既有认知、读取缓存、相关片段与轮次状态减少重复处理。实际上下文长度、文件支持与费用由 Provider 决定。

建议使用支持 **1M 上下文的多模态模型**处理资料密集的项目。大上下文不会替代来源校验，也不代表每次请求都需要填满上下文。

## 数据与权限边界

| 数据 | 保存位置 | 约束 |
| --- | --- | --- |
| 正式记录、附件副本、恢复状态 | `%APPDATA%\MSLDesktop` | 独立于源码和安装目录 |
| 缓存、日志、临时文件 | `%LOCALAPPDATA%\MSLDesktop` | 按归属规则清理可再生成内容 |
| API Key | Windows 凭据管理器 | 数据库仅保存凭据引用，不进入备份或同步包 |
| 工作目录源文件 | 用户绑定的原目录 | 应用不创建、修改、移动或删除源文件 |
| 备份／同步数据 | 用户选择的独立文件夹 | 可使用 WPS、OneDrive 等云盘客户端的本地同步目录 |

**本地优先不等于所有计算离线。** 使用远程模型时，所选任务的记录和相关文件内容会发送至对应 Provider。请根据实际数据政策选择模型服务和工作目录。

专家主动上传的资料复制到应用持久资料目录；源文件保留原样。源目录的可重建索引与正式附件采用不同的存储和清理规则。项目认知内容保存在应用管理的数据位置，不向工作目录写入说明文件。

### 备份与多设备同步

运行中的 SQLite 数据库始终保存在本机。备份提供可恢复快照；同步交换工作台记录和受支持的附件，二者用途不同。

- `.mslbackup` 包含正式记录、专家附件及适用设置。恢复前校验并保留恢复前副本，更换设备需要重新配置凭据及本机目录。
- 首次连接已有同步数据时要求选择方向；之后按记录关系和字段变化合并，存在冲突时保留候选值。
- 默认同步间隔为 180 分钟，可调整，也可手动触发。自动运行需要应用或托盘保持运行。
- 使用同一同步文件夹的设备应保持相同应用版本；旧版本可能无法读取新版本新增的数据字段，需要先升级。
- API Key、本机目录绑定与索引配置、模型路由、缓存及源工作文件不参与跨设备同步。已保存的原话、问答引用等文本可能包含文件路径，应按工作资料同等保护同步文件夹。
- “同步完成”表示本机同步目录读写完成；云端传输进度由云盘客户端负责。
- 备份及同步文件未提供端到端加密，应使用私有且可信的文件夹。离线双机不具备实时强一致锁。

详见 [备份与恢复](docs/backup-and-restore.md)、[同步数据范围及限制](docs/sync-data-catalog.md)。

## 源码组织

```text
src/
  lib/components/       页面及可复用控件
  lib/services/         导航、交互和展示逻辑
  lib/stores/           前端状态
  lib/types/            前后端数据约定
src-tauri/
  src/commands/         Tauri IPC 边界
  src/db/               数据访问与业务事务
  src/ai/               模型路由、任务规格、建议和校验
  src/documents/        工作目录文档提取
  src/materials/        专家正式资料与读取
  src/workspace/        只读目录监听
  src/scheduler/        后台调度
  src/storage/          数据路径与缓存治理
  src/sync/             多设备交换与冲突处理
  src/backup/           快照、校验与恢复
  migrations/           追加式数据库迁移
tests/                  前端逻辑及组件回归
scripts/                隔离验证、发布和安装检查
docs/                   架构、数据边界与维护文档
```

## 开发与验证

Windows 开发环境需要 Node.js 22+、pnpm、Rust MSVC 工具链、Visual Studio C++ Build Tools 及 WebView2 Runtime。先安装依赖：

```powershell
pnpm install --frozen-lockfile
```

**启动开发版或运行写入测试前，先隔离四个数据目录。** 以下设置仅影响当前终端及其子进程。所有测试只使用合成资料及本地 Mock Provider。

```powershell
$testRoot = Join-Path $env:LOCALAPPDATA ("MSLDesktop-Dev\.test-runtime\" + [guid]::NewGuid().ToString('N'))
foreach ($name in @('APPDATA', 'LOCALAPPDATA', 'TEMP', 'TMP')) {
    $path = Join-Path $testRoot $name
    New-Item -ItemType Directory -Path $path -Force | Out-Null
    [Environment]::SetEnvironmentVariable($name, $path, 'Process')
}
$env:MSL_ISOLATED_TEST = '1'
pnpm tauri dev
```

在相同隔离环境中运行检查：

```powershell
pnpm check
node --test --test-concurrency=1 tests/*.test.mjs scripts/dashboard-ui.test.mjs
pnpm check:installer
pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml --lib -- --test-threads=1
pnpm check:publish
```

原生 IPC、持久化、模拟模型、迁移、同步和响应式布局的验收入口见 [维护验证清单](docs/smoke-checklist.md)。自动测试不覆盖真实云盘的全部网络状态、硬件故障或长期运行条件，发布结论应注明实际验证范围。

## 构建与发布

```powershell
pnpm tauri build
```

Windows 安装包输出到：

```text
src-tauri/target/release/bundle/nsis/MSL Desktop_<version>_x64-setup.exe
```

发布时保持 `package.json`、`src-tauri/tauri.conf.json`、`Cargo.toml` 和 `Cargo.lock` 的包版本一致。`vX.Y.Z` 标签触发 Windows 构建工作流，发布固定文件名 `MSL-Desktop-Windows-x64.exe` 及 SHA-256 校验值，更新 `release/latest.json`。

官网独立托管安装包。发布需要同步官网安装包、校验值和版本元数据，并核对 GitHub 与官网的下载摘要；普通源码提交不会自动替换公开安装包。

`scripts/install-preserving-data.ps1` 用于已授权的本机升级：关闭旧进程后备份数据、执行保留数据更新、核对安装名称／版本／二进制并比较数据文件摘要。该脚本不会自动启动正式应用。

安装模板基于固定的 Tauri CLI 2.11.4，保留旧版安装登记标识。升级 CLI 时需复核 `src-tauri/windows/installer.nsi` 并执行隔离安装验证。

## 贡献与数据安全

- 功能变更同时提交相应回归测试；数据库变更追加迁移，保留旧数据兼容性。
- 不提交数据库、附件、备份包、同步目录、真实工作资料、API Key、测试运行产物或个人截图。
- `.gitignore` 和 `pnpm check:publish` 提供路径级保护；仍需人工检查差异、示例和截图内容。
- 已提交的内容会留在 Git 历史中。增加忽略规则无法撤回已公开的数据。

进一步阅读：[架构](docs/architecture.md) · [备份](docs/backup-and-restore.md) · [同步](docs/sync-data-catalog.md) · [验收](docs/smoke-checklist.md)
