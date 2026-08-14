<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  type Work = {
    id: number;
    title: string;
    status: string;
    summary: string | null;
    created_at: number;
    updated_at: number;
    archived_at: number | null;
  };
  type ResumePoint = {
    id: number;
    work_id: number;
    current_state: string;
    next_step: string;
    remember: string;
    source: string;
    created_at: number;
  };
  type WorkFileRef = {
    id: number;
    work_id: number;
    workspace_id: number | null;
    path: string;
    label: string | null;
    pinned: boolean;
    created_at: number;
  };
  type ContinueWork = {
    work: Work;
    latest_resume: ResumePoint | null;
    last_activity_at: number | null;
    files: WorkFileRef[];
  };
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
  type WaitingItem = {
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
  type InboxItem = {
    id: number;
    content: string;
    created_at: number;
    processed_at: number | null;
    converted_to_type: string | null;
    converted_to_id: number | null;
  };
  type TodayData = {
    continue_works: ContinueWork[];
    today_tasks: Task[];
    today_calendar: CalendarEvent[];
    waiting_followups: WaitingItem[];
    inbox_pending: InboxItem[];
  };

  let data = $state<TodayData | null>(null);
  let error = $state("");

  const KIND_LABEL: Record<string, string> = {
    meeting: "会议",
    kol_visit: "专家拜访",
    deadline: "截止",
    travel: "出差",
    work_block: "工作块",
    other: "其他",
  };

  function pad(n: number) {
    return String(n).padStart(2, "0");
  }
  function fmtTime(ts: number | null): string {
    if (!ts) return "";
    const d = new Date(ts * 1000);
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }
  function fmtDayTime(ts: number): string {
    const d = new Date(ts * 1000);
    return `${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }

  function dayRange(): [number, number] {
    const now = new Date();
    const start = new Date(now.getFullYear(), now.getMonth(), now.getDate());
    const end = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1);
    return [Math.floor(start.getTime() / 1000), Math.floor(end.getTime() / 1000)];
  }

  async function load() {
    const [start, end] = dayRange();
    try {
      data = await invoke("get_today", { dayStart: start, dayEnd: end });
    } catch (e) {
      error = String(e);
    }
  }

  async function completeTask(t: Task) {
    try {
      await invoke("complete_task", { id: t.id });
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  async function resolveWaiting(w: WaitingItem) {
    try {
      await invoke("resolve_waiting", { id: w.id });
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  $effect(() => {
    load();
  });
</script>

<div class="today">
  <h1>Today</h1>
  {#if error}<div class="status error">{error}</div>{/if}

  <div class="cols">
    <div class="col-main">
      <!-- A. Continue -->
      <section class="card">
        <div class="card-title">CONTINUE</div>
        {#if !data}
          <div class="muted">加载中…</div>
        {:else if data.continue_works.length === 0}
          <div class="muted">暂无进行中的 Work</div>
        {/if}
        {#each data?.continue_works ?? [] as c (c.work.id)}
          <div class="cont-item">
            <div class="cont-title">{c.work.title} <span class="muted">{c.work.status}</span></div>
            {#if c.latest_resume}
              <div class="muted">上次做到：{c.latest_resume.current_state || "—"}</div>
              <div class="muted">下一步：{c.latest_resume.next_step || "—"}</div>
            {/if}
            {#if c.files.length > 0}
              <div class="muted files">
                最近文件：
                {#each c.files.slice(0, 3) as f (f.id)}
                  <span>{f.label || f.path.split(/[\\/]/).pop()}</span>
                {/each}
              </div>
            {/if}
            {#if c.last_activity_at}
              <div class="muted">上次活动：{fmtTime(c.last_activity_at)}</div>
            {/if}
          </div>
        {/each}
      </section>

      <!-- B. Today -->
      <section class="card">
        <div class="card-title">TODAY</div>
        {#if (data?.today_tasks.length ?? 0) === 0 && (data?.today_calendar.length ?? 0) === 0}
          <div class="muted">今天暂无任务与日程</div>
        {/if}
        {#each data?.today_tasks ?? [] as t (t.id)}
          <div class="row-item">
            <span class:strike={t.status === "done"}>{t.title}</span>
            <span class="muted">{t.due_at ? `截止 ${fmtTime(t.due_at)}` : ""}</span>
            {#if t.status !== "done"}
              <button onclick={() => completeTask(t)}>完成</button>
            {/if}
          </div>
        {/each}
        {#each data?.today_calendar ?? [] as ev (ev.id)}
          <div class="row-item">
            <span class="muted">{fmtDayTime(ev.start_at)}</span>
            <span>{ev.title}</span>
            <span class="muted">{KIND_LABEL[ev.kind] ?? ev.kind}</span>
          </div>
        {/each}
      </section>

      <!-- E. Morning Brief 占位 -->
      <section class="card brief">
        <div class="card-title">MORNING BRIEF</div>
        <div class="muted">连接 AI 后可生成 Morning Brief（Stage 8 实现）</div>
      </section>
    </div>

    <div class="col-side">
      <!-- C. Waiting -->
      <section class="card">
        <div class="card-title">WAITING</div>
        {#if (data?.waiting_followups.length ?? 0) === 0}
          <div class="muted">暂无需要跟进的事项</div>
        {/if}
        {#each data?.waiting_followups ?? [] as w (w.id)}
          <div class="row-item">
            <div class="w-block">
              <div>{w.title}</div>
              <div class="muted">等待：{w.waiting_for || "—"}{w.follow_up_at ? ` · 跟进 ${fmtTime(w.follow_up_at)}` : ""}</div>
            </div>
            <button onclick={() => resolveWaiting(w)}>解决</button>
          </div>
        {/each}
      </section>

      <!-- D. Inbox -->
      <section class="card">
        <div class="card-title">INBOX</div>
        {#if (data?.inbox_pending.length ?? 0) === 0}
          <div class="muted">Inbox 已清空</div>
        {/if}
        {#each data?.inbox_pending ?? [] as it (it.id)}
          <div class="row-item inbox-item">{it.content}</div>
        {/each}
      </section>
    </div>
  </div>
</div>

<style>
  h1 {
    font-size: 18px;
    margin: 0 0 12px;
  }
  .cols {
    display: flex;
    gap: 16px;
    align-items: flex-start;
  }
  .col-main {
    flex: 1;
    min-width: 0;
  }
  .col-side {
    width: 300px;
    flex-shrink: 0;
  }
  .card {
    border: 1px solid #e4e7eb;
    border-radius: 8px;
    padding: 12px 14px;
    margin-bottom: 12px;
  }
  .card-title {
    font-weight: 600;
    font-size: 12px;
    letter-spacing: 0.05em;
    color: #4a5568;
    margin-bottom: 8px;
  }
  .muted {
    color: #6b7280;
    font-size: 12px;
  }
  .cont-item {
    padding: 8px 0;
    border-bottom: 1px solid #f0f2f4;
  }
  .cont-item:last-child {
    border-bottom: none;
  }
  .cont-title {
    font-weight: 600;
    font-size: 13px;
    margin-bottom: 2px;
  }
  .files span {
    margin-right: 8px;
  }
  .row-item {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    padding: 4px 0;
  }
  .w-block {
    flex: 1;
  }
  .inbox-item {
    color: #4a5568;
  }
  .strike {
    text-decoration: line-through;
    color: #9aa0a6;
  }
  .brief {
    background: #f7f8fa;
  }
  button {
    padding: 4px 10px;
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
</style>
