<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

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
  let error = $state("");
  let newTitle = $state("");
  let newPriority = $state("normal");
  let newDue = $state("");

  const STATUS_ORDER = ["next", "scheduled", "waiting", "paused", "done"];

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

  async function createTask() {
    if (!newTitle.trim()) return;
    try {
      await invoke("create_task", {
        workId: null,
        title: newTitle.trim(),
        priority: newPriority,
        dueAt: toSec(newDue),
        notes: null,
      });
      newTitle = "";
      newDue = "";
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  async function complete(t: Task) {
    try {
      await invoke("complete_task", { id: t.id });
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  async function remove(t: Task) {
    try {
      await invoke("delete_task", { id: t.id });
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  function isOverdue(t: Task): boolean {
    return t.status !== "done" && !!t.due_at && t.due_at < nowSec();
  }

  function visibleTasks(): Task[] {
    if (filter === "all") {
      // 未完成按截止时间排序
      return [...tasks].sort((a, b) => (a.due_at ?? 9e15) - (b.due_at ?? 9e15));
    }
    return tasks;
  }

  $effect(() => { load(); });
</script>

<div class="plan">
  <div class="row">
    <h1>Plan</h1>
    <select bind:value={filter} onchange={load}>
      <option value="all">全部</option>
      {#each STATUS_ORDER as s (s)}
        <option value={s}>{s}</option>
      {/each}
    </select>
  </div>

  {#if error}<div class="status error">{error}</div>{/if}

  <div class="create-row">
    <input bind:value={newTitle} placeholder="新任务…" onkeydown={(e) => e.key === "Enter" && createTask()} />
    <select bind:value={newPriority}>
      <option value="low">低</option>
      <option value="normal">普通</option>
      <option value="high">高</option>
    </select>
    <input type="datetime-local" bind:value={newDue} />
    <button onclick={createTask}>添加</button>
  </div>

  <ul class="task-list">
    {#each visibleTasks() as t (t.id)}
      <li class:done={t.status === "done"} class:overdue={isOverdue(t)}>
        <span class="prio prio-{t.priority}">{t.priority}</span>
        <span class="title">{t.title}</span>
        <span class="muted">{t.status}{t.due_at ? ` · 截止 ${fmtTime(t.due_at)}` : ""}</span>
        {#if isOverdue(t)}<span class="badge overdue">逾期</span>{/if}
        <span class="actions">
          {#if t.status !== "done"}
            <button onclick={() => complete(t)}>完成</button>
          {/if}
          <button onclick={() => remove(t)}>删除</button>
        </span>
      </li>
    {/each}
  </ul>
  {#if tasks.length === 0}
    <div class="muted empty">（暂无任务）</div>
  {/if}
</div>

<style>
  .plan h1 {
    font-size: 18px;
    margin: 0;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 12px;
  }
  .create-row {
    display: flex;
    gap: 8px;
    margin-bottom: 12px;
  }
  .create-row input:not([type]) {
    flex: 1;
    padding: 7px 10px;
    border: 1px solid #c8ccd1;
    border-radius: 6px;
    font-size: 13px;
  }
  .create-row input[type="datetime-local"] {
    padding: 6px 8px;
    border: 1px solid #c8ccd1;
    border-radius: 6px;
    font-size: 12px;
  }
  select,
  button {
    padding: 6px 10px;
    border: 1px solid #c8ccd1;
    border-radius: 6px;
    background: #fff;
    font-size: 12px;
    cursor: pointer;
  }
  .task-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .task-list li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-bottom: 1px solid #f0f2f4;
    font-size: 13px;
  }
  .task-list li.done .title {
    text-decoration: line-through;
    color: #9aa0a6;
  }
  .task-list li.overdue {
    background: #fdf3f2;
  }
  .prio {
    font-size: 11px;
    padding: 1px 6px;
    border-radius: 999px;
    color: #fff;
    width: 34px;
    text-align: center;
  }
  .prio-high {
    background: #b3261e;
  }
  .prio-normal {
    background: #6b7280;
  }
  .prio-low {
    background: #9aa0a6;
  }
  .title {
    flex: 1;
  }
  .muted {
    color: #6b7280;
    font-size: 12px;
  }
  .badge.overdue {
    background: #b3261e;
    color: #fff;
    font-size: 11px;
    padding: 1px 6px;
    border-radius: 4px;
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
