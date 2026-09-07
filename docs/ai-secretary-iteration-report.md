# AI 秘书与存储治理第二阶段报告

本报告只记录本阶段实际完成的实现与验证，不改写上一阶段历史。

## 已实现

- Provider：DeepSeek、OpenCode Go 固定模板；自定义 Provider 支持 Chat Completions、Responses、Anthropic Messages；模型目录与六类任务路由独立持久化。
- 文档：绑定工作目录后，对 DOCX、文本和 PDF 文本层做增量索引；缓存位于 LOCALAPPDATA，源正文不写入 SQLite；扫描版 PDF 明确标记 needs OCR。
- AI 秘书：AnalysisSnapshot 汇总 Work、Task、Waiting、Calendar、Inbox、Brief、Activity 与文档摘要，具备单文件/总量预算、source_ref 和 hash；结构化输出只允许创建/更新六类业务建议。
- 审阅：所有 AI 建议先进入可编辑 Review Center；确认前不写正式业务表，确认时事务性写入并记录 Activity；拒绝、过期草稿、重复建议和用户编辑保护均有持久化边界。
- 调度与 Brief：支持 30–1440 分钟周期、每日 06:00 和手动运行；Brief 支持 draft/kept/superseded 保留语义；无 Provider 时继续使用非空本地双语摘要。
- 首页与翻译：Brief 位于首页顶部；Dashboard 在目标中英文尺寸无页面溢出；翻译自动判断中英方向，支持书面/口语，输入输出仅保存在组件内存。
- 存储治理：使用量分项、清理预览、受保护对象、TTL/LRU 缓存清理、活动历史压缩、过期建议回收和官方 WebView 清理 API；源目录、正式实体、凭据、pending/confirmed 建议与 kept Brief 不在清理候选中。

## 验证结果

- `pnpm check`：0 errors / 0 warnings。
- `pnpm build`：通过。
- `cargo fmt --check`：通过。
- `cargo test`：66 passed / 0 failed。
- Dashboard CDP：中文/英文 × 1024×640/1440×900，4/4 无 document/body/dashboard 溢出。
- 隔离 UI smoke：17/17；release 功能 smoke：11/11；无 unexpected runtime error。
- 隔离 DB：version 6、integrity_check=ok、foreign_key_check=0；持久化重启后业务数据可读取。
- release 产物：
  - `src-tauri/target/release/msl-desktop.exe`，19,119,616 bytes，SHA-256 `1F2D89C38580C7A6671C75292282B716C92768D880FA9CEAFBABE19C46EC1B9D`。
  - `src-tauri/target/release/bundle/nsis/msl-desktop_0.1.0_x64-setup.exe`，5,018,631 bytes，SHA-256 `9D16C93F0833DA188A6EFA00D951D86BB73AD3E84534216F29D02004F3472080`。

## 隔离与安全

测试使用 `.test-runtime/luna-ai-secretary` 下的 APPDATA、LOCALAPPDATA、TEMP、TMP、workspace、artifacts 和 mock-provider；未调用真实 Provider 或真实 API Key。执行报告不记录工作文件正文、完整 Provider 响应或凭据值。正式 APPDATA 未启动测试实例，正式数据库只做元数据 gate 与副本迁移验证。

## 当前限制与后续门

- 调度 due 规则、持久化状态与手动 run command 已完成并有 fake-clock 测试；常驻 60 秒 tick 与完整“模型调用→建议入队”的后台编排仍应在后续阶段接入，当前不会伪造 AI 结果。
- 工作草稿入口会创建可追踪 analysis run 并检查 work_draft 路由；真实 Provider 调用需用户在设置中配置模型，测试继续使用本地 mock/adapter 单测。
- WebView 清理通过 Tauri 官方 API 已注册并受高级操作入口控制，未在自动化 smoke 中清空用户浏览数据。
- 最终视觉成熟度仍需用户或更高能力模型审阅；在该门明确通过前，执行报告保持 `PARTIALLY_COMPLETED`。

详细逐步 Action/Expected/Observed/Evidence/Verdict 见 `docs/luna-ai-secretary-iteration/EXECUTION_REPORT.md`。
