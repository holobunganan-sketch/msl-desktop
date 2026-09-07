<script lang="ts">
  import ProposalPreview from './ProposalPreview.svelte';
  import {navigateTo} from '$lib/services/navigation';
  import {proposalPresentation,actionTimeLabel} from '$lib/services/proposalPresentation';
  import {aiJobs} from '$lib/stores/aiJobs';
  let activeTasks=$state<Task[]>([]);
  let receiptTarget=$state<{kind:string;id:number;workId:number|null}|null>(null);
  const analysisBusy=$derived($aiJobs.some(job=>job.command==='run_analysis_now'&&job.status==='running'));
  async function analyze(){try{await command('run_analysis_now',{trigger:'manual'});await load();}catch(e){dataError=String(e);}}

  import { invoke } from "@tauri-apps/api/core";
  import { locale, t, translateStatus } from "$lib/i18n";
  import { command, confirmAiProposal, deferAiProposal, getToday, listLatestAnalysisProposals, listWorks, updateAiProposalClassification } from "$lib/services/api";
  import type { AiProposal } from "$lib/types/domain";
  import { dataRevision, invalidate } from "$lib/stores/dataRevision";
  const savedRoutes=new Map<number,Pick<AiProposal,"kind"|"operation"|"target_id">>();
  import Icon from "$lib/components/ui/Icon.svelte";
  import { decisionPayload } from "$lib/services/proposalPayload";

  type Work = { id: number; title: string; status: string; summary: string | null; created_at: number; updated_at: number; archived_at: number | null };
  type ResumePoint = { id: number; work_id: number; current_state: string; next_step: string; remember: string; source: string; created_at: number };
  type WorkFileRef = { id: number; work_id: number; workspace_id: number | null; path: string; label: string | null; pinned: boolean; created_at: number };
  type ContinueWork = { work: Work; latest_resume: ResumePoint | null; last_activity_at: number | null; files: WorkFileRef[] };
  type Task = { id: number; work_id: number | null; title: string; status: string; priority: string; due_at: number | null; scheduled_start: number | null; scheduled_end: number | null; notes: string | null; created_at: number; updated_at: number; completed_at: number | null };
  type WaitingItem = { id: number; work_id: number | null; title: string; waiting_for: string; started_at: number; follow_up_at: number | null; status: string; notes: string | null; created_at: number; updated_at: number; resolved_at: number | null };
  type CalendarEvent = { id: number; work_id: number | null; title: string; start_at: number; end_at: number | null; all_day: boolean; location: string | null; notes: string | null; kind: string; created_at: number; updated_at: number };
  type InboxItem = { id: number; content: string; created_at: number; processed_at: number | null; converted_to_type: string | null; converted_to_id: number | null };
  type TodayData = { continue_works: ContinueWork[]; today_tasks: Task[]; today_calendar: CalendarEvent[]; waiting_followups: WaitingItem[]; inbox_pending: InboxItem[] };
  type SyncStatus = { root: string | null; paused: boolean; baseline_count: number; last_scan: number | null; last_warning: string | null };
  type RecentFile = { path: string; event_type: string; display_text: string; timestamp: number };
  type BriefResult = { content: string; ai_used: boolean; warning: string | null; source_counts: Record<string, number>; source_preview: Array<{ type: string; title?: string | null; display?: string | null; status?: string | null }>; period_start: number; period_end: number; locale: string };
  type AnalysisRun = { id:number; trigger:string; status:string; started_at:number; finished_at:number|null; summary:string|null; error_code:string|null; error_message:string|null };
  type AnalysisSchedule = { id:number; enabled:boolean; interval_minutes:number; daily_enabled:boolean; daily_hour:number; daily_minute:number; last_interval_run_at:number|null; last_daily_local_date:string|null; updated_at:number };

  let data = $state<TodayData | null>(null);
  let sync = $state<SyncStatus | null>(null);
  let recentFiles = $state<RecentFile[]>([]);
  let dataError = $state("");
  let syncError = $state("");
  let briefError = $state("");
  let brief = $state<string | null>(null);
  let briefResult = $state<BriefResult | null>(null);
  let pendingProposals = $state(0);
  let latestProposals = $state<AiProposal[]>([]);
  let works = $state<Work[]>([]);
  let decisionBusy = $state<number | null>(null);
  let decisionMessage = $state("");
  let lastReceipt=$state<string|null>(null);
  let earlierPending=$state(0);
  let scheduleBusy = $state(false);
  let briefDate = $state("");
  let generating = $state(false);
  let showSources = $state(false);
  let periodPreset = $state<"yesterday" | "7d" | "custom">("yesterday");
  let customStart = $state("");
  let customEnd = $state("");
  let analysisRuns = $state<AnalysisRun[]>([]);
  let analysisSchedule = $state<AnalysisSchedule | null>(null);
  let currentLocale = $derived($locale);
  const tt = (key: Parameters<typeof t>[0], params: Record<string, string | number> = {}) => t(key, params, currentLocale);

  function pad(n: number) { return String(n).padStart(2, "0"); }
  function fmtTime(ts: number | null): string {
    if (!ts) return "";
    const d = new Date(ts * 1000);
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }
  function fmtDayTime(ts: number): string { const d = new Date(ts * 1000); return `${pad(d.getHours())}:${pad(d.getMinutes())}`; }
  function dayRange(): [number, number] { const now = new Date(); const start = new Date(now.getFullYear(), now.getMonth(), now.getDate()); return [Math.floor(start.getTime() / 1000), Math.floor(new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1).getTime() / 1000)]; }
  function briefRange(): [number, number] {
    const [todayStart] = dayRange();
    if (periodPreset === "7d") return [todayStart - 6 * 86400, todayStart + 86400];
    if (periodPreset === "custom" && customStart && customEnd) return [Math.floor(new Date(`${customStart}T00:00:00`).getTime() / 1000), Math.floor(new Date(`${customEnd}T23:59:59`).getTime() / 1000)];
    return [todayStart - 86400, todayStart];
  }
  function nav(view:string,id?:number,workId?:number|null){navigateTo(view,id,workId);}
  function bulletLines(value: string): string[] {
    return value
      .split(/\r?\n/)
      .map((line) => line.trim().replace(/^[•·▪‣*-]\s*/, ""))
      .map((line) => line.replace(/\s*[（(\[][^）)\]]*(?:source_type|entity_id|workspace_id|source_id|\[source_)[^）)\]]*[）)\]]/gi, "").trim())
      .filter(Boolean);
  }
  function briefWarningText(value: string): string {
    return /local summary|本地摘要|API|network|网络|choices|content/i.test(value)
      ? tt("brief.aiFallbackShort")
      : value;
  }

  function payloadForKind(item: AiProposal, kind: string): Record<string, unknown> {
    return decisionPayload(item, kind);
  }
  async function loadData() { const [start, end] = dayRange(); try { [data,activeTasks]=await Promise.all([getToday(start,end),command<Task[]>('list_tasks',{status:null,workId:null})]);activeTasks=activeTasks.filter(t=>t.status!=='done').sort((a,b)=>{const at=a.scheduled_start??a.due_at??Number.MAX_SAFE_INTEGER;const bt=b.scheduled_start??b.due_at??Number.MAX_SAFE_INTEGER;return at-bt||(a.priority==='high'?-1:0)-(b.priority==='high'?-1:0)||a.id-b.id;}); dataError = ""; } catch (e) { dataError = String(e); } }
  async function loadSync() { try { sync = await invoke("workspace_sync_status"); syncError = ""; } catch (e) { syncError = String(e); } }
  async function loadFiles() { try { recentFiles = await invoke("recent_files", { limit: 6 }); } catch { recentFiles = []; } }
  async function loadBrief() { try { const now = new Date(); briefDate = `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`; const cached = await invoke("get_morning_brief", { date: briefDate }) as { content: string } | null; brief = cached?.content ?? null; } catch { brief = null; } }
  async function loadProposals() { try { [latestProposals, works] = await Promise.all([listLatestAnalysisProposals(20), listWorks(null)]); pendingProposals = latestProposals.length; const pending=await command<AiProposal[]>("list_ai_proposals",{status:"pending",limit:500});earlierPending=pending.filter(item=>!latestProposals.some(latest=>latest.id===item.id)).length; } catch(e) { dataError=String(e); } }
  async function loadAnalysisStatus() { try { [analysisRuns, analysisSchedule] = await Promise.all([invoke<AnalysisRun[]>("list_analysis_runs", { limit: 4 }), invoke<AnalysisSchedule>("get_analysis_schedule")]); } catch { analysisRuns = []; analysisSchedule = null; } }
  async function load() { await Promise.all([loadData(), loadSync(), loadFiles(), loadBrief(), loadProposals(), loadAnalysisStatus()]); }
  async function generateBrief(force: boolean) { generating = true; briefError = ""; try { const [todayStart, todayEnd] = dayRange(); const [periodStart, periodEnd] = briefRange(); briefResult = await command("generate_brief", { date: briefDate, periodStart, periodEnd, todayStart, todayEnd, locale: currentLocale, force }) as BriefResult; brief = briefResult.content; showSources = false; } catch (e) { briefError = String(e); } generating = false; }
  async function completeTask(item: Task) { try { await invoke("complete_task", { id: item.id }); invalidate("works","tasks","waiting","calendar","brief");await loadData(); } catch (e) { dataError = String(e); } }
  async function confirmDecision(item: AiProposal) {
    decisionBusy = item.id; decisionMessage = ""; dataError = "";
    try {
      const payload = payloadForKind(item, item.kind);
      if (proposalPresentation(item,works,currentLocale).needsAttention) throw new Error(tt("calendar.invalidStart"));
      const edited = await updateAiProposalClassification(item.id, item.updated_at, item.kind, item.work_id, item.title, payload);
      Object.assign(item,edited);
      savedRoutes.set(edited.id,{kind:edited.kind,operation:edited.operation,target_id:edited.target_id});
      const receipt=await confirmAiProposal(edited.id, edited.updated_at, JSON.parse(edited.payload_json));lastReceipt=receipt.receipt_id;receiptTarget={kind:receipt.kind,id:receipt.target_id,workId:edited.work_id};
      latestProposals = latestProposals.filter((candidate) => candidate.id !== item.id);
      pendingProposals = latestProposals.length;
      const preview=proposalPresentation(edited,works,currentLocale);decisionMessage=`${currentLocale==='en-US'?'Saved:':'已完成：'} ${preview.action} · ${preview.scope}`;
      invalidate("works","tasks","waiting","calendar","inbox","proposals","brief");
      await loadData();
    } catch (e) { dataError = String(e); } finally { decisionBusy = null; }
  }
  async function deferDecision(item: AiProposal) {
    decisionBusy = item.id; decisionMessage = ""; dataError = "";
    try {
      await deferAiProposal(item.id, item.updated_at);
      latestProposals = latestProposals.filter((candidate) => candidate.id !== item.id);
      pendingProposals = latestProposals.length;
      decisionMessage = tt("dashboard.decisionDeferred");
    } catch (e) { dataError = String(e); } finally { decisionBusy = null; }
  }
  async function undoLastDecision(){if(!lastReceipt)return;decisionBusy=-1;try{await command("undo_ai_confirmation",{receiptId:lastReceipt});lastReceipt=null;decisionMessage=currentLocale==="en-US"?"Undone; suggestion returned for review.":"已撤销，建议已回到待确认列表。";await Promise.all([loadData(),loadProposals()]);}catch(e){dataError=String(e);}finally{decisionBusy=null;}}
  async function saveInterval(value: number) {
    if (!analysisSchedule || !Number.isFinite(value)) return;
    const interval = Math.max(30, Math.min(1440, Math.round(value)));
    scheduleBusy = true; dataError = "";
    try {
      analysisSchedule = await command<AnalysisSchedule>("save_analysis_schedule", { schedule: { ...analysisSchedule, interval_minutes: interval } });
      decisionMessage = tt("dashboard.intervalSaved");
    } catch (e) { dataError = String(e); } finally { scheduleBusy = false; }
  }

  const scheduledToday=$derived(activeTasks.filter(item=>item.scheduled_start&&item.scheduled_start>=dayRange()[0]&&item.scheduled_start<dayRange()[1]));
  const appointmentCount=$derived((data?.today_calendar.length??0)+scheduledToday.length);
  const briefItems = $derived(bulletLines(brief || tt("brief.notGenerated")));
  const lastCompletedAnalysis = $derived(analysisRuns.find((run) => run.status === "completed") ?? analysisRuns[0] ?? null);

  $effect(() => { $dataRevision.global; $dataRevision.brief; $dataRevision.workspace; load(); });
