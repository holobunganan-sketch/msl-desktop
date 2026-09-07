# Collaborative Flow Implementation Plan

> 使用 executing-plans 在当前会话顺序实施，用户禁止委派与 Git 破坏性/历史操作。每项先 RED 后 GREEN，再更新执行报告。

**Goal:** 减少记录负担和重复请求，产出可读可核查报告，确认后的时间节点自然进入日历。
**Architecture:** 保留 Svelte/Tauri/SQLite 现有实体、后台队列和确认撤销；增加紧凑序列化/全局复用单行状态、报告契约、日历投影纯函数和阶段提示。
**Tech Stack:** Svelte 5、TypeScript、Rust、rusqlite、现有 Mock/CDP。
**Spec:** `docs/superpowers/specs/2026-09-06-collaborative-flow-design.md`。

## Global Constraints

工作目录只读；AI 建议确认后写入；不委派、不提交、不撤销用户改动；全部运行测试四目录与 WebView2 隔离、本地 Mock；不得读取或调用真实 Key。版本 0.2.1，视觉待审保持 PARTIALLY_COMPLETED。

## Task 1 无损压缩与周期复用

Files: `src-tauri/src/ai/efficiency.rs`、`analysis.rs`、`commands/ai_secretary.rs`、`db/migrations.rs`、`migrations/0012_ai_efficiency.sql`、`AnalysisScheduleSettings.svelte`。
Interfaces: `compact_json(&Value)->Value`；`reuse_fingerprint(&AnalysisSnapshot,&Value,i64)->String`；SQLite 单行统计接口；命中返回原成功 run ID，当前核查状态 reused。

- [x] 测试 compact_json 保留 0/false/数组 null/全部文本；相同时钟漂移可复用，跨节点/日期/提示/配置/状态不得命中；失败运行不得作为基线；建议仍属于原批次。
- [x] 运行 Rust 新测试观察预期 RED。
- [x] 实现非空保留递归序列化、指纹、成功基线/统计、周期接入；失败不推进基线，手动不复用。
- [x] 新测试 GREEN，前端添加复用说明和累计字符计数，重跑检查。

## Task 2 可读报告契约

Files: `src-tauri/src/ai/report_contract.rs`、`report-spec.md`、`reports.rs`、`commands/reports.rs`、`ReportsView.svelte`。
Interfaces: `validate_and_render(&ReportSnapshot,&str)->Result<String,String>`；生成和唯一修复都经过同一验证；正文存原 reports.content，质量依据放 source_counts_json。

- [x] 构造真实快照引用，测试正确报告渲染、错误 JSON/伪造引用/不明项目/技术字段/空条目失败；空证据报告不得编成果。
- [x] 测试 RED 后实现证据匹配、分类和长度检查、确定性编号渲染，独立 spec 指示成果/变化/阻碍/下一步。
- [x] 报告输入保留周期内变动和历史边界；最多一次修复，Mock 分别验证正常与错误分支。
- [x] 回顾页面改为正文主栏，设置/周期折叠、复制正文，保持清理和后台运行；测试 GREEN。

## Task 3 时间投影与协作指引

Files: `src/lib/services/calendarProjection.ts`、`CalendarView.svelte`、`proposalPresentation.ts`、`ProposalPreview.svelte`、`NaturalCapture.svelte`、`MattersView.svelte`、`ai/schema.rs`、`secretary-spec.md`。
Interfaces: `projectCalendar(tasks,waiting,start,end)` 返回原实体类型/ID、时间类型和可显示节点；`proposalPresentation` 返回 timeBasis/calendarHint。

- [x] Node 测试同任务排期/截止、等待跟进、重复时刻去重、范围和点击目标身份；Rust 测试推算时间缺理由和日期错误拒绝。
- [x] RED 后实现日历投影、原实体跳转和类型标识；预约与截止不混淆，确认前队列不出现在日历。
- [x] 建议说明“采用后自动进入日历”，推测标注；记录保存与整理回执准确；事项阶段呈现下一步指引，保留编辑深度。
- [x] 全部新增 Node/Rust GREEN。

## Task 4 集成与交付

- [x] `pnpm check`、`pnpm build`、Node tests、`cargo fmt --check`、`cargo test`。
- [x] 隔离本地 Mock 请求计数：第二次无变化周期零新增请求；修改/跨节点触发；手动重新分析；失败不复用。
- [x] 原生 UI 记录、建议确认、日历节点、撤销、报告修复与复制/折叠、后台切页；重启持久化。
- [x] 正式数据库已备份副本迁移、只读与缓存安全回归；新版本 release/NSIS。
- [x] 查看双尺寸和大字号截图，更新本轮报告的 Action/Expected/Observed/Evidence/Verdict；用户视觉认可前不标全完成。

执行状态：机械验收 PASS，实际截图已审查并提交；最终用户视觉认可尚未获得。整体按要求标记 PARTIALLY_COMPLETED，安装包 0.2.1 已生成但未替换正式安装。证据见 docs/iterations/2026-09-06-collaborative-flow.md。
