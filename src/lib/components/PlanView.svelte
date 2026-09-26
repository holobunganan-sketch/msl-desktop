<script lang="ts">
  import EmptyState from "$lib/components/ui/EmptyState.svelte";
  import StatusLine from "$lib/components/ui/StatusLine.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { locale, t, translateStatus } from "$lib/i18n";
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

  import {onMount,untrack} from 'svelte';
  import {navigateTo} from '$lib/services/navigation';
  import NaturalCapture from './NaturalCapture.svelte';
  import ManualCompletionFeedback from './ManualCompletionFeedback.svelte';
  import ItemDeleteDialog from './ItemDeleteDialog.svelte';
  import {completeManual} from '$lib/stores/manualCompletions';
  let {focusId=null,mode='active',workId=null}:{focusId?:number|null;mode?:'active'|'done';workId?:number|null}=$props();
  let deleteTarget=$state<Task|null>(null);
  let focused=$state(false);
  let scheduleId=$state<number|null>(null);
  let scheduleStart=$state('');let scheduleEnd=$state('');let scheduleBusy=$state(false);
  let progressTask=$state<Task|null>(null);
  async function saveSchedule(){
    if(scheduleId===null||scheduleBusy)return;scheduleBusy=true;error='';
    try{await invoke('schedule_work_task',{id:scheduleId,start:toSec(scheduleStart),end:toSec(scheduleEnd)});scheduleId=null;invalidate('tasks','calendar','works','brief');await load();}
    catch(e){error=String(e);}finally{scheduleBusy=false;}
  }
  onMount(async()=>{await load();if(focusId){const item=tasks.find(t=>t.id===focusId);if(item){filter='all';openEdit(item);focused=true;}else error=currentLocale==='en-US'?'This item no longer exists.':'该事项已不存在。';}});
  type Task = {
    id: number;
    work_id: number | null;
    title: string;
    status: string;
    priority: string;
    due_at: number | null;
    scheduled_start: number | null;
    scheduled_end: number | null;
    notes: string | null;
    created_at: number;
    updated_at: number;
    completed_at: number | null;
  };

  let tasks = $state<Task[]>([]);
  let filter = $state("all"); // all | next | done | ...
  let projectFilter=$state(untrack(()=>workId?String(workId):"all"));
  let error = $state("");
  let newTitle = $state("");
  let newPriority = $state("normal");
  let newDue = $state("");
  let formNotes = $state("");
  let formWorkId = $state<number | null>(null);
  let showForm = $state(false);
  let editingId = $state<number | null>(null);
  let saving = $state(false);
  let works = $state<Array<{ id: number; title: string; status: string }>>([]);
  let currentLocale = $derived($locale);
  const tt = (key: Parameters<typeof t>[0], params: Record<string, string | number> = {}) => t(key, params, currentLocale);

  const STATUS_ORDER = ["next", "doing", "scheduled", "waiting", "paused", "done"];

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
      const st = filter === "all" ? null : filter;
      tasks = await invoke("list_tasks", { status: st, workId: null });
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
    newPriority = "normal";
    newDue = "";
    formNotes = "";
    formWorkId = Number(projectFilter)>0?Number(projectFilter):null;
    showForm = true;
  }

  function openEdit(task: Task) {
    editingId = task.id;
    newTitle = task.title;
    newPriority = task.priority;
    newDue = task.due_at ? fmtTime(task.due_at).replace(" ","T") : "";
    formNotes = task.notes ?? "";
    formWorkId = task.work_id;
    showForm = true;
  }

  async function createTask() {
    if (!newTitle.trim()) {
      error = tt("common.required");
      return;
    }
    saving = true;
    error = "";
    try {
      if (editingId === null) {
        await invoke("create_task", { workId: formWorkId, title: newTitle.trim(), priority: newPriority, dueAt: toSec(newDue), notes: formNotes.trim() || null });
      } else {
        await invoke("update_task", { id: editingId, workId: formWorkId, title: newTitle.trim(), priority: newPriority, dueAt: toSec(newDue), notes: formNotes.trim() || null });
      }
      showForm = false;
      addToast(tt("settings.saved"), "success");
      invalidate("tasks", "works", "brief");
      newTitle = "";
      newDue = "";
      await load();
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  async function complete(t: Task) {
    try {
      await completeManual('task',t.id);
      invalidate("tasks", "works", "brief");
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  async function remove(t: Task) {
    deleteTarget=JSON.parse(JSON.stringify(t));
  }

  function isOverdue(t: Task): boolean {
    return t.status !== "done" && !!t.due_at && t.due_at < nowSec();
  }

  function visibleTasks(): Task[] {
    const scoped=tasks.filter(task=>(focused||mode==='done'||filter==='done'?focused||task.status==='done':task.status!=='done')).filter(task=>projectFilter==="all"||(projectFilter==="independent"?task.work_id===null:task.work_id===Number(projectFilter)));
    if (filter === "all") {
      // 未完成按截止时间排序
      return [...scoped].sort((a, b) => (a.due_at ?? 9e15) - (b.due_at ?? 9e15));
    }
    return scoped;
  }
  const taskPage=$derived(paginate(visibleTasks(),listPage));
  $effect(()=>{filter;projectFilter;mode;listPage=1;});

  $effect(() => {
    $dataRevision.tasks;
    $dataRevision.works;
    load();
    loadWorks();
  });
</script>

<div class="plan">
  <div class="row">
    <h1>{currentLocale==='en-US'?(mode==='done'?'Completed tasks':'Your next actions'):(mode==='done'?'做完的事项':'接下来要做的事')}</h1>
    <AppButton testid="task-create" label={tt("common.create")} onclick={() => openNew()} />
    {#if mode!=='done'}<select bind:value={filter} onchange={load} aria-label={tt('common.status')}>
      <option value="all">{tt("common.all")}</option>
      {#each STATUS_ORDER as s (s)}
        <option value={s}>{translateStatus(s, currentLocale)}</option>
      {/each}
    </select>{/if}
  </div>

  <ManualCompletionFeedback {error} onrefresh={load}/>
  <ItemDeleteDialog record={deleteTarget} kind="task" {works} onclose={()=>deleteTarget=null} oncomplete={load}/>
  <ProjectFilter {works} bind:value={projectFilter}/>
  <ListPager view={taskPage} onchange={(page)=>listPage=page} testid="task-pagination"/>

  <Modal bind:open={showForm} title={editingId === null ? tt("common.create") : tt("common.edit")} onclose={() => (showForm = false)}>
    <form class="modal-form" onsubmit={(event) => { event.preventDefault(); createTask(); }}>
      <label for="task-title">{tt("task.title")} *</label>
      <input id="task-title" bind:value={newTitle} placeholder={tt("task.placeholder")} />
      <ProjectScope {works} bind:value={formWorkId} id="task-work"/>
      <label for="task-priority">{tt("task.priority.normal")}</label>
      <select id="task-priority" bind:value={newPriority}>
        <option value="low">{tt("task.priority.low")}</option>
        <option value="normal">{tt("task.priority.normal")}</option>
        <option value="high">{tt("task.priority.high")}</option>
      </select>
      <label for="task-due">{tt("task.due")}</label>
      <input id="task-due" type="datetime-local" bind:value={newDue} />
      <label for="task-notes">{tt("work.summary")}</label>
      <textarea id="task-notes" rows="3" bind:value={formNotes}></textarea>
      <StatusLine message={error}/>
      <div class="modal-actions">
        <button type="button" onclick={() => (showForm = false)}>{tt("common.cancel")}</button>
        <AppButton testid="task-save" type="submit" loading={saving} label={editingId === null ? tt("common.create") : tt("common.save")} />
      </div>
    </form>
  </Modal>

  <Modal open={scheduleId!==null} title={currentLocale==='en-US'?'Arrange this task':'为这件事安排时间'} onclose={()=>scheduleId=null}>
    <form class="modal-form" onsubmit={e=>{e.preventDefault();void saveSchedule();}}>
      <p>{currentLocale==='en-US'?'This same task will appear in Calendar. Clearing the time keeps the task.':'这件事会同时出现在日历中。清空时间只取消安排，事项仍保留。'}</p>
      <label>{currentLocale==='en-US'?'Start':'开始时间'}<input data-testid="task-schedule-start" type="datetime-local" bind:value={scheduleStart}/></label>
      <label>{currentLocale==='en-US'?'End (optional)':'结束时间（可选）'}<input data-testid="task-schedule-end" type="datetime-local" bind:value={scheduleEnd}/></label>
      <StatusLine message={error}/>
      <AppButton type="submit" testid="task-schedule-save" loading={scheduleBusy}>{tt('common.save')}</AppButton>
    </form>
  </Modal>
  <Modal open={progressTask!==null} title={currentLocale==='en-US'?'Record progress':'记一下进展'} onclose={()=>progressTask=null}>
    {#if progressTask}<NaturalCapture context={{workId:progressTask.work_id,entityKind:'task',entityId:progressTask.id}} label={progressTask.title}/>{/if}
  </Modal>
  {#if taskPage.total}
  <ul class="task-list">
    {#each taskPage.items as t (t.id)}
      <li data-testid={`task-row-${t.id}`} class:done={t.status === "done"} class:overdue={isOverdue(t)}>
        <span class="prio prio-{t.priority}">{tt(t.priority === "high" ? "task.priority.high" : t.priority === "low" ? "task.priority.low" : "task.priority.normal")}</span>
        <div class="task-copy"><strong class="title">{t.title}</strong><div class="task-meta">
        <ProjectBadge {works} workId={t.work_id}/>{#if t.scheduled_start}<span>{currentLocale==='en-US'?'Arranged':'已安排'} · {fmtTime(t.scheduled_start)}</span>{/if}
        <span class="muted">{translateStatus(t.status, currentLocale)}{t.due_at ? ` · ${tt("task.due")} ${fmtTime(t.due_at)}` : ""}</span>
        {#if isOverdue(t)}<span class="badge overdue">{tt("task.overdue")}</span>{/if}
        </div></div>
        <span class="actions">
          {#if t.status !== "done"}
          <button onclick={() => complete(t)}>{tt("common.complete")}</button>
          {/if}
          <button data-testid={`task-schedule-${t.id}`} onclick={()=>{scheduleId=t.id;scheduleStart=t.scheduled_start?fmtTime(t.scheduled_start).replace(' ','T'):'';scheduleEnd=t.scheduled_end?fmtTime(t.scheduled_end).replace(' ','T'):'';}}>{currentLocale==='en-US'?'Set time':'安排时间'}</button>
          <button onclick={()=>progressTask=t}>{currentLocale==='en-US'?'Record progress':'记进展'}</button>
          <button onclick={() => openEdit(t)}>{tt("common.edit")}</button>
          <button onclick={() => remove(t)}>{tt("common.delete")}</button>
        </span>
      </li>
    {/each}
  </ul>
  {:else}
    <EmptyState compact framed title={tt("common.empty")}/>
  {/if}
</div>

<style>
  .plan h1 {
    font-size: 18px;
    margin: 0;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;
  }
  .modal-form { display: grid; gap: 8px; }
  .modal-form label { font-size: 12px; font-weight: 600; }
  .modal-form input,
  .modal-form textarea,
  .modal-form select { width: 100%; border: 1px solid var(--color-border); border-radius: var(--radius-sm); padding: 9px 10px; background: var(--color-surface); }
  .modal-actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 10px; }
  select,
  button {
    padding: 6px 10px;
    border: 1px solid var(--color-border);
    border-radius: 6px;
    background: var(--color-surface-raised);
    font-size: 12px;
    cursor: pointer;
  }
  .task-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .task-list li {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: start;
    gap: 12px;
    padding: 16px;
    border-bottom: 1px solid #e9eef0;
    font-size: 13px;
  }
  .task-list li.done .title {
    text-decoration: line-through;
    color: var(--color-subtle);
  }
  .task-list li.overdue {
    background: var(--color-danger-soft);
  }
  .prio {
    font-size: 12px;
    padding: 4px 9px;
    border-radius: 999px;
    color: #fff;
    width: max-content;
    white-space: nowrap;
    text-align: center;
  }
  .prio-high {
    background: var(--color-danger);
  }
  .prio-normal {
    background: var(--color-muted);
  }
  .prio-low {
    background: var(--color-subtle);
  }
  .title {
    font-size: 15px;
    font-weight: 600;
    overflow-wrap: anywhere;
  }
  .task-copy { min-width: 0; display: grid; gap: 8px; }
  .task-meta { display: flex; flex-wrap: wrap; align-items: center; gap: 6px 16px; line-height: 1.5; overflow-wrap: anywhere; }
  .actions { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: 6px; }
  .muted {
    color: var(--color-muted);
    font-size: 12px;
  }
  .badge.overdue {
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
    .task-list li { grid-template-columns: auto minmax(0, 1fr); }
    .actions { grid-column: 2; justify-content: flex-start; }
  }
</style>
