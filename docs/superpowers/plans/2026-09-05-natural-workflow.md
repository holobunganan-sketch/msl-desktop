# Natural Workflow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans inline. User prohibits delegation and Git mutations. Steps use checkbox syntax.

**Goal:** 用记录、决定、推进的日常流程重构界面，保留全部已有数据与安全机制。

**Architecture:** 统一导航与建议预览为前端服务，共用原有实体编辑器及确认后端。新增事项聚合容器，不迁移任务/等待实体。结构化 capture 上下文和 task 时间安排由后端维护，日历读取同一任务。

**Tech Stack:** Svelte 5 / TypeScript / Tauri 2 / Rust / SQLite / Node tests / isolated CDP.

**Spec:** docs/superpowers/specs/2026-09-05-natural-workflow-design.md

## Global Constraints

- 保留 SQLite 正式实体与现有确认、撤销、分类记忆、后台任务服务。
- AI 建议确认前只写队列。工作目录只读。
- 全部运行与写入测试隔离 APPDATA、LOCALAPPDATA、TEMP、TMP 和 WebView2 目录，仅本地 Mock AI。
- 禁止读取或调用真实凭据。不提交、不丢弃现有修改。

## Task 1 — navigation and readable decisions

Files: create src/lib/services/navigation.ts, proposalPresentation.ts; tests/naturalWorkflow.test.mjs; modify +page.svelte and target components.

Interfaces: `resolveDestination(kind:string,id?:number,workId?:number|null)` returns `{view,section?,id?,workId?}`; `navigateTo(target)` emits dashboard:navigate with object; `proposalPresentation(item,works,locale)` returns action/scope/time/needsAttention.

- [x] Write failing literal fixture tests: work ID retained, task enters matters/active, review enters matters/review, progress opens parent project, unknown destination rejected; proposal update differs from create and missing calendar time requires adjustment.
- [x] Run `node --experimental-strip-types --test tests/naturalWorkflow.test.mjs`, verify feature absent failures.
- [x] Implement typed navigation normalization plus presentation derived from existing proposal payload; wire one shell destination state and view props.
- [x] Rerun tests; verify old string events still resolve, unknown routes ignored.

## Task 2 — workflow navigation and capture

Files: +page.svelte, new MattersView.svelte, QuickCapture.svelte, api.ts; backend inbox capture context, analysis_snapshot.rs; migrations only if structured context needs storage.

- [x] Add isolated UI assertions for five primary routes and a quick capture saved once, project context preserved, no formal writes before confirmation.
- [x] Run against prior release and observe absent workflow route/behavior.
- [x] Add matter sections with existing Inbox/Plan/Waiting/Review views. Preserve legacy view events through normalized routes. Pass focus IDs and show completed filter explicitly.
- [x] Add capture action with busy lock, optional immediate AI organize, input retention on failure and persistent scoped evidence.
- [x] Run check, Rust capture tests and CDP assertions.

## Task 3 — Today and decisions

Files: TodayView.svelte, AiReviewCenter.svelte, new ProposalPreview.svelte, app.css.

- [x] Add CDP assertions: no visible route select until adjust, old pending suggestion remains accessible, adopt yields exact destination and undo; confirm invalid/missing date blocked.
- [x] Implement shared preview; default review preview plus expandable detailed form. Keep saved draft concurrency handling and transactional confirmations. Use readable action labels.
- [x] Compact home brief; remove metric strip; two main cards; collapse diagnostics; remove nested list max heights. Link exact items.
- [x] Run Node tests and layout/confirmation checks.

## Task 4 — project-first progress and continuity

Files: WorksView.svelte, InboxView.svelte, PlanView.svelte, WaitingView.svelte, CalendarView.svelte; commands/task schedule.

- [x] Add backend schedule tests: same task id, notes/project retained, clear schedule, invalid ranges rejected; add UI project deep link and scoped capture assertions.
- [x] Implement schedule command using TaskRepo updates and activity log; no calendar INSERT. Calendar reads scheduled tasks alongside independent events and opens original task.
- [x] Reorder project view around goal/current/next/blockers; natural progress capture; collapse folders/history. Add exact item navigation and explicit receipts.
- [x] Verify task reparent/completed state persistence and project source directories unchanged.

## Task 5 — full isolated verification and release

Files: scripts/natural-workflow-cdp.py, docs/iterations/2026-09-05-natural-workflow.md; version metadata if release succeeds.

- [x] Run `pnpm check`, Node tests, `pnpm build`, `cargo fmt --check`, `cargo test` with isolated write locations.
- [x] Exercise complete Mock capture→suggestion→adjust/adopt→project→schedule→calendar→completion→progress cycle; switch pages during AI; reopen DB and verify persistence/integrity.
- [x] Run source-readonly/cache safety tests and schema migration against an isolated formal DB backup, without reading row contents or credentials.
- [x] Build `pnpm tauri build`, capture Chinese/English UI at 1440×1000 and 1100×720 including larger font. Inspect screenshots and record gaps honestly; do not install without current user installation request.

## Execution record

State: PARTIALLY_COMPLETED — all implementation and mechanical verification steps passed; final visual approval is pending. Action/Expected/Observed/Evidence/Verdict are recorded in docs/iterations/2026-09-05-natural-workflow.md.
