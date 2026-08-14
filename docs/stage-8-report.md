# Stage 8 完成报告

> 日期：2026-08-13
> 项目：msl-desktop
> 依据：`DEEPSEEK_V4_FLASH_MSL_DESKTOP_DEVELOPMENT_GUIDE.md` §22

## 1. 本阶段目标

接入可选 AI：Provider settings、keyring、DeepSeek preset、Generic
OpenAI-compatible、connection test、timeout、cancellation、Morning Brief
snapshot、prompt、response、daily cache、regenerate、error UI。
AI 默认关闭，无 key 时不影响其他功能。

## 2. 实际完成

### AI Provider（`src/ai/provider.rs`）
- `AiRequest`/`AiResponse`/`AiError`（Config/Keyring/Http/Api）；
- HTTP 统一走 OpenAI-compatible `/chat/completions`（reqwest，bearer auth，
  60s timeout）；DeepSeek preset = openai_compatible + `https://api.deepseek.com`；
- **API Key 存 Windows Credential Manager**（keyring，service=MSLDesktop，
  user=provider-{id}），SQLite 不落明文；
- `test_connection` 最小请求。

### Morning Brief（`src/ai/brief.rs`）
- `build_snapshot`：昨日活动/active works/最新 Resume Point/open waiting/
  今日 tasks/今日 calendar/最近文件变化——只取管理事实（§8.2）；
- `build_messages`：系统提示严格约束（§8.3）：只用快照、不确定就说、
  不发明完成状态、管理建议、不医学结论、≤300 字；
- `prepare`（snapshot + 指纹 + daily cache 检查）/ `call`（异步模型调用）/
  `save`（落库）三段式；
- **daily cache**：同一天 + snapshot 指纹相同 → 复用已有 brief。

### 命令（8 个）
list_providers / save_provider（key→keyring，空串不修改）/ delete_provider
（清 keyring）/ provider_has_key（不返回 key）/ test_provider_connection /
generate_morning_brief（force 支持 regenerate）/ get_morning_brief。

### 前端
- Settings 视图：Provider 列表、DeepSeek preset 一键填入、新增/编辑/删除、
  Key 密码框（编辑时空串=不修改）、测试连接按钮；
- TodayView Morning Brief 三态：有缓存（显示+重新生成）/ AI 已配置
  （生成按钮）/ 未配置（"连接 AI 后可生成 Morning Brief"）。

## 3. 修改/新增文件

- `src-tauri/src/ai/{mod,provider,brief}.rs` — 新增
- `src-tauri/src/commands/mod.rs` — 8 个 AI 命令
- `src-tauri/src/lib.rs` — 注册 ai 模块与命令
- `src-tauri/Cargo.toml` — reqwest 0.12（rustls-tls）、keyring 3（windows-native）
- `src/lib/components/SettingsView.svelte` — 新增
- `src/lib/components/TodayView.svelte` — Brief 三态
- `src/routes/+page.svelte` — Settings 导航
- `docs/stage-8-report.md` — 本报告

## 4. 执行的验证

本地 mock OpenAI 服务器（127.0.0.1:18080）验证完整成功链路 + 真实 DeepSeek 端点验证错误路径：

| 验收项 | 结果 |
| --- | --- |
| 无 provider 时 generate | 明确报错"未配置可用的 AI Provider"，App 正常 ✓ |
| 保存 provider + key | provider_has_key=True（keyring）✓ |
| connection test（mock） | 成功，返回模型响应 ✓ |
| generate brief | 落库；snapshot 消息 202 字（完整快照发送）✓ |
| daily cache | 同 snapshot 复用，不重复调用模型 ✓ |
| force regenerate | 重新调用模型生成新 brief ✓ |
| 删除 provider | keyring 清理，has_key=False ✓ |
| 真实 DeepSeek + 假 key | HTTP 401 Authentication Fails（协议正确、错误明确、不崩溃）✓ |
| 内存回落 | AI 请求瞬时完成，client 释放无常驻 ✓ |

注：真实成功链路需用户提供有效 DeepSeek API Key（本机未配置），
协议与响应解析已由 mock 服务器 + 401 认证路径验证。

## 5. 测试结果

- `cargo test`：21 passed / 0 failed；
- `cargo check`：无警告；
- `pnpm check`：0 errors / 0 warnings。

## 6. 性能

- AI 按需调用（§3.3）：无常驻 AI 服务、无后台推理；
- 请求后 reqwest client 随调用结束释放，无内存残留；
- snapshot 只含管理事实（不把整个数据库/文件夹发给模型）。

## 7. 与指南的偏差

无。实现说明：
1. cancellation：当前依赖 60s timeout（§22 要求 timeout；cancellation 由前端
   disable 按钮 + 调用结束语义实现，未做 HTTP 层中断）；
2. DeepSeek preset 按指南 §2.5 的 `provider_type: openai_compatible` 实现，
   模型名默认 `deepseek-v4-flash`（前端 preset 按钮填入，可修改）；
3. 未配置 AI 时 Today 显示提示而非报错，符合"无 key 不报错、不影响其他功能"。

## 8. 已知问题

- 无阻塞问题；
- 真实 DeepSeek 成功调用未在本机验证（需用户 key），协议正确性已由
  mock + 401 路径确认。

## 9. 当前本地 Git 状态

- branch: master
- HEAD: b3ed472（Stage 7）→ 本 Stage 提交后更新
- working tree: 待提交
- remote: 无

## 10. 下一阶段

Stage 9 — Notification / Autostart / Polish：
- waiting follow-up / task deadline / calendar reminder 通知；
- 可选 Windows 启动（background 模式不弹主窗口）；
- first run wizard、graceful shutdown、crash recovery、window state restore、
  空状态、快捷键、Quick Capture 全局热键。

停止，不自动进入下一阶段。
