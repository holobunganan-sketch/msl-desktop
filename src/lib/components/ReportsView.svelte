<script lang="ts">
  import StatusLine from "$lib/components/ui/StatusLine.svelte";
  import { onMount } from "svelte";
  import {reportBlocks} from "$lib/services/reportReading";
  import AppButton from "$lib/components/ui/AppButton.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";
  import Modal from "$lib/components/ui/Modal.svelte";
  import { command, normalizeError } from "$lib/services/api";
  import { reportItems, sourceDestination, type ReportItem, type ReportSource } from "$lib/services/workflowContinuity";
  import { sourceLabel } from "$lib/services/knowledge";
  import { navigateTo } from "$lib/services/navigation";
  import { captureNote } from "$lib/services/capture";
  import { draftKey, readDraft, writeDraft, clearDraft } from "$lib/services/captureDrafts";
  import type { InboxItem, Report, ReportSchedule, Work } from "$lib/types/domain";
  import { locale, t } from "$lib/i18n";
  import { dataRevision } from "$lib/stores/dataRevision";

  let {focusId = null}:{focusId?:number|null} = $props();
  let reports = $state<Report[]>([]);
  let works = $state<Work[]>([]);
  let schedule = $state<ReportSchedule | null>(null);
  let selectedId = $state<number | null>(null);
  let customStart = $state("");
  let customEnd = $state("");
  let busy = $state<string | null>(null);
  let error = $state("");
  let message = $state("");
  let currentLocale = $derived($locale);
  let selected = $derived(reports.find((report) => report.id === selectedId) ?? null);
  let entries = $derived(selected ? reportItems(selected) : []);
  let captureOpen = $state(false);
  let captureBusy = $state(false);
  let captureError = $state("");
  let captureText = $state("");
  let captureSavedId = $state<number|null>(null);
  let captureTarget = $state<{key:string;origin:string;workId:number|null;headline:string}|null>(null);
  let savedCaptures = $state<Record<string,{id:number;content:string}>>({});
  let loadRequest = 0;
  let loaded = $state(false);
  let mounted = $state(false);
  const tt = (key: Parameters<typeof t>[0], vars: Record<string, string | number> = {}) => t(key, vars, currentLocale);

  function pad(value: number): string { return String(value).padStart(2, "0"); }
  function dateValue(date: Date): string { return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`; }
  function fmtDate(timestamp: number): string { return new Intl.DateTimeFormat(currentLocale, { year: "numeric", month: "2-digit", day: "2-digit" }).format(new Date(timestamp * 1000)); }
  function fmtTime(timestamp: number | null): string { return timestamp ? new Date(timestamp * 1000).toLocaleString(currentLocale === "en-US" ? "en-US" : "zh-CN", { hour12: false }) : "—"; }
  function statusText(report: Report): string { return tt(`reports.status${report.status.charAt(0).toUpperCase()}${report.status.slice(1)}` as Parameters<typeof t>[0]); }
  function scheduleTime(hour: number, minute: number): string { return `${pad(hour)}:${pad(minute)}`; }
  function setScheduleTime(kind: "weekly" | "monthly", value: string) { if (!schedule) return; const [hour, minute] = value.split(":").map(Number); schedule = kind === "weekly" ? { ...schedule, weekly_hour: hour, weekly_minute: minute } : { ...schedule, monthly_hour: hour, monthly_minute: minute }; }
  function sourceWeekCount(report: Report): number { try { const value = JSON.parse(report.source_report_ids_json); return Array.isArray(value) ? value.length : 0; } catch { return 0; } }
  function limitedCoverage(report:Report):boolean { try { const pack=JSON.parse(report.evidence_json??'{}');return !!pack.coverage_notes?.length||pack.source_counts?.period_changes_omitted>0; } catch { return false; } }
  function horizonLabel(item:ReportItem):string { return item.horizon==='current'?(currentLocale==='en-US'?'Current state · may include later changes':'当前情况 · 可能含周期后变化'):item.horizon==='next'?(currentLocale==='en-US'?'Next-period recommendation':'下期建议'):(currentLocale==='en-US'?'Progress this period':'本期进展'); }
  function sourceTarget(source:ReportSource){return source.trust==='deleted'?null:sourceDestination(source.location);}
  function sourceName(source:ReportSource):string { return typeof source.title==='string'&&source.title.trim()?source.title:sourceLabel(source.location?.entity_kind??source.source_type.replace(/^task_.*/, 'task').replace('weekly_report','report'),currentLocale==='en-US'); }
  function sourceUnavailable(source:ReportSource):string { return source.trust==='deleted'?(currentLocale==='en-US'?'Source deleted · historical reference':'来源已删除 · 历史引用'):source.location?.remote_only?(currentLocale==='en-US'?'On another device':'来源位于其他设备'):(currentLocale==='en-US'?'Source cannot be located':'来源暂时无法定位'); }
  function openCapture(report:Report,item:ReportItem,index:number){
    const key=`${draftKey({workId:item.project_id,entityKind:'report',entityId:report.id})}:${index}`;
    // Human-readable provenance stays attached after the draft is edited and saved.
    const projectName=works.find(work=>work.id===item.project_id)?.title??'独立事项 / Independent work';
    const origin=`${report.kind==='weekly'?'周报 / Weekly report':'月报 / Monthly report'} · ${dateValue(new Date(report.period_start*1000))} — ${dateValue(new Date((report.period_end-1)*1000))} · ${projectName} · ${index+1}. ${item.headline}`;
    captureTarget={key,origin,workId:item.project_id,headline:item.headline};
    captureText=savedCaptures[key]?.content??(readDraft(key)||item.next_action);
    captureSavedId=savedCaptures[key]?.id??null;captureError='';captureOpen=true;
  }
  async function saveCapture(){
    if(captureBusy||!captureTarget||!captureText.trim()||captureSavedId)return;
    const target=captureTarget,content=`${captureText.trim()}\n\n${target.origin}`;
    captureBusy=true;captureError='';
    try{
      // Includes processed notes, so revisiting a report cannot recreate an adopted action.
      const existing=(await command<InboxItem[]>('list_inbox')).find(note=>note.content===content||note.content.endsWith(`\n\n${target.origin}`));
      const note=existing??await captureNote(content,{workId:target.workId,entityKind:target.workId===null?null:'work',entityId:target.workId});
      const savedText=note.content.slice(0,-(`\n\n${target.origin}`).length);
      savedCaptures={...savedCaptures,[target.key]:{id:note.id,content:savedText}};clearDraft(target.key);captureSavedId=note.id;captureText=savedText;
    }catch(value){captureError=normalizeError(value);}finally{captureBusy=false;}
  }

  async function copyReport(){if(!selected?.content)return;try{await navigator.clipboard.writeText(selected.content);message=currentLocale==='en-US'?'Report copied.':'已复制可读正文。';}catch{error=currentLocale==='en-US'?'Copy is unavailable. Select the text to copy.':'暂时无法自动复制，可以选中正文复制。';}}
  async function load() {
    const request=++loadRequest;
    try {
      const result=await Promise.all([command<Report[]>("list_reports", { limit: 100 }), command<ReportSchedule>("get_report_schedule"),command<Work[]>('list_works',{status:null})]);
      if(request!==loadRequest)return;
      [reports,schedule,works]=result;
      loaded=true;
      if (selectedId === null || (!reports.some((report) => report.id === selectedId)&&selectedId!==focusId)) selectedId = reports[0]?.id ?? null;
      error = "";
    } catch (value) { if(request===loadRequest)error = normalizeError(value); }
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

  onMount(() => { const now = new Date(); const prior = new Date(now); prior.setDate(now.getDate() - 7); customStart = dateValue(prior); customEnd = dateValue(now);mounted=true;return()=>{loadRequest++;}; });
  $effect(()=>{if(focusId!==null)selectedId=focusId;});
  $effect(()=>{$dataRevision.analysis;$dataRevision.works;if(mounted)void load();});
</script>

<div class="reports-page">
  <header class="page-head"><div><h1>{tt("reports.title")}</h1><p>{currentLocale==='en-US'?'Read results, project progress, blockers and the next useful actions.':'看清这段时间做成了什么、项目推进到哪里，以及下一步如何推进。'}</p></div><div class="primary-actions"><AppButton testid="generate-weekly-report" loading={busy === "weekly-default"} onclick={() => generate("weekly")}>{tt("reports.generateWeekly")}</AppButton><AppButton testid="generate-monthly-report" variant="secondary" loading={busy === "monthly-default"} onclick={() => generate("monthly")}>{tt("reports.generateMonthly")}</AppButton></div></header>
  <div class="feedback error feedback success stable-feedback"><StatusLine message={error||message} error={!!error}/></div>
  <div class="reports-grid" class:without-reports={!reports.length}>
    <aside class="report-history">
      <div class="panel-head"><div><h2>{tt("reports.history")}</h2><small>{reports.length}</small></div><div class="history-actions"><button class="clear-history" data-testid="clear-weekly-history" onclick={clearWeeklyHistory} disabled={busy === "clear-weekly"}>{tt("reports.clearWeeklyHistory")}</button><button onclick={load} aria-label={tt("common.refresh")}><Icon name="refresh" size={15} /></button></div></div>
      {#if reports.length}<div class="history-list">{#each reports as report (report.id)}<button class:active={selected?.id === report.id} onclick={() => (selectedId = report.id)}><span class="report-kind">{report.kind === "weekly" ? tt("reports.weekly") : tt("reports.monthly")}</span><strong>{tt("reports.period", { start: fmtDate(report.period_start), end: fmtDate(report.period_end - 1) })}</strong><small>{fmtTime(report.generated_at || report.created_at)} · {statusText(report)}</small></button>{/each}</div>{:else}<div class="empty"><Icon name="reports" size={26} /><strong>{tt("reports.empty")}</strong><p>{tt("reports.emptyHint")}</p></div>{/if}
    </aside>

    {#if selected}<main class="report-viewer">
        <div class="report-title"><div><span>{selected.kind === "weekly" ? tt("reports.weekly") : tt("reports.monthly")}</span><h2>{tt("reports.period", { start: fmtDate(selected.period_start), end: fmtDate(selected.period_end - 1) })}</h2><small>#{selected.id} · {statusText(selected)}{selected.kind === "monthly" ? ` · ${tt("reports.sourceWeeks", { count: sourceWeekCount(selected) })}` : ""}</small></div>{#if selected.content}<AppButton testid="copy-report" variant="secondary" onclick={copyReport}>{currentLocale==='en-US'?'Copy report':'复制正文'}</AppButton>{/if}{#if selected.status === "failed"}<AppButton variant="secondary" loading={busy === `retry-${selected.id}`} onclick={() => retry(selected)}>{tt("reports.retry")}</AppButton>{/if}</div>
        {#if selected.status === "running"}<div class="report-state"><span class="spinner"></span><p>{tt("reports.statusRunning")}</p></div>
        {:else if selected.status === "failed"}<div class="report-state failed"><Icon name="activity" size={24} /><strong>{selected.error_code}</strong><p>{selected.error_message}</p></div>
        {:else}
          <p class="report-reading-note">{currentLocale==='en-US'?'Keep the useful next steps. Recommendations are saved for your review and do not change work items.':'有用的下一步可以先记下来。建议经您确认后才会进入正式事项。'}</p>
          <ol class="weekly-content" class:monthly-content={selected.kind==='monthly'} data-testid="readable-report">
            {#if entries.length}
              {#each entries as item,index}
                {@const project=works.find(work=>work.id===item.project_id)}
                <li data-testid="report-entry">
                  <div class="entry-context">
                    {#if project}<button class="text-action" data-testid="report-project" onclick={()=>navigateTo('work',project.id)}>{project.title} ↗</button>
                    {:else if item.project_id!==null}<span>{currentLocale==='en-US'?'Project unavailable · historical record':'项目不可用 · 历史记录'}</span>
                    {:else}<span>{currentLocale==='en-US'?'Independent work':'独立事项'}</span>{/if}
                    <span>{horizonLabel(item)}</span>
                  </div>
                  <strong>{item.headline}</strong>
                  {#if item.certainty!=='observed'}<span class="certainty">{item.certainty==='inferred'?(currentLocale==='en-US'?'Inference · verify':'推测 · 待核实'):(currentLocale==='en-US'?'Insufficient evidence':'资料不足')}</span>{/if}
                  <p>{item.change}</p>
                  {#if item.impact}<p><span class="detail-label">{currentLocale==='en-US'?'Impact':'工作影响'}</span>{item.impact}</p>{/if}
                  {#if item.next_action}<p><span class="detail-label">{currentLocale==='en-US'?'Next step':'下一步'}</span>{item.next_action}</p>{/if}
                  <div class="entry-actions">
                    {#if item.sources.length}
                      <details class="report-sources" data-testid="report-sources"><summary>{currentLocale==='en-US'?'View evidence':'查看依据'} · {item.sources.length}</summary>
                        <ul>{#each item.sources as source}
                          {@const destination=sourceTarget(source)}
                          <li>{#if destination}<button class="text-action" data-testid="report-source" onclick={()=>navigateTo(destination)}>{sourceName(source)} ↗</button>{:else}<span>{sourceName(source)} · {sourceUnavailable(source)}</span>{/if}{#if source.timestamp}<small>{fmtTime(source.timestamp)}</small>{/if}</li>
                        {/each}</ul>
                      </details>
                    {/if}
                    {#if item.next_action&&(item.project_id===null||project)}<button class="text-action prepare-action" data-testid="report-capture-action" onclick={()=>selected&&openCapture(selected,item,index)}>{currentLocale==='en-US'?'Prepare this next step':'把这一步记下来'}</button>{/if}
                  </div>
                </li>
              {/each}
            {:else}
              {#each reportBlocks(selected.content) as block}<li><strong>{block.headline}</strong>{#if block.body}<p>{block.body}</p>{/if}</li>{/each}
            {/if}
          </ol>
          {#if entries.length&&limitedCoverage(selected)}<p class="report-reading-note coverage-note">{currentLocale==='en-US'?'Some records or documents exceeded the evidence range. Unmentioned work may still have progressed.':'部分记录或资料超出本次输入范围；未提及的工作不能据此认定没有进展。'}</p>{/if}
        {/if}
    </main>{:else if loaded&&focusId!==null&&selectedId===focusId}<div class="report-state">{currentLocale==='en-US'?'This report is unavailable or has been removed. Select another report from history.':'这份报告已不可用或已清理。可以从历史记录中选择其他报告。'}</div>{/if}

    <details class="report-controls" data-testid="report-options"><summary>{currentLocale==='en-US'?'Generation schedule & custom period':'生成时间与自定义周期'}</summary><div class="report-options-body">
      <section class="control-card"><div><h2>{tt("reports.customPeriod")}</h2><p>{tt("reports.customHint")}</p></div><div class="date-grid"><label>{tt("brief.start")}<input type="date" bind:value={customStart} /></label><label>{tt("brief.end")}<input type="date" bind:value={customEnd} /></label></div><div class="control-actions"><AppButton variant="secondary" loading={busy === "weekly-custom"} onclick={() => generate("weekly", true)}>{tt("reports.generateCustomWeekly")}</AppButton><AppButton variant="secondary" loading={busy === "monthly-custom"} onclick={() => generate("monthly", true)}>{tt("reports.generateCustomMonthly")}</AppButton></div></section>
      {#if schedule}<section class="control-card" data-testid="report-schedule-card"><div><h2>{tt("reports.schedule")}</h2><p>{tt("reports.deepAnalysis")}</p></div><div class="schedule-block"><label class="toggle"><input type="checkbox" bind:checked={schedule.weekly_enabled} /><span>{tt("reports.weeklySchedule")}</span></label><label>{tt("reports.weekday")}<select bind:value={schedule.weekly_weekday}>{#each [0,1,2,3,4,5,6] as day}<option value={day}>{tt(`reports.weekday${day}` as Parameters<typeof t>[0])}</option>{/each}</select></label><label>{tt("reports.time")}<input type="time" value={scheduleTime(schedule.weekly_hour, schedule.weekly_minute)} onchange={(event) => setScheduleTime("weekly", event.currentTarget.value)} /></label></div><div class="schedule-block"><label class="toggle"><input type="checkbox" bind:checked={schedule.monthly_enabled} /><span>{tt("reports.monthlySchedule")}</span></label><label>{tt("reports.monthDay")}<input type="number" min="1" max="28" bind:value={schedule.monthly_day} /></label><label>{tt("reports.time")}<input type="time" value={scheduleTime(schedule.monthly_hour, schedule.monthly_minute)} onchange={(event) => setScheduleTime("monthly", event.currentTarget.value)} /></label></div><AppButton loading={busy === "schedule"} onclick={saveSchedule}>{tt("common.save")}</AppButton></section>{/if}
    </div></details>
  </div>
</div>

<Modal bind:open={captureOpen} dismissible={!captureBusy} title={currentLocale==='en-US'?'Keep the next step':'记下这一步'}>
  {#if captureTarget}
    <div class="report-capture">
      <p class="capture-origin">{captureTarget.headline}</p>
      <label>{currentLocale==='en-US'?'Adjust the suggestion before saving':'先调整建议，再保存'}
        <textarea data-testid="report-capture-text" rows="5" value={captureText} disabled={captureBusy||captureSavedId!==null} oninput={event=>{captureText=event.currentTarget.value;if(captureTarget)writeDraft(captureTarget.key,captureText);}}></textarea>
      </label>
      <small>{currentLocale==='en-US'?'Saved in To organize with its report source. You decide when to ask the secretary and confirm any resulting changes.':'保存后留在待整理，并保留报告来源。需要时再交给秘书；正式事项仍由您确认。'}</small>
      <StatusLine message={captureError||(captureSavedId?(currentLocale==='en-US'?'This step is already saved. No duplicate was created.':'这一步已记下，不会重复添加。'):'')} error={!!captureError}/>
      <div class="capture-actions">{#if captureSavedId}<AppButton testid="report-capture-view" onclick={()=>{if(captureSavedId)navigateTo('inbox',captureSavedId);captureOpen=false;}}>{currentLocale==='en-US'?'View saved note':'查看已保存记录'}</AppButton>{:else}<AppButton testid="report-capture-confirm" loading={captureBusy} disabled={!captureText.trim()} onclick={saveCapture}>{currentLocale==='en-US'?'Confirm & save for review':'确认保存到待整理'}</AppButton>{/if}</div>
    </div>
  {/if}
</Modal>

<style>
  .reports-page{display:grid;gap:20px;min-width:0}.page-head{display:flex;flex-wrap:wrap;align-items:flex-start;justify-content:space-between;gap:16px}.page-head h1{margin:0 0 8px;font:500 29px var(--font-serif)}.page-head p{margin:0;line-height:1.7;color:var(--color-muted);font-size:15px}
  .primary-actions,.history-actions{display:flex;flex-wrap:wrap;gap:8px}.reports-grid{display:grid;grid-template-columns:270px minmax(0,1fr);gap:20px;align-items:start;min-width:0}.report-history,.report-viewer,.control-card,.report-controls{border:1px solid var(--color-border);border-radius:18px;background:var(--color-surface);min-width:0;box-shadow:var(--shadow-sm)}
  .panel-head{padding:18px;display:grid;gap:12px;border-bottom:1px solid var(--color-border)}.panel-head>div{display:flex;align-items:center;gap:10px;flex-wrap:wrap}.panel-head h2{margin:0;font-size:17px}.panel-head small{color:var(--color-muted)}.panel-head button{font:inherit;font-size:13px;border:1px solid var(--color-border);background:var(--color-surface);color:var(--color-muted);padding:7px 10px;border-radius:8px;cursor:pointer}.panel-head .clear-history{color:var(--color-danger)}
  .history-list{padding:10px;max-height:600px;overflow:auto}.history-list button{width:100%;display:grid;gap:7px;padding:14px;border:1px solid transparent;border-radius:12px;background:transparent;text-align:left;color:var(--color-text);cursor:pointer}.history-list button.active{background:var(--color-primary-soft);border-color:var(--color-border)}.history-list strong{font-size:15px;overflow-wrap:anywhere}.history-list small{color:var(--color-muted);font-size:13px}.report-kind{color:var(--color-primary);font-size:13px}
  .report-viewer{padding:clamp(20px,3vw,38px);overflow-wrap:anywhere}.report-title{display:flex;flex-wrap:wrap;justify-content:space-between;align-items:start;gap:14px;border-bottom:1px solid var(--color-border);padding-bottom:20px}.report-title span{font-size:14px;color:var(--color-primary)}.report-title h2{font:500 26px var(--font-serif);margin:9px 0}.report-title small{color:var(--color-muted);font-size:13px}.report-reading-note{font-size:14px;line-height:1.65;color:var(--color-muted);margin:18px 0}
  .weekly-content{margin:0;padding:0 0 0 28px;font-size:16px;line-height:1.85;color:var(--color-text)}.weekly-content li{padding:18px 0 20px 7px;border-bottom:1px solid var(--color-border)}.weekly-content li:last-child{border:0}.weekly-content strong{font-size:18px;font-weight:650}.weekly-content p{margin:10px 0 0;white-space:pre-line;line-height:1.9;color:var(--color-text)}.monthly-content{line-height:1.9}
  .entry-context{display:flex;flex-wrap:wrap;gap:6px 14px;color:var(--color-muted);font-size:13px;line-height:1.6;margin-bottom:9px}.text-action{font:inherit;text-align:left;border:0;background:transparent;color:var(--color-primary);padding:3px 0;cursor:pointer;overflow-wrap:anywhere}.text-action:focus-visible{outline:2px solid var(--color-primary);outline-offset:3px}.certainty{display:inline-block;margin-left:9px;font-size:13px;color:var(--color-muted)}.detail-label{color:var(--color-muted);font-size:14px;margin-right:12px}.entry-actions{display:flex;align-items:start;flex-wrap:wrap;gap:10px 24px;margin-top:12px;font-size:14px}.report-sources{flex:1 1 180px;min-width:0;color:var(--color-muted)}.report-sources summary{cursor:pointer;min-height:30px}.report-sources ul{padding:0;list-style:none;margin:6px 0 0}.report-sources li{padding:4px 0;border:0;font-size:13px;line-height:1.65}.report-sources small{display:block;font-size:12px}.prepare-action{min-height:30px}.coverage-note{border-left:2px solid var(--color-border);padding-left:12px}.report-capture{display:grid;gap:14px;min-width:0}.capture-origin{margin:0;line-height:1.7;overflow-wrap:anywhere}.report-capture label{display:grid;gap:9px;font-size:14px}.report-capture textarea{width:100%;min-height:150px;resize:vertical;line-height:1.75;font:inherit;padding:12px}.report-capture small{font-size:13px;line-height:1.7;color:var(--color-muted)}.capture-actions{display:flex;justify-content:flex-end;min-height:40px}
  .report-controls{grid-column:1/-1;padding:18px 22px}.report-controls>summary{cursor:pointer;font-size:16px;font-weight:600}.report-options-body{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:18px;padding-top:18px}.control-card{padding:22px;display:grid;align-content:start;gap:16px;box-shadow:none}.control-card h2{font-size:18px;margin:0 0 8px}.control-card p{font-size:14px;line-height:1.7;color:var(--color-muted);margin:0}.date-grid,.schedule-block{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:12px}.control-card label{display:grid;gap:8px;min-width:0;font-size:14px}.control-card input,.control-card select{min-width:0;width:100%;padding:10px;border:1px solid var(--color-border);border-radius:9px;background:var(--color-surface);color:var(--color-text);font:inherit}.control-actions{display:flex;flex-wrap:wrap;gap:9px}.schedule-block .toggle{grid-column:1/-1;display:flex;align-items:center;gap:10px}.toggle input{width:auto}.schedule-block{border-top:1px solid var(--color-border);padding-top:14px}
  .feedback{padding:14px 18px;border-radius:12px;font-size:15px;line-height:1.6}.error,.failed{color:var(--color-danger)}.feedback.error{background:var(--color-danger-soft)}.feedback.success{background:var(--color-success-soft);color:var(--color-success)}.report-state,.empty{display:grid;justify-items:center;gap:12px;padding:40px 24px;text-align:center;color:var(--color-muted)}.report-state p,.empty p{margin:0;font-size:15px;line-height:1.7}.without-reports{grid-template-columns:minmax(0,1fr)}.spinner{height:24px;width:24px;border:2px solid var(--color-border);border-right-color:var(--color-primary);border-radius:50%;animation:spin .8s linear infinite}@keyframes spin{to{transform:rotate(360deg)}}
  @container(max-width:900px){.reports-grid{grid-template-columns:minmax(0,1fr)}.report-viewer{grid-row:1}.history-list{max-height:260px}.report-options-body{grid-template-columns:minmax(0,1fr)}}@container(max-width:520px){.date-grid,.schedule-block{grid-template-columns:minmax(0,1fr)}.report-title h2{font-size:22px}}
</style>
