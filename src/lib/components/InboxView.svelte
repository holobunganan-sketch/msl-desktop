<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  type InboxItem = {
    id: number;
    content: string;
    created_at: number;
    processed_at: number | null;
    converted_to_type: string | null;
    converted_to_id: number | null;
  };

  let items = $state<InboxItem[]>([]);
  let error = $state("");

  function fmtTime(ts: number): string {
    const d = new Date(ts * 1000);
    const pad = (n: number) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }

  async function load() {
    try {
      items = await invoke("list_inbox");
    } catch (e) {
      error = String(e);
    }
  }

  async function toTask(it: InboxItem) {
    const due = prompt(`转为任务：\n"${it.content}"\n截止时间（留空=无，格式 2026-08-15 09:00）`);
    if (due === null) return; // 取消
    try {
      await invoke("convert_inbox_to_task", {
        inboxId: it.id,
        workId: null,
        priority: "normal",
        dueAt: parseDate(due) ?? null,
        notes: null,
      });
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  async function toWaiting(it: InboxItem) {
    const who = prompt(`转为等待事项：\n"${it.content}"\n等待谁？`);
    if (who === null) return;
    const follow = prompt(`跟进时间（留空=无，格式 2026-08-15 09:00）`);
    if (follow === null) return;
    try {
      await invoke("convert_inbox_to_waiting", {
        inboxId: it.id,
        workId: null,
        waitingFor: who.trim(),
        followUpAt: parseDate(follow) ?? null,
      });
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  async function toCalendar(it: InboxItem) {
    const start = prompt(`转为日历事件：\n"${it.content}"\n开始时间（格式 2026-08-15 09:00）`);
    if (start === null) return;
    const end = prompt(`结束时间（留空=无）`);
    if (end === null) return;
    const startSec = parseDate(start);
    if (!startSec) {
      error = "时间格式无法解析";
      return;
    }
    try {
      await invoke("convert_inbox_to_calendar", {
        inboxId: it.id,
        workId: null,
        startAt: startSec,
        endAt: parseDate(end),
        allDay: false,
        kind: "other",
      });
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  async function remove(it: InboxItem) {
    try {
      await invoke("delete_inbox_item", { id: it.id });
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  function parseDate(s: string): number | null {
    if (!s || !s.trim()) return null;
    const t = Date.parse(s.replace(/-/g, "/").replace(" ", "T"));
    return isNaN(t) ? null : Math.floor(t / 1000);
  }

  $effect(() => { load(); });
</script>

<div class="inbox">
  <h1>Inbox</h1>
  <div class="muted hint">Quick Capture 内容都在这里。可转为 Task / Waiting / Calendar。</div>
  {#if error}<div class="status error">{error}</div>{/if}

  <ul class="in-list">
    {#each items as it (it.id)}
      <li class:processed={it.processed_at !== null}>
        <span class="content">{it.content}</span>
        <span class="muted">{fmtTime(it.created_at)}</span>
        {#if it.processed_at}
          <span class="muted tag">已转 {it.converted_to_type}</span>
        {:else}
          <span class="actions">
            <button onclick={() => toTask(it)}>转任务</button>
            <button onclick={() => toWaiting(it)}>转等待</button>
            <button onclick={() => toCalendar(it)}>转日历</button>
            <button onclick={() => remove(it)}>删除</button>
          </span>
        {/if}
      </li>
    {/each}
  </ul>
  {#if items.length === 0}
    <div class="muted empty">（Inbox 为空）</div>
  {/if}
</div>

<style>
  h1 {
    font-size: 18px;
    margin: 0 0 4px;
  }
  .hint {
    margin-bottom: 12px;
  }
  .in-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .in-list li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-bottom: 1px solid #f0f2f4;
    font-size: 13px;
  }
  .in-list li.processed .content {
    color: #9aa0a6;
  }
  .content {
    flex: 1;
  }
  .muted {
    color: #6b7280;
    font-size: 12px;
  }
  .tag {
    background: #eaecef;
    padding: 2px 8px;
    border-radius: 999px;
  }
  .actions button {
    margin-left: 4px;
  }
  button {
    padding: 5px 10px;
    border: 1px solid #c8ccd1;
    border-radius: 6px;
    background: #fff;
    font-size: 12px;
    cursor: pointer;
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
