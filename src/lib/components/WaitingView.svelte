<script lang="ts">
  import EmptyState from "$lib/components/ui/EmptyState.svelte";
  import StatusLine from "$lib/components/ui/StatusLine.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { locale, t } from "$lib/i18n";
  import Modal from "$lib/components/ui/Modal.svelte";
  import AppButton from "$lib/components/ui/AppButton.svelte";
  import { addToast } from "$lib/stores/toast";
  import { dataRevision, invalidate } from "$lib/stores/dataRevision";
  import ProjectScope from "./ProjectScope.svelte";
  import ProjectFilter from "./ProjectFilter.svelte";
  import ProjectBadge from "./ProjectBadge.svelte";
  import ListPager from './ui/ListPager.svelte';
  import {paginate} from '$lib/services/pagination';
  let listPage=$state(1);
  const waitingPage=$derived(paginate(sorted(),listPage));
  $effect(()=>{projectFilter;mode;listPage=1;});
  let projectFilter=$state("all");

  import {onMount} from 'svelte';
  import NaturalCapture from './NaturalCapture.svelte';
  let {focusId=null,mode='active'}:{focusId?:number|null;mode?:'active'|'done'}=$props();
  let focused=$state(false);let progressWaiting=$state<Waiting|null>(null);
  onMount(async()=>{await load();if(focusId){const item=items.find(t=>t.id===focusId);if(item){focused=true;openEdit(item);}else error=currentLocale==='en-US'?'This item no longer exists.':'该事项已不存在。';}});
  type Waiting = {
    id: number;
    work_id: number | null;
    title: string;
    waiting_for: string;
    started_at: number;
    follow_up_at: number | null;
    status: string;
    notes: string | null;
    created_at: number;
    updated_at: number;
    resolved_at: number | null;
  };

  let items = $state<Waiting[]>([]);
  let error = $state("");
  let newTitle = $state("");
  let newFor = $state("");
  let currentLocale = $derived($locale);
  const tt = (key: Parameters<typeof t>[0], params: Record<string, string | number> = {}) => t(key, params, currentLocale);
  let newFollowUp = $state("");
  let formWorkId = $state<number | null>(null);
  let formNotes = $state("");
  let formStarted = $state("");
  let showForm = $state(false);
  let editingId = $state<number | null>(null);
  let saving = $state(false);
  let works = $state<Array<{ id: number; title: string; status: string }>>([]);

  function nowSec() {
    return Math.floor(Date.now() / 1000);
  }

  function fmtTime(ts: number | null): string {
    if (!ts) return "";
    const d = new Date(ts * 1000);
    const pad = (n: number) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }

  function toSec(dateStr: string): number | null {
    if (!dateStr) return null;
    const t = Date.parse(dateStr);
    return isNaN(t) ? null : Math.floor(t / 1000);
  }

  async function load() {
    try {
      items = await invoke("list_waiting", { status: null, workId: null });
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

  function openNew() {
    editingId = null;
    newTitle = "";
    newFor = "";
    newFollowUp = "";
    formStarted = fmtTime(nowSec()).replace(" ","T");
    formNotes = "";
    formWorkId = null;
    showForm = true;
  }

  function openEdit(item: Waiting) {
    editingId = item.id;
    newTitle = item.title;
    newFor = item.waiting_for;
    newFollowUp = item.follow_up_at ? fmtTime(item.follow_up_at).replace(" ","T") : "";
    formStarted = fmtTime(item.started_at).replace(" ","T");
    formNotes = item.notes ?? "";
    formWorkId = item.work_id;
    showForm = true;
  }

  async function create() {
    if (!newTitle.trim()) {
      error = tt("common.required");
      return;
    }
    saving = true;
    error = "";
    try {
      const started = toSec(formStarted) ?? nowSec();
      if (editingId === null) {
        await invoke("create_waiting", { workId: formWorkId, title: newTitle.trim(), waitingFor: newFor.trim(), followUpAt: toSec(newFollowUp), notes: formNotes.trim() || null });
      } else {
        await invoke("update_waiting", { id: editingId, workId: formWorkId, title: newTitle.trim(), waitingFor: newFor.trim(), startedAt: started, followUpAt: toSec(newFollowUp), notes: formNotes.trim() || null });
      }
      showForm = false;
      addToast(tt("settings.saved"), "success");
      invalidate("waiting", "works", "brief");
      newTitle = "";
      newFor = "";
      newFollowUp = "";
      await load();
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  async function resolve(w: Waiting) {
    try {
      await invoke("resolve_waiting", { id: w.id });
      invalidate("waiting", "works", "brief");
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  async function remove(w: Waiting) {
    try {
      await invoke("delete_waiting", { id: w.id });
      invalidate("waiting", "works", "brief");
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  function daysWaiting(w: Waiting): number {
    return Math.max(0, Math.floor((nowSec() - w.started_at) / 86400));
  }

  function needsFollowUp(w: Waiting): boolean {
    return w.status === "open" && !!w.follow_up_at && w.follow_up_at <= nowSec();
  }

  function sorted(): Waiting[] {
    return items.filter(item=>focused||(mode==='done'?item.status==='resolved':item.status!=='resolved')).filter(item=>projectFilter==="all"||(projectFilter==="independent"?item.work_id===null:item.work_id===Number(projectFilter))).sort((a, b) => Number(a.status === "resolved") - Number(b.status === "resolved"));
  }

  $effect(() => {
    $dataRevision.waiting;
    $dataRevision.works;
    load();
    loadWorks();
  });
</script>

<div class="waiting">
  <div class="page-head">
    <h1>{currentLocale==='en-US'?(mode==='done'?'Resolved waiting':'Waiting for a response'):(mode==='done'?'已结束的等待':'正在等回应的事')}</h1>
    <AppButton testid="waiting-create" label={tt("common.create")} onclick={() => openNew()} />
  </div>
  <div class="status error stable-feedback"><StatusLine message={error}/></div>
  <ProjectFilter {works} bind:value={projectFilter}/>
  <ListPager view={waitingPage} onchange={(page)=>listPage=page} testid="waiting-pagination"/>

  <Modal bind:open={showForm} title={editingId === null ? tt("common.create") : tt("common.edit")} onclose={() => (showForm = false)}>
    <form class="modal-form" onsubmit={(event) => { event.preventDefault(); create(); }}>
      <label for="waiting-title">{tt("waiting.title")} *</label>
      <input id="waiting-title" bind:value={newTitle} placeholder={tt("waiting.placeholder")} />
      <ProjectScope {works} bind:value={formWorkId} id="waiting-work"/>
      <label for="waiting-for">{tt("waiting.for")}</label>
      <input id="waiting-for" bind:value={newFor} />
      <label for="waiting-started">{tt("common.startTime")}</label>
      <input id="waiting-started" type="datetime-local" bind:value={formStarted} />
      <label for="waiting-follow">{tt("common.followUpTime")}</label>
      <input id="waiting-follow" type="datetime-local" bind:value={newFollowUp} />
      <label for="waiting-notes">{tt("work.summary")}</label>
      <textarea id="waiting-notes" rows="3" bind:value={formNotes}></textarea>
      <StatusLine message={error}/>
      <div class="modal-actions">
        <button type="button" onclick={() => (showForm = false)}>{tt("common.cancel")}</button>
        <AppButton testid="waiting-save" type="submit" loading={saving} label={editingId === null ? tt("common.create") : tt("common.save")} />
      </div>
    </form>
  </Modal>

  <Modal open={progressWaiting!==null} title={currentLocale==='en-US'?'Record an update':'收到回应了吗？'} onclose={()=>progressWaiting=null}>{#if progressWaiting}<NaturalCapture context={{workId:progressWaiting.work_id,entityKind:'waiting',entityId:progressWaiting.id}} label={progressWaiting.title}/>{/if}</Modal>
  {#if sorted().length}
  <ul class="w-list">
    {#each waitingPage.items as w (w.id)}
      <li class:resolved={w.status === "resolved"} class:overdue={needsFollowUp(w)}>
        <div class="waiting-copy"><strong class="title">{w.title}</strong><div class="waiting-meta">
        <ProjectBadge {works} workId={w.work_id}/>
        {#if w.waiting_for}<span class="muted">{tt("waiting.waitingDetail", { person: w.waiting_for })}</span>{/if}
        <span class="muted">{tt("waiting.daysWaiting", { days: daysWaiting(w) })}</span>
        {#if w.follow_up_at}<span class="muted">{tt("work.followUpDetail", { time: fmtTime(w.follow_up_at) })}</span>{/if}
        {#if needsFollowUp(w)}<span class="badge">{tt("waiting.followUp")}</span>{/if}
        </div></div>
        <span class="actions">
          {#if w.status !== "resolved"}
            <button onclick={() => resolve(w)}>{tt("common.resolve")}</button>
          {/if}
          <button onclick={()=>progressWaiting=w}>{currentLocale==='en-US'?'Record response':'记一下回应'}</button>
          <button onclick={() => openEdit(w)}>{tt("common.edit")}</button>
          <button onclick={() => remove(w)}>{tt("common.delete")}</button>
        </span>
      </li>
    {/each}
  </ul>
  {:else}
    <EmptyState compact framed title={tt("common.empty")}/>
  {/if}
</div>

<style>
  h1 {
    font-size: 18px;
    margin: 0 0 12px;
  }
  .page-head { display: flex; align-items: center; justify-content: space-between; gap: 16px; margin-bottom: 14px; }
  .page-head h1 { margin: 0; }
  .modal-form { display: grid; gap: 8px; }
  .modal-form label { font-size: 12px; font-weight: 600; }
  .modal-form input,
  .modal-form textarea { width: 100%; border: 1px solid var(--color-border); border-radius: var(--radius-sm); padding: 9px 10px; background: var(--color-surface); }
  .modal-actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 10px; }
  button {
    padding: 6px 10px;
    border: 1px solid var(--color-border);
    border-radius: 6px;
    background: var(--color-surface-raised);
    font-size: 12px;
    cursor: pointer;
  }
  .w-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .w-list li {
    display: grid;
    grid-template-columns: minmax(0,1fr) auto;
    align-items: start;
    gap: 12px;
    padding: 16px;
    border-bottom: 1px solid #e9eef0;
    font-size: 13px;
  }
  .w-list li.resolved .title {
    text-decoration: line-through;
    color: var(--color-subtle);
  }
  .w-list li.overdue {
    background: var(--color-danger-soft);
  }
  .title {
    font-size: 15px;
    font-weight: 600;
    overflow-wrap: anywhere;
  }
  .waiting-copy { min-width: 0; display: grid; gap: 8px; }
  .waiting-meta { display: flex; flex-wrap: wrap; align-items: center; gap: 6px 16px; line-height: 1.5; overflow-wrap: anywhere; }
  .actions { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: 6px; }
  .muted {
    color: var(--color-muted);
    font-size: 12px;
  }
  .badge {
    background: var(--color-danger);
    color: #fff;
    font-size: 11px;
    padding: 1px 6px;
    border-radius: 4px;
  }
  .actions button {
    margin: 0;
  }
  .status.error {
    color: var(--color-danger);
    font-size: 13px;
    margin: 6px 0;
  }
  @container (max-width: 720px) {
    .w-list li { grid-template-columns: minmax(0,1fr); }
    .actions { justify-content: flex-start; }
  }
</style>
