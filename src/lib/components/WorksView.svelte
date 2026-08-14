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
  type ActivityEvent = {
    id: number;
    timestamp: number;
    event_type: string;
    workspace_id: number | null;
    work_id: number | null;
    entity_type: string | null;
    entity_id: number | null;
    path: string | null;
    display_text: string;
    metadata_json: string | null;
    dedupe_key: string | null;
  };
  type WorkDetail = {
    work: Work;
    latest_resume: ResumePoint | null;
    resume_history: ResumePoint[];
    files: WorkFileRef[];
    tasks: Task[];
    waiting: WaitingItem[];
    calendar: CalendarEvent[];
    recent_activity: ActivityEvent[];
  };

  let works = $state<Work[]>([]);
  let selectedId = $state<number | null>(null);
  let detail = $state<WorkDetail | null>(null);
  let error = $state("");
  let newTitle = $state("");

  // 新建 Resume Point 表单
  let rpState = $state("");
  let rpNext = $state("");
  let rpRemember = $state("");

  // 关联文件表单
  let filePath = $state("");
  let fileLabel = $state("");

  function fmtTime(ts: number | null): string {
    if (!ts) return "";
    const d = new Date(ts * 1000);
    const pad = (n: number) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }

  async function loadWorks() {
    try {
      works = await invoke("list_works", { status: null });
    } catch (e) {
      error = String(e);
    }
  }

  async function createWork() {
    if (!newTitle.trim()) return;
    try {
      const w: Work = await invoke("create_work", { title: newTitle.trim(), status: "active" });
      newTitle = "";
      await loadWorks();
      await openDetail(w.id);
    } catch (e) {
      error = String(e);
    }
  }

  async function openDetail(id: number) {
    selectedId = id;
    rpState = "";
    rpNext = "";
    rpRemember = "";
    filePath = "";
    try {
      detail = await invoke("get_work_detail", { id });
    } catch (e) {
      error = String(e);
    }
  }

  async function changeStatus(w: Work, status: string) {
    try {
      await invoke("update_work", { id: w.id, title: w.title, status, summary: w.summary });
      await loadWorks();
      if (selectedId) await openDetail(selectedId);
    } catch (e) {
      error = String(e);
    }
  }

  async function archive(w: Work) {
    if (!confirm(`归档 Work「${w.title}」？`)) return;
    try {
      await invoke("archive_work", { id: w.id });
      await loadWorks();
      if (selectedId === w.id) {
        selectedId = null;
        detail = null;
      }
    } catch (e) {
      error = String(e);
    }
  }

  async function saveResumePoint() {
    if (!selectedId) return;
    try {
      await invoke("create_resume_point", {
        workId: selectedId,
        currentState: rpState.trim(),
        nextStep: rpNext.trim(),
        remember: rpRemember.trim(),
      });
      await openDetail(selectedId);
    } catch (e) {
      error = String(e);
    }
  }

  async function addFile() {
    if (!selectedId || !filePath.trim()) return;
    try {
      await invoke("add_work_file_ref", {
        workId: selectedId,
        workspaceId: null,
        path: filePath.trim(),
        label: fileLabel.trim() || null,
      });
      await openDetail(selectedId);
    } catch (e) {
      error = String(e);
    }
  }

  async function togglePin(f: WorkFileRef) {
    try {
      await invoke("update_work_file_ref", { id: f.id, label: f.label, pinned: !f.pinned });
      if (selectedId) await openDetail(selectedId);
    } catch (e) {
      error = String(e);
    }
  }

  async function removeFile(f: WorkFileRef) {
    try {
      await invoke("remove_work_file_ref", { id: f.id });
      if (selectedId) await openDetail(selectedId);
    } catch (e) {
      error = String(e);
    }
  }

  async function completeTask(t: Task) {
    try {
      await invoke("complete_task", { id: t.id });
      if (selectedId) await openDetail(selectedId);
    } catch (e) {
      error = String(e);
    }
  }

  async function resolveWaiting(w: WaitingItem) {
    try {
      await invoke("resolve_waiting", { id: w.id });
      if (selectedId) await openDetail(selectedId);
    } catch (e) {
      error = String(e);
    }
  }

  async function openFile(path: string) {
    try {
      await invoke("open_file", { path });
    } catch (e) {
      error = String(e);
    }
  }

  $effect(() => {
    loadWorks();
  });
</script>