</script>

<div class="dashboard-c1">
  {#if dataError}<div class="module-error" role="alert"><span>{dataError}</span><button onclick={()=>{dataError='';void load();}}>{tt('common.retry')}</button></div>{/if}
  <section class="brief-hero" data-testid="dashboard-brief-hero">
    <div class="brief-eyebrow">{currentLocale==='en-US'?'YOUR WORKDAY':'今天的工作'} · {briefDate}</div>
    <h1>{currentLocale==='en-US'?'Pick up where work left off.':'接着往前做，秘书帮您理清。'}</h1>
    <p class="day-judgment">{currentLocale==='en-US'?`${appointmentCount} events today · ${pendingProposals} new decisions · ${data?.waiting_followups.length??0} follow-ups`:`今天 ${appointmentCount} 项日程 · ${pendingProposals} 条新建议 · ${data?.waiting_followups.length??0} 件等待需跟进`}</p>
    <details class="brief-details">
      <summary>{currentLocale==='en-US'?'Read secretary brief':'展开秘书简报'}</summary>
      <ul class="brief-summary" data-testid="dashboard-brief-summary">{#each briefItems as line,i(i)}<li>{line}</li>{/each}</ul>
      <div class="brief-toolbar" data-testid="dashboard-brief-actions"><button onclick={()=>generateBrief(Boolean(brief))} disabled={generating}>{generating?tt('dashboard.generating'):tt('dashboard.regenerateBrief')}</button><button onclick={()=>showSources=!showSources} disabled={!briefResult}>{tt('brief.showSources')}</button>
        <details><summary>{tt('brief.range')}</summary><div class="range-fields"><select bind:value={periodPreset}><option value="yesterday">{tt('brief.yesterday')}</option><option value="7d">{tt('brief.last7')}</option><option value="custom">{tt('brief.custom')}</option></select>{#if periodPreset==='custom'}<input type="date" bind:value={customStart} aria-label={tt('brief.start')}/><input type="date" bind:value={customEnd} aria-label={tt('brief.end')}/>{/if}</div></details>
      </div>
      {#if briefResult?.warning}<p class="notice">{briefWarningText(briefResult.warning)}</p>{/if}
      {#if briefError}<p class="notice">{briefError}</p>{/if}
      {#if showSources&&briefResult}<div class="source-drawer">{#each briefResult.source_preview.slice(0,5) as source,i(i)}<span>{source.title||source.display||source.type}</span>{/each}</div>{/if}
    </details>
  </section>
  <section class="focus-grid">
    <section class="desk-card agenda-card">
      <div class="card-head"><div><span class="eyebrow">01 · {currentLocale==='en-US'?'CONTINUE':'继续做'}</span><h2>{currentLocale==='en-US'?'Keep moving':'继续推进'}</h2><p>{currentLocale==='en-US'?'Your next actions and today’s arrangements.':'接下来要做的事，以及今天的安排。'}</p></div><button class="text-button" onclick={()=>nav('plan')}>{tt('common.all')} ↗</button></div>
      <div class="agenda-list">
        {#each (data?.today_calendar??[]).slice(0,2) as item(item.id)}
          <div class="agenda-row"><span class="agenda-time">{fmtDayTime(item.start_at)}</span><button class="agenda-body" onclick={()=>nav('calendar',item.id)}><strong>{item.title}</strong><small>{currentLocale==='en-US'?'Today’s arrangement':'今天的安排'} · {works.find(w=>w.id===item.work_id)?.title||(currentLocale==='en-US'?'Standalone':'独立事项')}</small></button></div>
        {/each}
        {#each activeTasks.slice(0,4) as item(item.id)}
          <div class="agenda-row"><span class="agenda-time">{item.scheduled_start?actionTimeLabel(item.scheduled_start,Math.floor(Date.now()/1000)):item.due_at?fmtTime(item.due_at).slice(5,10):(currentLocale==='en-US'?'Next':'下一步')}</span><button class="agenda-body" onclick={()=>nav('task',item.id)}><strong>{item.title}</strong><small>{works.find(w=>w.id===item.work_id)?.title||(currentLocale==='en-US'?'Standalone item':'独立事项')} · {translateStatus(item.status,currentLocale)}</small></button><button class="row-action" onclick={()=>completeTask(item)} aria-label={`${tt('common.complete')} ${item.title}`}>✓</button></div>
        {/each}
        {#if activeTasks.length<3}{#each (data?.continue_works??[]).slice(0,3-activeTasks.length) as entry(entry.work.id)}<div class="agenda-row"><span class="agenda-time">{currentLocale==='en-US'?'Project':'项目'}</span><button class="agenda-body" onclick={()=>nav('works',entry.work.id)}><strong>{entry.work.title}</strong><small>{entry.latest_resume?.next_step||tt('work.resumeEmpty')}</small></button></div>{/each}{/if}
        {#if data&&!activeTasks.length&&!data.today_calendar.length&&!data.continue_works.length}<div class="empty-state"><strong>{currentLocale==='en-US'?'Start with one small note':'从记下一件事开始'}</strong><p>{currentLocale==='en-US'?'Use the capture box above. The secretary can help arrange the next step.':'在上方记一件事，秘书会帮您准备下一步安排。'}</p></div>{/if}
      </div>
      {#if activeTasks.length>4}<button class="more-link" onclick={()=>nav('plan')}>{currentLocale==='en-US'?`View ${activeTasks.length-4} more actions`:`还有 ${activeTasks.length-4} 件事，查看全部`} →</button>{/if}
    </section>
    <section class="desk-card decision-card" data-testid="latest-decision-card">
      <div class="card-head"><div><span class="eyebrow">02 · {currentLocale==='en-US'?'DECIDE':'您来决定'}</span><h2>{tt('dashboard.needsDecision')}</h2><p>{currentLocale==='en-US'?'Latest arrangements, ready for your approval.':'最近一次整理的安排，看一眼就能决定。'}</p></div><button class="text-button" onclick={()=>nav('review')}>{tt('common.all')} ↗</button></div>
      {#if decisionMessage}<div class="decision-message" role="status">{decisionMessage}{#if lastReceipt&&receiptTarget}<div><button onclick={()=>nav(receiptTarget!.kind,receiptTarget!.id,receiptTarget!.workId)}>{currentLocale==='en-US'?'View item':'查看去向'}</button><button disabled={decisionBusy!==null} onclick={undoLastDecision}>{currentLocale==='en-US'?'Undo':'撤销'}</button></div>{/if}</div>{/if}
      <div class="decision-list">
        {#each latestProposals.slice(0,3) as item(item.id)}
          <article class="decision-row" data-testid={`latest-decision-${item.id}`}><h3>{item.title}</h3><ProposalPreview {item} {works}/><div class="decision-actions"><button class="primary" disabled={decisionBusy!==null||proposalPresentation(item,works,currentLocale).needsAttention} onclick={()=>confirmDecision(item)}>{currentLocale==='en-US'?'Accept':'采用安排'}</button><button onclick={()=>nav('review',item.id)}>{currentLocale==='en-US'?'Adjust':'调整'}</button><button disabled={decisionBusy!==null} onclick={()=>deferDecision(item)}>{currentLocale==='en-US'?'Later':'稍后'}</button></div></article>
        {/each}
      </div>
      {#if !latestProposals.length}<div class="empty-state"><strong>{currentLocale==='en-US'?'No new decisions for now':'眼下没有新的安排要决定'}</strong><p>{currentLocale==='en-US'?'Keep working. New suggestions will appear after organizing.':'安心推进工作，整理完成后，新建议会出现在这里。'}</p><button onclick={analyze} disabled={analysisBusy}>{analysisBusy?(currentLocale==='en-US'?'Organizing…':'后台整理中…'):(currentLocale==='en-US'?'Ask secretary to organize':'让秘书整理一次')}</button></div>{/if}
      {#if earlierPending||latestProposals.length>3}<button class="more-link" onclick={()=>nav('review')}>{currentLocale==='en-US'?`View remaining and deferred suggestions (${earlierPending+Math.max(0,latestProposals.length-3)})`:`还有 ${earlierPending+Math.max(0,latestProposals.length-3)} 条其他或暂缓建议，继续查看`} →</button>{/if}
    </section>
  </section>
  <section class="attention-strip">
    <div><strong>{currentLocale==='en-US'?'Worth a look':'值得留意'}</strong><p>{(data?.waiting_followups??[]).slice(0,2).map(item=>item.title).join(' · ')||(currentLocale==='en-US'?'No waiting items need following up today.':'今天暂无到期的等待事项。')}</p></div><button onclick={()=>nav('waiting')}>{currentLocale==='en-US'?'View waiting':'查看等待'} ↗</button>
    {#if data?.inbox_pending.length}<button onclick={()=>nav('inbox')}>{data.inbox_pending.length} {currentLocale==='en-US'?'notes to organize':'条记录待整理'} ↗</button>{/if}
  </section>
  <details class="secretary-line" data-testid="dashboard-secretary-card"><summary>{currentLocale==='en-US'?'Secretary':'秘书'} · {analysisSchedule?.enabled?`${currentLocale==='en-US'?'Every':'每'} ${analysisSchedule.interval_minutes} ${tt('settings.minutes')}`:(currentLocale==='en-US'?'Periodic organizing paused':'周期整理已暂停')} · {currentLocale==='en-US'?'Adjust rhythm':'调整节奏'}</summary>
    <div class="secretary-controls"><label>{tt('dashboard.analysisRhythm')}<select data-testid="dashboard-analysis-interval" value={String(analysisSchedule?.interval_minutes??180)} onchange={event=>saveInterval(Number(event.currentTarget.value))} disabled={!analysisSchedule||scheduleBusy}>{#each [30,60,180,360,720,1440] as minutes}<option value={String(minutes)}>{minutes} {tt('settings.minutes')}</option>{/each}{#if analysisSchedule&&![30,60,180,360,720,1440].includes(analysisSchedule.interval_minutes)}<option value={String(analysisSchedule.interval_minutes)}>{analysisSchedule.interval_minutes} {tt('settings.minutes')}</option>{/if}</select></label><button onclick={()=>nav('settings')}>{currentLocale==='en-US'?'More settings':'更多设置'}</button><button onclick={analyze} disabled={analysisBusy}>{currentLocale==='en-US'?'Organize now':'现在整理'}</button></div>
    <p>{tt('dashboard.lastAnalysis')}：{lastCompletedAnalysis?fmtTime(lastCompletedAnalysis.finished_at||lastCompletedAnalysis.started_at):'—'}</p>
    <details><summary>{currentLocale==='en-US'?'Directory changes':'查看目录变化'}</summary>{#each recentFiles as file(file.path+file.timestamp)}<p>{file.path.split(/[\\/]/).pop()} · {file.event_type}</p>{/each}<button onclick={()=>nav('workspace')}>{tt('common.all')}</button></details>
    {#if syncError}<p class="notice">{syncError}</p>{/if}
  </details>
</div>
<style>
  .dashboard-c1{display:grid;gap:20px;padding-bottom:12px;min-width:0}button{cursor:pointer;font:inherit;font-size:14px;padding:9px 13px;border:1px solid var(--color-border);border-radius:9px;background:var(--color-surface);color:var(--color-primary)}button:disabled{opacity:.5;cursor:default}summary{cursor:pointer;font-size:14px;line-height:1.7}p{line-height:1.65}
  .brief-hero{padding:24px 28px;border:1px solid var(--color-border);border-radius:20px;background:linear-gradient(115deg,var(--color-surface),var(--color-primary-soft));min-width:0}.brief-eyebrow,.eyebrow{color:var(--color-muted);font-size:12px;letter-spacing:.08em;font-weight:600}.brief-hero h1{font:500 clamp(25px,2.7vw,34px) var(--font-serif);line-height:1.5;margin:10px 0}.day-judgment{margin:0 0 10px;color:var(--color-muted);font-size:15px}.brief-details>summary{color:var(--color-primary)}.brief-summary{display:grid;gap:9px;margin:16px 0;padding-left:22px;font-size:15px;line-height:1.7}.brief-toolbar,.range-fields{display:flex;flex-wrap:wrap;align-items:center;gap:9px}.range-fields{padding:10px 0}.range-fields input,.range-fields select{padding:8px}.notice{font-size:14px;color:var(--color-warning);overflow-wrap:anywhere}.source-drawer{display:flex;gap:8px;flex-wrap:wrap;margin-top:10px;font-size:13px;color:var(--color-muted)}
  .focus-grid{display:grid;grid-template-columns:minmax(0,1fr) minmax(0,1fr);gap:20px;align-items:start}.desk-card{border:1px solid var(--color-border);border-radius:18px;background:var(--color-surface);padding:24px;min-width:0;box-shadow:var(--shadow-sm)}.card-head{display:flex;justify-content:space-between;gap:14px;align-items:start;margin-bottom:15px}.card-head h2{font-size:22px;margin:7px 0;line-height:1.4}.card-head p{color:var(--color-muted);font-size:14px;margin:0}.text-button{border:0;padding:5px;white-space:nowrap;background:transparent}
  .agenda-row{display:grid;grid-template-columns:48px minmax(0,1fr) auto;gap:12px;align-items:start;padding:18px 0;border-bottom:1px solid var(--color-border)}.agenda-row:last-child{border-bottom:0}.agenda-time{white-space:pre-line;font-size:13px;color:var(--color-muted);padding-top:5px}.agenda-body{display:grid;gap:8px;text-align:left;border:0;padding:0;background:transparent;min-width:0}.agenda-body strong{font-size:18px;line-height:1.55;overflow-wrap:anywhere;color:var(--color-text)}.agenda-body small{font-size:14px;line-height:1.6;color:var(--color-muted)}.row-action{min-width:33px;padding:6px}
  .decision-list{display:grid;gap:14px}.decision-row{padding:17px;border:1px solid var(--color-border);border-radius:13px;background:var(--color-surface-muted);min-width:0}.decision-row h3{font-size:18px;line-height:1.6;margin:0 0 8px;overflow-wrap:anywhere}.decision-actions{display:flex;flex-wrap:wrap;gap:8px;margin-top:14px}.primary{background:var(--color-primary);color:white;border-color:var(--color-primary)}.decision-message{padding:12px;margin-bottom:12px;border-radius:10px;background:var(--color-success-soft);font-size:14px;line-height:1.6}.decision-message>div{display:flex;gap:8px;margin-top:8px}.empty-state{padding:32px 8px;line-height:1.6}.empty-state strong{font-size:17px}.empty-state p{font-size:14px;color:var(--color-muted);margin:10px 0 16px}.more-link{display:block;width:100%;text-align:left;margin-top:15px;border:0;background:var(--color-primary-soft);font-size:14px;line-height:1.6}
  .attention-strip{display:flex;flex-wrap:wrap;gap:16px;align-items:center;padding:18px 22px;border:1px solid var(--color-border);border-radius:14px;background:var(--color-surface)}.attention-strip>div{flex:1 1 320px;min-width:0}.attention-strip strong{font-size:15px}.attention-strip p{margin:5px 0 0;font-size:14px;color:var(--color-muted);overflow-wrap:anywhere}.secretary-line{padding:4px 8px;color:var(--color-muted);min-width:0}.secretary-controls{display:flex;flex-wrap:wrap;gap:12px;align-items:end;padding:16px 0}.secretary-controls label{display:grid;gap:8px;font-size:14px}.secretary-controls select{padding:9px}.secretary-line p{font-size:14px;overflow-wrap:anywhere}.module-error{display:flex;flex-wrap:wrap;gap:12px;align-items:center;background:var(--color-danger-soft);border-radius:12px;padding:14px;color:var(--color-danger);font-size:14px}.module-error span{flex:1 1 300px;overflow-wrap:anywhere}
  @container(max-width:850px){.focus-grid{grid-template-columns:1fr}.brief-hero{padding:20px}.desk-card{padding:20px}}@container(max-width:500px){.agenda-row{grid-template-columns:38px minmax(0,1fr) auto;gap:8px}.card-head h2{font-size:21px}}
</style>
