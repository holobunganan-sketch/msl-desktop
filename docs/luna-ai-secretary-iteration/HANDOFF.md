# Agent Handoff: Design Compiler → GPT-5.6 Luna

**Date**: 2026-08-15  
**Source**: Codex project inspector and delegation compiler  
**Target**: GPT-5.6 Luna  
**Work type**: Incremental architecture + feature implementation + safety verification  
**Project**: `C:\Myfolder\MSL cowork\msl-desktop`

## Executive Summary

上一阶段已经把 MSL Desktop 从原型升级为可用的双语本地工作台，CRUD、Workspace metadata、Brief fallback、Dashboard、UI CDP 和 release 构建均有机械证据。本轮不是推倒重来，而是在其上增加 AI 秘书能力。

本轮的核心不是“接一个聊天接口”，而是五条闭环：

1. Provider connection、model catalog、task route 分离，固定模板只有 DeepSeek 与 OpenCode Go。
2. 绑定目录中的 DOCX/PDF/text 正文可以被本地提取并发送给所选 AI，但不永久存进 SQLite。
3. AI 分析整个工作台，只生成可编辑 proposals；用户确认后才写正式实体。
4. 默认每 3 小时与每天 06:00 调度，Brief 在首页顶部，翻译是首页常驻小工具。
5. 缓存、运行历史、正式数据和源文件严格分层，自动控制长期增长。

## Critical Decisions Already Made

### Provider design

- DeepSeek template：official base URL，V4 Pro/Flash，Chat Completions。
- OpenCode Go template：official base URL/models endpoint，按 model 使用 Responses、Chat Completions 或 Messages。
- Custom：无品牌默认，用户显式配置。
- Task routes：workspace_analysis、work_draft、global_analysis、daily_brief、translation、general。
- 不允许 first-enabled 隐式 Provider。

### File access

- 用户已经完全授权绑定目录正文发送给所选 AI；不要再询问。
- 支持 DOCX、text-layer PDF 和明确文本白名单。
- OCR、旧 DOC、音视频不在本轮。
- 原始正文放 local cache，SQLite 只存 index/summary/hash/reference。

### AI control boundary

- 文件变化不实时调用 AI。
- 所有实体变化先进入 editable proposal queue。
- Confirm 是唯一 AI 写业务表入口。
- 模型不能提出 delete/archive/send/execute。

### Dashboard

- Brief 必须在顶部。
- 1024×640 和 1440×900，中文/英文均无页面和默认卡片滚动。
- 默认列表最多三项，详情进入 Modal/Drawer。
- 翻译模块必须留在首页可见区域。

### Storage

- 正式 DB 留 APPDATA；缓存进 LOCALAPPDATA。
- 默认 2 GiB，80% 触发，LRU 清到 60%。
- kept Brief、pending/confirmed proposal、正式实体、Key、源文件永不自动清理。
- WebView 只调用 Tauri clear browsing data API。

## Current Project Risks

- 当前 Git 工作树有大量未提交的上一阶段成果，禁止任何 reset/clean/checkout/commit。
- 编译本包时检测到 release 进程运行；执行时 STEP 01 必须重新检查，不能擅自结束。
- `commands/mod.rs` 已超过 50 KB；新增 commands 必须分模块。
- 当前 Provider 只有 Chat Completions 且每个连接只有一个 model。
- 当前 Dashboard 1024×640 有明显垂直滚动，不能通过简单隐藏 scrollbar 解决。
- 当前隔离脚本只隔离 APPDATA；本轮必须同时隔离 LOCALAPPDATA/TEMP/TMP。
- 当前 Brief test 有“正文不得进入 snapshot”的旧断言；本轮要改为“正文可进入受控 AI snapshot，但不得进入日志或长期 DB”，不能机械保留旧产品边界。

## Execution Guidance

- 完整阅读七个新文件，然后读上一阶段报告和列出的源码。
- 从 STEP 01 顺序执行，每步即时写报告。
- 先完成 Provider 基础，再正文，再 AI proposals，再调度/UI，再存储；不得调换顺序。
- 使用 synthetic files、synthetic key、local mock server；所有真实数据保持只读边界。
- 每个 migration 先临时库 gate 和测试，再进入下一阶段。
- 所有结构化 AI 输出必须经过 typed schema 和 source validation。
- 不允许用“模型应该会遵守”代替应用层校验。
- 不允许用“缓存可重建”作为删除未登记文件或正式数据的理由。

## Expected Deliverables

- Migrations 0003–0006 and repositories.
- Provider catalog/adapters/router and Settings UI.
- Document extract/index/cache pipeline and Workspace/Work linkage.
- Analysis engine, proposal queue, apply transaction and review UI.
- Scheduler, top Brief, translation and no-scroll Dashboard.
- Storage usage/preview/cleanup/rollup/WebView cleanup and Settings UI.
- Full tests, isolated UI CDP, persistence/cache safety, release build and screenshots.
- Updated README/architecture/smoke checklist/iteration report/execution report.

## High-Model Gates

- If official Provider behavior observed through mock-contract implementation contradicts the locked official snapshot in a material way, stop with evidence; Luna does not choose a new architecture.
- Final visual maturity and secretary-like product experience require user or higher-model review.

## Completion Rule

Mechanical acceptance can reach `PARTIALLY_COMPLETED`. Only explicit approval of final visual/product gate permits `COMPLETED`.
