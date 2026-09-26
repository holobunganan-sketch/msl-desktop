<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { locale, t, translateStatus } from "$lib/i18n";
  import {resolveDestination,type Destination} from '$lib/services/navigation';
  import Modal from './ui/Modal.svelte';

  type Work = { id: number; title: string; status: string };
  type FileHit = { path: string; label: string | null; work_id: number | null };
  type Task = { id: number; title: string; status: string; due_at: number | null };
  type WaitingItem = { id: number; title: string; waiting_for: string; status: string };
  type CalendarEvent = { id: number; title: string; start_at: number; kind: string };
  type InboxItem = { id: number; content: string };
  type ResumePoint = { id: number; work_id: number; current_state: string; next_step: string };
  type ActivityEvent = { id: number; timestamp: number; display_text: string; event_type: string;entity_type?:string|null;entity_id?:number|null;work_id?:number|null;path?:string|null };
  type SearchResults = {
    works: Work[];
    files: FileHit[];
    tasks: Task[];
    waiting: WaitingItem[];
    calendar: CalendarEvent[];
    inbox: InboxItem[];
    resume_points: ResumePoint[];
    activity: ActivityEvent[];
  };

  let { open = $bindable(false), onSelect = (_target:Destination) => {} } = $props();
  let activity=$state<ActivityEvent|null>(null);

  let query = $state("");
  let results = $state<SearchResults | null>(null);
  let inputEl = $state<HTMLInputElement | undefined>(undefined);
  let currentLocale = $derived($locale);
  const tt = (key: Parameters<typeof t>[0], params: Record<string, string | number> = {}) => t(key, params, currentLocale);

  let debounceTimer: ReturnType<typeof setTimeout> | undefined;

  function doSearch(q: string) {
    const trimmed = q.trim();
    if (!trimmed) {
      results = null;
      return;
    }
    invoke("search", { query: trimmed })
      .then((r) => {
        results = r as SearchResults;
      })
      .catch(() => {});
  }

  function onInput() {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => doSearch(query), 150);
  }

  function pick(kind: string, id: number,workId?:number) {
    const destination=resolveDestination(kind,id,workId);if(!destination)return;onSelect(destination);
    close();
  }
  function pickFile(path:string){onSelect({view:'workspace',filePath:path});close();}
  function pickActivity(value:ActivityEvent){const destination=value.entity_type&&value.entity_id?resolveDestination(value.entity_type,value.entity_id,value.work_id):null;if(destination){onSelect(destination);close();}else if(value.path)pickFile(value.path);else{activity=value;close();}}

  function close() {
    open = false;
    query = "";
    results = null;
  }

  function fmtTime(ts: number): string {
    const d = new Date(ts * 1000);
    const pad = (n: number) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }

  function total(): number {
    if (!results) return 0;
    return (
      results.works.length +
      results.files.length +
      results.tasks.length +
      results.waiting.length +
      results.calendar.length +
      results.inbox.length +
      results.resume_points.length +
      results.activity.length
    );
  }

  function fmtStatus(s: string): string {
    return s || "—";
  }

  $effect(() => {
    if (open) {
      setTimeout(() => inputEl?.focus(), 10);
    }
    // 组件卸载/重渲染时清理防抖定时器
    return () => clearTimeout(debounceTimer);
  });
</script>

<svelte:window
  onkeydown={(e) => {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
      e.preventDefault();
      open = true;
    }
    if (open && e.key === "Escape") close();
  }}
/>

