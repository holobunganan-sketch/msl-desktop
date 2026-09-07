# Agent Handoff: Codex Audit/Compiler → GPT-5.6 Luna

**Date**: 2026-08-14  
**Source Agent**: Codex project auditor and task compiler  
**Target Agent**: GPT-5.6 Luna  
**Phase Transition**: Diagnosis and architecture decisions → Implementation and verification  
**Feature**: MSL Desktop workbench reliability, Brief, Dashboard and bilingual rebuild  
**Work Type**: Refactor + Bug Fix + Feature

## Executive Summary

当前应用的 command 和基础 repository 大多存在，但产品闭环缺失：表单会静默返回、Quick Capture 会提前清空、各实体固定 `work_id=null`、页面之间不刷新、工作目录没有首次基线或离线差异、Brief 漏掉 Inbox 和多类任务、界面没有设计系统和 i18n。下一阶段必须严格按 `STEPS.md` 顺序实现，先可靠性和数据关联，再文件事实和 Brief，最后 Dashboard/双语与端到端验证。

## Completed Work

### Primary Deliverables

- [x] 源码结构、真实运行、隔离写入、正式数据库、watcher 和 Brief 链路审计。
- [x] 任务边界、禁止事项、架构决策和 25 个原子执行步骤。
- [x] 可计算 Acceptance 标准和执行报告模板。
- [x] delegation execution pack mechanical validation。

### Quality Gate Score

- Compiler validator：PASS，0 errors。
- 工程实现：尚未开始；由 Luna 执行。

## Critical Decisions Made

1. **可靠性优先于视觉**
   - Decision：交互反馈、后端校验、Work 关联和刷新必须先完成。
   - Rationale：当前问题不是单纯 CSS，而是信息没有进入统一上下文。
   - Impact：Luna 不得先做一轮换皮后宣布完成。

2. **文件只做元数据事实，不解析正文**
   - Decision：基线、离线差异和 watcher 只处理路径、mtime、size、事件类型。
   - Rationale：保持 local-first、性能与隐私边界。
   - Impact：Brief 可总结“哪些文件发生变化”，不能声称理解文件正文。

3. **Brief 必须有本地 fallback**
   - Decision：AI 只增强叙述；无 Provider/Key/网络时仍返回结构化摘要。
   - Rationale：核心工作台不能因 AI 配置而失效。
   - Impact：测试不需要真实 Key 或网络。

4. **无损追加 migration**
   - Decision：新增 version 2，不修改 version 1，不清理旧数据。
   - Rationale：正式数据库已有 Workspace、Provider、Inbox 和 Brief。
   - Impact：正式数据只允许在副本中验证迁移。

5. **Luna 不越过最终视觉门**
   - Decision：机械验收可由 Luna完成；视觉成熟度由用户或更高能力模型审阅截图。
   - Rationale：这是物质性主观判断。
   - Impact：未审阅前报告状态最多为 `PARTIALLY_COMPLETED`。

## Context for Next Agent

### Must Read

- [ ] `docs/luna-workbench-rebuild/START_HERE.md`
- [ ] `docs/luna-workbench-rebuild/TASK.md`
- [ ] `docs/luna-workbench-rebuild/CONTEXT.md`
- [ ] `docs/luna-workbench-rebuild/STEPS.md`
- [ ] `docs/luna-workbench-rebuild/ACCEPTANCE.md`
- [ ] `docs/luna-workbench-rebuild/EXECUTION_REPORT.md`

### Key Insights

- 干净环境真实页面点击能创建主要实体，因此不能把问题误判为 command 全部缺失。
- 正式数据库审计时 Activity 为 0，且目录绑定后没有新文件修改；现有代码不会回顾绑定前文件。
- 正式 Inbox 有记录，但 BriefSnapshot 没有 Inbox 字段，这是 Brief 空洞的直接证据。
- 现有 smoke 主要直接 invoke 后端，没有覆盖 UI；当前 WebView2 还会拒绝带 Origin 的 CDP websocket。
- Provider 凭据只按整数 ID 命名，隔离数据库的 provider id 1 曾被误识别为已有 Key，必须修复。

### Assumptions Made

