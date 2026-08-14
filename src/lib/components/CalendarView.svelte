<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  type CalendarEvent = {
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
  const KIND_LABEL: Record<string, string> = {
    meeting: "会议",
    kol_visit: "专家拜访",
    deadline: "截止",
    travel: "出差",
    work_block: "工作块",
    other: "其他",
  };

  let mode = $state<"day" | "week">("day");
  let anchor = $state<Date>(new Date());
  let events = $state<CalendarEvent[]>([]);
  let error = $state("");
  let showForm = $state(false);

  // 新建表单
  let editing = $state<CalendarEvent | null>(null);
  let formTitle = $state("");
  let formStart = $state("");
  let formEnd = $state("");
  let formKind = $state("other");

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
      events = await invoke("list_calendar_events", {
        start: Math.floor(start.getTime() / 1000),
        end: Math.floor(end.getTime() / 1000),
      });
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
  }

  function openEdit(ev: CalendarEvent) {
    editing = ev;
    showForm = true;
    formTitle = ev.title;
    formStart = toLocalInput(new Date(ev.start_at * 1000));
    formEnd = ev.end_at ? toLocalInput(new Date(ev.end_at * 1000)) : "";
    formKind = ev.kind;
  }

  async function saveForm() {
    const startSec = toSec(formStart);
    if (!startSec) {
      error = "开始时间格式无效";
      return;
    }
    try {
      if (editing) {
        await invoke("update_calendar_event", {
          id: editing.id,
          title: formTitle.trim(),
          startAt: startSec,
          endAt: toSec(formEnd),
          allDay: false,
          kind: formKind,
          location: null,
          notes: null,
        });
      } else {
        await invoke("create_calendar_event", {
          workId: null,
          title: formTitle.trim(),
          startAt: startSec,
          endAt: toSec(formEnd),
          allDay: false,
          kind: formKind,
          location: null,
          notes: null,
        });
      }
      editing = null;
      showForm = false;
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  async function remove(ev: CalendarEvent) {
    try {
      await invoke("delete_calendar_event", { id: ev.id });
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

  function sorted(): CalendarEvent[] {
    return [...events].sort((a, b) => a.start_at - b.start_at);
  }

  $effect(() => {
    load();
  });
</script>

<div class="calendar">
  <div class="row">
    <h1>Calendar</h1>
    <button onclick={() => (mode = "day")} class:active={mode === "day"}>Day</button>
    <button onclick={() => (mode = "week")} class:active={mode === "week"}>Week</button>
    <span class="spacer"></span>
    <button onclick={() => (mode === "day" ? shift(-1) : shiftWeek(-1))}>‹</button>
    <span class="anchor">{anchor.toLocaleDateString()}</span>
    <button onclick={() => (mode === "day" ? shift(1) : shiftWeek(1))}>›</button>
    <button onclick={openNew}>新建事件</button>
  </div>

  {#if error}<div class="status error">{error}</div>{/if}

  {#if showForm}
    <div class="form">
      <input bind:value={formTitle} placeholder="标题" />
      <input type="datetime-local" bind:value={formStart} />
      <input type="datetime-local" bind:value={formEnd} placeholder="结束（可选）" />
      <select bind:value={formKind}>
        {#each KINDS as k (k)}
          <option value={k}>{KIND_LABEL[k]}</option>
        {/each}
      </select>
      <button onclick={saveForm}>{editing ? "保存" : "添加"}</button>
      <button onclick={() => { showForm = false; editing = null; }}>取消</button>
    </div>
  {/if}

  <ul class="ev-list">
    {#each sorted() as ev (ev.id)}
      <li>
        <span class="time">{fmtTime(ev.start_at)}{ev.end_at ? ` – ${fmtTime(ev.end_at)}` : ""}</span>
        <span class="kind kind-{ev.kind}">{KIND_LABEL[ev.kind] ?? ev.kind}</span>
        <span class="title">{ev.title}</span>
        <span class="actions">
          <button onclick={() => openEdit(ev)}>编辑</button>
          <button onclick={() => remove(ev)}>删除</button>
        </span>
      </li>
    {/each}
  </ul>
  {#if events.length === 0}
    <div class="muted empty">（该时段无事件）</div>
  {/if}
</div>

<style>
  .row {
    display: flex;
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
    border: 1px solid #c8ccd1;
    border-radius: 6px;
    background: #fff;
    font-size: 12px;
    cursor: pointer;
  }
  button.active {
    background: #dbe9f7;
    font-weight: 600;
  }
  .form {
    display: flex;
    gap: 8px;
    margin-bottom: 12px;
    flex-wrap: wrap;
  }
  .form input:not([type]) {
    flex: 1;
    min-width: 160px;
    padding: 7px 10px;
    border: 1px solid #c8ccd1;
    border-radius: 6px;
    font-size: 13px;
  }
  .form input[type="datetime-local"] {
    padding: 6px 8px;
    border: 1px solid #c8ccd1;
    border-radius: 6px;
    font-size: 12px;
  }
  .ev-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .ev-list li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-bottom: 1px solid #f0f2f4;
    font-size: 13px;
  }
  .time {
    color: #6b7280;
    font-size: 12px;
    min-width: 180px;
  }
  .kind {
    font-size: 11px;
    padding: 1px 6px;
    border-radius: 999px;
    background: #eaecef;
    min-width: 52px;
    text-align: center;
  }
  .title {
    flex: 1;
  }
  .actions button {
    margin-left: 4px;
  }
  .status.error {
    color: #b3261e;
    font-size: 13px;
    margin: 6px 0;
  }
  .empty {
    padding: 12px 0;
  }
</style>
