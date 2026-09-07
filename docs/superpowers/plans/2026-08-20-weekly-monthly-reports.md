# Weekly and Monthly Reports Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans for inline execution. Do not create subagents for this project.

**Goal:** Add model-generated weekly and monthly reports with automatic schedules, custom periods, history and a dedicated sidebar page.

**Architecture:** Store reports and report scheduling in migration 0007. Build report snapshots from the existing bounded analysis snapshot; monthly snapshots embed overlapping weekly reports. Route each report kind through the provider catalog and persist explicit running/completed/failed states.

**Tech Stack:** Rust, rusqlite, chrono, Tauri 2, Svelte 5, TypeScript, local HTTP Mock Provider.

**Spec:** `docs/superpowers/specs/2026-08-20-secretary-decisions-and-reports-design.md`

## Global Constraints

- Preserve the dirty worktree; do not reset, checkout, clean, stash, rebase, commit, or discard existing changes.
- Use isolated APPDATA, LOCALAPPDATA, TEMP and TMP for every running test.
- Weekly and monthly report success requires an actual routed model response; no local fake report.
- Exclude AI translation inputs and outputs.
- Do not log document bodies, Provider responses, or credentials.

---

### Task 1: Report persistence and schedule

**Files:**
- Create: `src-tauri/src/db/reports.rs`
- Modify: `src-tauri/src/db/mod.rs`
- Modify: `src-tauri/migrations/0007_decisions_reports.sql`
- Modify: `src-tauri/src/db/migrations.rs`

**Interfaces:**
- Produces: `Report`, `ReportSchedule`, `ReportRepo`, `ReportScheduleRepo`.

- [ ] Add failing in-memory tests for report lifecycle, seven-day/custom periods, overlapping weekly lookup, schedule persistence and migration preservation.
- [ ] Run tests and confirm missing schema/module failures.
- [ ] Implement repository types and bound SQL queries.
- [ ] Register migration 0007 and verify idempotence plus v6-to-v7 preservation.
- [ ] Run targeted persistence tests until green.

### Task 2: Report snapshot and model contracts

**Files:**
- Create: `src-tauri/src/ai/reports.rs`
- Modify: `src-tauri/src/ai/mod.rs`
- Modify: `src-tauri/src/ai/prompts.rs`
- Modify: `src-tauri/src/ai/schema.rs`
- Modify: `src-tauri/src/db/provider.rs`

**Interfaces:**
- Produces: `ReportSnapshot`, `build_report_snapshot`, `build_report_request`, `normalize_weekly_numbering`.

- [ ] Add failing tests proving weekly snapshots cover the prior seven days, monthly snapshots include overlapping weekly reports, and translation data is absent.
- [ ] Add failing tests proving weekly output is a numbered list and monthly prompts demand deeper cross-project analysis.
- [ ] Implement bounded snapshots using `analysis_snapshot::build` and report excerpts.
- [ ] Add `weekly_report` and `monthly_report` to task routing validation.
- [ ] Run targeted AI tests until green.

### Task 3: Commands and background scheduling

**Files:**
- Create: `src-tauri/src/commands/reports.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/scheduler/mod.rs`
- Modify: `src-tauri/src/commands/ai_secretary.rs`

**Interfaces:**
- Produces Tauri commands: `list_reports`, `generate_report`, `retry_report`, `keep_report`, `get_report_schedule`, `save_report_schedule`.

- [ ] Add failing scheduler tests for Sunday 17:00, configurable weekly time, month-day 1 at 09:00, cross-period dedupe and manual runs.
- [ ] Add failing command-level tests that a missing route records failed and a provider response records completed.
- [ ] Refactor the existing secretary scheduler into a spawned low-frequency worker used by global analysis and reports.
- [ ] Implement report commands and persist attempt keys before automatic calls.
- [ ] Register commands and start the scheduler during app setup.
- [ ] Run scheduler and command tests until green.

### Task 4: Reports page and navigation

**Files:**
- Create: `src/lib/components/ReportsView.svelte`
- Modify: `src/routes/+page.svelte`
- Modify: `src/lib/services/api.ts`
- Modify: `src/lib/types/domain.ts`
- Modify: `src/lib/components/Icon.svelte`
- Modify: `src/lib/i18n/zh-CN.ts`
- Modify: `src/lib/i18n/en-US.ts`

**Interfaces:**
- Produces: `nav-reports`, report history, weekly/monthly generation buttons, custom range and schedule controls.

- [ ] Add failing CDP assertions for sidebar navigation and report controls.
- [ ] Run current isolated build and record expected failures.
- [ ] Build a responsive report page with history rail, report viewer, automatic schedule editor and custom date range.
- [ ] Show explicit running/failed/completed states and retry controls.
- [ ] Render weekly numbered lines and monthly structured text without unsafe HTML.
- [ ] Run pnpm check and focused UI smoke until green.

### Task 5: Mock AI, persistence and release gate

**Files:**
- Modify: `scripts/mock-ai-provider.py`
- Modify: `scripts/release-functional-cdp.py`
- Modify: `scripts/layout-audit-cdp.py`
- Modify: `docs/architecture.md`
- Modify: `docs/smoke-checklist.md`

- [ ] Extend the Mock Provider with deterministic weekly and monthly responses.
- [ ] Generate a weekly report for an exact seven-day period and verify numbered persisted content.
- [ ] Generate a monthly report and verify its snapshot references the synthetic weekly report.
- [ ] Restart the isolated app and verify report/schedule persistence.
- [ ] Verify cache cleanup preserves reports and report schedules.
- [ ] Run pnpm check, pnpm build, cargo fmt --check, cargo test, isolated UI/CDP, Mock AI, persistence and layout audits.
- [ ] Build the NSIS installer, install it silently without launching the app, and verify no test process remains.

