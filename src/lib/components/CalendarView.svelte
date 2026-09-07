<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { locale, t, translateKind } from "$lib/i18n";
  import Modal from "$lib/components/ui/Modal.svelte";
  import AppButton from "$lib/components/ui/AppButton.svelte";
  import { addToast } from "$lib/stores/toast";
  import { dataRevision, invalidate } from "$lib/stores/dataRevision";
  import ProjectScope from "./ProjectScope.svelte";
  import ProjectFilter from "./ProjectFilter.svelte";
  import ProjectBadge from "./ProjectBadge.svelte";
  let projectFilter=$state("all");

  import {onMount} from 'svelte';
  import {navigateTo} from '$lib/services/navigation';
  import type {Task,WaitingItem} from '$lib/types/domain';
  import {projectCalendar} from '$lib/services/calendarProjection';
  let {focusId=null}:{focusId?:number|null}=$props();
  onMount(async()=>{if(focusId){try{const location=await invoke<{at:number}>('get_entity_location',{kind:'calendar',id:focusId});anchor=new Date(location.at*1000);await load();const ev=events.find(e=>e.id===focusId);if(ev)openEdit(ev);}catch(e){error=String(e);}}});
  type CalendarEvent = {
    task_id?:number;
    waiting_id?:number;
    time_kind?:string;
    completed?:boolean;
    id: number;
    work_id: number | null;
    title: string;
    start_at: number;
    end_at: number | null;
    all_day: boolean;
    location: string | null;
    notes: string | null;
    kind: string;
    created_at: number;
    updated_at: number;
  };

  const KINDS = ["meeting", "kol_visit", "deadline", "travel", "work_block", "other"];
  let mode = $state<"day" | "week">("week");
  let anchor = $state<Date>(new Date());
  let events = $state<CalendarEvent[]>([]);
  let error = $state("");
  let showForm = $state(false);

  // 新建表单
  let editing = $state<CalendarEvent | null>(null);
  let formTitle = $state("");
  let formStart = $state("");
  let formEnd = $state("");
  let currentLocale = $derived($locale);
  const tt = (key: Parameters<typeof t>[0], params: Record<string, string | number> = {}) => t(key, params, currentLocale);
  let formKind = $state("other");
  let formAllDay = $state(false);
  let formWorkId = $state<number | null>(null);
  let formLocation = $state("");
  let formNotes = $state("");
  let saving = $state(false);
  let works = $state<Array<{ id: number; title: string; status: string }>>([]);

  function pad(n: number) {
    return String(n).padStart(2, "0");
  }
  function toLocalInput(d: Date): string {
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }
  function fmtTime(ts: number): string {
    const d = new Date(ts * 1000);
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }
  function toSec(s: string): number | null {
    if (!s) return null;
    const t = Date.parse(s);
    return isNaN(t) ? null : Math.floor(t / 1000);
  }

  function dayStart(d: Date): Date {
    const c = new Date(d);
    c.setHours(0, 0, 0, 0);
    return c;
  }
  function weekStart(d: Date): Date {
    const c = dayStart(d);
    const day = c.getDay(); // 0=Sun
    c.setDate(c.getDate() - ((day + 6) % 7)); // Monday
    return c;
  }

  async function load() {
    const start = mode === "day" ? dayStart(anchor) : weekStart(anchor);
    const end = mode === "day" ? new Date(start.getTime() + 86400_000) : new Date(start.getTime() + 7 * 86400_000);
    try {
      const independent = await invoke<CalendarEvent[]>("list_calendar_events", {
        start: Math.floor(start.getTime() / 1000),
        end: Math.floor(end.getTime() / 1000),
      });
      const tasks=await invoke<Task[]>('list_tasks',{status:null,workId:null});
      const waiting=await invoke<WaitingItem[]>('list_waiting',{status:null,workId:null});
      events=[...independent,...projectCalendar(tasks,waiting,start.getTime()/1000,end.getTime()/1000)];
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
    editing = null;
    showForm = true;
    formTitle = "";
    formStart = toLocalInput(new Date());
    formEnd = "";
    formKind = "other";
    formAllDay = false;
    formWorkId = null;
    formLocation = "";
    formNotes = "";
  }

  function openEdit(ev: CalendarEvent) {
    if(ev.task_id){navigateTo('task',ev.task_id);return;}
    if(ev.waiting_id){navigateTo('waiting',ev.waiting_id);return;}
    editing = ev;
    showForm = true;
    formTitle = ev.title;
    formStart = toLocalInput(new Date(ev.start_at * 1000));
    formEnd = ev.end_at ? toLocalInput(new Date(ev.end_at * 1000)) : "";
    formKind = ev.kind;
    formAllDay = ev.all_day;
    formWorkId = ev.work_id;
    formLocation = ev.location ?? "";
    formNotes = ev.notes ?? "";
  }

  async function saveForm() {
    const startSec = toSec(formStart);
    if (!startSec || !formTitle.trim()) {
      error = !formTitle.trim() ? tt("common.required") : tt("calendar.invalidStart");
      return;
    }
    const endSec = formAllDay ? startSec + 86400 : toSec(formEnd);
    if (endSec !== null && endSec < startSec) {
      error = tt("calendar.endBeforeStart");
      return;
    }
    saving = true;
    try {
      if (editing) {
        await invoke("update_calendar_event", {
          id: editing.id,
          workId: formWorkId,
          title: formTitle.trim(),
          startAt: startSec,
          endAt: endSec,
          allDay: formAllDay,
          kind: formKind,
          location: formLocation.trim() || null,
          notes: formNotes.trim() || null,
        });
      } else {
        await invoke("create_calendar_event", {
          workId: formWorkId,
          title: formTitle.trim(),
          startAt: startSec,
          endAt: endSec,
          allDay: formAllDay,
          kind: formKind,
          location: formLocation.trim() || null,
          notes: formNotes.trim() || null,
        });
      }
      editing = null;
      showForm = false;
      addToast(tt("settings.saved"), "success");
      invalidate("calendar", "works", "brief");
      await load();
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  async function remove(ev: CalendarEvent) {
    try {
      await invoke("delete_calendar_event", { id: ev.id });
      invalidate("calendar", "works", "brief");
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  function shift(days: number) {
    const d = new Date(anchor);
    d.setDate(d.getDate() + days);
    anchor = d;
  }

  function shiftWeek(weeks: number) {
    const d = new Date(anchor);
    d.setDate(d.getDate() + weeks * 7);
    anchor = d;
  }

  function timeKind(ev:CalendarEvent):string{
    if(ev.time_kind==='deadline')return currentLocale==='en-US'?'Task deadline':'任务截止';
    if(ev.time_kind==='followup')return currentLocale==='en-US'?'Follow-up reminder':'等待跟进';
    if(ev.time_kind==='appointment')return currentLocale==='en-US'?'Task appointment':'任务安排';
    return translateKind(ev.kind,currentLocale);
  }
  function sorted(): CalendarEvent[] {
    return events.filter(event=>projectFilter==="all"||(projectFilter==="independent"?event.work_id===null:event.work_id===Number(projectFilter))).sort((a, b) => a.start_at - b.start_at);
  }

  function weekDays(): Date[] {
    const start = weekStart(anchor);
    return Array.from({ length: 7 }, (_, index) => {
      const day = new Date(start);
      day.setDate(start.getDate() + index);
      return day;
    });
  }

  function eventsForDay(day: Date): CalendarEvent[] {
    const start = Math.floor(dayStart(day).getTime() / 1000);
    const end = start + 86400;
    return sorted().filter((event) => event.start_at < end && (event.end_at ?? event.start_at + 1) >= start);
  }

  function rangeLabel(): string {
    const start = mode === "day" ? dayStart(anchor) : weekStart(anchor);
    const end = mode === "day" ? start : new Date(start.getTime() + 6 * 86400_000);
    return `${start.toLocaleDateString(currentLocale)}${mode === "week" ? ` – ${end.toLocaleDateString(currentLocale)}` : ""}`;
  }

  $effect(() => {
    $dataRevision.calendar;
    $dataRevision.tasks;
    $dataRevision.works;
    $dataRevision.tasks;
    $dataRevision.waiting;
    load();
    loadWorks();
  });
</script>

<div class="calendar">
  <div class="row">
    <h1>{tt("calendar.title")}</h1>
    <button onclick={() => (mode = "day")} class:active={mode === "day"}>{tt("calendar.day")}</button>
    <button onclick={() => (mode = "week")} class:active={mode === "week"}>{tt("calendar.week")}</button>
    <span class="spacer"></span>
    <button aria-label={tt("calendar.previous")} onclick={() => (mode === "day" ? shift(-1) : shiftWeek(-1))} title={tt("calendar.previous")}>‹</button>
    <button class="today-button" onclick={() => (anchor = new Date())}>{tt("common.today")}</button>
    <span class="anchor">{rangeLabel()}</span>
    <button aria-label={tt("calendar.next")} onclick={() => (mode === "day" ? shift(1) : shiftWeek(1))} title={tt("calendar.next")}>›</button>
    <button data-testid="calendar-create" onclick={openNew}>{tt("calendar.new")}</button>
  </div>

  {#if error}<div class="status error">{error}</div>{/if}
  <ProjectFilter {works} bind:value={projectFilter}/>

  <Modal bind:open={showForm} title={editing ? tt("common.edit") : tt("calendar.new")} onclose={() => { showForm = false; editing = null; }}>
    <form class="modal-form" onsubmit={(event) => { event.preventDefault(); saveForm(); }}>
      <label for="calendar-title">{tt("calendar.titlePlaceholder")} *</label>
      <input id="calendar-title" bind:value={formTitle} placeholder={tt("calendar.titlePlaceholder")} />
      <ProjectScope {works} bind:value={formWorkId} id="calendar-work"/>
      <label for="calendar-kind">{tt("calendar.title")}</label>
      <select id="calendar-kind" bind:value={formKind}>
        {#each KINDS as k (k)}
          <option value={k}>{translateKind(k, currentLocale)}</option>
        {/each}
      </select>
      <label class="check"><input type="checkbox" bind:checked={formAllDay} /> {tt("calendar.allDay")}</label>
      <label for="calendar-start">{tt("calendar.start")}</label>
      <input id="calendar-start" type={formAllDay ? "date" : "datetime-local"} bind:value={formStart} />
      {#if !formAllDay}
        <label for="calendar-end">{tt("calendar.endPlaceholder")}</label>
        <input id="calendar-end" type="datetime-local" bind:value={formEnd} />
      {/if}
      <label for="calendar-location">{tt("calendar.location")}</label>
      <input id="calendar-location" bind:value={formLocation} />
      <label for="calendar-notes">{tt("calendar.notes")}</label>
      <textarea id="calendar-notes" rows="3" bind:value={formNotes}></textarea>
      <div class="modal-actions">
        {#if editing}<button type="button" class="danger" data-testid="calendar-delete" onclick={() => remove(editing as CalendarEvent)}>{tt("common.delete")}</button>{/if}
        <button type="button" onclick={() => { showForm = false; editing = null; }}>{tt("common.cancel")}</button>
        <AppButton testid="calendar-save" type="submit" loading={saving} label={editing ? tt("common.save") : tt("common.add")} />
      </div>
    </form>
  </Modal>

  {#if mode === "week"}
    <p class="calendar-hint">{currentLocale==='en-US'?'Appointments, deadlines and follow-ups appear automatically after you accept the arrangement. Click any item to edit the original.':'采用安排后，预约、截止和等待跟进自动出现在这里；点击可修改原事项，无需再次录入。'}</p>
    <div class="week-grid">
      {#each weekDays() as day (day.toISOString())}
        <section class="day-column">
          <header><strong>{day.toLocaleDateString(currentLocale, { weekday: "short" })}</strong><span>{day.toLocaleDateString(currentLocale, { month: "numeric", day: "numeric" })}</span></header>
          <div class="all-day-slot">{#each eventsForDay(day).filter((event) => event.all_day) as ev (ev.id)}<button data-task-id={ev.task_id} data-waiting-id={ev.waiting_id} data-time-kind={ev.time_kind} class:completed={ev.completed} class="event-card all-day" onclick={() => openEdit(ev)}><small>{timeKind(ev)}{ev.completed?(currentLocale==='en-US'?' · Completed':' · 已完成'):''}</small><span>{ev.title}</span><ProjectBadge {works} workId={ev.work_id}/></button>{/each}</div>
          <div class="event-stack">
            {#each eventsForDay(day).filter((event) => !event.all_day) as ev (ev.id)}
              <button data-task-id={ev.task_id} data-waiting-id={ev.waiting_id} data-time-kind={ev.time_kind} class:completed={ev.completed} class={"event-card kind-" + ev.kind} onclick={() => openEdit(ev)}>
                <span class="event-time">{fmtTime(ev.start_at).slice(-5)}</span>
                <span>{ev.title}</span>
                <ProjectBadge {works} workId={ev.work_id}/>
                <small>{timeKind(ev)}{ev.location ? " · " + ev.location : ""}</small>
              </button>
            {/each}
          </div>
        </section>
      {/each}
    </div>
  {:else}
    <div class="day-timeline">
      {#each sorted() as ev (ev.id)}
        <button data-task-id={ev.task_id} data-waiting-id={ev.waiting_id} data-time-kind={ev.time_kind} class:completed={ev.completed} class={"event-card kind-" + ev.kind} onclick={() => openEdit(ev)}>
          <span class="event-time">{ev.all_day ? tt("calendar.allDay") : fmtTime(ev.start_at)}</span>
          <span>{ev.title}</span>
          <ProjectBadge {works} workId={ev.work_id}/>
          <small>{timeKind(ev)}{ev.location ? " · " + ev.location : ""}</small>
        </button>
      {/each}
    </div>
  {/if}
  {#if sorted().length === 0}
    <div class="muted empty">{tt("calendar.empty")}</div>
  {/if}
</div>

<style>
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
    margin-bottom: 12px;
  }
  h1 {
    font-size: 18px;
    margin: 0;
  }
  .spacer {
    flex: 1;
  }
  .anchor {
    font-size: 13px;
    font-weight: 600;
  }
  button {
    padding: 6px 10px;
    border: 1px solid var(--color-border);
    border-radius: 6px;
    background: var(--color-surface-raised);
    font-size: 12px;
    cursor: pointer;
  }
  button.active {
    background: var(--color-primary-soft);
    font-weight: 600;
  }
  .modal-form { display: grid; gap: 8px; }
  .modal-form label { font-size: 12px; font-weight: 600; }
  .modal-form input,
  .modal-form textarea,
  .modal-form select { width: 100%; border: 1px solid var(--color-border); border-radius: var(--radius-sm); padding: 9px 10px; background: var(--color-surface); }
  .modal-form .check { display: flex; align-items: center; gap: 8px; }
  .modal-actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 10px; }
  .week-grid { display: grid; grid-template-columns: repeat(7, minmax(0, 1fr)); gap: 8px; min-width: 0; }
  .day-column { min-height: 360px; border: 1px solid var(--color-border); border-radius: var(--radius-sm); background: var(--color-surface); overflow: hidden; }
  .day-column > header { display: flex; justify-content: space-between; gap: 8px; padding: 10px; background: var(--color-surface-muted); border-bottom: 1px solid var(--color-border); font-size: 12px; }
  .day-column > header span { color: var(--color-muted); }
  .all-day-slot, .event-stack { display: grid; gap: 6px; padding: 8px; }
  .all-day-slot { min-height: 40px; border-bottom: 1px dashed var(--color-border); }
  .event-card { display: grid; gap: 3px; width: 100%; border: 1px solid #d3e0e4; border-left: 3px solid var(--color-primary); border-radius: var(--radius-sm); background: #edf2f3; padding: 8px; text-align: left; font-size: 12px; cursor: pointer; overflow-wrap: anywhere; }
  .event-card:hover { filter: brightness(.98); }
  .event-card small { color: var(--color-muted); font-size: 11px; }
  .event-card.all-day { padding: 6px; }
  .event-time { color: var(--color-muted); font-size: 11px; }
  .day-timeline { display: grid; gap: 8px; min-height: 320px; padding: 8px; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: repeating-linear-gradient(to bottom, var(--color-surface), var(--color-surface) 59px, var(--color-border) 60px); }
  .kind-kol_visit { border-left-color: #8793a2; background: #eff1f4; }
  .kind-deadline { border-left-color: var(--color-danger); background: var(--color-danger-soft); }
  .kind-travel { border-left-color: var(--color-warning); background: var(--color-warning-soft); }
  .kind-work_block { border-left-color: var(--color-success); background: var(--color-success-soft); }
  .status.error {
    color: var(--color-danger);
    font-size: 13px;
    margin: 6px 0;
  }
  .empty {
    padding: 12px 0;
  }
.calendar-hint{font-size:14px;line-height:1.6;color:var(--color-muted);margin:12px 0}.event-card.completed{opacity:.65}.event-card.completed>span:not(.event-time){text-decoration:line-through}
  @container(max-width:1000px){.week-grid{grid-template-columns:repeat(3,minmax(0,1fr))}.day-column{min-height:130px}.all-day-slot:empty{display:none}.event-card{font-size:14px}}
  @container(max-width:650px){.week-grid{grid-template-columns:repeat(2,minmax(0,1fr))}.day-column>header{flex-wrap:wrap}}
</style>
