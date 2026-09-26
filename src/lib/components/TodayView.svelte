<script lang="ts">
 import DashboardOverview from './DashboardOverview.svelte';
 import {dashboardGreeting,fileChangeLabel} from '$lib/services/dashboardPresentation';
 import StatusLine from "$lib/components/ui/StatusLine.svelte";
 import ManualCompletionFeedback from './ManualCompletionFeedback.svelte';
 import {completeManual} from '$lib/stores/manualCompletions';
 import {actionableTasks,taskReason} from '$lib/services/workflowContinuity';
  import ProposalPreview from './ProposalPreview.svelte';
  import ProposalLifecycleDialog from './ProposalLifecycleDialog.svelte';
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
  let deleteCandidate = $state<AiProposal|null>(null);
  let decisionMessage = $state("");
  let lastReceipt=$state<string|null>(null);
  let earlierPending=$state(0);
  let scheduleBusy = $state(false);
  let briefDate = $state(localDate());
  let briefSubmitting = $state(false);
  const generating = $derived(briefSubmitting || $aiJobs.some(job=>job.command==='generate_brief'&&job.status==='running'));
  let briefExpanded = $state(false);
  let showSources = $state(false);
  let periodPreset = $state<"yesterday" | "7d" | "custom">("yesterday");
  let customStart = $state("");
  let customEnd = $state("");
  let analysisRuns = $state<AnalysisRun[]>([]);
  let analysisSchedule = $state<AnalysisSchedule | null>(null);
  let currentLocale = $derived($locale);
  const greeting = $derived(dashboardGreeting(currentLocale));
  const tt = (key: Parameters<typeof t>[0], params: Record<string, string | number> = {}) => t(key, params, currentLocale);

  function pad(n: number) { return String(n).padStart(2, "0"); }
  function localDate() { const now=new Date();return `${now.getFullYear()}-${pad(now.getMonth()+1)}-${pad(now.getDate())}`; }
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
  async function loadData() { const [start, end] = dayRange(); try { const [today,tasks,projects]=await Promise.all([getToday(start,end),command<Task[]>('list_tasks',{status:null,workId:null}),listWorks(null)]);data=today;works=projects;activeTasks=actionableTasks(tasks,projects); dataError = ""; } catch (e) { dataError = String(e); } }
  async function loadSync() { try { sync = await invoke("workspace_sync_status"); syncError = ""; } catch (e) { syncError = String(e); } }
  async function loadFiles() { try { recentFiles = await invoke("recent_files", { limit: 6 }); } catch { recentFiles = []; } }
  async function loadBrief() {
    try {
      briefDate=localDate();
      const cached=await invoke("get_morning_brief",{date:briefDate}) as {content:string}|null;
      brief=cached?.content??null;
      const completed=$aiJobs.find(job=>job.command==='generate_brief'&&job.status==='completed'&&job.args.date===briefDate&&(job.result as BriefResult|null)?.content===brief);
      briefResult=completed?completed.result as BriefResult:null;
    } catch {brief=null;briefResult=null;}
  }
  async function loadProposals() { try { [latestProposals, works] = await Promise.all([listLatestAnalysisProposals(20), listWorks(null)]); pendingProposals = latestProposals.length; const pending=await command<AiProposal[]>("list_ai_proposals",{status:"pending",limit:500});earlierPending=pending.filter(item=>!latestProposals.some(latest=>latest.id===item.id)).length; } catch(e) { dataError=String(e); } }
  async function loadAnalysisStatus() { try { [analysisRuns, analysisSchedule] = await Promise.all([invoke<AnalysisRun[]>("list_analysis_runs", { limit: 4 }), invoke<AnalysisSchedule>("get_analysis_schedule")]); } catch { analysisRuns = []; analysisSchedule = null; } }
  async function load() { await Promise.all([loadData(), loadSync(), loadFiles(), loadBrief(), loadProposals(), loadAnalysisStatus()]); }
  async function generateBrief(force: boolean) {
    if(generating)return;
    briefSubmitting = true; briefExpanded = true; briefError = "";
    try {
      if(periodPreset==='custom'&&(!customStart||!customEnd||customStart>customEnd))throw new Error(currentLocale==='en-US'?'Choose a valid start and end date.':'请选择有效的起止日期，开始日期不能晚于结束日期。');
      briefDate=localDate();
      const [todayStart,todayEnd]=dayRange(),[periodStart,periodEnd]=briefRange();
      briefResult=await command("generate_brief",{date:briefDate,periodStart,periodEnd,todayStart,todayEnd,locale:currentLocale,force}) as BriefResult;
      brief=briefResult.content;showSources=false;
    } catch(e){briefError=String(e);} finally {briefSubmitting=false;}
  }
  async function completeTask(item: Task) { try { await completeManual('task',item.id);await loadData(); } catch (e) { dataError = String(e); } }
  async function confirmDecision(item: AiProposal) {
    if(decisionBusy!==null)return;
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
    if(decisionBusy!==null)return;
    decisionBusy = item.id; decisionMessage = ""; dataError = "";
    try {
      await deferAiProposal(item.id, item.updated_at);
      latestProposals = latestProposals.filter((candidate) => candidate.id !== item.id);
      pendingProposals = latestProposals.length;
      decisionMessage = tt("dashboard.decisionDeferred");
    } catch (e) { dataError = String(e); } finally { decisionBusy = null; }
  }
  async function undoLastDecision(){if(!lastReceipt)return;decisionBusy=-1;try{await command("undo_ai_confirmation",{receiptId:lastReceipt});lastReceipt=null;decisionMessage=currentLocale==="en-US"?"Undone; suggestion returned for review.":"已撤销，建议已回到待确认列表。";await Promise.all([loadData(),loadProposals()]);}catch(e){dataError=String(e);}finally{decisionBusy=null;}}
  async function reviewDeleted(){
    lastReceipt=null;receiptTarget=null;
    decisionMessage=currentLocale==='en-US'?'Review record removed. Existing work items remain unchanged.':'审阅记录已删除，已有工作事项保持原样。';
    await loadProposals();
  }
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
  const briefItems = $derived(brief ? bulletLines(brief) : []);
  const briefHighlights = $derived(briefItems.filter(line =>
    !/^(?:今日日程|等待与阻塞|当前无|today.s calendar|waiting and blockers)[:：]?\s*(?:无|none|no )/i.test(line)
  ).slice(0, 3));
  const lastCompletedAnalysis = $derived(analysisRuns.find((run) => run.status === "completed") ?? analysisRuns[0] ?? null);

  $effect(() => { $dataRevision.global; $dataRevision.brief; $dataRevision.workspace; load(); });