- Windows/pnpm/Rust 环境仍可用，由 STEP 01 验证。
- 用户希望保留全部现有数据，由 migration 副本测试验证。
- 不引入大型 UI/i18n/state 依赖足以完成目标；如果证据推翻该假设必须停止。

## Technical Specifications

### Architecture Decisions

- Frontend：Svelte store + typed API + domain revisions + shared UI components + local typed i18n dictionaries。
- Backend：command 层校验、repository SQL、migration v2、workspace inventory/reconcile、typed BriefSnapshot。
- Brief：deterministic local renderer 为基础，OpenAI-compatible Provider 为可选增强。
- UI：CSS design tokens、固定应用壳、响应式 12 列 Dashboard、无大型 UI 框架。

### Data Models

- `provider_settings.credential_ref`：旧行 `provider-{id}`，新行 `provider-{uuid-v4}`。
- `workspace_file_state`：workspace+path 主键，保存 mtime/size/seen_at。
- `daily_briefs` 新增 period、locale、snapshot JSON、ai_used、warning。

### API Contracts

- `workspace_sync_status`：watcher、baseline、last scan、warning。
- `preview_brief_snapshot`：只读、不调用 AI、不写库。
- `generate_brief`：返回 BriefResult，失败时 local fallback。
- `list_briefs` / `get_brief`：历史读取。
- 旧 `generate_morning_brief` 保留兼容 wrapper。

## Dependencies & Blockers

### Dependencies Required

- [x] Svelte 5 / SvelteKit / Tauri 2 / rusqlite / notify 已存在。
- [x] Python websocket-client 在审计环境中可导入。
- [ ] Rust uuid v4 需在 STEP 07 添加并由 Cargo 验证。

### Known Blockers

- NONE at handoff。执行过程中按 stop codes 处理环境变化。

## Security & Privacy

- 禁止测试写入正式 APPDATA。
- 禁止读取或输出真实 Key。
- 禁止把文件正文发送给 AI。
- 禁止使用真实 Provider 做测试。
- 正式数据库只复制后在隔离副本做 migration 验证。

## Testing Requirements

### Test Cases Needed

- [ ] 后端非法输入和 Work 外键校验。
- [ ] migration v1→v2、幂等、旧行兼容。
- [ ] Provider credential_ref 跨数据库隔离。
- [ ] workspace 首次 baseline、离线增删改、重复 reconcile。
- [ ] Brief 全来源、范围、缓存、双语、local fallback、mock AI。
- [ ] UI 表单、同页刷新、Work 关联、语言持久化、Dashboard、Calendar、Brief。

### Acceptance Criteria

- [ ] `ACCEPTANCE.md` 全部机械项 PASS。
- [ ] 最终视觉门由用户或更高能力模型明确通过。

## Questions for Target Agent

- 无需在开始前提问。只有触发 stop code 时才向用户报告具体证据和所需决定。

## Files Created/Modified

| File | Action | Purpose |
|---|---|---|
| `docs/luna-workbench-rebuild/START_HERE.md` | CREATED | Luna 执行入口 |
| `docs/luna-workbench-rebuild/TASK.md` | CREATED | 目标、范围和架构决定 |
| `docs/luna-workbench-rebuild/CONTEXT.md` | CREATED | 已验证项目上下文 |
| `docs/luna-workbench-rebuild/STEPS.md` | CREATED | 25 个原子实施步骤 |
| `docs/luna-workbench-rebuild/ACCEPTANCE.md` | CREATED | 可计算验收标准 |
| `docs/luna-workbench-rebuild/EXECUTION_REPORT.md` | CREATED | Luna 逐步填写证据 |
| `docs/luna-workbench-rebuild/HANDOFF.md` | CREATED | 本交接文档 |

## Next Steps

1. Luna 完整阅读执行包并在报告中确认。
2. 从 STEP 01 开始执行，不跳步。
3. 每步记录证据；PASS 自动继续，STOP 立即停止。
4. STEP 25 提交截图和机械验收，等待视觉门。

## Validation Checklist

- [x] 目标和交付物完整。
- [x] 关键决定已锁定。
- [x] 现状与证据已传递。
- [x] 安全边界和停止代码明确。
- [x] 测试与验收可计算。
- [x] 下一步无歧义。

**Target Agent**：开始工作前必须确认已经完整阅读上述文件；不得只读取本页摘要后直接改代码。