{#if open}
  <div
    class="overlay"
    role="presentation"
    onclick={(e) => e.target === e.currentTarget && close()}
  >
    <div class="panel">
      <input
        bind:this={inputEl}
        bind:value={query}
        oninput={onInput}
        placeholder={tt("search.placeholder")}
      />
      {#if results}
        <div class="results">
          {#if total() === 0}
            <div class="muted empty">{tt("common.noResults", { query })}</div>
          {/if}

          {#if results.works.length > 0}
            <div class="group">
              <div class="group-title">{tt("search.works")}</div>
              {#each results.works as w (w.id)}
                <button class="item" onclick={() => pick("work", w.id)}>
                  <span class="name">{w.title}</span>
                  <span class="muted">{fmtStatus(w.status)}</span>
                </button>
              {/each}
            </div>
          {/if}

          {#if results.files.length > 0}
            <div class="group">
              <div class="group-title">{tt("search.files")}</div>
              {#each results.files as f (f.path)}
                <button class="item" onclick={() => pickFile(f.path)}>
                  <span class="name">{f.label || f.path.split(/[\\/]/).pop()}</span>
                  <span class="muted path">{f.path}</span>
                </button>
              {/each}
            </div>
          {/if}

          {#if results.tasks.length > 0}
            <div class="group">
              <div class="group-title">{tt("search.tasks")}</div>
              {#each results.tasks as t (t.id)}
                <button class="item" onclick={() => pick("task", t.id)}>
                  <span class="name">{t.title}</span>
                  <span class="muted">{t.status}{t.due_at ? ` · ${fmtTime(t.due_at)}` : ""}</span>
                </button>
              {/each}
            </div>
          {/if}

          {#if results.waiting.length > 0}
            <div class="group">
              <div class="group-title">{tt("search.waiting")}</div>
              {#each results.waiting as w (w.id)}
                <button class="item" onclick={() => pick("waiting", w.id)}>
                  <span class="name">{w.title}</span>
                  <span class="muted">{tt("search.waitingDetail", { person: w.waiting_for || "—", status: fmtStatus(w.status) })}</span>
                </button>
              {/each}
            </div>
          {/if}

          {#if results.calendar.length > 0}
            <div class="group">
              <div class="group-title">{tt("search.calendar")}</div>
              {#each results.calendar as ev (ev.id)}
                <button class="item" onclick={() => pick("calendar", ev.id)}>
                  <span class="name">{ev.title}</span>
                  <span class="muted">{fmtTime(ev.start_at)} · {ev.kind}</span>
                </button>
              {/each}
            </div>
          {/if}

          {#if results.inbox.length > 0}
            <div class="group">
              <div class="group-title">{tt("search.inbox")}</div>
              {#each results.inbox as it (it.id)}
                <button class="item" onclick={() => pick("inbox", it.id)}>
                  <span class="name">{it.content}</span>
                </button>
              {/each}
            </div>
          {/if}

          {#if results.resume_points.length > 0}
            <div class="group">
              <div class="group-title">{tt("search.resume")}</div>
              {#each results.resume_points as rp (rp.id)}
                <button class="item" onclick={() => pick("resume", rp.id, rp.work_id)}>
                  <span class="name">{rp.current_state || rp.next_step}</span>
                </button>
              {/each}
            </div>
          {/if}

          {#if results.activity.length > 0}
            <div class="group">
              <div class="group-title">{tt("search.activity")}</div>
              {#each results.activity as a (a.id)}
                <button class="item" onclick={() => pickActivity(a)}>
                  <span class="name">{a.display_text}</span>
                  <span class="muted">{fmtTime(a.timestamp)}</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>
      {:else if query.trim()}
        <div class="muted empty">{tt("common.searching")}</div>
      {/if}
    </div>
  </div>
{/if}

<Modal open={activity!==null} title={currentLocale==='en-US'?'Recorded activity':'工作记录'} onclose={()=>activity=null}><p>{activity?.display_text}</p>{#if activity}<p>{fmtTime(activity.timestamp)}</p>{/if}<p>{currentLocale==='en-US'?'This is a historical event without an editable destination.':'这是一条历史事件，没有对应的可编辑事项。'}</p></Modal>
<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.35);
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding: calc(var(--viewport-height) * .1) 16px 16px;
    z-index: 1000;
  }
  .panel {
    width: 560px;
    max-width: 100%;
    max-height: calc(var(--viewport-height) * .8);
    background: #fff;
    border-radius: 10px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.25);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
  .panel input {
    border: none;
    border-bottom: 1px solid #e4e7eb;
    padding: 14px 16px;
    font-size: 14px;
    outline: none;
  }
  .results {
    overflow-y: auto;
    padding: 8px 0;
  }
  .group {
    padding: 4px 0;
  }
  .group-title {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.05em;
    color: #4a5568;
    padding: 6px 16px 2px;
  }
  .item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    width: 100%;
    text-align: left;
    border: none;
    background: none;
    padding: 7px 16px;
    font-size: 13px;
    cursor: pointer;
  }
  .item:hover {
    background: #f1f3f5;
  }
  .name {
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .path {
    max-width: 40%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .muted {
    color: #6b7280;
    font-size: 12px;
  }
  .empty {
    padding: 16px;
    text-align: center;
  }
</style>
