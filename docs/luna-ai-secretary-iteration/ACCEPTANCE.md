# Acceptance

## Acceptance Criteria

### A. Preservation and isolation

- [ ] Current Git changes were preserved; no reset/checkout/clean/stash/commit/rebase occurred.
- [ ] All application launches used isolated APPDATA, LOCALAPPDATA, TEMP and TMP.
- [ ] Formal `%APPDATA%\MSLDesktop` DB/WAL/SHM metadata remained unchanged from STEP 01 through STEP 35.
- [ ] No real API Key, Authorization value, x-api-key value, real work body or full Provider response was printed.
- [ ] No test called a real DeepSeek, OpenCode Go or custom Provider.
- [ ] Existing Work/Task/Waiting/Calendar/Inbox CRUD, bilingual UI, tray lifecycle and local Brief fallback still work.

### B. Provider catalog and protocols

- [ ] Migration 0003 upgrades v2 without losing old Provider rows or credential_ref.
- [ ] Provider connection, provider model and AI task route are separate persisted entities.
- [ ] DeepSeek fixed template pre-fills `https://api.deepseek.com`, Bearer, V4 Pro/Flash and Chat Completions.
- [ ] OpenCode Go fixed template pre-fills `https://opencode.ai/zen/go/v1` and `/models`.
- [ ] OpenCode Go Responses, Chat Completions and Messages model mappings match `CONTEXT.md`.
- [ ] Remote model refresh updates availability without deleting saved models/routes.
- [ ] Unknown discovered models are disabled and require explicit protocol selection.
- [ ] Custom Provider form has no DeepSeek/OpenCode branded defaults and requires explicit protocol/endpoint.
- [ ] API Key remains in Windows Credential Manager and is never returned to UI.
- [ ] Chat Completions mock success/error/parse/redaction tests pass.
- [ ] Responses mock success/error/parse/redaction tests pass.
- [ ] Anthropic Messages mock success/error/parse/redaction tests pass.
- [ ] Connection test uses the selected model protocol, not legacy `provider_settings.model`.
- [ ] Six fixed task routes can each select a different Provider model.
- [ ] Exact task route precedes explicitly configured general route.
- [ ] Missing/disabled/unavailable/no-key routes report a visible error and never silently choose first enabled Provider.

### C. Document intelligence

- [ ] Migration 0004 is additive and contains no long-term full_content/raw_body/prompt_body column.
- [ ] Formal DB remains under APPDATA; extracted/chunks/model catalog caches use LOCALAPPDATA cache root.
- [ ] Cache relative-path traversal, absolute path and symlink/reparse attempts are rejected.
- [ ] DOCX paragraphs, tables and configured Word XML parts extract from synthetic fixture.
- [ ] Text-layer PDF extracts successfully.
- [ ] Image-only PDF reports `needs_ocr` rather than ready/empty success.
- [ ] UTF-8, UTF-16LE/BE and GBK text fixtures decode; invalid binary reports failure/unsupported.
- [ ] Files over 50 MiB report `too_large` without unbounded allocation.
- [ ] Extracted text over 2,000,000 characters follows the fixed truncation rule.
- [ ] Chunks use 6,000 characters and 300 overlap with source hash/index.
- [ ] Unchanged hash does not trigger re-extraction.
- [ ] Watcher/reconcile marks document dirty but does not call AI immediately.
- [ ] Source deletion removes index state/orphans cache without deleting any other source file.
- [ ] Workspace UI shows ready/unsupported/failed/needs OCR/too large counts and per-file reason.
- [ ] Work and Workspace can be linked/unlinked; unlink never deletes Work, Workspace or source files.

### D. AI analysis and proposal control

- [ ] Migration 0005 creates default 180-minute and 06:00 schedule and preserves old Brief as kept.
- [ ] AnalysisSnapshot contains all workbench domains, document summaries, selected chunks and source_counts.
- [ ] Per-file 40,000 and total 120,000 character budgets are enforced with truncated counts.
- [ ] Snapshot/logs contain no Key or cache absolute path.
- [ ] Only one analysis can run at a time; all failure paths release the guard.
- [ ] Structured output rejects unknown kind/operation/field, fake source, illegal enum/time and missing update target.
- [ ] AI can only propose create/update for work/task/waiting/calendar/inbox/resume_point.
- [ ] No delete/archive/complete/resolve/send/execute proposal is accepted.
- [ ] Repeated equivalent analyses do not create unbounded duplicate pending proposals.
- [ ] A user-edited pending proposal is not overwritten by later automatic analysis.
- [ ] Before confirmation, target business table counts remain unchanged.
- [ ] Confirmation revalidates edited payload and uses an atomic transaction.
- [ ] Confirmation writes the correct entity, marks proposal confirmed and creates an Activity record.
- [ ] Rejection changes only proposal/audit state and does not write target entities.
- [ ] Stale updated_at prevents accidental overwrite.
- [ ] Review Center supports previous/next, editable fields, source references, confirm, reject and later.
- [ ] Pending proposals survive window close and process restart.

