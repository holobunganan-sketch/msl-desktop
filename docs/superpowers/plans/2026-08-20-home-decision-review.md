# Home Decision and Seven-Day Review Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans for inline execution. Do not create subagents for this project.

**Goal:** Build a clean homepage summary, latest-run decision card, compact editable secretary schedule, and seven-day AI review.

**Architecture:** Extend the existing proposal repository and confirmation boundary. Keep homepage editing shallow and route deeper edits to the existing review component. Reuse the current atomic apply layer for every confirmed write.

**Tech Stack:** Rust, rusqlite, Tauri 2, Svelte 5, TypeScript, CDP UI smoke.

**Spec:** `docs/superpowers/specs/2026-08-20-secretary-decisions-and-reports-design.md`

## Global Constraints

- Preserve the dirty worktree; do not reset, checkout, clean, stash, rebase, commit, or discard existing changes.
- Use isolated APPDATA, LOCALAPPDATA, TEMP and TMP for every running test.
- Use a local Mock Provider only.
- Every confirmed write must pass through `confirm_proposal`.
- Do not expose source document text in logs or reports.

---

### Task 1: Proposal query and editing contracts

**Files:**
- Modify: `src-tauri/src/db/ai.rs`
- Modify: `src-tauri/src/commands/ai_secretary.rs`
- Modify: `src-tauri/src/ai/apply.rs`
- Modify: `src-tauri/migrations/0007_decisions_reports.sql`
- Modify: `src-tauri/src/db/migrations.rs`

**Interfaces:**
- Produces: `ProposalRepo::list_latest_run_pending(limit)`, `ProposalRepo::list_since(cutoff,status,limit)`, `ProposalRepo::update_classification(...)`, `ProposalRepo::defer(...)`.

- [ ] Add failing repository tests proving latest-run isolation, seven-day filtering, classification edits, optimistic locking and defer-without-write.
- [ ] Run the targeted tests and confirm failures are caused by missing methods/schema.
- [ ] Add migration 0007 and implement repository methods with bound parameters and clamped limits.
- [ ] Add Tauri commands `list_latest_analysis_proposals`, `list_recent_ai_proposals`, `update_ai_proposal_classification`, and `defer_ai_proposal`.
- [ ] Extend confirmation validation so Task/Waiting may use `work_id=null`, while Calendar requires `start_at`.
- [ ] Run targeted Rust tests until green.

### Task 2: Clean summary and classification-aware AI prompt

**Files:**
- Modify: `src-tauri/src/ai/prompts.rs`
- Modify: `src-tauri/src/ai/analysis.rs`
- Modify: `src-tauri/src/ai/brief.rs`

**Interfaces:**
- Produces: summary without inline technical references; proposals with explicit long-project or temporary classification rationale.

- [ ] Add failing tests for clean summary normalization and proposal prompt requirements.
- [ ] Verify the tests fail against current inline source markers and missing classification rule.
- [ ] Implement source-marker removal for display summaries while retaining `source_refs` in structured evidence.
- [ ] Update GlobalAnalysis instructions to distinguish Work from Task/Waiting and recommend `work_id` or temporary scope.
- [ ] Run targeted tests until green.

### Task 3: Homepage decision card and schedule editor

**Files:**
- Modify: `src/lib/components/TodayView.svelte`
- Modify: `src/lib/services/api.ts`
- Modify: `src/lib/types/domain.ts`
- Modify: `src/lib/i18n/zh-CN.ts`
- Modify: `src/lib/i18n/en-US.ts`
- Modify: `scripts/layout-audit-cdp.py`

**Interfaces:**
- Consumes: latest-run proposal commands and `save_analysis_schedule`.
- Produces: `data-testid=latest-decision-card`, row destination/work selectors, confirm/defer/review buttons, `data-testid=dashboard-analysis-interval`.

- [ ] Extend the CDP audit with failing assertions for clean brief text, latest decision card, direct interval input and absence of recent-analysis card.
- [ ] Run against the current isolated build and record the expected failures.
- [ ] Replace the existing count-only decision card with up to five latest-run rows and destination-aware fields.
- [ ] Confirm rows through the existing atomic command; defer without entity writes; send deep edits to AI Review.
- [ ] Reduce the secretary card and add preset/custom interval controls.
- [ ] Remove the recent-analysis card and rebalance the desk grid.
- [ ] Run pnpm check and the focused CDP audit until green.

### Task 4: Seven-day AI review

**Files:**
- Modify: `src/lib/components/AiReviewCenter.svelte`
- Modify: `src/lib/services/api.ts`
- Modify: `src/lib/types/domain.ts`
- Modify: `src/lib/i18n/zh-CN.ts`
- Modify: `src/lib/i18n/en-US.ts`
- Modify: `scripts/release-functional-cdp.py`

**Interfaces:**
- Consumes: `list_recent_ai_proposals` and `update_ai_proposal_classification`.
- Produces: seven-day list, status/run filters, editable pending/deferred entries and read-only decided entries.

- [ ] Add failing release-functional assertions for seven-day filters and editable destination/work scope.
- [ ] Run against the current isolated release and record expected failures.
- [ ] Implement status tabs, analysis-run grouping and deep fields for kind/work_id/payload.
- [ ] Keep confirmed/rejected/superseded rows read-only with decision metadata.
- [ ] Run pnpm check, cargo tests and isolated CDP smoke until green.

### Task 5: Decision workflow regression gate

**Files:**
- Modify: `scripts/release-functional-cdp.py`
- Modify: `scripts/layout-audit-cdp.py`

- [ ] Seed synthetic latest and older proposals in the isolated database through Tauri commands.
- [ ] Verify homepage shows only latest-run rows and AI Review shows all synthetic seven-day rows.
- [ ] Confirm a temporary Task and a Work-linked Waiting item; verify persisted `work_id` values.
- [ ] Defer an Inbox-origin suggestion and verify no target entity is created.
- [ ] Run 1024×640, 1280×720 and 1440×900 layout checks across every page.

