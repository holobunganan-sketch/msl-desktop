<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open as dialogOpen } from "@tauri-apps/plugin-dialog";
  import QuickCapture from "$lib/components/QuickCapture.svelte";
  import PlanView from "$lib/components/PlanView.svelte";
  import WaitingView from "$lib/components/WaitingView.svelte";
  import InboxView from "$lib/components/InboxView.svelte";
  import CalendarView from "$lib/components/CalendarView.svelte";
  import WorksView from "$lib/components/WorksView.svelte";

  type DirEntry = {
    name: string;
    path: string;
    is_dir: boolean;
    modified: number;
    size: number;
  };
  type Workspace = {
    id: number;
    name: string;
    root_path: string;
    enabled: boolean;
    created_at: number;
    updated_at: number;
  };

  type View = "today" | "workspace" | "works" | "plan" | "waiting" | "inbox" | "calendar";

  let view = $state<View>("today");
  let currentPath = $state("");
  let entries = $state<DirEntry[]>([]);
  let workspaces = $state<Workspace[]>([]);
  let statusMsg = $state("");
  let loading = $state(false);
  let breadcrumb = $state<string[]>([]);

  function fmtSize(bytes: number): string {
    if (bytes <= 0) return "";
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }

  function fmtTime(ts: number): string {
    if (!ts) return "";
    const d = new Date(ts * 1000);
    const pad = (n: number) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }

  async function loadWorkspaces() {
    try {
      workspaces = await invoke("get_workspaces");
    } catch (e) {
      statusMsg = String(e);
    }
  }

  async function bindWorkspace() {
    const dir = await dialogOpen({ directory: true });
    if (typeof dir !== "string" || !dir) return;
    statusMsg = "";
    try {
      await invoke("bind_workspace", { name: "主工作目录", path: dir });
      workspaces = await invoke("get_workspaces");
      await loadDir(dir);
    } catch (e) {
      statusMsg = String(e);
    }
  }

  async function loadDir(path: string) {
    loading = true;
    statusMsg = "";
    try {
      currentPath = path;
      entries = await invoke("list_dir", { path });
      breadcrumb = path.split(/[\\/]/).filter(Boolean);
    } catch (e) {
      statusMsg = String(e);
    }
    loading = false;
  }

  async function openFile(path: string) {
    try {
      await invoke("open_file", { path });
    } catch (e) {
      statusMsg = String(e);
    }
  }

  async function reveal(path: string) {
    try {
      await invoke("reveal_in_explorer", { path });
    } catch (e) {
      statusMsg = String(e);
    }
  }

  function goTo(segIndex: number) {
    const prefix = breadcrumb.slice(0, segIndex + 1).join("\\");
    const drive = /^[A-Za-z]:$/.test(prefix) ? prefix + "\\" : prefix;
    loadDir(drive);
  }

  function isDriveRoot(): boolean {
    return /^[A-Za-z]:\\?$/.test(currentPath);
  }

  async function enterEntry(e: DirEntry) {
    if (e.is_dir) {
      await loadDir(e.path);
    } else {
      await openFile(e.path);
    }
  }

  $effect(() => {
    if (view === "workspace") {
      loadWorkspaces();
      if (!currentPath) loadDir("C:\\");
    }
  });
</script>

