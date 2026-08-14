<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

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
  let newFollowUp = $state("");

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
      items = await invoke("list_waiting", { status: null });
    } catch (e) {
      error = String(e);
    }
  }

  async function create() {
    if (!newTitle.trim()) return;
    try {
      await invoke("create_waiting", {
        workId: null,
        title: newTitle.trim(),
        waitingFor: newFor.trim(),
        followUpAt: toSec(newFollowUp),
        notes: null,
      });
      newTitle = "";
      newFor = "";
      newFollowUp = "";
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  async function resolve(w: Waiting) {
    try {
      await invoke("resolve_waiting", { id: w.id });
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  async function remove(w: Waiting) {
    try {
      await invoke("delete_waiting", { id: w.id });
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
    return [...items].sort((a, b) => Number(b.status === "resolved") - Number(a.status === "resolved"));
  }

  $effect(() => { load(); });
</script>

<div class="waiting">
  <h1>Waiting</h1>
  {#if error}<div class="status error">{error}</div>{/if}

  <div class="create-row">
    <input bind:value={newTitle} placeholder="在等待什么…" onkeydown={(e) => e.key === "Enter" && create()} />
    <input bind:value={newFor} placeholder="等待谁（如：王老师）" />
    <input type="datetime-local" bind:value={newFollowUp} />
    <button onclick={create}>添加</button>
  </div>

  <ul class="w-list">
    {#each sorted() as w (w.id)}
      <li class:resolved={w.status === "resolved"} class:overdue={needsFollowUp(w)}>
        <span class="title">{w.title}</span>
        <span class="muted">{w.waiting_for ? `等待：${w.waiting_for}` : ""}</span>
        <span class="muted">已等 {daysWaiting(w)} 天</span>
        {#if w.follow_up_at}<span class="muted">跟进 {fmtTime(w.follow_up_at)}</span>{/if}
        {#if needsFollowUp(w)}<span class="badge">今天需跟进</span>{/if}
        <span class="actions">
          {#if w.status !== "resolved"}
            <button onclick={() => resolve(w)}>解决</button>
          {/if}
          <button onclick={() => remove(w)}>删除</button>
        </span>
      </li>
    {/each}
  </ul>
  {#if items.length === 0}
    <div class="muted empty">（暂无等待事项）</div>
  {/if}
</div>

<style>
  h1 {
    font-size: 18px;
    margin: 0 0 12px;
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
  button {
    padding: 6px 10px;
    border: 1px solid #c8ccd1;
    border-radius: 6px;
    background: #fff;
    font-size: 12px;
    cursor: pointer;
  }
  .w-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .w-list li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-bottom: 1px solid #f0f2f4;
    font-size: 13px;
  }
  .w-list li.resolved .title {
    text-decoration: line-through;
    color: #9aa0a6;
  }
  .w-list li.overdue {
    background: #fdf3f2;
  }
  .title {
    flex: 1;
  }
  .muted {
    color: #6b7280;
    font-size: 12px;
  }
  .badge {
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
