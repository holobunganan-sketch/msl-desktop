# Context

## Known Facts

- 项目根为 `C:\Myfolder\MSL cowork\msl-desktop`，运行环境为 Windows 与 PowerShell。
- 当前项目是 Git 工作树，且存在大量上一阶段尚未提交的修改和新增文件；这些是需要保留的现有成果。
- 2026-08-15 编译本执行包时，`pnpm check` 为 0 errors / 0 warnings，`cargo test` 为 34 passed / 0 failed。
- 编译本执行包时发现 PID 24988 正在运行 `src-tauri\target\release\msl-desktop.exe`；Luna 执行 STEP 01 时必须重新检查，不能沿用该 PID，也不能擅自结束用户进程。
- 上一阶段执行报告状态为 `PARTIALLY_COMPLETED`，机械验收已经通过，视觉门尚未由用户明确关闭。
- 当前 Dashboard 在 1024×640 截图中有明显页面垂直滚动条，Brief 位于页面底部，不符合本轮“一眼尽收眼底、Brief 在顶部”的要求。
- 当前 `TodayView.svelte` 依次渲染欢迎区、Quick Capture、四指标、工作/时间线、Waiting/Inbox/文件变化，最后才渲染 Brief，因此内容高度必然超过最小窗口。
- 当前 `provider_settings` 每行只有一个 `model`，`list_enabled()` 依赖第一个启用 Provider；不存在模型目录或按任务路由。
- 当前 `ai/provider.rs` 只支持 OpenAI-compatible `/chat/completions`。
- 当前 Provider Key 使用稳定 `credential_ref` 存入 Windows Credential Manager，SQLite 不保存明文 Key；本轮必须保留。
- 当前工作目录 `inventory.rs` 明确只读取 metadata，不读取正文；这是上一阶段旧边界，本轮用户已经明确撤销该限制。
- 当前 `workspace_file_state` 只保存 workspace、path、mtime、size、seen_at，不保存内容 hash、提取状态或摘要。
- 当前 Brief 已包含 Work、Resume、Task、Waiting、Calendar、Inbox、Activity 和 file changes，并有 deterministic local fallback，但不包含文件正文派生信息。
- 当前提醒调度每 30 秒检查 Waiting、Task、Calendar，不存在 AI 分析调度和防重入状态。
- 当前正式数据库默认位于 `%APPDATA%\MSLDesktop\msl-desktop.db`；缓存还没有独立目录或清理器。
- 当前所有应用运行测试只隔离 `APPDATA`，本轮必须同时隔离 `LOCALAPPDATA`、`TEMP` 和 `TMP`，否则正文缓存或 WebView 缓存可能落到正式位置。
- 当前窗口最小尺寸是 1024×640，默认尺寸是 1240×720。

## Official Provider Snapshot — 2026-08-15

以下快照已经由高能力模型从官方文档核对，Luna 不得重新猜测协议：

### DeepSeek fixed template

- Template kind：`deepseek`。
- Base URL：`https://api.deepseek.com`。
- Auth：Bearer API Key。
- Models endpoint：允许尝试 `${base_url}/models`；失败不影响内置模型。
- `deepseek-v4-pro`：Chat Completions，endpoint `/chat/completions`。
- `deepseek-v4-flash`：Chat Completions，endpoint `/chat/completions`。
- 不再预置旧的 `deepseek-chat` 和 `deepseek-reasoner`。
- 官方依据：`https://api-docs.deepseek.com/api/create-chat-completion/`。

### OpenCode Go fixed template

- Template kind：`opencode_go`。
- Base URL：`https://opencode.ai/zen/go/v1`。
- Models endpoint：`https://opencode.ai/zen/go/v1/models`。
- Auth：API Key；所有请求发 Bearer；Anthropic Messages 适配器同时发 `x-api-key` 和 `anthropic-version: 2023-06-01`，不得记录值。
- Responses `/responses`：`grok-4.5`、`gpt-5.6-luna`。
- Chat Completions `/chat/completions`：`glm-5.3`、`glm-5.2`、`glm-5.1`、`kimi-k3`、`kimi-k2.7-code`、`kimi-k2.6`、`deepseek-v4-pro`、`deepseek-v4-flash`、`mimo-v2.5`、`mimo-v2.5-pro`、`hy3`。
- Anthropic Messages `/messages`：`minimax-m3`、`minimax-m2.7`、`minimax-m2.5`、`qwen3.8-max`、`qwen3.7-max`、`qwen3.7-plus`、`qwen3.6-plus`。
- `/models` 返回的模型列表可能变化；已知映射保留，远端不存在的模型标记 unavailable，不删除用户配置。
- 远端出现未知模型时创建 disabled catalog row，显示“需要选择协议”，不得默认调用。
- 官方依据：`https://dev.opencode.ai/docs/go/`，页面更新时间 2026-08-14。

### Mature project design reference

- Provider 与 Model 分离，模型携带 endpoint/protocol/capability，参考 `https://github.com/anomalyco/opencode/blob/dev/specs/v2/provider-model.md`。
- 自定义 Provider 必须显式声明 base URL、model 和协议；混合协议按 model 覆盖，参考 `https://github.com/anomalyco/opencode/blob/dev/packages/web/src/content/docs/providers.mdx`。
- WebView 清理使用 Tauri 2 的 `clear_all_browsing_data`，依据 `https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindow.html`。