<div class="app">
  <aside class="nav">
    <div class="brand">MSL Desktop</div>
    <button class="nav-item" class:active={view === "today"} onclick={() => (view = "today")}>
      Today
    </button>
    <button class="nav-item" class:active={view === "workspace"} onclick={() => (view = "workspace")}>
      Workspace
    </button>
    <button class="nav-item" class:active={view === "works"} onclick={() => (view = "works")}>
      Works
    </button>
    <button class="nav-item" class:active={view === "plan"} onclick={() => (view = "plan")}>
      Plan
    </button>
    <button class="nav-item" class:active={view === "waiting"} onclick={() => (view = "waiting")}>
      Waiting
    </button>
    <button class="nav-item" class:active={view === "calendar"} onclick={() => (view = "calendar")}>
      Calendar
    </button>
    <button class="nav-item" class:active={view === "inbox"} onclick={() => (view = "inbox")}>
      Inbox
    </button>
    <div class="nav-spacer"></div>
    <button class="nav-item" disabled title="后续阶段">Settings</button>
  </aside>

  <main class="content">
    <QuickCapture />
    <div class="view-body">
      {#if view === "today"}
        <section class="today">
          <h1>Today</h1>
          <p class="muted">
            恢复工作上下文的页面（Stage 6 实现）。当前可用：Workspace 文件浏览、
            Plan 任务、Waiting、Calendar、Inbox。
          </p>
          <div class="card">
            <div class="card-title">Morning Brief</div>
            <div class="muted">连接 AI 后可生成 Morning Brief</div>
          </div>
        </section>
      {:else if view === "workspace"}
        <section class="workspace">
          <div class="ws-header">
            <h1>Workspace</h1>
            <button onclick={bindWorkspace}>绑定工作目录…</button>
          </div>

          {#if workspaces.length > 0}
            <div class="ws-list">
              {#each workspaces as ws (ws.id)}
                <button class="ws-chip" onclick={() => loadDir(ws.root_path)} title={ws.root_path}>
                  {ws.name} · {ws.root_path}
                </button>
              {/each}
            </div>
          {/if}

          <div class="breadcrumb">
            {#each breadcrumb as seg, i (i)}
              <button onclick={() => goTo(i)}>{seg}</button>
              {#if i < breadcrumb.length - 1}<span>›</span>{/if}
            {/each}
            {#if isDriveRoot()}<button onclick={() => loadDir("C:\\")}>C:\</button>{/if}
          </div>

          {#if statusMsg}<div class="status error">{statusMsg}</div>{/if}

          {#if loading}
            <div class="muted">加载中…</div>
          {:else}
            <table class="file-table">
              <thead>
                <tr>
                  <th>名称</th>
                  <th>修改时间</th>
                  <th>大小</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                {#each entries as e (e.path)}
                  <tr ondblclick={() => enterEntry(e)}>
                    <td>
                      <button class="file-name" class:dir={e.is_dir} onclick={() => enterEntry(e)}>
                        {e.is_dir ? "📁" : "📄"} {e.name}
                      </button>
                    </td>
                    <td class="muted">{fmtTime(e.modified)}</td>
                    <td class="muted">{e.is_dir ? "" : fmtSize(e.size)}</td>
                    <td class="actions">
                      <button onclick={() => openFile(e.path)} title="打开">打开</button>
                      <button onclick={() => reveal(e.path)} title="在资源管理器中定位">定位</button>
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
            {#if entries.length === 0}
              <div class="muted empty">（空目录）</div>
            {/if}
          {/if}
        </section>
      {:else if view === "works"}
        <WorksView />
      {:else if view === "plan"}
        <PlanView />
      {:else if view === "waiting"}
        <WaitingView />
      {:else if view === "inbox"}
        <InboxView />
      {:else if view === "calendar"}
        <CalendarView />
      {/if}
    </div>
  </main>
</div>

<style>
  .app {
    display: flex;
    height: 100vh;
    font-family: "Segoe UI", system-ui, sans-serif;
    color: #1f2328;
  }

  .nav {
    width: 180px;
    flex-shrink: 0;
    border-right: 1px solid #e4e7eb;
    padding: 12px 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    background: #f7f8fa;
  }

  .brand {
    font-weight: 600;
    font-size: 14px;
    padding: 8px 10px 16px;
  }

  .nav-item {
    text-align: left;
    padding: 7px 10px;
    border: none;
    background: none;
    border-radius: 6px;
    font-size: 13px;
    cursor: pointer;
    color: #1f2328;
  }
  .nav-item:hover:not(:disabled) {
    background: #eaecef;
  }
  .nav-item.active {
    background: #dbe9f7;
    font-weight: 600;
  }
  .nav-item:disabled {
    color: #9aa0a6;
    cursor: default;
  }
  .nav-spacer {
    flex: 1;
  }

  .content {
    flex: 1;
    overflow: auto;
    padding: 20px 24px;
  }

  .view-body {
    margin-top: 14px;
  }

  h1 {
    font-size: 18px;
    margin: 0 0 12px;
  }
  .muted {
    color: #6b7280;
    font-size: 13px;
  }
  .card {
    border: 1px solid #e4e7eb;
    border-radius: 8px;
    padding: 16px;
    margin-top: 16px;
  }
  .card-title {
    font-weight: 600;
    margin-bottom: 6px;
  }

  .ws-header {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 10px;
  }
  .ws-header button,
  .actions button,
  .breadcrumb button {
    border: 1px solid #c8ccd1;
    background: #fff;
    border-radius: 6px;
    padding: 4px 10px;
    font-size: 12px;
    cursor: pointer;
  }
  .ws-header button:hover,
  .actions button:hover,
  .breadcrumb button:hover {
    background: #f1f3f5;
  }

  .ws-list {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 12px;
  }
  .ws-chip {
    border: 1px solid #c8ccd1;
    background: #f7f8fa;
    border-radius: 999px;
    padding: 4px 12px;
    font-size: 12px;
    cursor: pointer;
    max-width: 420px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .breadcrumb {
    display: flex;
    align-items: center;
    gap: 4px;
    margin: 8px 0;
    flex-wrap: wrap;
  }
  .breadcrumb span {
    color: #9aa0a6;
  }

  .status.error {
    color: #b3261e;
    font-size: 13px;
    margin: 6px 0;
  }

  .file-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 13px;
  }
  .file-table th {
    text-align: left;
    color: #6b7280;
    font-weight: 500;
    border-bottom: 1px solid #e4e7eb;
    padding: 6px 8px;
  }
  .file-table td {
    padding: 6px 8px;
    border-bottom: 1px solid #f0f2f4;
  }
  .file-name {
    cursor: pointer;
    border: none;
    background: none;
    text-align: left;
    padding: 0;
    font: inherit;
    color: inherit;
  }
  .file-name.dir {
    font-weight: 600;
  }
  .actions {
    white-space: nowrap;
    text-align: right;
  }
  .actions button {
    margin-left: 6px;
  }
  .empty {
    padding: 12px 0;
  }
</style>
