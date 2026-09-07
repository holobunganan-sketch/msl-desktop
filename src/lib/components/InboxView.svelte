<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { command, createInboxItem } from "$lib/services/api";
  import { aiJobs } from "$lib/stores/aiJobs";
  import { locale, t, translateKind } from "$lib/i18n";
  import Modal from "$lib/components/ui/Modal.svelte";
  import AppButton from "$lib/components/ui/AppButton.svelte";
  import { addToast } from "$lib/stores/toast";
  import { dataRevision, invalidate } from "$lib/stores/dataRevision";
  import ProjectScope from "./ProjectScope.svelte";

  import NaturalCapture from './NaturalCapture.svelte';
  import {onMount,tick} from 'svelte';
  import {navigateTo} from '$lib/services/navigation';
  let {focusId=null}:{focusId?:number|null}=$props();
  let showHistory=$state(false);
  onMount(async()=>{await load();if(focusId){showHistory=true;await tick();document.querySelector(`[data-inbox-id="${focusId}"]`)?.scrollIntoView({block:'center'});}});
  type InboxItem = {
    id: number;
    content: string;
    created_at: number;
    processed_at: number | null;
    converted_to_type: string | null;
    converted_to_id: number | null;
  };

  let items = $state<InboxItem[]>([]);
  let works = $state<Array<{ id: number; title: string; status: string }>>([]);
  let error = $state("");
  let currentLocale = $derived($locale);
  const tt = (key: Parameters<typeof t>[0], params: Record<string, string | number> = {}) => t(key, params, currentLocale);
  let conversion = $state<"task" | "waiting" | "calendar" | "resume_point" | null>(null);
  let projectMode = $state(false);
  let selected = $state<InboxItem | null>(null);
  let formTitle = $state("");
  let formWorkId = $state<number | null>(null);
  let formPriority = $state("normal");
  let formDue = $state("");
  let formWaitingFor = $state("");
  let formFollowUp = $state("");
  let formStart = $state("");
  let formEnd = $state("");
  let formKind = $state("other");
  let saving = $state(false);
  let captureText=$state("");
  let captureBusy=$state(false);
  let queued=$state(false);
  function organizing(id:number){return $aiJobs.some(job=>job.command==="organize_inbox_item"&&job.args.inboxId===id&&job.status==="running");}
  async function organize(item:InboxItem){error="";queued=true;try{await command("organize_inbox_item",{inboxId:item.id});}catch(e){error=String(e);}}
  async function captureAndOrganize(){
    if(!captureText.trim()||captureBusy)return;captureBusy=true;error="";
    try{const item=await createInboxItem(captureText.trim());captureText="";await load();void organize(item);}
    catch(e){error=String(e);}finally{captureBusy=false;}
  }

  function fmtTime(ts: number): string {
    const d = new Date(ts * 1000);
    const pad = (n: number) => String(n).padStart(2, "0");
    return d.getFullYear() + "-" + pad(d.getMonth() + 1) + "-" + pad(d.getDate()) + " " + pad(d.getHours()) + ":" + pad(d.getMinutes());
  }

  function parseDate(value: string): number | null {
    if (!value || !value.trim()) return null;
    const parsed = Date.parse(value.replace(" ", "T"));
    return Number.isNaN(parsed) ? null : Math.floor(parsed / 1000);
  }

  async function load() {
    try {
      items = await invoke<InboxItem[]>("list_inbox");
    } catch (e) {
      error = String(e);
    }
  }

  async function loadWorks() {
    try {
      const all = await invoke<Array<{ id: number; title: string; status: string }>>("list_works", { status: null });
      works = all;
    } catch (e) {
      error = String(e);
    }
  }

  function openConversion(type: "task" | "waiting" | "calendar" | "resume_point", item: InboxItem, project = false) {
    projectMode = project;
    selected = item;
    conversion = type;
    formTitle = item.content;
    formWorkId = null;
    formPriority = "normal";
    formDue = "";
    formWaitingFor = "";
    formFollowUp = "";
    const now=new Date(); formStart = new Date(now.getTime()-now.getTimezoneOffset()*60000).toISOString().slice(0,16);
    formEnd = "";
    formKind = "other";
    error = "";
  }

  function closeConversion() {
    conversion = null;
    selected = null;
  }

  async function convert() {
    if (!selected || !formTitle.trim()) {
      error = tt("common.required");
      return;
    }
    if ((projectMode || conversion === "resume_point") && !formWorkId) { error=tt("inbox.chooseProject"); return; }
    saving = true;
    error = "";
    try {
      if (conversion === "resume_point") {
        await invoke("convert_inbox_to_resume_point",{inboxId:selected.id,workId:formWorkId,currentState:formTitle.trim()});
      } else if (conversion === "task") {
        await invoke("convert_inbox_to_task", { inboxId: selected.id, workId: formWorkId, title: formTitle.trim(), priority: formPriority, dueAt: parseDate(formDue), notes: null });
      } else if (conversion === "waiting") {
        await invoke("convert_inbox_to_waiting", { inboxId: selected.id, workId: formWorkId, title: formTitle.trim(), waitingFor: formWaitingFor.trim(), followUpAt: parseDate(formFollowUp) });
      } else if (conversion === "calendar") {
        const start = parseDate(formStart);
        if (!start) {
          error = tt("inbox.invalidStart");
          return;
        }
        const end = parseDate(formEnd);
        if (end !== null && end < start) {
          error = tt("inbox.endBeforeStart");
          return;
        }
        await invoke("convert_inbox_to_calendar", { inboxId: selected.id, workId: formWorkId, title: formTitle.trim(), startAt: start, endAt: end, allDay: false, kind: formKind });
      }
      addToast(tt("settings.saved"), "success");
      invalidate("inbox", "tasks", "waiting", "calendar", "works", "brief");
      closeConversion();
      await load();
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  async function remove(item: InboxItem) {
    try {
      await invoke("delete_inbox_item", { id: item.id });
      invalidate("inbox", "brief");
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  $effect(() => {
    $dataRevision.inbox;
    $dataRevision.works;
    load();
    loadWorks();
  });
</script>

<div class="inbox">
  <div class="page-head">
    <div>
      <h1>{tt("inbox.title")}</h1>
      <div class="muted hint">{tt("inbox.hint")}</div>
    </div>
    <span class="count">{items.filter((item) => !item.processed_at).length}</span>
  </div>
  {#if error}<div class="status error" role="alert">{error}</div>{/if}

  <Modal open={conversion !== null} title={projectMode?tt("inbox.toProject"):tt("inbox.convertTitle", { type: conversion === "task" ? tt("inbox.toTask") : conversion === "waiting" ? tt("inbox.toWaiting") : tt("inbox.toCalendar") })} onclose={closeConversion}>
    <form class="modal-form" onsubmit={(event) => { event.preventDefault(); convert(); }}>
      {#if projectMode}<p class="project-hint">{tt("inbox.projectHint")}</p><label for="project-item-kind">{tt("inbox.projectType")}</label><select id="project-item-kind" bind:value={conversion}><option value="task">{tt("inbox.toTask")}</option><option value="waiting">{tt("inbox.toWaiting")}</option><option value="calendar">{tt("inbox.toCalendar")}</option><option value="resume_point">{tt("inbox.progress")}</option></select>{/if}
      <label for="inbox-title">{tt("common.create")} *</label>
      <textarea id="inbox-title" rows="3" bind:value={formTitle}></textarea>
      <ProjectScope {works} bind:value={formWorkId} id="inbox-work" required={projectMode||conversion==="resume_point"}/>
      <p class="project-hint">{tt("scope.itemHint")}</p>
      {#if conversion === "task"}
        <label for="inbox-priority">{tt("task.priority.normal")}</label>
        <select id="inbox-priority" bind:value={formPriority}>
          <option value="low">{tt("task.priority.low")}</option>
          <option value="normal">{tt("task.priority.normal")}</option>
          <option value="high">{tt("task.priority.high")}</option>
        </select>
        <label for="inbox-due">{tt("task.due")}</label><input id="inbox-due" type="datetime-local" bind:value={formDue} />
      {:else if conversion === "waiting"}
        <label for="inbox-who">{tt("waiting.for")}</label><input id="inbox-who" bind:value={formWaitingFor} />
        <label for="inbox-follow">{tt("waiting.followUp")}</label><input id="inbox-follow" type="datetime-local" bind:value={formFollowUp} />
      {:else if conversion === "calendar"}
        <label for="inbox-kind">{tt("calendar.title")}</label>
        <select id="inbox-kind" bind:value={formKind}>{#each ["meeting", "kol_visit", "deadline", "travel", "work_block", "other"] as kind}<option value={kind}>{translateKind(kind, currentLocale)}</option>{/each}</select>
        <label for="inbox-start">{tt("calendar.start")}</label><input id="inbox-start" type="datetime-local" bind:value={formStart} />
        <label for="inbox-end">{tt("calendar.endPlaceholder")}</label><input id="inbox-end" type="datetime-local" bind:value={formEnd} />
      {/if}
      <div class="modal-actions">
        <button type="button" onclick={closeConversion}>{tt("common.cancel")}</button>
        <AppButton type="submit" loading={saving} label={tt("common.save")} />
      </div>
      {#if error}<div class="status error" role="alert">{error}</div>{/if}
    </form>
  </Modal>

  <NaturalCapture/>
  <label class="history-switch"><input type="checkbox" bind:checked={showHistory}/>{currentLocale==='en-US'?'Include organized notes':'同时查看已整理的原始记录'}</label>
  {#if queued}<p class="organize-note">{currentLocale==="en-US"?"You can keep working. Suggestions appear in AI Review when ready.":"可以继续工作。整理完成后，建议会出现在 AI 审阅中。"}<button onclick={()=>window.dispatchEvent(new CustomEvent("dashboard:navigate",{detail:"review"}))}>{currentLocale==="en-US"?"Open review":"查看审阅"}</button></p>{/if}
  <ul class="in-list">
    {#each items.filter(item=>showHistory||!item.processed_at) as item (item.id)}
      <li data-inbox-id={item.id} class:focused={focusId===item.id} class:processed={item.processed_at !== null}>
        <span class="content">{item.content}</span>
        <span class="muted">{fmtTime(item.created_at)}</span>
        {#if item.processed_at}
          <button onclick={()=>navigateTo(item.converted_to_type??'inbox',item.converted_to_id??undefined)}>{currentLocale==='en-US'?'View organized item':'查看整理后的事项'} ↗</button>
        {:else}
          <span class="actions">
            <button class="project-action" data-testid={`inbox-organize-${item.id}`} disabled={organizing(item.id)} onclick={()=>organize(item)}>{organizing(item.id)?(currentLocale==="en-US"?"Organizing…":"后台整理中…"):(currentLocale==="en-US"?"AI organize":"交给 AI 整理")}</button>
            <details class="manual-routing"><summary>{currentLocale==='en-US'?'Arrange myself':'自己安排'}</summary><div><button class="project-action" data-testid={`inbox-project-${item.id}`} onclick={() => openConversion("task",item,true)}>{tt("inbox.toProject")}</button>
            <button data-testid={`inbox-task-${item.id}`} onclick={() => openConversion("task", item)}>{tt("inbox.toTask")}</button>
            <button onclick={() => openConversion("waiting", item)}>{tt("inbox.toWaiting")}</button>
            <button onclick={() => openConversion("calendar", item)}>{tt("inbox.toCalendar")}</button>
            <button onclick={() => remove(item)}>{tt("common.delete")}</button></div></details>
          </span>
        {/if}
      </li>
    {/each}
  </ul>
  {#if items.length === 0}<div class="muted empty">{tt("inbox.empty")}</div>{/if}
</div>

<style>
  .history-switch{display:flex;gap:9px;align-items:center;margin:16px 0;color:var(--color-muted);font-size:14px}.manual-routing summary{padding:8px;cursor:pointer;font-size:14px}.manual-routing>div{display:flex;flex-wrap:wrap;gap:8px;padding:10px 0}.focused{outline:2px solid var(--color-primary);outline-offset:-2px}
  .organize-note{font-size:14px;color:var(--color-muted);line-height:1.6}
  .organize-note button{font:inherit;font-size:14px;padding:10px 14px;min-height:40px;border:0;border-radius:10px;background:var(--color-primary);color:white;cursor:pointer}.organize-note{display:flex;gap:12px;align-items:center;flex-wrap:wrap}
  .actions .project-action{background:var(--color-primary-soft);color:var(--color-primary);font-weight:650}.project-hint{font-size:14px;line-height:1.65;color:var(--color-muted)}.content{overflow-wrap:anywhere;white-space:pre-wrap}.in-list li{flex-wrap:wrap!important;gap:12px!important;padding:18px 12px!important}.in-list li .content{flex-basis:50%;font-size:16px;line-height:1.65}.in-list li .actions{margin-left:auto}.modal-form{gap:10px!important}
  .page-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; margin-bottom: 14px; }
  h1 { font-size: 18px; margin: 0 0 4px; }
  .hint { margin-bottom: 0; }
  .count { display: inline-grid; place-items: center; min-width: 28px; height: 28px; border-radius: 999px; background: var(--color-primary-soft); color: var(--color-primary); font-size: 12px; font-weight: 700; }
  .in-list { list-style: none; margin: 0; padding: 0; }
  .in-list li { display: flex; align-items: center; gap: 10px; padding: 10px; border-bottom: 1px solid var(--color-border); font-size: 13px; }
  .in-list li.processed .content { color: var(--color-muted); text-decoration: line-through; }
  .content { flex: 1; min-width: 0; }
  .muted { color: var(--color-muted); font-size: 12px; }
  .actions { display: flex; flex-wrap: wrap; gap: 4px; }
  button { padding: 5px 10px; border: 1px solid var(--color-border); border-radius: var(--radius-sm); background: var(--color-surface); font-size: 12px; cursor: pointer; }
  .modal-form { display: grid; gap: 8px; }
  .modal-form label { font-size: 12px; font-weight: 600; }
  .modal-form input, .modal-form textarea, .modal-form select { width: 100%; border: 1px solid var(--color-border); border-radius: var(--radius-sm); padding: 9px 10px; background: var(--color-surface); }
  .modal-actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 10px; }
  .status.error { color: var(--color-danger); font-size: 13px; margin: 6px 0; }
  .empty { padding: 18px 0; }
</style>
