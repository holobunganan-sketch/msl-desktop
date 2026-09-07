<script lang="ts">
  import ProposalPreview from './ProposalPreview.svelte';
  import {navigateTo} from '$lib/services/navigation';
  import {proposalPresentation} from '$lib/services/proposalPresentation';
  import {listAiProposals} from '$lib/services/api';
  let {focusId=null}:{focusId?:number|null}=$props();
  let focusConsumed=false;
  let adjusting=$state(false);
  let showDecline=$state(false);
  let receipt=$state<{receipt_id:string;kind:string;target_id:number;workId:number|null;message:string}|null>(null);
  async function later(){const item=items[index];if(!item||busy)return;busy=true;try{await deferAiProposal(item.id,item.updated_at);invalidate('proposals','brief');await load();}catch(e){error=friendlyError(e);}finally{busy=false;}}
  async function undo(){if(!receipt||busy)return;busy=true;try{await command('undo_ai_confirmation',{receiptId:receipt.receipt_id});receipt=null;invalidate('works','tasks','waiting','calendar','inbox','proposals','brief');await load();}catch(e){error=friendlyError(e);}finally{busy=false;}}
  import ReviewTools from "$lib/components/ReviewTools.svelte";
  import ProposalRouting from "$lib/components/ProposalRouting.svelte";
  import { decisionPayload, reviewPayload } from "$lib/services/proposalPayload";
  import ProposalStatus from "./ProposalStatus.svelte";
  import { command, deferAiProposal } from "$lib/services/api";
  import { invalidate } from "$lib/stores/dataRevision";
  import { onMount } from "svelte";
  import AppButton from "$lib/components/ui/AppButton.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";
  import { confirmAiProposal, getClassificationMemoryStats, listAnalysisRuns, listRecentAiProposals, listWorks, rejectAiProposal, runAnalysisNow, updateAiProposalClassification } from "$lib/services/api";
  import type { AiProposal, AnalysisRun, ClassificationMemoryStats, Work } from "$lib/types/domain";
  import { locale, t } from "$lib/i18n";
  import { aiJobs } from "$lib/stores/aiJobs";
  import { dataRevision } from "$lib/stores/dataRevision";

  let currentLocale = $derived($locale);
  const tt = (key: Parameters<typeof t>[0], vars: Record<string, string | number> = {}) => t(key, vars, currentLocale);
  let allItems = $state<AiProposal[]>([]);
  let analysisRuns = $state<AnalysisRun[]>([]);
  let works = $state<Work[]>([]);
  let memory = $state<ClassificationMemoryStats>({ pattern_count: 0, feedback_count: 0, accepted_count: 0, corrected_count: 0, rejected_count: 0, updated_at: null });
  let index = $state(0);
  let payloadText = $state("{}");
  let repairedStatus = $state(false);
  let statusFilter = $state("pending");
  let runFilter = $state("all");
  let busy = $state(false);
  let selectedIds = $state<number[]>([]);
  let rejectionCode = $state("unspecified");
  let analysisBusy = $derived($aiJobs.some(job=>["run_analysis_now","retry_analysis_run","start_workspace_work_draft"].includes(job.command) && job.status==="running"));
  let error = $state("");
  let loaded = $state(false);
  const savedRoutes=new Map<number,Pick<AiProposal,"kind"|"operation"|"target_id">>();

  const items = $derived(allItems.filter((item) => {
    const display = item.status === "pending" && item.deferred_at ? "deferred" : item.status;
    return (statusFilter === "all" || display === statusFilter) && (runFilter === "all" || String(item.analysis_run_id ?? "none") === runFilter);
  }));
  const latestRun = $derived(analysisRuns.find(run=>run.status!=='reused') ?? null);
  const latestPendingCount = $derived(latestRun ? allItems.filter((item) => item.analysis_run_id === latestRun.id && item.status === "pending" && !item.deferred_at).length : 0);
  const runIds = $derived([...new Set(allItems.map((item) => item.analysis_run_id).filter((id): id is number => id !== null))]);

  function displayStatus(item: AiProposal): string { return item.status === "pending" && item.deferred_at ? "deferred" : item.status; }
  function statusText(item: AiProposal): string { const value = displayStatus(item); return tt(`aiReview.status${value.charAt(0).toUpperCase()}${value.slice(1)}` as Parameters<typeof t>[0]); }
  function fmtTime(value: number | null): string { return value ? new Date(value * 1000).toLocaleString(currentLocale === "en-US" ? "en-US" : "zh-CN", { hour12: false }) : "—"; }
  function friendlyError(value: unknown): string {
    return String(value instanceof Error ? value.message : value).replace(/^Error:\s*/i, "").replace(/^migration error:\s*/i, "").replace(/^sqlite error:\s*/i, "");
  }
  function parsePayload(): Record<string, unknown> {
    const parsed: unknown = JSON.parse(payloadText || "{}");
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) throw new SyntaxError(tt("aiReview.invalidJson"));
    return parsed as Record<string, unknown>;
  }
  function payloadValue(keys: string | string[]): unknown {
    let payload: Record<string, unknown>;
    try { payload = parsePayload(); } catch { return undefined; }
    for (const key of Array.isArray(keys) ? keys : [keys]) {
      const value = payload[key];
      if (value !== undefined && value !== null && value !== "") return value;
    }
    return undefined;
  }
  function payloadString(keys: string | string[]): string {
    const value = payloadValue(keys);
    return typeof value === "string" || typeof value === "number" ? String(value) : "";
  }
  function payloadBool(key: string): boolean { return payloadValue(key) === true; }
  function setPayloadField(key: string, value: unknown): void {
    let payload: Record<string, unknown>;
    try { payload = parsePayload(); } catch { payload = {}; }
    payload[key] = value;
    payloadText = JSON.stringify(payload, null, 2);
    if(items[index])items[index].payload_json=payloadText;
    error = "";
  }
  function localDateTime(value: unknown): string {
    if (typeof value !== "number" || !Number.isFinite(value)) return "";
    const source = new Date(value * 1000);
    const local = new Date(source.getTime() - source.getTimezoneOffset() * 60_000);
    return local.toISOString().slice(0, 16);
  }
  function setDateTime(key: string, value: string): void { setPayloadField(key, value ? Math.floor(new Date(value).getTime() / 1000) : null); setPayloadField("time_basis", "explicit"); setPayloadField("time_reason", null); }
  function preparePayload(item: AiProposal): void {
    let payload: Record<string, unknown>;
    try {
      const parsed: unknown = JSON.parse(item.payload_json || "{}");
      payload = parsed && typeof parsed === "object" && !Array.isArray(parsed) ? parsed as Record<string, unknown> : {};
    } catch { payload = {}; }
    const originalStatus=payload.status;
    payload=reviewPayload(item);
    repairedStatus=originalStatus!==payload.status;
    payloadText = JSON.stringify(payload, null, 2);
  }
  function updateTitle(item: AiProposal, value: string): void { item.title = value; if (item.kind !== "inbox") setPayloadField("title", value); }
  function updateKind(item: AiProposal, value: string): void {
    const payload=decisionPayload({...item,payload_json:payloadText},value);
    const saved=savedRoutes.get(item.id);
    item.operation=saved?.kind===value?saved.operation:"create";
    item.target_id=saved?.kind===value?saved.target_id:null;
    item.kind=value;
    if(value==="work"||value==="inbox")item.work_id=null;
    item.payload_json=JSON.stringify(payload);
    preparePayload(item);error="";
  }
  async function load(preserveDraft = false) {
    const editing = preserveDraft ? items[index] : null;
    const editingPayload = payloadText;
    try {
      [allItems, works, analysisRuns, memory] = await Promise.all([
        Promise.all([listRecentAiProposals(Math.floor(Date.now() / 1000) - 7 * 86400, null, 500),listAiProposals('pending',500)]).then(([recent,pending])=>[...recent,...pending.filter(p=>!recent.some(r=>r.id===p.id))]), listWorks(null), listAnalysisRuns(20), getClassificationMemoryStats()
      ]);
      for(const item of allItems)savedRoutes.set(item.id,{kind:item.kind,operation:item.operation,target_id:item.target_id});
      if(focusId&&!focusConsumed){const found=allItems.find(p=>p.id===focusId);if(found){statusFilter=displayStatus(found);index=items.findIndex(p=>p.id===focusId);}focusConsumed=true;}
      index = Math.min(index, Math.max(0, items.length - 1));
      const retained = editing ? allItems.findIndex(item=>item.id===editing.id && item.updated_at===editing.updated_at && item.status==="pending") : -1;
      if (editing && retained>=0) { allItems[retained]=editing; index=items.findIndex(item=>item.id===editing.id);payloadText=editingPayload; }
      else queueMicrotask(selectCurrent);
      error = "";
    } catch (cause) { error = friendlyError(cause); } finally { loaded = true; }
  }
  async function analyzeNow() {
    if(analysisBusy) return;
    let failure = "";
    try { await runAnalysisNow("manual"); } catch (cause) { failure = friendlyError(cause); }
    finally { await load(); if (failure) error = failure; }
  }
  onMount(()=>{let revision=$dataRevision.analysis;return dataRevision.subscribe(value=>{if(value.analysis!==revision){revision=value.analysis;if(loaded)void load(true);}});});
  function selectCurrent() { const item = items[index]; if (item) preparePayload(item); else payloadText = "{}"; error = ""; }
  function selectItem(next: number) { adjusting=false;showDecline=false;index = next; selectCurrent(); }
  function move(delta: number) { adjusting=false;showDecline=false;index = Math.max(0, Math.min(items.length - 1, index + delta)); selectCurrent(); }
  function resetFilter() { index = 0; queueMicrotask(selectCurrent); }
  async function saveDraft(): Promise<AiProposal | null> {
    const item = items[index];
    if (!item || item.status !== "pending") return null;
    busy = true;
    try {
      const payload = parsePayload();
      if (item.kind === "inbox") payload.content ||= item.title.trim(); else payload.title = item.title.trim();
      const next = await updateAiProposalClassification(item.id, item.updated_at, item.kind, item.work_id, item.title.trim(), payload);
      savedRoutes.set(next.id,{kind:next.kind,operation:next.operation,target_id:next.target_id});
      const allIndex = allItems.findIndex((candidate) => candidate.id === item.id);
      if (allIndex >= 0) allItems[allIndex] = next;
      allItems = allItems;
      preparePayload(next);
      error = "";
      return next;
    } catch (cause) { error = cause instanceof SyntaxError ? tt("aiReview.invalidJson") : friendlyError(cause); return null; }
    finally { busy = false; }
  }
  async function confirm() {
    if(busy)return;
    const item=items[index];
    if(item&&proposalPresentation({...item,payload_json:payloadText},works,currentLocale).needsAttention){adjusting=true;error=currentLocale==='en-US'?'Please confirm the missing project or time.':'请先补充项目归属或安排时间。审阅内容仍保留。';return;}
    const edited = await saveDraft();
    if (!edited) return;
    busy = true;
    try { const result=await confirmAiProposal(edited.id, edited.updated_at, parsePayload());const preview=proposalPresentation(edited,works,currentLocale);receipt={...result,workId:edited.work_id,message:`${preview.action} · ${preview.scope}`};adjusting=false; invalidate("works","tasks","waiting","calendar","inbox","proposals","brief"); await load(); }
    catch (cause) { error = friendlyError(cause); }
    finally { busy = false; }
  }
  async function reject() {
    const item = items[index];
    if (!item || item.status !== "pending") return;
    busy = true;
    try { if(rejectionCode==="not_now")await deferAiProposal(item.id,item.updated_at);else await rejectAiProposal(item.id, item.updated_at, rejectionCode); await load(); }
    catch (cause) { error = friendlyError(cause); }
    finally { busy = false; }
  }
  onMount(load);
  async function confirmGroup() {
    const currentId=items[index]?.id;
    if(selectedIds.includes(currentId) && !(await saveDraft())) return;
    const selected=allItems.filter(item=>selectedIds.includes(item.id)&&item.status==="pending");
    if(!selected.length)return;
    busy=true;error="";
    try{
      const ready:AiProposal[]=[];
      for(const item of selected){
        if(item.id===currentId){ready.push(item);continue;}
        // Save every selected item's edited routing before atomic confirmation.
        // Switching between cards must not cause older database drafts to win.
        const next=await updateAiProposalClassification(item.id,item.updated_at,item.kind,item.work_id,item.title,decisionPayload(item,item.kind));
        allItems[allItems.findIndex(candidate=>candidate.id===item.id)]=next;
        savedRoutes.set(next.id,{kind:next.kind,operation:next.operation,target_id:next.target_id});
        ready.push(next);
      }
      await command("confirm_ai_proposal_group",{items:ready.map(item=>({id:item.id,expectedUpdatedAt:item.updated_at}))});selectedIds=[];invalidate("works","tasks","waiting","calendar","inbox","proposals","brief");await load();
    }
    catch(e){error=friendlyError(e);}finally{busy=false;}
  }