## Environment

- Platform：Windows 11，PowerShell。
- Working location：`C:\Myfolder\MSL cowork\msl-desktop`。
- Frontend：Svelte 5、SvelteKit 2、TypeScript 5.6、Vite 6、pnpm。
- Backend：Tauri 2、Rust 2021、rusqlite 0.37、reqwest 0.12、tokio、keyring、notify。
- Existing scripts：`pnpm check`、`pnpm build`、`pnpm tauri build`、`cargo fmt --check`、`cargo test`。
- UI automation：Python websocket-client + WebView2 CDP，现有 `scripts/ui-smoke-cdp.py`。
- Permissions：工作区可写；不得假定管理员权限；不得要求真实 Provider 凭据。
- Network：获取新 Rust crate 可能需要网络；产品核心测试不依赖外部网络。
- Git：只允许只读 `git status`、`git diff`；禁止任何会改变索引、HEAD 或工作树的 Git 操作。

## Required Schema Shape

### Migration 0003 — AI provider catalog

- `provider_settings` 追加：`template_kind TEXT NOT NULL DEFAULT 'custom'`、`auth_mode TEXT NOT NULL DEFAULT 'bearer'`、`models_endpoint TEXT`、`last_models_refresh_at INTEGER`。
- 新表 `provider_models`：id、provider_id、model_id、display_name、protocol、endpoint_path、capabilities_json、source、enabled、available、created_at、updated_at；`UNIQUE(provider_id, model_id)`。
- 新表 `ai_task_routes`：task_kind 主键、provider_model_id nullable FK、updated_at。
- 旧 Provider 的 `model` 非空时迁入一条 `provider_models`；旧 Provider 默认 custom/chat_completions。

### Migration 0004 — document intelligence

- 新表 `work_workspace_links`：work_id、workspace_id、is_primary、created_at；联合主键。
- 新表 `document_index`：workspace_id、path、relative_path、extension、size、modified_at、content_hash、extract_status、char_count、cache_rel_path、summary、summary_hash、summary_model_id、last_extracted_at、last_analyzed_at、last_accessed_at、error_code、error_message；workspace+path 唯一。
- 新表 `cache_entries`：category、relative_path、content_hash、size_bytes、rebuildable、owner_type、owner_id、created_at、last_accessed_at、expires_at；relative_path 唯一。
- 数据库只保存 cache 相对路径，不保存完整提取正文。

### Migration 0005 — AI secretary

- 新表 `analysis_schedule_state`：单例 id=1、enabled、interval_minutes、daily_enabled、daily_hour、daily_minute、last_interval_run_at、last_daily_local_date、updated_at；默认 180 分钟与 06:00。
- 新表 `analysis_runs`：trigger、status、period_start、period_end、provider_model_id、started_at、finished_at、source_counts_json、snapshot_hash、summary、brief_id、error_code、error_message、created_at。
- 新表 `ai_proposals`：analysis_run_id nullable、kind、operation、target_id、work_id、workspace_id、dedupe_key、title、payload_json、reason、source_refs_json、confidence、user_edited、status、created_at、updated_at、decided_at。
- `daily_briefs` 追加：`retention_state TEXT NOT NULL DEFAULT 'kept'`、`analysis_run_id INTEGER`、`superseded_at INTEGER`；新自动 Brief 使用 draft，旧记录保留为 kept。

### Migration 0006 — storage governance

- 新表 `daily_activity_rollups`：rollup_date、workspace_id、work_id、event_type、event_count、first_at、last_at、sample_text、created_at；以日期、workspace、work、event_type 的 COALESCE 表达式建立唯一索引。
- 新表 `storage_cleanup_runs`：trigger、status、started_at、finished_at、bytes_before、bytes_after、deleted_counts_json、error_code、error_message。
- app settings 使用固定键：`cache_auto_enabled=true`、`cache_limit_bytes=2147483648`、`cache_high_water_percent=80`、`cache_target_percent=60`、`cache_last_auto_cleanup_at`。

## Fixed Task Kinds

- `workspace_analysis`
- `work_draft`
- `global_analysis`
- `daily_brief`
- `translation`
- `general`

数据库与前端只使用以上稳定值。新增其他 task kind 属于范围变化。

## Supported Documents