### E. Workspace-to-Work secretary flow

- [ ] Binding a workspace performs baseline/index only; no Work is automatically inserted.
- [ ] User can click “分析目录并生成工作草稿”.
- [ ] Missing work_draft model shows configuration guidance rather than a dead button.
- [ ] New workspace run creates at most the fixed proposal limits and puts Work first.
- [ ] Work proposal is editable in ordinary fields, not raw JSON.
- [ ] Confirming Work creates primary work_workspace_link and binds later proposals.
- [ ] Rejecting Work does not auto-delete or auto-confirm later proposals.
- [ ] Re-analysis of a workspace already linked to Work does not create a duplicate Work proposal.
- [ ] Confirmed downstream proposals appear in Task/Waiting/Calendar/Inbox/Resume and their Work detail.

### F. Scheduling, Brief and translation

- [ ] Scheduler tick is 60 seconds and uses injected clock tests, not long sleep.
- [ ] Interval setting range 30–1440 minutes; default is 180.
- [ ] Daily default is local 06:00 and persists across restart.
- [ ] Daily and interval due at the same tick coalesce into one daily run.
- [ ] Busy scheduler does not start a second run or advance last state.
- [ ] Failure does not retry every minute; manual retry remains available.
- [ ] Daily Brief uses workbench facts, file changes, document summaries and proposal counts.
- [ ] AI Brief has period progress, risks/waiting, today hard schedule and 1–5 suggested advances.
- [ ] No Provider/Key/network still returns non-empty local bilingual Brief.
- [ ] Automatic Brief is draft; newer same-day draft supersedes only old draft.
- [ ] User-kept Brief is never superseded or automatically deleted.
- [ ] Dashboard top Brief shows maximum 3 advances and 2 risks; full content/source is reachable.
- [ ] Translation auto-detects Chinese/English direction according to fixed rule.
- [ ] Written and spoken styles produce distinct mock prompts.
- [ ] Empty and over-20,000-character translation is rejected before model call.
- [ ] Translation source/result is not written to DB, cache, Activity or logs.
- [ ] Translation failure preserves source text and points to model settings.

### G. Dashboard usability

- [ ] Brief is the first Dashboard content block below the app topbar.
- [ ] Quick Capture is compact in topbar and remains functional.
- [ ] Five required metrics are visible and clickable.
- [ ] Today/continue, Waiting/Inbox/files, AI secretary/translation columns are all visible.
- [ ] Default lists show at most 3 rows and an explicit remaining count.
- [ ] No Dashboard default card has an internal scrollbar.
- [ ] Detail Modal/Drawer makes truncated content reachable and keyboard accessible.
- [ ] Chinese 1024×640 has no body/document/Dashboard horizontal or vertical overflow.
- [ ] English 1024×640 has no body/document/Dashboard horizontal or vertical overflow.
- [ ] Chinese 1440×900 has no body/document/Dashboard horizontal or vertical overflow.
- [ ] English 1440×900 has no body/document/Dashboard horizontal or vertical overflow.
- [ ] All required module bounding rectangles lie within viewport and no action is clipped by overflow hidden.

### H. Storage governance

- [ ] Migration 0006 preserves old data and does not overwrite existing cache settings.
- [ ] Storage usage separates formal DB, extracted, chunks, model catalog, previews, temp, logs and history.
- [ ] Source workspace size is never reported as deletable application cache.
- [ ] Default total cache cap is 2 GiB; high water 80%; LRU target 60%.
- [ ] Smart preview reports estimated bytes/count, protected objects and warnings before deletion.
- [ ] Smart cleanup applies fixed TTL rules and uses last_accessed LRU only above high water.
- [ ] Files touched after preview cutoff are skipped.
- [ ] Analysis/index/cleanup mutual exclusion prevents deleting in-use cache.
- [ ] Cache deletion changes document cache state to pending while preserving summary/hash.
- [ ] Cleanup audit records counts/bytes/error without source filenames or bodies.
- [ ] Automatic cleanup runs at most once per 24 hours.
- [ ] Activity older than 90 days is rolled up before raw deletion; total event counts reconcile.
- [ ] Repeated Activity compaction is idempotent.
- [ ] Superseded Brief draft and rejected/superseded proposal TTL apply only to eligible rows.
- [ ] Pending/confirmed proposals and kept Briefs survive Smart and Advanced cleanup.
- [ ] WebView browsing data is cleared only via Tauri API, never by deleting its data directory.
- [ ] Optional VACUUM runs only after explicit advanced confirmation and integrity_check remains ok.
- [ ] Source fixture hashes, formal entity counts and synthetic credential presence are unchanged after cleanup.