<div class="works">
  <h1>Works</h1>
  {#if error}<div class="status error">{error}</div>{/if}

  <div class="layout">
    <!-- 左侧：Work 列表 -->
    <div class="list-pane">
      <div class="create-row">
        <input bind:value={newTitle} placeholder="新 Work 标题…" onkeydown={(e) => e.key === "Enter" && createWork()} />
        <button onclick={createWork}>新建</button>
      </div>
      <ul class="work-list">
        {#each works as w (w.id)}
          <li class:active={selectedId === w.id}>
            <button class="work-item" onclick={() => openDetail(w.id)}>
              <span class="wt">{w.title}</span>
              <span class="muted">{w.status}{w.updated_at ? ` · ${fmtTime(w.updated_at)}` : ""}</span>
            </button>
          </li>
        {/each}
      </ul>
      {#if works.length === 0}
        <div class="muted empty">（暂无 Work）</div>
      {/if}
    </div>

    <!-- 右侧：Work 详情 -->
    <div class="detail-pane">
      {#if detail}
        {@const w = detail.work}
        <div class="detail-head">
          <h2>{w.title}</h2>
          <div class="status-actions">
            {#each ["active", "paused", "waiting", "done"] as s (s)}
              <button class:on={w.status === s} onclick={() => changeStatus(w, s)}>{s}</button>
            {/each}
            <button onclick={() => archive(w)}>归档</button>
          </div>
          {#if w.summary}<div class="muted summary">{w.summary}</div>{/if}
        </div>

        <!-- Current State / Next Step / Remember（置顶） -->
        <section class="resume card">
          <div class="card-title">RESUME POINT</div>
          {#if detail.latest_resume}
            <div class="rp-current"><b>上次做到：</b>{detail.latest_resume.current_state || "—"}</div>
            <div class="rp-next"><b>下一步：</b>{detail.latest_resume.next_step || "—"}</div>
            <div class="rp-remember"><b>需要记住：</b>{detail.latest_resume.remember || "—"}</div>
          {:else}
            <div class="muted">还没有 Resume Point，记录一下当前进度吧。</div>
          {/if}

          <div class="rp-form">
            <input bind:value={rpState} placeholder="我现在做到…" />
            <input bind:value={rpNext} placeholder="下一步…" />
            <input bind:value={rpRemember} placeholder="需要记住…" />
            <button onclick={saveResumePoint}>保存 Resume Point</button>
          </div>
          {#if detail.resume_history.length > 1}
            <details class="history">
              <summary>历史 Resume Points（{detail.resume_history.length}）</summary>
              {#each detail.resume_history as rp (rp.id)}
                <div class="hist-item">
                  <div class="muted">{fmtTime(rp.created_at)}</div>
                  <div>{rp.current_state} → {rp.next_step}</div>
                </div>
              {/each}
            </details>
          {/if}
        </section>

        <!-- WAITING -->
        <section class="card">
          <div class="card-title">WAITING</div>
          {#if detail.waiting.filter((x) => x.status === "open").length === 0}
            <div class="muted">无等待事项</div>
          {/if}
          {#each detail.waiting as wq (wq.id)}
            {#if wq.status === "open"}
              <div class="row-item">
                <span>{wq.title}</span>
                <span class="muted">等待：{wq.waiting_for}{wq.follow_up_at ? ` · 跟进 ${fmtTime(wq.follow_up_at)}` : ""}</span>
                <button onclick={() => resolveWaiting(wq)}>解决</button>
              </div>
            {/if}
          {/each}
        </section>

        <!-- FILES -->
        <section class="card">
          <div class="card-title">FILES</div>
          <div class="file-form">
            <input bind:value={filePath} placeholder="文件路径（如 C:\Work\方案V3.docx）" />
            <input bind:value={fileLabel} placeholder="标签（可选）" />
            <button onclick={addFile}>关联文件</button>
          </div>
          {#if detail.files.length === 0}
            <div class="muted">未关联文件</div>
          {/if}
          {#each detail.files as f (f.id)}
            <div class="row-item" class:file-pinned={f.pinned}>
              <button class="fname" onclick={() => openFile(f.path)} title={f.path}>
                {f.pinned ? "📌" : "📄"} {f.label || f.path.split(/[\\/]/).pop()}
              </button>
              <button onclick={() => togglePin(f)}>{f.pinned ? "取消置顶" : "置顶"}</button>
              <button onclick={() => removeFile(f)}>移除</button>
            </div>
          {/each}
        </section>

        <!-- CALENDAR -->
        <section class="card">
          <div class="card-title">CALENDAR</div>
          {#if detail.calendar.length === 0}
            <div class="muted">无关键日期</div>
          {/if}
          {#each detail.calendar as ev (ev.id)}
            <div class="row-item">
              <span>{fmtTime(ev.start_at)}</span>
              <span>{ev.title}</span>
              <span class="muted">{ev.kind}</span>
            </div>
          {/each}
        </section>

        <!-- TASKS -->
        <section class="card">
          <div class="card-title">TASKS</div>
          {#if detail.tasks.length === 0}
            <div class="muted">无任务</div>
          {/if}
          {#each detail.tasks as t (t.id)}
            <div class="row-item" class:task-done={t.status === "done"}>
              <span class:strike={t.status === "done"}>{t.title}</span>
              <span class="muted">{t.status}{t.due_at ? ` · ${fmtTime(t.due_at)}` : ""}</span>
              {#if t.status !== "done"}
                <button onclick={() => completeTask(t)}>完成</button>
              {/if}
            </div>
          {/each}
        </section>

        <!-- RECENT ACTIVITY -->
        <section class="card">
          <div class="card-title">RECENT ACTIVITY</div>
          {#if detail.recent_activity.length === 0}
            <div class="muted">暂无活动</div>
          {/if}
          {#each detail.recent_activity as a (a.id)}
            <div class="row-item">
              <span class="muted">{fmtTime(a.timestamp)}</span>
              <span>{a.display_text}</span>
            </div>
          {/each}
        </section>
      {:else}
        <div class="muted empty">从左侧选择一个 Work，或新建一个。</div>
      {/if}
    </div>
  </div>
</div>

<style>
  h1 {
    font-size: 18px;
    margin: 0 0 12px;
  }
  .layout {
    display: flex;
    gap: 16px;
    align-items: flex-start;
  }
  .list-pane {
    width: 260px;
    flex-shrink: 0;
  }
  .detail-pane {
    flex: 1;
    min-width: 0;
  }
  .create-row {
    display: flex;
    gap: 6px;
    margin-bottom: 10px;
  }
  .create-row input {
    flex: 1;
    padding: 7px 10px;
    border: 1px solid #c8ccd1;
    border-radius: 6px;
    font-size: 13px;
  }
  .work-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .work-list li {
    margin-bottom: 4px;
  }
  .work-list li.active .work-item {
    background: #dbe9f7;
  }
  .work-item {
    width: 100%;
    text-align: left;
    border: 1px solid #e4e7eb;
    background: #fff;
    border-radius: 8px;
    padding: 8px 10px;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .work-item:hover {
    background: #f1f3f5;
  }
  .wt {
    font-weight: 600;
    font-size: 13px;
  }
  .muted {
    color: #6b7280;
    font-size: 12px;
  }
  .empty {
    padding: 12px 0;
  }
  .detail-head h2 {
    margin: 0 0 8px;
    font-size: 18px;
  }
  .status-actions {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .status-actions button.on {
    background: #dbe9f7;
    font-weight: 600;
  }
  .summary {
    margin-top: 6px;
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
  .resume {
    background: #f7fbff;
    border-color: #bcd7ee;
  }
  .rp-current,
  .rp-next,
  .rp-remember {
    font-size: 13px;
    margin-bottom: 4px;
  }
  .rp-form {
    display: flex;
    gap: 6px;
    margin-top: 10px;
    flex-wrap: wrap;
  }
  .rp-form input {
    flex: 1;
    min-width: 120px;
    padding: 6px 8px;
    border: 1px solid #c8ccd1;
    border-radius: 6px;
    font-size: 12px;
  }
  .history {
    margin-top: 10px;
    font-size: 12px;
  }
  .hist-item {
    margin-top: 4px;
  }
  .row-item {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
    padding: 4px 0;
  }
  .row-item .fname {
    cursor: pointer;
    flex: 1;
    color: #2f6db3;
    border: none;
    background: none;
    text-align: left;
    padding: 0;
    font: inherit;
  }
  .row-item.file-pinned .fname {
    font-weight: 600;
  }
  .strike {
    text-decoration: line-through;
    color: #9aa0a6;
  }
  button {
    padding: 5px 10px;
    border: 1px solid #c8ccd1;
    border-radius: 6px;
    background: #fff;
    font-size: 12px;
    cursor: pointer;
  }
  .file-form {
    display: flex;
    gap: 6px;
    margin-bottom: 8px;
  }
  .file-form input {
    flex: 1;
    min-width: 120px;
    padding: 6px 8px;
    border: 1px solid #c8ccd1;
    border-radius: 6px;
    font-size: 12px;
  }
  .status.error {
    color: #b3261e;
    font-size: 13px;
    margin: 6px 0;
  }
</style>