- DOCX：`.docx`；读取 `word/document.xml`、header/footer、footnotes/endnotes 中的 `w:t`、段落、tab、break，不执行宏或外部关系。
- PDF：`.pdf`；只提取文字层；提取后 trim 为空则 `needs_ocr`。
- 明确文本：`.txt`、`.md`、`.markdown`、`.csv`、`.json`、`.jsonl`、`.log`、`.yaml`、`.yml`、`.toml`、`.xml`、`.html`、`.htm`、`.sql`、`.py`、`.rs`、`.ts`、`.js`、`.svelte`。
- 无扩展名文件：小于 1 MiB、无 NUL 字节且可按支持编码解码时作为 text。
- 编码顺序：UTF-8 BOM、UTF-8、UTF-16LE/BE BOM、GBK；替换字符比例超过 1% 则 `failed_encoding`。
- 单文件读取上限：50 MiB；提取文本最多保留 2,000,000 Unicode characters，超过时记录 truncated=true 并保留开头和结尾，不标记失败。
- 分段：每段 6,000 characters，重叠 300 characters；缓存文件名使用内容 hash，不使用源文件名。
- 默认排除目录：`.git`、`node_modules`、`target`、`.svelte-kit`、`build`、`dist`、`.cache`、`.test-runtime`、`output`。
- 默认排除临时文件规则继续复用 `watcher::is_temp_file`。

## AI Output Contract

所有结构化分析必须返回单个 JSON object，不依赖 Markdown。顶层字段：

- `summary: string`
- `proposals: array`

每条 proposal 字段固定为：

- `kind`: `work|task|waiting|calendar|inbox|resume_point`
- `operation`: `create|update`
- `target_id`: integer 或 null
- `work_id`: integer 或 null
- `workspace_id`: integer 或 null
- `title`: string
- `payload`: object，仅允许目标实体已有字段
- `reason`: string
- `source_refs`: array，每项含 source_type、source_id/path、content_hash 可选
- `confidence`: 0 到 1

禁止模型输出 delete、archive、send 或 execute 操作。解析器必须拒绝未知 kind、未知字段、无来源的 update、不存在 target、非法时间和非法枚举。

## Storage Boundaries

- 正式数据：`%APPDATA%\MSLDesktop\msl-desktop.db`、用户配置、Keyring、确认建议、正式实体、kept Brief。
- 可重建缓存：`%LOCALAPPDATA%\MSLDesktop\cache` 下的 extracted/chunks/model-catalog/previews。
- 临时数据：`%LOCALAPPDATA%\MSLDesktop\tmp`。
- 日志：`%LOCALAPPDATA%\MSLDesktop\logs`，如果项目没有文件日志则该类别为 0，不得为完成清理功能而引入冗长正文日志。
- 源文件：绑定 workspace 下的用户文件，永不属于缓存。

## Assumptions

- Node、pnpm、Rust MSVC 工具链可用 -> STEP 01 机械验证；失败 -> `E05-TOOL-UNAVAILABLE`。
- 当前 schema 最高版本仍为 2 -> 每个 migration gate 在全新与 v2 临时库验证；不符 -> `E02-ENVIRONMENT-MISMATCH`。
- 新 Rust crate 可以由 Cargo 获取或缓存中存在 -> STEP 06 gate；失败 -> `E06-DEPENDENCY-MISSING`。
- Tauri 当前版本提供 `clear_all_browsing_data` -> 编译验证；API 不存在 -> `E02-ENVIRONMENT-MISMATCH`，不得改为删除 WebView 目录。
- 当前 CRUD、i18n 与 UI CDP 基础继续工作 -> 基线与回归 smoke 验证；基线失败 -> `E09-VALIDATION-FAILED`。
- 用户明确允许产品运行时把绑定目录正文发给选定 AI -> 不再加入授权确认；如果系统权限拒绝某个文件，只记录 skipped/error。
- 视觉主观成熟度仍需高能力审阅 -> 最终门前状态最多 `PARTIALLY_COMPLETED`。

## Boundaries

- 可以修改 `msl-desktop` 内源码、migration、测试、脚本和文档。
- 不得修改 `C:\Myfolder\MSL cowork` 下其他项目。
- 不得触碰正式数据库、真实工作目录或真实凭据来制造验收结果。
- 不得重新执行或覆盖上一阶段执行报告。
- 不得移除托盘、单实例、窗口销毁后 Core 常驻、通知、开机启动、现有 CRUD、i18n 或 local Brief fallback。
- 不得降低 SQLite 外键、WAL、busy timeout 或 migration 事务保障。
- 新代码要拆分模块；不得继续把全部新增命令堆入已经超过 50 KB 的 `commands/mod.rs`。

## Dependencies

- 保留现有前端依赖，不新增前端包。
- Rust 新增：`sha2`、`zip`、`quick-xml`、`pdf-extract`、`encoding_rs`、`chrono`；如 `tokio::sync` 被使用则给现有 tokio 增加 `sync` feature。
- Mock HTTP 使用 Rust 标准库 `TcpListener` 或项目内最小测试 helper，不新增重型 mock server 框架。
- 文档解析、AI 分析和缓存扫描使用 blocking task；不得阻塞 Tauri UI 主线程。

## Current State Snapshot

- Provider：单行单模型、Chat Completions only。
- Brief：事实完整但正文缺失，卡片位于 Dashboard 底部。
- Workspace：metadata baseline/reconcile/watcher 可用，正文未读取。
- Scheduler：提醒轮询可用，AI 调度不存在。
- Dashboard：功能区完整但 1024×640 明显滚动。
- Storage：正式 DB 路径明确，cache root、manifest、cleanup、rollup 不存在。
- Tests：前端静态检查和 34 个 Rust tests 通过；上一阶段 UI smoke 17/17 可复用扩展。