</script>

<div class="dashboard-c1">
  <section class="dashboard-intro" data-testid="dashboard-brief-hero">
    <div class="intro-copy">
      <div class="brief-eyebrow">{currentLocale==='en-US'?'TODAY’S WORKSPACE':'今日工作台'} <span>· {briefDate}</span></div>
      <h1>{greeting.title}</h1>
      <div class="intro-actions"><p class="day-judgment">{currentLocale==='en-US'?`${appointmentCount} arrangements today · ${pendingProposals} new suggestions to review`:`今天 ${appointmentCount} 项日程，${pendingProposals} 条新建议待确认。`}</p><button class="brief-generate" data-testid="generate-daily-brief" onclick={()=>generateBrief(Boolean(brief))} disabled={generating} aria-busy={generating}><Icon name="reports" size={16}/><span>{generating?(currentLocale==='en-US'?'Preparing brief…':'简报整理中…'):(currentLocale==='en-US'?'Prepare daily brief':'整理每日简报')}</span></button></div>
      <ManualCompletionFeedback error={dataError} onrefresh={loadData}/>
    </div>
    <aside class="quiet-note" aria-label={currentLocale==='en-US'?'Our philosophy':'我们的理念'}>
      <p>{greeting.note}</p>
      <span>{currentLocale==='en-US'?'YOUR MEDICAL AFFAIRS WORKSPACE':'把时间留给值得跟进的事'}</span>
      <svg viewBox="0 0 350 120" preserveAspectRatio="xMaxYMax meet" aria-hidden="true"><path d="M0 120L69 76L102 97L189 27L227 56L281 7L350 60V120Z" fill="currentColor" opacity=".09"/><path d="M135 120L220 69L244 85L281 7L315 48L350 27V120Z" fill="currentColor" opacity=".13"/><path d="M240 87L281 7L275 45L290 49L274 55Z" fill="white" opacity=".85"/></svg>
    </aside>
  </section>
  <section class="brief-rail">
    <details class="brief-details" bind:open={briefExpanded}>
      <summary>{currentLocale==='en-US'?'Secretary brief':'秘书简报'}{#if generating}<span class="brief-progress" role="status">{currentLocale==='en-US'?'Preparing in background':'正在后台整理'}</span>{/if}</summary>
      {#if briefHighlights.length}
        <ul class="brief-highlights" data-testid="dashboard-brief-highlights">{#each briefHighlights as line,i(i)}<li title={line}>{line}</li>{/each}</ul>
        {#if briefItems.length>briefHighlights.length}
          <details class="brief-full"><summary>{currentLocale==='en-US'?`Read all ${briefItems.length} points`:`查看全部 ${briefItems.length} 条`}</summary><ul class="brief-summary" data-testid="dashboard-brief-summary">{#each briefItems as line,i(i)}<li>{line}</li>{/each}</ul></details>
        {/if}
      {:else}<p class="brief-empty">{generating?(currentLocale==='en-US'?'You can keep working while your brief is prepared.':'简报正在整理，您可以继续处理其他事项。'):tt('brief.notGenerated')}</p>{/if}
      <div class="brief-toolbar" data-testid="dashboard-brief-actions"><button onclick={()=>showSources=!showSources} disabled={!briefResult}>{tt('brief.showSources')}</button>
        <details><summary>{tt('brief.range')}</summary><div class="range-fields"><select bind:value={periodPreset} aria-label={tt('brief.range')}><option value="yesterday">{tt('brief.yesterday')}</option><option value="7d">{tt('brief.last7')}</option><option value="custom">{tt('brief.custom')}</option></select>{#if periodPreset==='custom'}<input type="date" bind:value={customStart} aria-label={tt('brief.start')}/><input type="date" bind:value={customEnd} aria-label={tt('brief.end')}/>{/if}</div></details>
      </div>
      {#if briefResult?.warning}<p class="notice">{briefWarningText(briefResult.warning)}</p>{/if}
      {#if briefError}<p class="notice" role="alert">{briefError}</p>{/if}
      {#if showSources&&briefResult}<div class="source-drawer">{#each briefResult.source_preview.slice(0,5) as source,i(i)}<span>{source.title||source.display||source.type}</span>{/each}</div>{/if}
    </details>
  </section>
  <DashboardOverview en={currentLocale==='en-US'} counts={{
    projects:data?works.filter(work=>!work.archived_at&&work.status!=='archived').length:null,
    tasks:data?activeTasks.length:null,
    waiting:data?data.waiting_followups.length:null,
    calendar:data?appointmentCount:null,
    inbox:data?data.inbox_pending.length:null,
    decisions:data?pendingProposals+earlierPending:null
  }}/>
  <section class="focus-grid">
    <section class="desk-card agenda-card">
      <div class="card-head"><div><span class="eyebrow">01 · {currentLocale==='en-US'?'CONTINUE':'继续做'}</span><h2>{currentLocale==='en-US'?'Keep moving':'继续推进'}</h2><p>{currentLocale==='en-US'?'Next actions, connected to your projects.':'下一步行动，连着对应的项目。'}</p></div><button class="text-button" onclick={()=>nav('plan')}>{tt('common.all')} ↗</button></div>
      <div class="agenda-list">
        {#each activeTasks.slice(0,4) as item(item.id)}
          <div class="agenda-row"><span class="agenda-time">{item.scheduled_start?actionTimeLabel(item.scheduled_start,Math.floor(Date.now()/1000)):item.due_at?fmtTime(item.due_at).slice(5,10):(currentLocale==='en-US'?'Next':'下一步')}</span><button class="agenda-body" onclick={()=>nav('task',item.id,item.work_id)}><strong>{item.title}</strong><small>{works.find(w=>w.id===item.work_id)?.title||(currentLocale==='en-US'?'Standalone item':'独立事项')} · {taskReason(item,Math.floor(Date.now()/1000),currentLocale==='en-US')}</small></button><button class="row-action" onclick={()=>completeTask(item)} aria-label={`${tt('common.complete')} ${item.title}`}>✓</button></div>
        {/each}
        {#if activeTasks.length<3}{#each (data?.continue_works??[]).slice(0,3-activeTasks.length) as entry(entry.work.id)}<div class="agenda-row"><span class="agenda-time">{currentLocale==='en-US'?'Project':'项目'}</span><button class="agenda-body" onclick={()=>nav('works',entry.work.id)}><strong>{entry.work.title}</strong><small>{entry.latest_resume?.next_step||tt('work.resumeEmpty')}</small></button></div>{/each}{/if}
        {#if data&&!activeTasks.length&&!data.continue_works.length}<div class="empty-state"><strong>{currentLocale==='en-US'?'Start with one small note':'从记下一件事开始'}</strong><p>{currentLocale==='en-US'?'Use the capture box above. The secretary can help arrange the next step.':'在上方记一件事，秘书会帮您准备下一步安排。'}</p></div>{/if}
      </div>
      {#if activeTasks.length>4}<button class="more-link" onclick={()=>nav('plan')}>{currentLocale==='en-US'?`View ${activeTasks.length-4} more actions`:`还有 ${activeTasks.length-4} 件事，查看全部`} →</button>{/if}
    </section>
    <section class="desk-card decision-card" data-testid="latest-decision-card">
      <div class="card-head"><div><span class="eyebrow">02 · {currentLocale==='en-US'?'DECIDE':'您来决定'}</span><h2>{tt('dashboard.needsDecision')}</h2><p>{currentLocale==='en-US'?'Latest arrangements, ready for your approval.':'最近一次整理的安排，看一眼就能决定。'}</p></div><button class="text-button" onclick={()=>nav('review')}>{tt('common.all')} ↗</button></div>
      {#if decisionMessage}<div class="decision-message" role="status">{decisionMessage}{#if lastReceipt&&receiptTarget}<div><button onclick={()=>nav(receiptTarget!.kind,receiptTarget!.id,receiptTarget!.workId)}>{currentLocale==='en-US'?'View item':'查看去向'}</button><button disabled={decisionBusy!==null} onclick={undoLastDecision}>{currentLocale==='en-US'?'Undo':'撤销'}</button></div>{/if}</div>{/if}
      <div class="decision-list">
        {#each latestProposals.slice(0,3) as item(item.id)}
          <article class="decision-row" data-testid={`latest-decision-${item.id}`}><h3>{item.title}</h3><ProposalPreview {item} {works} compact/><div class="decision-actions"><button class="primary" disabled={decisionBusy!==null||proposalPresentation(item,works,currentLocale).needsAttention} onclick={()=>confirmDecision(item)}>{currentLocale==='en-US'?'Accept':'采用安排'}</button><button disabled={decisionBusy!==null} onclick={()=>nav('review',item.id)}>{currentLocale==='en-US'?'Adjust':'调整'}</button><button disabled={decisionBusy!==null} onclick={()=>deferDecision(item)}>{currentLocale==='en-US'?'Later':'稍后'}</button><details class="decision-more"><summary data-testid={`decision-more-${item.id}`} aria-label={currentLocale==='en-US'?'Other actions for this suggestion':'这条建议的其他操作'}>{currentLocale==='en-US'?'More':'更多'}</summary><div><button data-testid={`decision-delete-${item.id}`} disabled={decisionBusy!==null} onclick={event=>{event.currentTarget.closest('details')?.removeAttribute('open');deleteCandidate={...item};}}>{currentLocale==='en-US'?'Delete record':'删除记录'}</button></div></details></div></article>
        {/each}
      </div>
      {#if !latestProposals.length}<div class="empty-state"><strong>{currentLocale==='en-US'?'No new decisions for now':'眼下没有新的安排要决定'}</strong><p>{currentLocale==='en-US'?'Keep working. New suggestions will appear after organizing.':'安心推进工作，整理完成后，新建议会出现在这里。'}</p><button onclick={analyze} disabled={analysisBusy}>{analysisBusy?(currentLocale==='en-US'?'Organizing…':'后台整理中…'):(currentLocale==='en-US'?'Ask secretary to organize':'让秘书整理一次')}</button></div>{/if}
      {#if earlierPending||latestProposals.length>3}<button class="more-link" onclick={()=>nav('review')}>{currentLocale==='en-US'?`View remaining and deferred suggestions (${earlierPending+Math.max(0,latestProposals.length-3)})`:`还有 ${earlierPending+Math.max(0,latestProposals.length-3)} 条其他或暂缓建议，继续查看`} →</button>{/if}
    </section>

    <aside class="dashboard-side">
      <section class="desk-card schedule-card">
        <div class="small-card-head"><h2>{currentLocale==='en-US'?'Today’s calendar':'今日日程'}</h2><button class="text-button" onclick={()=>nav('calendar')} aria-label={currentLocale==='en-US'?'Open calendar':'打开日历'}><Icon name="chevron-right" size={16}/></button></div>
        <div class="schedule-date">{briefDate}</div>
        {#each (data?.today_calendar??[]).slice(0,2) as item(item.id)}
          <div class="agenda-row"><span class="agenda-time">{fmtDayTime(item.start_at)}</span><button class="agenda-body" onclick={()=>nav('calendar',item.id)}><strong>{item.title}</strong><small>{currentLocale==='en-US'?'Today’s arrangement':'今天的安排'} · {works.find(w=>w.id===item.work_id)?.title||(currentLocale==='en-US'?'Standalone':'独立事项')}</small></button></div>
        {/each}
        {#each scheduledToday.slice(0,2) as task(task.id)}
          <button class="schedule-item" onclick={()=>nav('task',task.id)}><span>{fmtDayTime(task.scheduled_start!)}</span><strong>{task.title}</strong></button>
        {/each}
        {#if data&&!appointmentCount}<p class="secondary-empty">{currentLocale==='en-US'?'No fixed arrangements today.':'今天暂无固定安排，可以按自己的节奏推进。'}</p>{/if}
      </section>
      <section class="desk-card files-card">
        <div class="small-card-head"><h2>{currentLocale==='en-US'?'Recent materials':'资料动态'}</h2><button class="text-button" onclick={()=>nav('workspace')} aria-label={currentLocale==='en-US'?'Open directories':'打开工作目录'}><Icon name="chevron-right" size={16}/></button></div>
        {#each recentFiles.slice(0,3) as file(file.path+file.timestamp)}
          <button class="material-row" onclick={()=>navigateTo({view:'workspace',filePath:file.path})} title={file.path.split(/[\\/]/).pop()}>
            <span class="file-icon"><Icon name="file" size={20}/></span><span><strong>{file.path.split(/[\\/]/).pop()}</strong><small>{fmtTime(file.timestamp).slice(5)} · {fileChangeLabel(file.event_type,currentLocale)}</small></span>
          </button>
        {:else}<p class="secondary-empty">{currentLocale==='en-US'?'Changes in linked project folders will appear here.':'关联项目目录后，在这里查看资料变化。'}</p>{/each}
      </section>
    </aside>
  </section>
  <section class="attention-strip">
    <div><strong>{currentLocale==='en-US'?'Worth a look':'值得留意'}</strong><p>{(data?.waiting_followups??[]).slice(0,2).map(item=>item.title).join(' · ')||(currentLocale==='en-US'?'No waiting items need following up today.':'今天暂无到期的等待事项。')}</p></div><button onclick={()=>nav('waiting')}>{currentLocale==='en-US'?'View waiting':'查看等待'} ↗</button>
    {#if data?.inbox_pending.length}<button onclick={()=>nav('inbox')}>{data.inbox_pending.length} {currentLocale==='en-US'?'notes to organize':'条记录待整理'} ↗</button>{/if}
  </section>
  <details class="secretary-line" data-testid="dashboard-secretary-card"><summary>{currentLocale==='en-US'?'Secretary':'秘书'} · {analysisSchedule?.enabled?`${currentLocale==='en-US'?'Every':'每'} ${analysisSchedule.interval_minutes} ${tt('settings.minutes')}`:(currentLocale==='en-US'?'Periodic organizing paused':'周期整理已暂停')} · {currentLocale==='en-US'?'Adjust rhythm':'调整节奏'}</summary>
    <div class="secretary-controls"><label>{tt('dashboard.analysisRhythm')}<select data-testid="dashboard-analysis-interval" value={String(analysisSchedule?.interval_minutes??180)} onchange={event=>saveInterval(Number(event.currentTarget.value))} disabled={!analysisSchedule||scheduleBusy}>{#each [30,60,180,360,720,1440] as minutes}<option value={String(minutes)}>{minutes} {tt('settings.minutes')}</option>{/each}{#if analysisSchedule&&![30,60,180,360,720,1440].includes(analysisSchedule.interval_minutes)}<option value={String(analysisSchedule.interval_minutes)}>{analysisSchedule.interval_minutes} {tt('settings.minutes')}</option>{/if}</select></label><button onclick={()=>nav('settings')}>{currentLocale==='en-US'?'More settings':'更多设置'}</button><button onclick={analyze} disabled={analysisBusy}>{currentLocale==='en-US'?'Organize now':'现在整理'}</button></div>
    <p>{tt('dashboard.lastAnalysis')}：{lastCompletedAnalysis?fmtTime(lastCompletedAnalysis.finished_at||lastCompletedAnalysis.started_at):'—'}</p>
    <details><summary>{currentLocale==='en-US'?'Directory changes':'查看目录变化'}</summary>{#each recentFiles as file(file.path+file.timestamp)}<p>{file.path.split(/[\\/]/).pop()} · {fileChangeLabel(file.event_type,currentLocale)}</p>{/each}<button onclick={()=>nav('workspace')}>{tt('common.all')}</button></details>
    {#if syncError}<p class="notice">{syncError}</p>{/if}
  </details>
</div>
<ProposalLifecycleDialog item={deleteCandidate} action="delete" onclose={()=>deleteCandidate=null} oncomplete={reviewDeleted} onbusychange={value=>decisionBusy=value?(deleteCandidate?.id??-1):null}/>
<style>
  .decision-more{position:relative;margin-left:auto;align-self:center;min-width:0}
  .decision-more>summary{padding:9px 8px;min-height:40px;display:flex;align-items:center;list-style:none;color:var(--color-muted);border-radius:9px}
  .decision-more>summary:hover{background:var(--color-primary-soft)}
  .decision-more>summary:focus-visible{outline:2px solid var(--color-primary);outline-offset:2px}
  .decision-more>div{position:absolute;z-index:10;right:0;bottom:calc(100% + 6px);min-width:136px;padding:5px;border:1px solid var(--color-border);border-radius:10px;background:var(--color-surface);box-shadow:var(--shadow-md)}
  .decision-more>div button{width:100%;color:var(--color-danger);border:0;text-align:left}
  .dashboard-c1{display:grid;gap:16px;padding-bottom:12px;min-width:0}button{cursor:pointer;font:inherit;font-size:14px;padding:9px 13px;border:1px solid var(--color-border);border-radius:9px;background:var(--color-surface);color:var(--color-primary)}button:disabled{opacity:.5;cursor:default}summary{cursor:pointer;font-size:14px;line-height:1.7}p{line-height:1.65}
  .dashboard-intro{display:grid;grid-template-columns:minmax(0,1fr) 310px;gap:28px;align-items:center;min-width:0;padding:2px 2px 6px}.intro-copy{min-width:0}.brief-eyebrow,.eyebrow{font-size:12px;font-weight:600;color:var(--color-muted);letter-spacing:.055em}.brief-eyebrow span{font-weight:400;letter-spacing:0}.intro-copy h1{font-size:clamp(24px,2vw,28px);font-weight:650;line-height:1.5;margin:12px 0 8px;letter-spacing:-.03em}.day-judgment{font-size:14px;margin:0;color:var(--color-muted)}
  .quiet-note{position:relative;isolation:isolate;min-height:100px;padding:16px 20px;border:1px solid var(--color-border);border-radius:10px;background:linear-gradient(120deg,var(--color-surface-muted),var(--color-primary-soft));overflow:hidden}.quiet-note p{position:relative;z-index:1;max-width:215px;margin:0 0 7px;font-size:15px;line-height:1.8;font-weight:500}.quiet-note span{position:relative;z-index:1;font-size:11px;letter-spacing:.04em;color:var(--color-muted)}.quiet-note svg{position:absolute;right:0;bottom:0;width:210px;height:105px;color:var(--color-primary);z-index:0}
  .brief-rail{border:1px solid var(--color-border);border-radius:10px;background:var(--color-surface);padding:12px 18px}.brief-details>summary{color:var(--color-primary);font-weight:550}.brief-highlights{display:grid;gap:8px;margin:14px 0 10px;padding-left:20px;font-size:14px;line-height:1.7}.brief-highlights li{overflow-wrap:anywhere}.brief-full>summary{font-size:13px;color:var(--color-muted)}.brief-summary{display:grid;gap:9px;max-height:300px;overflow:auto;scrollbar-gutter:stable;margin:12px 0;padding:0 12px 0 22px;font-size:15px;line-height:1.7;overflow-wrap:anywhere}.brief-empty{font-size:14px;color:var(--color-muted)}.brief-toolbar,.range-fields{display:flex;flex-wrap:wrap;align-items:center;gap:9px;margin-top:12px}.range-fields{padding:10px 0}.range-fields input,.range-fields select{padding:8px}.notice{font-size:14px;color:var(--color-warning);overflow-wrap:anywhere}.source-drawer{display:flex;gap:8px;flex-wrap:wrap;margin-top:10px;font-size:13px;color:var(--color-muted)}
  .focus-grid{display:grid;grid-template-columns:minmax(0,1.15fr) minmax(0,1.25fr) minmax(215px,.8fr);gap:16px;align-items:stretch}.desk-card{border:1px solid var(--color-border);border-radius:12px;background:var(--color-surface);padding:20px;min-width:0;box-shadow:var(--shadow-sm)}.agenda-card,.decision-card{display:flex;flex-direction:column}.agenda-card>.more-link,.decision-card>.more-link{margin-top:auto}.decision-card>.empty-state{flex:1}.card-head{display:flex;justify-content:space-between;gap:14px;align-items:start;margin-bottom:15px}.card-head h2{font-size:20px;margin:7px 0;line-height:1.4}.card-head p{color:var(--color-muted);font-size:14px;margin:0}.text-button{border:0;padding:5px;white-space:nowrap;background:transparent}
  .agenda-row{display:grid;grid-template-columns:48px minmax(0,1fr) auto;gap:12px;align-items:start;padding:16px 0;border-bottom:1px solid var(--color-border)}.agenda-row:last-child{border-bottom:0}.agenda-time{white-space:pre-line;font-size:13px;color:var(--color-muted);padding-top:5px}.agenda-body{display:grid;gap:8px;text-align:left;border:0;padding:0;background:transparent;min-width:0}.agenda-body strong{font-size:16px;line-height:1.55;overflow-wrap:anywhere;color:var(--color-text)}.agenda-body small{font-size:13px;line-height:1.6;color:var(--color-muted)}.row-action{min-width:33px;padding:6px}
  .decision-list{display:grid;gap:14px}.decision-row{padding:14px;border:1px solid var(--color-border);border-radius:9px;background:var(--color-surface-muted);min-width:0}.decision-row h3{font-size:16px;line-height:1.6;margin:0 0 8px;overflow-wrap:anywhere}.decision-actions{display:flex;flex-wrap:wrap;gap:6px;margin-top:12px}.decision-actions>button{padding:8px 10px;font-size:13px}.primary{background:var(--color-primary);color:white;border-color:var(--color-primary)}.decision-message{padding:12px;margin-bottom:12px;border-radius:10px;background:var(--color-success-soft);font-size:14px;line-height:1.6}.decision-message>div{display:flex;gap:8px;margin-top:8px}.empty-state{padding:32px 8px;line-height:1.6}.empty-state strong{font-size:17px}.empty-state p{font-size:14px;color:var(--color-muted);margin:10px 0 16px}.more-link{display:block;width:100%;text-align:left;margin-top:15px;border:0;background:var(--color-primary-soft);font-size:14px;line-height:1.6}
  .attention-strip{display:flex;flex-wrap:wrap;gap:16px;align-items:center;padding:18px 22px;border:1px solid var(--color-border);border-radius:14px;background:var(--color-surface)}.attention-strip>div{flex:1 1 320px;min-width:0}.attention-strip strong{font-size:15px}.attention-strip p{margin:5px 0 0;font-size:14px;color:var(--color-muted);overflow-wrap:anywhere}.secretary-line{padding:4px 8px;color:var(--color-muted);min-width:0}.secretary-controls{display:flex;flex-wrap:wrap;gap:12px;align-items:end;padding:16px 0}.secretary-controls label{display:grid;gap:8px;font-size:14px}.secretary-controls select{padding:9px}.secretary-line p{font-size:14px;overflow-wrap:anywhere}

  .dashboard-side{display:grid;gap:16px;min-width:0}.small-card-head{display:flex;justify-content:space-between;align-items:center;gap:8px;margin-bottom:12px}.small-card-head h2{font-size:16px;margin:0}.schedule-date{font-size:12px;color:var(--color-muted);background:var(--color-surface-muted);border-radius:5px;padding:7px 9px}.schedule-card .agenda-row{grid-template-columns:40px minmax(0,1fr);gap:8px;padding:12px 0}.schedule-card .agenda-body strong{font-size:14px;font-weight:550}.schedule-card .agenda-body small{font-size:12px}.schedule-item{display:grid;gap:6px;text-align:left;width:100%;padding:12px 0 12px 10px;border:0;border-left:2px solid var(--color-primary);border-radius:0;margin-top:14px;background:transparent}.schedule-item span{font-size:12px;color:var(--color-muted)}.schedule-item strong{font-size:14px;line-height:1.6;color:var(--color-text)}.secondary-empty{font-size:13px;color:var(--color-muted);line-height:1.8;margin:10px 0 0}
  .material-row{display:flex;gap:10px;align-items:center;width:100%;padding:12px 0;border:0;border-bottom:1px solid var(--color-border);border-radius:0;text-align:left;background:transparent}.material-row:last-child{border-bottom:0}.file-icon{display:grid;place-items:center;width:31px;height:37px;border-radius:6px;background:var(--color-primary-soft);color:var(--color-primary);flex-shrink:0}.material-row>span:last-child{min-width:0;display:grid;gap:5px}.material-row strong{font-size:13px;font-weight:550;color:var(--color-text);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}.material-row small{font-size:11px;color:var(--color-muted)}
  .intro-actions{display:flex;flex-wrap:wrap;align-items:center;gap:12px 20px}.brief-generate{display:inline-flex;gap:8px;align-items:center;justify-content:center;min-width:156px;min-height:40px;background:var(--color-surface)}.brief-progress{display:inline-block;margin-left:14px;font-size:13px;color:var(--color-muted);font-weight:400}
  @container(max-width:1120px){.focus-grid{grid-template-columns:minmax(0,1fr) minmax(0,1fr)}.dashboard-side{grid-column:1/-1;grid-template-columns:minmax(0,1fr) minmax(0,1fr)}.dashboard-intro{grid-template-columns:minmax(0,1fr) 270px}.quiet-note{padding:18px}}
  @container(max-width:740px){.focus-grid{grid-template-columns:minmax(0,1fr)}.dashboard-intro{grid-template-columns:minmax(0,1fr)}.quiet-note{display:none}.desk-card{padding:18px}.intro-copy h1{font-size:24px}.card-head h2{font-size:20px}}
  @container(max-width:500px){.dashboard-side{grid-template-columns:minmax(0,1fr)}.agenda-row{grid-template-columns:38px minmax(0,1fr) auto;gap:8px}}
</style>
