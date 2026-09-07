<script lang="ts">
  import { onMount } from "svelte";
  import {reportBlocks} from "$lib/services/reportReading";
  import AppButton from "$lib/components/ui/AppButton.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";
  import { command, normalizeError } from "$lib/services/api";
  import type { Report, ReportSchedule } from "$lib/types/domain";
  import { locale, t } from "$lib/i18n";
  import { dataRevision } from "$lib/stores/dataRevision";

  let reports = $state<Report[]>([]);
  let schedule = $state<ReportSchedule | null>(null);
  let selectedId = $state<number | null>(null);
  let customStart = $state("");
  let customEnd = $state("");
  let busy = $state<string | null>(null);
  let error = $state("");
  let message = $state("");
  let currentLocale = $derived($locale);
  let selected = $derived(reports.find((report) => report.id === selectedId) ?? reports[0] ?? null);
  const tt = (key: Parameters<typeof t>[0], vars: Record<string, string | number> = {}) => t(key, vars, currentLocale);

  function pad(value: number): string { return String(value).padStart(2, "0"); }
  function dateValue(date: Date): string { return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`; }
  function fmtDate(timestamp: number): string { return new Intl.DateTimeFormat(currentLocale, { year: "numeric", month: "2-digit", day: "2-digit" }).format(new Date(timestamp * 1000)); }
  function fmtTime(timestamp: number | null): string { return timestamp ? new Date(timestamp * 1000).toLocaleString(currentLocale === "en-US" ? "en-US" : "zh-CN", { hour12: false }) : "—"; }
  function statusText(report: Report): string { return tt(`reports.status${report.status.charAt(0).toUpperCase()}${report.status.slice(1)}` as Parameters<typeof t>[0]); }
  function scheduleTime(hour: number, minute: number): string { return `${pad(hour)}:${pad(minute)}`; }
  function setScheduleTime(kind: "weekly" | "monthly", value: string) { if (!schedule) return; const [hour, minute] = value.split(":").map(Number); schedule = kind === "weekly" ? { ...schedule, weekly_hour: hour, weekly_minute: minute } : { ...schedule, monthly_hour: hour, monthly_minute: minute }; }
  function sourceWeekCount(report: Report): number { try { const value = JSON.parse(report.source_report_ids_json); return Array.isArray(value) ? value.length : 0; } catch { return 0; } }

  async function copyReport(){if(!selected?.content)return;try{await navigator.clipboard.writeText(selected.content);message=currentLocale==='en-US'?'Report copied.':'已复制可读正文。';}catch{error=currentLocale==='en-US'?'Copy is unavailable. Select the text to copy.':'暂时无法自动复制，可以选中正文复制。';}}
  async function load() {
    try {
      [reports, schedule] = await Promise.all([command<Report[]>("list_reports", { limit: 100 }), command<ReportSchedule>("get_report_schedule")]);
      if (selectedId === null || !reports.some((report) => report.id === selectedId)) selectedId = reports[0]?.id ?? null;
      error = "";
    } catch (value) { error = normalizeError(value); }
  }
  async function generate(kind: "weekly" | "monthly", custom = false) {
    busy = `${kind}-${custom ? "custom" : "default"}`; error = ""; message = "";
    try {
      let periodStart: number | null = null; let periodEnd: number | null = null;
      if (custom) {
        if (!customStart || !customEnd) throw new Error(tt("common.required"));
        periodStart = Math.floor(new Date(`${customStart}T00:00:00`).getTime() / 1000);
        const exclusiveEnd = new Date(`${customEnd}T00:00:00`); exclusiveEnd.setDate(exclusiveEnd.getDate() + 1);
        periodEnd = Math.floor(exclusiveEnd.getTime() / 1000);
        if (periodEnd <= periodStart) throw new Error(tt("common.invalid"));
      }
      const id = await command<number>("generate_report", { kind, periodStart, periodEnd });
      await load(); selectedId = id;
    } catch (value) { error = normalizeError(value); await load(); } finally { busy = null; }
  }
  async function retry(report: Report) { busy = `retry-${report.id}`; error = ""; try { const id = await command<number>("retry_report", { reportId: report.id }); await load(); selectedId = id; } catch (value) { error = normalizeError(value); await load(); } finally { busy = null; } }
  async function saveSchedule() { if (!schedule) return; busy = "schedule"; error = ""; try { schedule = await command<ReportSchedule>("save_report_schedule", { schedule }); message = tt("reports.scheduleSaved"); } catch (value) { error = normalizeError(value); } finally { busy = null; } }
  async function clearWeeklyHistory() {
    const count = reports.filter((report) => report.kind === "weekly" && report.status !== "running").length;
    if (!count) { message = tt("reports.noWeeklyHistory"); return; }
    if (!confirm(tt("reports.clearWeeklyConfirm"))) return;
    busy = "clear-weekly"; error = ""; message = "";
    try {
      const cleared = await command<number>("clear_report_history", { kind: "weekly" });
      message = tt("reports.weeklyHistoryCleared", { count: cleared });
      await load();
    } catch (value) { error = normalizeError(value); } finally { busy = null; }
  }

  onMount(() => { const now = new Date(); const prior = new Date(now); prior.setDate(now.getDate() - 7); customStart = dateValue(prior); customEnd = dateValue(now); void load(); });
  $effect(()=>{$dataRevision.analysis; void load();});
</script>

<div class="reports-page">
  <header class="page-head"><div><h1>{tt("reports.title")}</h1><p>{currentLocale==='en-US'?'Read results, project progress, blockers and the next useful actions.':'看清这段时间做成了什么、项目推进到哪里，以及下一步如何推进。'}</p></div><div class="primary-actions"><AppButton testid="generate-weekly-report" loading={busy === "weekly-default"} onclick={() => generate("weekly")}>{tt("reports.generateWeekly")}</AppButton><AppButton testid="generate-monthly-report" variant="secondary" loading={busy === "monthly-default"} onclick={() => generate("monthly")}>{tt("reports.generateMonthly")}</AppButton></div></header>
  {#if error}<div class="feedback error" role="alert">{error}</div>{/if}{#if message}<div class="feedback success">{message}</div>{/if}
  <div class="reports-grid" class:without-reports={!reports.length}>
    <aside class="report-history">
      <div class="panel-head"><div><h2>{tt("reports.history")}</h2><small>{reports.length}</small></div><div class="history-actions"><button class="clear-history" data-testid="clear-weekly-history" onclick={clearWeeklyHistory} disabled={busy === "clear-weekly"}>{tt("reports.clearWeeklyHistory")}</button><button onclick={load} aria-label={tt("common.refresh")}><Icon name="refresh" size={15} /></button></div></div>
      {#if reports.length}<div class="history-list">{#each reports as report (report.id)}<button class:active={selected?.id === report.id} onclick={() => (selectedId = report.id)}><span class="report-kind">{report.kind === "weekly" ? tt("reports.weekly") : tt("reports.monthly")}</span><strong>{tt("reports.period", { start: fmtDate(report.period_start), end: fmtDate(report.period_end - 1) })}</strong><small>{fmtTime(report.generated_at || report.created_at)} · {statusText(report)}</small></button>{/each}</div>{:else}<div class="empty"><Icon name="reports" size={26} /><strong>{tt("reports.empty")}</strong><p>{tt("reports.emptyHint")}</p></div>{/if}
    </aside>

    {#if selected}<main class="report-viewer">
        <div class="report-title"><div><span>{selected.kind === "weekly" ? tt("reports.weekly") : tt("reports.monthly")}</span><h2>{tt("reports.period", { start: fmtDate(selected.period_start), end: fmtDate(selected.period_end - 1) })}</h2><small>#{selected.id} · {statusText(selected)}{selected.kind === "monthly" ? ` · ${tt("reports.sourceWeeks", { count: sourceWeekCount(selected) })}` : ""}</small></div>{#if selected.content}<AppButton testid="copy-report" variant="secondary" onclick={copyReport}>{currentLocale==='en-US'?'Copy report':'复制正文'}</AppButton>{/if}{#if selected.status === "failed"}<AppButton variant="secondary" loading={busy === `retry-${selected.id}`} onclick={() => retry(selected)}>{tt("reports.retry")}</AppButton>{/if}</div>
        {#if selected.status === "running"}<div class="report-state"><span class="spinner"></span><p>{tt("reports.statusRunning")}</p></div>{:else if selected.status === "failed"}<div class="report-state failed"><Icon name="activity" size={24} /><strong>{selected.error_code}</strong><p>{selected.error_message}</p></div>{:else}<p class="report-reading-note">{currentLocale==='en-US'?'References and time ranges are checked against supplied records. Inferences remain labeled; recommendations do not change your work items.':'来源引用与时间范围已校验；推测会单独标明。报告中的建议不会直接改动您的事项。'}</p><ol class="weekly-content" class:monthly-content={selected.kind==='monthly'} data-testid="readable-report">{#each reportBlocks(selected.content) as block}<li><strong>{block.headline}</strong>{#if block.body}<p>{block.body}</p>{/if}</li>{/each}</ol>{/if}
    </main>{/if}

    <details class="report-controls" data-testid="report-options"><summary>{currentLocale==='en-US'?'Generation schedule & custom period':'生成时间与自定义周期'}</summary><div class="report-options-body">
      <section class="control-card"><div><h2>{tt("reports.customPeriod")}</h2><p>{tt("reports.customHint")}</p></div><div class="date-grid"><label>{tt("brief.start")}<input type="date" bind:value={customStart} /></label><label>{tt("brief.end")}<input type="date" bind:value={customEnd} /></label></div><div class="control-actions"><AppButton variant="secondary" loading={busy === "weekly-custom"} onclick={() => generate("weekly", true)}>{tt("reports.generateCustomWeekly")}</AppButton><AppButton variant="secondary" loading={busy === "monthly-custom"} onclick={() => generate("monthly", true)}>{tt("reports.generateCustomMonthly")}</AppButton></div></section>
      {#if schedule}<section class="control-card" data-testid="report-schedule-card"><div><h2>{tt("reports.schedule")}</h2><p>{tt("reports.deepAnalysis")}</p></div><div class="schedule-block"><label class="toggle"><input type="checkbox" bind:checked={schedule.weekly_enabled} /><span>{tt("reports.weeklySchedule")}</span></label><label>{tt("reports.weekday")}<select bind:value={schedule.weekly_weekday}>{#each [0,1,2,3,4,5,6] as day}<option value={day}>{tt(`reports.weekday${day}` as Parameters<typeof t>[0])}</option>{/each}</select></label><label>{tt("reports.time")}<input type="time" value={scheduleTime(schedule.weekly_hour, schedule.weekly_minute)} onchange={(event) => setScheduleTime("weekly", event.currentTarget.value)} /></label></div><div class="schedule-block"><label class="toggle"><input type="checkbox" bind:checked={schedule.monthly_enabled} /><span>{tt("reports.monthlySchedule")}</span></label><label>{tt("reports.monthDay")}<input type="number" min="1" max="28" bind:value={schedule.monthly_day} /></label><label>{tt("reports.time")}<input type="time" value={scheduleTime(schedule.monthly_hour, schedule.monthly_minute)} onchange={(event) => setScheduleTime("monthly", event.currentTarget.value)} /></label></div><AppButton loading={busy === "schedule"} onclick={saveSchedule}>{tt("common.save")}</AppButton></section>{/if}
    </div></details>
  </div>
</div>

<style>
  .reports-page{display:grid;gap:20px;min-width:0}.page-head{display:flex;flex-wrap:wrap;align-items:flex-start;justify-content:space-between;gap:16px}.page-head h1{margin:0 0 8px;font:500 29px var(--font-serif)}.page-head p{margin:0;line-height:1.7;color:var(--color-muted);font-size:15px}
  .primary-actions,.history-actions{display:flex;flex-wrap:wrap;gap:8px}.reports-grid{display:grid;grid-template-columns:270px minmax(0,1fr);gap:20px;align-items:start;min-width:0}.report-history,.report-viewer,.control-card,.report-controls{border:1px solid var(--color-border);border-radius:18px;background:var(--color-surface);min-width:0;box-shadow:var(--shadow-sm)}
  .panel-head{padding:18px;display:grid;gap:12px;border-bottom:1px solid var(--color-border)}.panel-head>div{display:flex;align-items:center;gap:10px;flex-wrap:wrap}.panel-head h2{margin:0;font-size:17px}.panel-head small{color:var(--color-muted)}.panel-head button{font:inherit;font-size:13px;border:1px solid var(--color-border);background:var(--color-surface);color:var(--color-muted);padding:7px 10px;border-radius:8px;cursor:pointer}.panel-head .clear-history{color:var(--color-danger)}
  .history-list{padding:10px;max-height:600px;overflow:auto}.history-list button{width:100%;display:grid;gap:7px;padding:14px;border:1px solid transparent;border-radius:12px;background:transparent;text-align:left;color:var(--color-text);cursor:pointer}.history-list button.active{background:var(--color-primary-soft);border-color:var(--color-border)}.history-list strong{font-size:15px;overflow-wrap:anywhere}.history-list small{color:var(--color-muted);font-size:13px}.report-kind{color:var(--color-primary);font-size:13px}
  .report-viewer{padding:clamp(20px,3vw,38px);overflow-wrap:anywhere}.report-title{display:flex;flex-wrap:wrap;justify-content:space-between;align-items:start;gap:14px;border-bottom:1px solid var(--color-border);padding-bottom:20px}.report-title span{font-size:14px;color:var(--color-primary)}.report-title h2{font:500 26px var(--font-serif);margin:9px 0}.report-title small{color:var(--color-muted);font-size:13px}.report-reading-note{font-size:14px;line-height:1.65;color:var(--color-muted);margin:18px 0}
  .weekly-content{margin:0;padding:0 0 0 28px;font-size:16px;line-height:1.85;color:var(--color-text)}.weekly-content li{padding:18px 0 20px 7px;border-bottom:1px solid var(--color-border)}.weekly-content li:last-child{border:0}.weekly-content strong{font-size:18px;font-weight:650}.weekly-content p{margin:10px 0 0;white-space:pre-line;line-height:1.9;color:var(--color-text)}.monthly-content{line-height:1.9}
  .report-controls{grid-column:1/-1;padding:18px 22px}.report-controls>summary{cursor:pointer;font-size:16px;font-weight:600}.report-options-body{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:18px;padding-top:18px}.control-card{padding:22px;display:grid;align-content:start;gap:16px;box-shadow:none}.control-card h2{font-size:18px;margin:0 0 8px}.control-card p{font-size:14px;line-height:1.7;color:var(--color-muted);margin:0}.date-grid,.schedule-block{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:12px}.control-card label{display:grid;gap:8px;min-width:0;font-size:14px}.control-card input,.control-card select{min-width:0;width:100%;padding:10px;border:1px solid var(--color-border);border-radius:9px;background:var(--color-surface);color:var(--color-text);font:inherit}.control-actions{display:flex;flex-wrap:wrap;gap:9px}.schedule-block .toggle{grid-column:1/-1;display:flex;align-items:center;gap:10px}.toggle input{width:auto}.schedule-block{border-top:1px solid var(--color-border);padding-top:14px}
  .feedback{padding:14px 18px;border-radius:12px;font-size:15px;line-height:1.6}.error,.failed{color:var(--color-danger)}.feedback.error{background:var(--color-danger-soft)}.feedback.success{background:var(--color-success-soft);color:var(--color-success)}.report-state,.empty{display:grid;justify-items:center;gap:12px;padding:40px 24px;text-align:center;color:var(--color-muted)}.report-state p,.empty p{margin:0;font-size:15px;line-height:1.7}.without-reports{grid-template-columns:minmax(0,1fr)}.spinner{height:24px;width:24px;border:2px solid var(--color-border);border-right-color:var(--color-primary);border-radius:50%;animation:spin .8s linear infinite}@keyframes spin{to{transform:rotate(360deg)}}
  @container(max-width:900px){.reports-grid{grid-template-columns:minmax(0,1fr)}.report-viewer{grid-row:1}.history-list{max-height:260px}.report-options-body{grid-template-columns:minmax(0,1fr)}}@container(max-width:520px){.date-grid,.schedule-block{grid-template-columns:minmax(0,1fr)}.report-title h2{font-size:22px}}
</style>