</script>

<div class="review-page">
  <div class="page-head">
    <div><h1>{currentLocale==='en-US'?'Arrangements for your decision':'秘书准备好了这些安排'}</h1><p>{currentLocale==='en-US'?'Recent 7 days, plus anything still awaiting your decision.':'最近7天的记录，以及仍未处理的建议。采用后才会更新项目与事项。'}</p></div>
    <details class="filter-options"><summary>{currentLocale==='en-US'?'Filter & history':'筛选与历史'}</summary><div class="review-filters">
      <label>{tt("common.status")}<select bind:value={statusFilter} onchange={resetFilter}><option value="all">{tt("aiReview.filterAll")}</option><option value="pending">{tt("aiReview.statusPending")}</option><option value="deferred">{tt("aiReview.statusDeferred")}</option><option value="confirmed">{tt("aiReview.statusConfirmed")}</option><option value="rejected">{tt("aiReview.statusRejected")}</option><option value="superseded">{tt("aiReview.statusSuperseded")}</option></select></label>
      <label>{tt("aiReview.filterRun")}<select bind:value={runFilter} onchange={resetFilter}><option value="all">{tt("aiReview.allRuns")}</option>{#each runIds as runId}<option value={String(runId)}>#{runId}</option>{/each}</select></label>
      <AppButton variant="secondary" onclick={()=>load()} loading={!loaded}>{tt("common.refresh")}</AppButton>
    </div></details>
  </div>

  <div class="organizing-row"><details class="organizing-details" open={latestRun?.status==='failed'||latestRun?.status==='running'}><summary>{currentLocale==='en-US'?'Secretary activity & learning':'秘书整理情况与分类记忆'}</summary>
  <section class="workflow-status status-{latestRun?.status ?? 'idle'}" data-testid="analysis-workflow-status">
    <span class="workflow-icon"><Icon name={latestRun?.status === "failed" ? "activity" : latestRun?.status === "running" ? "clock" : "sparkles"} size={18} /></span>
    <div class="workflow-copy">
      <strong>{latestRun?.status === "failed" ? tt("aiReview.workflowFailed") : latestRun?.status === "running" ? tt("aiReview.workflowRunning") : latestRun?.status === "completed" ? tt("aiReview.workflowCompleted") : tt("aiReview.workflowIdle")}</strong>
      {#if latestRun?.status === "failed"}<small>{friendlyError(latestRun.error_message || latestRun.error_code || "—").replace(/global_analysis/g, tt("aiReview.title"))}</small>
      {:else if latestRun?.status === "completed"}<small>{latestPendingCount ? tt("aiReview.workflowPendingCount", { count: latestPendingCount }) : tt("aiReview.workflowNoSuggestion")} · {fmtTime(latestRun.finished_at)}</small>
      {:else if latestRun?.status === "running"}<small>#{latestRun.id} · {fmtTime(latestRun.started_at)}</small>{/if}
    </div>
    <ReviewTools feedbackCount={memory.feedback_count} onchange={()=>void load()}/>
  </section>
  </details><AppButton testid="review-run-analysis" loading={analysisBusy || latestRun?.status === "running"} onclick={analyzeNow}>{latestRun ? tt("aiReview.runAnalysis") : tt("aiReview.startAnalysis")}</AppButton></div>
  {#if error}<div class="error" role="alert"><span>{error}</span><button aria-label={currentLocale === "en-US" ? "Close" : "关闭"} onclick={() => error = ""}>×</button></div>{/if}

  {#if receipt}<div class="receipt" role="status"><span>{currentLocale==='en-US'?'Saved: ':'已完成：'}{receipt.message}</span><button data-testid="review-receipt-view" onclick={()=>navigateTo(receipt!.kind,receipt!.target_id,receipt!.workId)}>{currentLocale==='en-US'?'View item':'查看去向'}</button><button data-testid="review-receipt-undo" disabled={busy} onclick={undo}>{currentLocale==='en-US'?'Undo':'撤销'}</button></div>{/if}
  {#if items.length}
    {#if items.filter(item=>item.status==="pending").length>1}
      <details class="group-confirm" data-testid="group-review"><summary>{currentLocale==="en-US"?"Review related suggestions together":"一起处理相关建议"}</summary>
        <p>{currentLocale==="en-US"?"Select only reviewed items. The group is saved atomically and can be undone together.":"勾选已查看的事项。所选建议会一起保存，可在“分类记忆与撤销”中一并撤销；需要修改时先在下方编辑该条目。"}</p>
        {#each items.filter(item=>item.status==="pending") as item(item.id)}<label><input type="checkbox" value={item.id} bind:group={selectedIds}/><span><strong>{item.title}</strong><small>{tt(`proposal.kind.${item.kind}` as Parameters<typeof t>[0])} · {item.reason}</small></span></label>{/each}
        <button data-testid="confirm-proposal-group" disabled={busy||!selectedIds.length||selectedIds.length>20} onclick={confirmGroup}>{currentLocale==="en-US"?"Confirm selected":"确认所选"} ({selectedIds.length})</button>
      </details>
    {/if}
    <div class="review-workspace">
      <aside class="proposal-list">
      <div class="list-head"><strong>{currentLocale==='en-US'?'Suggestions':'待查看的安排'}</strong><span>{items.length}</span></div>
        <div class="list-scroll">{#each items as item, itemIndex (item.id)}<button class="proposal-item" class:active={itemIndex === index} onclick={() => selectItem(itemIndex)}><div><span class="kind">{tt(`proposal.kind.${item.kind}` as Parameters<typeof t>[0])}</span><span class="operation status-{displayStatus(item)}">{statusText(item)}</span></div><strong>{item.title}</strong><small>{tt("aiReview.sourceRun", { id: item.analysis_run_id ?? "—" })} · {fmtTime(item.created_at)}</small></button>{/each}</div>
      </aside>

      <section class="editor-panel">
        {#if items[index]}
        {@const item = items[index]}
        <div class="editor-head"><div><div class="eyebrow">{tt("aiReview.counter", { current: index + 1, total: items.length })}</div><h2>{item.title}</h2></div><div class="pager"><button disabled={index === 0 || busy} onclick={() => move(-1)} aria-label={tt("aiReview.previous")}>‹</button><button disabled={index >= items.length - 1 || busy} onclick={() => move(1)} aria-label={tt("aiReview.next")}>›</button></div></div>
        <div class="review-meta"><span>{statusText(item)}</span>{#if item.decided_at}<span>{fmtTime(item.decided_at)}</span>{/if}</div>
        <ProposalPreview {item} {works}/>
        <div class="safety-note"><Icon name="check" size={15} /><span>{item.status === "pending" ? (currentLocale==='en-US'?'Accept to save these changes. You can undo them afterward.':'采用后保存以上变更，完成后可以撤销。') : tt("aiReview.readOnly")}</span></div>
        {#if adjusting}<div class="editor-scroll">
          <div class="form-grid">
            <label>{tt("aiReview.titleField")}<input value={item.title} oninput={(event) => updateTitle(item, event.currentTarget.value)} disabled={item.status !== "pending"} /></label>
            {#key item.id}<ProposalRouting kind={item.kind} workId={item.work_id} operation={item.operation} {works} disabled={busy||item.status!=="pending"} onkindchange={value=>updateKind(item,value)} onworkchange={value=>{item.work_id=value;error="";}}/>{/key}
            <label class="reason-field">{tt("aiReview.reason")}<textarea value={item.reason} rows="2" readonly></textarea></label>
          </div>

          <div class="detail-card">
            {#if repairedStatus}<p class="repair-note" role="status">{tt("scope.statusRepair")}</p>{/if}
            <div class="detail-head"><div><strong>{tt("aiReview.details")}</strong><small>{tt(`aiReview.detailsHint.${item.kind}` as Parameters<typeof t>[0])}</small></div><span>{tt(`proposal.kind.${item.kind}` as Parameters<typeof t>[0])}</span></div>
            <div class="detail-grid">
              {#if item.kind === "work"}
                <ProposalStatus kind="work" value={payloadString("status")} onchange={value=>setPayloadField("status",value)} disabled={busy||item.status!=="pending"}/>
                <label>{tt("aiReview.currentState")}<textarea value={payloadString("current_state")} oninput={(event) => setPayloadField("current_state", event.currentTarget.value)} rows="2" disabled={item.status !== "pending"}></textarea></label>
                <label>{tt("aiReview.nextStep")}<textarea value={payloadString("next_step")} oninput={(event) => setPayloadField("next_step", event.currentTarget.value)} rows="2" disabled={item.status !== "pending"}></textarea></label>
                <label class="wide">{tt("aiReview.summaryField")}<textarea value={payloadString("summary")} oninput={(event) => setPayloadField("summary", event.currentTarget.value)} rows="2" disabled={item.status !== "pending"}></textarea></label>
              {:else if item.kind === "task"}
                <ProposalStatus kind="task" value={payloadString("status")} onchange={value=>setPayloadField("status",value)} disabled={busy||item.status!=="pending"}/>
                <label>{tt("aiReview.priority")}<select value={payloadString("priority") || "normal"} onchange={(event) => setPayloadField("priority", event.currentTarget.value)} disabled={item.status !== "pending"}><option value="low">{tt("aiReview.priorityLow")}</option><option value="normal">{tt("aiReview.priorityNormal")}</option><option value="high">{tt("aiReview.priorityHigh")}</option></select></label>
                <label>{tt("aiReview.dueAt")}<input type="datetime-local" value={localDateTime(payloadValue("due_at"))} onchange={(event) => setDateTime("due_at", event.currentTarget.value)} disabled={item.status !== "pending"} /></label>
                <label>{currentLocale==='en-US'?'Arranged start':'安排开始时间'}<input type="datetime-local" data-testid="review-scheduled-start" value={localDateTime(payloadValue('scheduled_start'))} onchange={e=>setDateTime('scheduled_start',e.currentTarget.value)} disabled={item.status!=='pending'}/></label>
                <label>{currentLocale==='en-US'?'Arranged end':'安排结束时间'}<input type="datetime-local" value={localDateTime(payloadValue('scheduled_end'))} onchange={e=>setDateTime('scheduled_end',e.currentTarget.value)} disabled={item.status!=='pending'}/></label>
                <label class="wide">{tt("aiReview.taskNotes")}<textarea value={payloadString("notes")} oninput={(event) => setPayloadField("notes", event.currentTarget.value)} rows="3" disabled={item.status !== "pending"}></textarea></label>
              {:else if item.kind === "waiting"}
                <ProposalStatus kind="waiting" value={payloadString("status")} onchange={value=>setPayloadField("status",value)} disabled={busy||item.status!=="pending"}/>
                <label>{tt("aiReview.waitingFor")}<input value={payloadString("waiting_for")} oninput={(event) => setPayloadField("waiting_for", event.currentTarget.value)} disabled={item.status !== "pending"} /></label>
                <label>{tt("aiReview.followUpAt")}<input type="datetime-local" value={localDateTime(payloadValue("follow_up_at"))} onchange={(event) => setDateTime("follow_up_at", event.currentTarget.value)} disabled={item.status !== "pending"} /></label>
                <label class="wide">{tt("aiReview.waitingNotes")}<textarea value={payloadString("notes")} oninput={(event) => setPayloadField("notes", event.currentTarget.value)} rows="3" disabled={item.status !== "pending"}></textarea></label>
              {:else if item.kind === "calendar"}
                <label>{tt("aiReview.startAt")}<input type="datetime-local" value={localDateTime(payloadValue("start_at"))} onchange={(event) => setDateTime("start_at", event.currentTarget.value)} disabled={item.status !== "pending"} /></label>
                <label>{tt("aiReview.endAt")}<input type="datetime-local" value={localDateTime(payloadValue("end_at"))} onchange={(event) => setDateTime("end_at", event.currentTarget.value)} disabled={item.status !== "pending"} /></label>
                <label>{tt("aiReview.location")}<input value={payloadString("location")} oninput={(event) => setPayloadField("location", event.currentTarget.value)} disabled={item.status !== "pending"} /></label>
                <label>{tt("aiReview.eventKind")}<select value={payloadString("kind") || "other"} onchange={(event) => setPayloadField("kind", event.currentTarget.value)} disabled={item.status !== "pending"}><option value="meeting">{tt("aiReview.eventMeeting")}</option><option value="kol_visit">{tt("aiReview.eventVisit")}</option><option value="deadline">{tt("aiReview.eventDeadline")}</option><option value="travel">{tt("aiReview.eventTravel")}</option><option value="work_block">{tt("aiReview.eventWorkBlock")}</option><option value="other">{tt("aiReview.eventOther")}</option></select></label>
                <label class="check-field"><input type="checkbox" checked={payloadBool("all_day")} onchange={(event) => setPayloadField("all_day", event.currentTarget.checked)} disabled={item.status !== "pending"} />{tt("aiReview.allDay")}</label>
                <label class="wide">{tt("aiReview.calendarNotes")}<textarea value={payloadString("notes")} oninput={(event) => setPayloadField("notes", event.currentTarget.value)} rows="2" disabled={item.status !== "pending"}></textarea></label>
              {:else if item.kind === "inbox"}
                <label class="wide">{tt("aiReview.inboxContent")}<textarea value={payloadString("content")} oninput={(event) => setPayloadField("content", event.currentTarget.value)} rows="4" disabled={item.status !== "pending"}></textarea></label>
              {:else if item.kind === "resume_point"}
                <label>{tt("aiReview.currentState")}<textarea value={payloadString("current_state")} oninput={(event) => setPayloadField("current_state", event.currentTarget.value)} rows="2" disabled={item.status !== "pending"}></textarea></label>
                <label>{tt("aiReview.nextStep")}<textarea value={payloadString("next_step")} oninput={(event) => setPayloadField("next_step", event.currentTarget.value)} rows="2" disabled={item.status !== "pending"}></textarea></label>
                <label class="wide">{tt("aiReview.remember")}<textarea value={payloadString("remember")} oninput={(event) => setPayloadField("remember", event.currentTarget.value)} rows="2" disabled={item.status !== "pending"}></textarea></label>
              {/if}
            </div>
          </div>

          <details class="advanced-panel">
            <summary>{tt("aiReview.advancedFields")}<small>{tt("aiReview.advancedHint")}</small></summary>
            <textarea class="payload" bind:value={payloadText} rows="5" spellcheck="false" disabled={item.status !== "pending"} aria-label={tt("aiReview.payload")}></textarea>
          </details>
        </div>
        {/if}
        {#if item.status === "pending"}<div class="review-actions">
          {#if adjusting}<AppButton variant="ghost" loading={busy} onclick={saveDraft}>{tt('aiReview.saveDraft')}</AppButton>{/if}
          <button data-testid="review-adjust" class="plain-action" onclick={()=>adjusting=!adjusting}>{adjusting?(currentLocale==='en-US'?'Back to preview':'收起调整'):(currentLocale==='en-US'?'Adjust':'调整')}</button>
          <button data-testid="review-defer" class="plain-action" disabled={busy} onclick={later}>{currentLocale==='en-US'?'Later':'稍后'}</button>
          <AppButton testid="review-confirm" loading={busy} onclick={confirm}>{currentLocale==='en-US'?'Accept arrangement':'采用安排'}</AppButton>
        </div>
        <details class="decline-options" bind:open={showDecline}><summary>{currentLocale==='en-US'?'Do not use this suggestion':'这条建议不需要采用'}</summary><label class="rejection-choice">{currentLocale==='en-US'?'Reason (optional)':'原因（可选）'}<select data-testid="rejection-reason" bind:value={rejectionCode}><option value="unspecified">{currentLocale==='en-US'?'No reason':'暂不说明'}</option><option value="wrong_category">{currentLocale==='en-US'?'Wrong category':'分类不对'}</option><option value="duplicate">{currentLocale==='en-US'?'Already handled':'已经处理过'}</option></select></label><AppButton variant="danger" loading={busy} onclick={reject}>{currentLocale==='en-US'?'Decline suggestion':'不采用这条建议'}</AppButton></details>
        {:else}<button class="plain-action" onclick={()=>adjusting=!adjusting}>{currentLocale==='en-US'?'View details':'查看详情'}</button>{/if}
        {/if}
      </section>
    </div>
  {:else if loaded}
    <section class="review-empty-shell" data-testid="review-empty-state">
      <div class="empty-orb"><Icon name="review" size={30} /></div>
      <div class="empty-copy">
        <span>{tt("aiReview.lastSevenDays")}</span>
        <h2>{tt("aiReview.empty")}</h2>
        <p>{tt("aiReview.emptyHint")}</p>
      </div>
    </section>
  {/if}
</div>

<style>
  .filter-options summary,.decline-options summary{cursor:pointer;color:var(--color-muted);font-size:14px;padding:8px 0}.filter-options .review-filters{padding-top:12px}.decline-options{margin-top:14px}.decline-options :global(.app-button){margin-top:12px}.plain-action,.receipt button{padding:9px 14px;border:1px solid var(--color-border);border-radius:9px;background:var(--color-surface);font-size:14px;cursor:pointer}.receipt{display:flex;flex-wrap:wrap;align-items:center;gap:10px;padding:14px;border:1px solid var(--color-border);border-radius:12px;background:var(--color-success-soft);font-size:14px}.receipt span{flex:1 1 240px}.editor-panel :global(.proposal-preview){padding:12px 0 18px}.list-head strong{font-size:14px}.review-actions{margin-top:18px}

  .repair-note{margin:0 0 12px;font-size:14px;line-height:1.6;color:var(--color-primary)}
  .group-confirm{padding:16px 20px;border:1px solid var(--color-border);border-radius:14px;background:var(--color-surface);min-width:0}.group-confirm summary{font-size:15px;font-weight:650;cursor:pointer}.group-confirm p{color:var(--color-muted);font-size:14px;line-height:1.6}.group-confirm label{display:flex;gap:12px;align-items:flex-start;margin:12px 0;font-size:15px}.group-confirm input{flex:0 0 auto;width:18px;height:18px;margin-top:3px}.group-confirm label span{display:grid;gap:4px;min-width:0;overflow-wrap:anywhere}.group-confirm small{font-size:13px;color:var(--color-muted);line-height:1.6}.group-confirm button{font:inherit;padding:10px 15px;background:var(--color-primary);color:white;border:0;border-radius:10px}.group-confirm button:disabled{opacity:.5}.rejection-choice{display:grid;gap:5px;font-size:13px;color:var(--color-muted)}.rejection-choice select{font:inherit;padding:8px;min-height:38px;border-radius:8px;border:1px solid var(--color-border);background:var(--color-surface);color:var(--color-text)}
  .review-page{min-width:0;display:flex;flex-direction:column;gap:16px}.page-head{min-height:58px;display:flex;align-items:center;justify-content:space-between;gap:18px}.page-head h1{margin:0 0 5px;font:500 26px var(--font-serif)}.page-head p{margin:0;color:var(--color-muted);font-size:13px}.review-filters{display:flex;align-items:end;gap:9px;flex-wrap:wrap;justify-content:flex-end}.review-filters label{display:grid;gap:4px;color:var(--color-muted);font-size:11px}.review-filters select{min-width:128px;height:38px;padding:6px 10px;border:1px solid var(--color-border);border-radius:10px;background:var(--color-surface);color:var(--color-text);font-size:13px}.workflow-status{min-width:0;display:grid;grid-template-columns:auto minmax(0,1fr) auto auto;align-items:center;gap:12px;padding:11px 13px;border:1px solid var(--color-border);border-radius:var(--radius-sm);background:var(--color-surface)}.workflow-icon{width:38px;height:38px;display:grid;place-items:center;border-radius:11px;background:var(--color-primary-soft);color:var(--color-primary)}.workflow-copy{min-width:0;display:grid;gap:2px}.workflow-copy strong{font-size:14px}.workflow-copy small{overflow-wrap:anywhere;color:var(--color-muted);font-size:12px;line-height:1.4;white-space:normal}.memory-chip{display:inline-flex;align-items:center;gap:6px;padding:7px 9px;border:1px solid #dbe6e2;border-radius:9px;background:var(--color-success-soft);color:#58756d;font-size:11px;white-space:nowrap}.workflow-status.status-failed{border-color:#ead7d2;background:#fbf5f3}.workflow-status.status-failed .workflow-icon{background:var(--color-danger-soft);color:var(--color-danger)}.workflow-status.status-running .workflow-icon{background:var(--color-warning-soft);color:var(--color-warning)}.error{grid-column:1/-1;display:flex;align-items:flex-start;justify-content:space-between;gap:12px;padding:9px 11px;border:1px solid #ead2d2;border-radius:9px;background:var(--color-danger-soft);color:var(--color-danger);font-size:12px;line-height:1.45}.error button{border:0;background:transparent;color:inherit;font-size:18px;cursor:pointer}.review-workspace{min-height:0;display:grid;grid-template-columns:300px minmax(0,1fr);gap:13px}.proposal-list,.editor-panel{min-width:0;min-height:0;border:1px solid var(--color-border);border-radius:var(--radius-md);background:var(--color-surface);box-shadow:var(--shadow-sm);overflow:hidden}.list-head{min-height:56px;display:flex;align-items:center;justify-content:space-between;padding:0 14px;border-bottom:1px solid #e5ebed}.list-head strong{font-size:14px}.list-head span{min-width:25px;height:25px;display:grid;place-items:center;border-radius:8px;background:var(--color-primary-soft);color:var(--color-primary);font-size:11px;font-weight:700}.list-scroll{min-height:0;max-height:650px;padding:8px;overflow:auto}.proposal-item{width:100%;display:grid;gap:7px;margin:2px 0;padding:11px;border:1px solid transparent;border-radius:11px;background:transparent;color:var(--color-text);text-align:left;cursor:pointer}.proposal-item:hover{background:var(--color-surface-muted)}.proposal-item.active{background:var(--color-primary-soft)}.proposal-item div{display:flex;gap:5px}.kind,.operation,.review-meta span{padding:3px 7px;border-radius:7px;background:#f0f4f5;color:#687e87;font-size:10px}.operation{background:var(--color-warning-soft);color:var(--color-warning)}.status-confirmed{background:var(--color-success-soft);color:var(--color-success)}.status-rejected,.status-superseded{background:var(--color-danger-soft);color:var(--color-danger)}.proposal-item strong{font-size:13px;line-height:1.4}.proposal-item small{color:#8b999f;font-size:10px}.editor-panel{display:grid;grid-template-rows:auto auto auto minmax(0,1fr) auto;padding:18px 20px}.editor-head{display:flex;align-items:flex-start;justify-content:space-between;gap:12px}.eyebrow{color:#7c9099;font-size:11px;letter-spacing:.08em;text-transform:uppercase}.editor-head h2{margin:5px 0 0;font:500 22px var(--font-serif)}.pager{display:flex;gap:6px}.pager button{width:34px;height:34px;border:1px solid var(--color-border);border-radius:9px;background:var(--color-surface-raised);color:#667c85;font-size:20px;cursor:pointer}.pager button:disabled{opacity:.4;cursor:not-allowed}.review-meta{display:flex;flex-wrap:wrap;gap:6px;padding:11px 0}.safety-note{display:flex;align-items:center;gap:8px;padding:9px 11px;border:1px solid #dbe6e2;border-radius:9px;background:var(--color-success-soft);color:#668077;font-size:11px}.editor-scroll{min-height:0;overflow:auto;padding:13px 4px 3px 0}.form-grid,.detail-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:12px}.form-grid label,.detail-grid label{display:grid;align-content:start;gap:6px;color:#607780;font-size:11px}.form-grid input,.form-grid textarea,.detail-grid input,.detail-grid select,.detail-grid textarea,.payload{width:100%;padding:9px 10px;border:1px solid var(--color-border);border-radius:9px;background:#fafcfc;color:var(--color-text);font-size:13px;line-height:1.45;resize:vertical}.form-grid :disabled,.detail-grid :disabled,.payload:disabled{opacity:.72}.reason-field{grid-column:1/-1}.reason-field textarea{color:#536a73;background:#f5f8f8}.detail-card{margin-top:13px;padding:13px;border:1px solid #e0e8ea;border-radius:12px;background:#fbfcfc}.detail-head{display:flex;align-items:flex-start;justify-content:space-between;gap:12px;margin-bottom:12px}.detail-head div{display:grid;gap:3px}.detail-head strong{font-size:14px}.detail-head small{color:var(--color-muted);font-size:11px}.detail-head>span{padding:4px 8px;border-radius:8px;background:var(--color-primary-soft);color:var(--color-primary);font-size:10px}.detail-grid .wide{grid-column:1/-1}.check-field{display:flex!important;align-items:center!important;grid-template-columns:auto 1fr;align-self:end;min-height:39px}.check-field input{width:16px;height:16px}.advanced-panel{margin-top:12px;border-top:1px solid #e5ebed;padding-top:10px}.advanced-panel summary{display:flex;align-items:center;gap:8px;color:#667c85;font-size:12px;font-weight:650;cursor:pointer}.advanced-panel summary small{color:#94a2a7;font-size:10px;font-weight:400}.payload{min-height:96px;margin-top:9px;font-family:"Cascadia Code",Consolas,monospace;font-size:11px}.review-actions{display:grid;grid-template-columns:auto 1fr auto auto;gap:9px;padding-top:13px;border-top:1px solid #e6ecee}.review-empty-shell{min-height:300px;display:grid;grid-template-columns:auto minmax(0,1fr);align-items:center;gap:22px;padding:clamp(28px,5vw,70px);border:1px solid var(--color-border);border-radius:var(--radius-md);background:linear-gradient(128deg,var(--color-surface) 0%,#f3f7f6 100%);box-shadow:var(--shadow-sm);overflow:hidden}.empty-orb{width:72px;height:72px;display:grid;place-items:center;border:1px solid #d1dfdb;border-radius:22px;background:var(--color-success-soft);color:#678078}.empty-copy{min-width:0}.empty-copy>span{color:#769087;font-size:11px;font-weight:700;letter-spacing:.1em;text-transform:uppercase}.empty-copy h2{margin:8px 0 7px;font:500 clamp(21px,2.2vw,31px) var(--font-serif)}.empty-copy p{max-width:620px;margin:0;color:var(--color-muted);font-size:13px;line-height:1.6}
  @container(max-width:1050px){.memory-chip{display:none}.workflow-status{grid-template-columns:auto minmax(0,1fr) auto}.review-workspace{grid-template-columns:250px minmax(0,1fr)}}
  @container(max-width:760px){.review-page{height:auto}.page-head,.review-filters{align-items:stretch;flex-direction:column}.workflow-status{grid-template-columns:auto minmax(0,1fr)}.workflow-status :global(.app-button){grid-column:1/-1}.review-workspace{grid-template-columns:1fr}.proposal-list{max-height:280px}.editor-panel{min-height:0}.form-grid,.detail-grid{grid-template-columns:1fr}.reason-field,.detail-grid .wide{grid-column:auto}.review-empty-shell{grid-template-columns:1fr;justify-items:start;padding:28px}.empty-orb{width:56px;height:56px;border-radius:17px}}
  .page-head { flex-wrap: wrap; }
  .review-workspace { align-items: start; }
  .proposal-list { display: flex; flex-direction: column; }
  .list-head { flex-shrink: 0; padding: 14px; gap: 12px; line-height: 1.5; }
  .list-head span,.pager { flex-shrink: 0; }
  .editor-panel { display: flex; flex-direction: column; overflow: visible; }
  .editor-scroll { overflow: visible; }
  .editor-head>div:first-child { min-width: 0; }
  .editor-head h2,.proposal-item strong,.error { overflow-wrap: anywhere; }
  .review-actions { display: flex; flex-wrap: wrap; justify-content: flex-end; }
  .review-actions>span { flex: 1; }
  .form-grid label,.detail-grid label { min-width: 0; }
  .advanced-panel summary { flex-wrap: wrap; }
  .safety-note { align-items: start; line-height: 1.6; }
  .error button { flex-shrink: 0; }
  @container(max-width:760px) { .workflow-status { grid-template-columns:auto minmax(0,1fr); } .workflow-status :global(.app-button) { grid-column:1/-1; justify-self:start; } }
  .workflow-status{display:flex;flex-wrap:wrap;gap:12px}.workflow-icon{flex:0 0 38px}.workflow-copy{flex:1 1 280px}.review-actions{gap:12px;align-items:end}
  .organizing-row{display:flex;flex-wrap:wrap;align-items:start;gap:14px}.organizing-details{flex:1 1 280px;min-width:0}.organizing-details>summary{font-size:14px;line-height:1.7;padding:7px 0;cursor:pointer;color:var(--color-muted)}.organizing-details .workflow-status{margin-top:10px}
  @container(max-width:760px){.review-workspace .editor-panel{grid-row:1}.review-workspace .proposal-list{grid-row:2;max-height:none}.page-head{gap:8px}.page-head p{line-height:1.6}}
</style>