### I. Engineering and release

- [ ] `pnpm check` exits 0 with 0 errors and 0 warnings.
- [ ] `pnpm build` exits 0.
- [ ] `cargo fmt --check` exits 0.
- [ ] Full `cargo test` exits 0 with no failed test.
- [ ] Expanded debug UI CDP smoke passes all flows with zero unexpected runtime errors.
- [ ] Isolated persistence/restart tests pass.
- [ ] Release `pnpm tauri build` exits 0.
- [ ] Release exe and NSIS installer exist with recorded size, timestamp and SHA-256.
- [ ] Release smoke uses isolated four-directory environment and mock Provider.
- [ ] Production DB copy upgrades v2→v6 with integrity_check=ok, foreign_key_check empty and old row counts non-decreasing.
- [ ] README、architecture、smoke checklist and iteration report match implemented behavior.
- [ ] Execution pack validator reports VALID.

## Validation Procedure

1. Read STEP 01 evidence and confirm environment/process/formal DB baseline.
2. Read STEP 03/10/15/26/35 evidence and reconcile migration versions, old row counts, integrity and FK checks.
3. Read Provider unit/mock/UI evidence and compare all fixed model protocols against `CONTEXT.md`.
4. Read document fixture evidence and verify every supported/failure status, incremental behavior and cache boundary.
5. Read analysis/proposal tests and confirm target tables stay unchanged before explicit confirmation.
6. Read scheduler fake-clock matrix and persistence evidence.
7. Read Brief and translation evidence, including local fallback and no translation persistence.
8. Read four Dashboard DOM metric sets and open every final screenshot.
9. Read storage synthetic test before/after hashes, entity counts, protected ids, rollup reconciliation and audit.
10. Confirm `pnpm check`、`pnpm build`、`cargo fmt --check`、full `cargo test` exit 0.
11. Confirm debug/release UI CDP outputs contain no unexpected runtime errors and use isolated paths.
12. Confirm release artifacts and hashes exist.
13. Confirm formal DB file metadata at STEP 35 equals STEP 01.
14. Run `python C:\Users\ZhouNan\.codex\skills\compiling-tasks-for-delegation\scripts\validate_execution_pack.py docs\luna-ai-secretary-iteration` and require `VALID`.
15. Submit screenshots to the high-model review gate.

## Required Evidence

- STEP 01/35 formal DB metadata comparison.
- Git short-status preservation summary.
- Four migration test sets and production-copy v2→v6 report.
- Provider protocol/catalog/route test matrix and mock request paths.
- Document fixture status table, extraction/hash/chunk assertions and cache location.
- Analysis run/proposal/apply/dedupe test matrix.
- Scheduler fake-clock table and restart evidence.
- Brief draft/kept/supersede and translation no-persistence evidence.
- Four Dashboard scroll metric records and screenshots.
- Storage usage/preview/cleanup/rollup/VACUUM/WebView evidence.
- Final static/unit/build/UI CDP results.
- Release artifact path, size, time and SHA-256.
- Updated documentation paths and validator output.

## Failure Conditions

- Any application test writes outside isolated APPDATA/LOCALAPPDATA/TEMP/TMP: `E19-SECURITY-BOUNDARY`.
- Any real Provider or real Key is used: `E19-SECURITY-BOUNDARY`.
- Any source file, formal entity, kept Brief, pending/confirmed proposal or credential is removed by cleanup: `E10-ACCEPTANCE-FAILED`.
- AI writes a formal entity before confirmation: `E10-ACCEPTANCE-FAILED`.
- Any Provider protocol mapping materially conflicts with the locked official snapshot: `E11-CONFLICTING-EVIDENCE`.
- Any migration loses old rows, fails integrity/FK, or requires destructive rewrite: `E10-ACCEPTANCE-FAILED`.
- Dashboard has page-level scroll or clipped required content at a target size/language: `E10-ACCEPTANCE-FAILED`.
- Any required engineering command exits nonzero: `E10-ACCEPTANCE-FAILED`.
- Any required mock/UI/persistence/cache flow fails: `E10-ACCEPTANCE-FAILED`.
- Formal DB metadata changes: `E19-SECURITY-BOUNDARY`.
- Execution pack validator is not VALID: `E09-VALIDATION-FAILED`.

## High-Model Review Gates

- Final visual maturity: Dashboard must feel like a compact daily command center, not a form collection or content page hidden by overflow.
- Information hierarchy: Daily Brief and today priorities must be visually dominant without crowding translation and pending decisions.
- Secretary interaction: review/edit/confirm flow must feel calm, understandable and non-technical.
- Settings weight: Provider/models/routes/schedule/storage must remain approachable despite technical power.
- Chinese naturalness and English completeness require visual/text review from user or higher-capability model.
