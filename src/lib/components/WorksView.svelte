<script lang="ts">
  import StatusLine from "$lib/components/ui/StatusLine.svelte";
  import ListPager from './ui/ListPager.svelte';
  import {paginate} from '$lib/services/pagination';
  import EmptyState from "$lib/components/ui/EmptyState.svelte";
  import NaturalCapture from './NaturalCapture.svelte';
  import {navigateTo} from '$lib/services/navigation';
  let {focusId=null,resumeId=null,onprojectchange=()=>{}}:{focusId?:number|null;resumeId?:number|null;onprojectchange?:(id:number)=>void}=$props();
  let projectCapture=$state(false);
  import CognitionPanel from "$lib/components/CognitionPanel.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { locale, t, translateKind, translateStatus } from "$lib/i18n";
  import Modal from "$lib/components/ui/Modal.svelte";
  import TypedDeleteDialog from './ui/TypedDeleteDialog.svelte';
  import AppButton from "$lib/components/ui/AppButton.svelte";
  import Icon from "$lib/components/ui/Icon.svelte";
  import { addToast } from "$lib/stores/toast";
  import { dataRevision, invalidate } from "$lib/stores/dataRevision";
  import { startWorkspaceWorkDraft } from "$lib/services/api";
  import { open as dialogOpen } from "@tauri-apps/plugin-dialog";
  import { aiJobs } from "$lib/stores/aiJobs";

  type Work = {
    id: number;
    revision:number;
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
  type Workspace = {id:number; name:string; root_path:string};
  let linkedFolders = $state<Workspace[]>([]);
  let availableFolders = $state<Workspace[]>([]);
  let existingFolderId = $state<number|null>(null);
  let folderBusy = $state(false);

  let works = $state<Work[]>([]);
  let selectedId = $state<number | null>(null);
  let detail = $state<WorkDetail | null>(null);
  let taskListPage=$state(1),waitingListPage=$state(1),calendarListPage=$state(1),fileListPage=$state(1);
  const projectTasks=$derived(paginate(detail?.tasks??[],taskListPage));
  const projectWaiting=$derived(paginate(detail?.waiting.filter(item=>item.status==='open')??[],waitingListPage));
  const projectCalendar=$derived(paginate(detail?.calendar??[],calendarListPage));
  const projectFiles=$derived(paginate(detail?.files??[],fileListPage));
  $effect(()=>{selectedId;taskListPage=1;waitingListPage=1;calendarListPage=1;fileListPage=1;});
  let error = $state("");
  let newTitle = $state("");
  let newSummary = $state("");
  let showCreate = $state(false);
  let createLoading = $state(false);
  let createError = $state("");
  let showEdit = $state(false);
  let editLoading = $state(false);
  let editTitle = $state("");
  let editSummary = $state("");
  let editStatus = $state("active");
  let showDelete = $state(false);
  let deleteSnapshot=$state<Work|null>(null);
  let deleteLoading = $state(false);
  let deleteError = $state("");
  let resumeError = $state("");

  // 新建 Resume Point 表单
  let rpState = $state("");
  let rpNext = $state("");
  let rpRemember = $state("");
  let quickProgress = $state("");

  let organizeBusy = $derived($aiJobs.some(job=>job.command==="start_workspace_work_draft" && job.args.workId===selectedId && job.status==="running"));
  let organizationWorkspaceId = $state<number | null>(null);
  let currentLocale = $derived($locale);
  const tt = (key: Parameters<typeof t>[0], params: Record<string, string | number> = {}) => t(key, params, currentLocale);

  function fmtTime(ts: number | null): string {
    if (!ts) return "";
    const d = new Date(ts * 1000);
    const pad = (n: number) => String(n).padStart(2, "0");
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
  }

  async function loadWorks() {
    try {
      works = await invoke("list_works", { status: null });
      if(selectedId===null){
        let id=focusId;
        if(resumeId){const location=await invoke<{work_id:number}>('get_entity_location',{kind:'resume_point',id:resumeId});id=location.work_id;}
        if(id)await openDetail(id);else if(works.length)await openDetail(works[0].id);
      }
    } catch (e) {
      error = String(e);
    }
  }

  async function createWork() {
    if (!newTitle.trim()) {
      createError = tt("common.required");
      return;
    }
    createLoading = true;
    createError = "";
    try {
      const w: Work = await invoke("create_work", { title: newTitle.trim(), status: "active" });
      if (newSummary.trim()) await invoke("update_work", {id:w.id,title:w.title,status:w.status,summary:newSummary.trim()});
      newTitle = "";
      newSummary = "";
      showCreate = false;
      addToast(tt("settings.saved"), "success");
      invalidate("works", "brief");
      await loadWorks();
      await openDetail(w.id);
    } catch (e) {
      createError = String(e);
    } finally {
      createLoading = false;
    }
  }

  function openCreate() {
    newTitle = "";
    newSummary = "";
    createError = "";
    showCreate = true;
  }

  function openEdit() {
    if (!detail) return;
    editTitle = detail.work.title;
    editSummary = detail.work.summary ?? "";
    editStatus = detail.work.status;
    showEdit = true;
  }

  async function saveEdit() {
    if (!detail) return;
    if (!editTitle.trim()) {
      error = tt("common.required");
      return;
    }
    editLoading = true;
    try {
      await invoke("update_work", { id: detail.work.id, title: editTitle.trim(), status: editStatus, summary: editSummary.trim() || null });
      showEdit = false;
      error = "";
      addToast(tt("settings.saved"), "success");
      invalidate("works", "brief");
      await loadWorks();
      await openDetail(detail.work.id);
    } catch (e) {
      error = String(e);
    } finally {
      editLoading = false;
    }
  }

  async function openDetail(id: number) {
    selectedId = id;
    onprojectchange(id);
    rpState = "";
    rpNext = "";
    rpRemember = "";
    quickProgress = "";
    try {
      const [workDetail, linkedWorkspaceIds, folders] = await Promise.all([
        invoke<WorkDetail>("get_work_detail", { id }),
        invoke<number[]>("list_work_workspaces", { workId: id }),
        invoke<Workspace[]>("get_workspaces"),
      ]);
      if (selectedId !== id) return;
      detail = workDetail;
      linkedFolders = folders.filter(folder=>linkedWorkspaceIds.includes(folder.id));
      availableFolders = folders.filter(folder=>!linkedWorkspaceIds.includes(folder.id));
      existingFolderId = null;
      organizationWorkspaceId = linkedWorkspaceIds[0] ?? null;
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
    if (!confirm(tt("work.archiveMessage", { title: w.title }))) return;
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

  function openDelete() {
    if (!detail) return;
    deleteError = "";
    deleteSnapshot={...detail.work};
    showDelete = true;
  }

  async function deleteWork(confirmationName:string) {
    if (!deleteSnapshot || deleteLoading) return;
    const id = deleteSnapshot.id;
    deleteLoading = true;
    deleteError = "";
    try {
      await invoke("delete_work", { id, confirmationName, expectedRevision:deleteSnapshot.revision });
      showDelete = false;
      selectedId = null;
      detail = null;
      linkedFolders = [];
      availableFolders = [];
      addToast(tt("work.deleteSuccess"), "success");
      invalidate("works", "tasks", "waiting", "calendar", "brief", "analysis");
      await loadWorks();
    } catch (e) {
      deleteError = String(e);
    } finally {
      deleteLoading = false;
    }
  }

  async function saveResumePoint() {
    if (!selectedId) return;
    if (!rpState.trim() && !rpNext.trim()) {
      resumeError = tt("common.required");
      return;
    }
    resumeError = "";
    try {
      await invoke("create_resume_point", {
        workId: selectedId,
        currentState: rpState.trim(),
        nextStep: rpNext.trim(),
        remember: rpRemember.trim(),
      });
      addToast(tt("settings.saved"), "success");
      invalidate("works", "brief");
      await openDetail(selectedId);
    } catch (e) {
      error = String(e);
    }
  }

  async function saveQuickProgress() {
    if (!selectedId || !quickProgress.trim()) {
      resumeError = tt("common.required");
      return;
    }
    resumeError = "";
    try {
      await invoke("create_resume_point", {
        workId: selectedId,
        currentState: quickProgress.trim(),
        nextStep: detail?.latest_resume?.next_step ?? "",
        remember: detail?.latest_resume?.remember ?? "",
      });
      addToast(tt("work.quickProgressSaved"), "success");
      invalidate("works", "brief");
      await openDetail(selectedId);
    } catch (e) {
      error = String(e);
    }
  }

  async function organizeWork() {
    if (!selectedId || organizeBusy) return;
    error = "";
    try {
      await startWorkspaceWorkDraft(organizationWorkspaceId, selectedId);
      addToast(tt("work.organizeComplete"), "success");
    } catch (e) {
      error = String(e);
      addToast(error, "error");
    }
  }

  async function attachFolder() {
    if (!selectedId || folderBusy) return;
    const id=selectedId;
    folderBusy=true; error="";
    try {
      const path=await dialogOpen({directory:true,multiple:false,title:tt("work.attachFolder")});
      if(typeof path!=="string") return;
      await invoke("attach_work_folder",{workId:id,path});
      invalidate("workspace","works"); await openDetail(id);
    } catch(e){error=String(e);} finally{folderBusy=false;}
  }
  async function linkExisting() {
    if (!selectedId || !existingFolderId) return;
    const id=selectedId;
    try {await invoke("link_work_workspace",{workId:id,workspaceId:existingFolderId,isPrimary:linkedFolders.length===0});invalidate("workspace","works");await openDetail(id);}catch(e){error=String(e);}
  }
  async function detachFolder(folder:Workspace) {
    if(!selectedId) return;
    const id=selectedId;
    try{await invoke("unlink_work_workspace",{workId:id,workspaceId:folder.id});invalidate("workspace","works");await openDetail(id);}catch(e){error=String(e);}
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
    $dataRevision.works;
    loadWorks();
  });
</script>

<div class="works">
  <div class="page-head">
    <div>
      <h1>{currentLocale==='en-US'?'Projects':'长期项目'}</h1>
      <p class="muted">{tt("work.description")}</p>
    </div>
    <AppButton testid="work-create" label={tt("work.new")} onclick={() => openCreate()} />
  </div>
  <div class="status error stable-feedback"><StatusLine message={error}/></div>

  <Modal bind:open={showCreate} title={tt("work.new")} onclose={() => (showCreate = false)}>
    <form class="modal-form" onsubmit={(event) => { event.preventDefault(); createWork(); }}>
      <label for="new-work-title">{tt("work.title")} *</label>
      <input id="new-work-title" bind:value={newTitle} placeholder={tt("work.placeholder")} />
      <label for="new-work-summary">{tt("work.summary")}</label><textarea id="new-work-summary" bind:value={newSummary} rows="3"></textarea>
      <div class="status error stable-feedback"><StatusLine message={createError}/></div>
      <div class="modal-actions">
        <button type="button" onclick={() => (showCreate = false)}>{tt("common.cancel")}</button>
        <AppButton testid="work-save" type="submit" loading={createLoading} label={tt("common.create")} />
      </div>
    </form>
  </Modal>

  <div class="layout">
    <!-- 左侧：Work 列表 -->
    <div class="list-pane">
      <div class="pane-head"><strong>{tt("work.allWorks")}</strong><span>{works.length}</span></div>
      {#if works.length}
      <ul class="work-list">
        {#each works as w (w.id)}
          <li class:active={selectedId === w.id}>
            <button class="work-item" onclick={() => openDetail(w.id)}>
              <span class="wt">{w.title}</span>
              <span class="muted">{translateStatus(w.status, currentLocale)}{w.updated_at ? ` · ${fmtTime(w.updated_at)}` : ""}</span>
            </button>
          </li>
        {/each}
      </ul>
      {:else}
        <EmptyState compact title={currentLocale==='en-US'?'No projects yet':'还没有项目'}/>
      {/if}
    </div>

    <!-- 右侧：Work 详情 -->
    <div class="detail-pane">
      {#if detail}
        {@const w = detail.work}
        <div class="detail-head">
          <h2>{w.title}</h2>
          <details class="project-options"><summary>{currentLocale==='en-US'?'Project settings':'项目设置'}</summary><div class="status-actions">
            {#each ["active", "paused", "waiting", "done"] as s (s)}
              <button class:on={w.status === s} onclick={() => changeStatus(w, s)}>{translateStatus(s, currentLocale)}</button>
            {/each}
            <button onclick={openEdit}>{tt("common.edit")}</button>
            <button onclick={() => archive(w)}>{tt("common.archive")}</button>
            <button class="delete-action" data-testid="work-delete" onclick={openDelete}>{tt("common.delete")}</button>
          </div>
          </details>
          <div class="project-goal"><span>{currentLocale==='en-US'?'Goal':'希望达成什么'}</span><p>{w.summary||(currentLocale==='en-US'?'Record a few words or let the secretary help clarify the goal.':'可以先说几句话，随后让秘书帮您理清目标。')}</p></div>
          <div class="project-primary-actions"><button data-testid="project-ask" onclick={()=>navigateTo({view:'qa',workId:w.id})}>{currentLocale==='en-US'?'Ask about this project':'问这个项目'}</button><button data-testid="project-record" onclick={()=>projectCapture=!projectCapture}>{currentLocale==='en-US'?'Record progress':'记进展'}</button><button onclick={()=>{projectCapture=true;}}>{currentLocale==='en-US'?'Add a matter':'加一件事'}</button><AppButton testid="organize-work-with-ai" loading={organizeBusy} onclick={organizeWork}>{currentLocale==='en-US'?'Let secretary organize':'让秘书整理'}</AppButton></div>
          {#if organizeBusy}<p role="status">{tt('work.backgroundHint')}</p>{/if}
          {#if projectCapture}<NaturalCapture context={{workId:w.id,entityKind:'work',entityId:w.id}} label={currentLocale==='en-US'?'Record for this project':'记在这个项目下'}/>{/if}
        </div>

        <!-- Current State / Next Step / Remember（置顶） -->
        <section class="resume card">
          <div class="card-title">{currentLocale==='en-US'?'Where we are · What comes next':'目前进展 · 下一步'}</div>
          {#if detail.waiting.some(item=>item.status==="open")}<div class="project-blockers"><b>{currentLocale==="en-US"?"Waiting on":"当前等待"}</b><span>{detail.waiting.filter(item=>item.status==="open").slice(0,3).map(item=>item.title).join(" · ")}</span></div>{/if}
          {#if detail.latest_resume}
            <div class="rp-current"><b>{tt("work.resumeCurrent")}</b>{detail.latest_resume.current_state || "—"}</div>
            <div class="rp-next"><b>{tt("work.resumeNext")}</b>{detail.latest_resume.next_step || "—"}</div>
            <div class="rp-remember"><b>{tt("work.resumeRemember")}</b>{detail.latest_resume.remember || "—"}</div>
          {:else}
            <div class="muted">{tt("work.resumeEmpty")}</div>
          {/if}

          <div class="status error stable-feedback"><StatusLine message={resumeError}/></div>
          <details class="progress-details" data-testid="work-progress-details">
            <summary>{tt("work.progressDetails")}</summary>
            <div class="rp-form">
              <label>{tt("work.resumeCurrent")}<input aria-label={tt("work.resumeCurrent")} bind:value={rpState} placeholder={tt("work.resumeState")} /></label>
              <label>{tt("work.resumeNext")}<input aria-label={tt("work.resumeNext")} bind:value={rpNext} placeholder={tt("work.resumeNextPlaceholder")} /></label>
              <label>{tt("work.resumeRemember")}<input aria-label={tt("work.resumeRemember")} bind:value={rpRemember} placeholder={tt("work.resumeRememberPlaceholder")} /></label>
              <button onclick={saveResumePoint}>{tt("work.saveResume")}</button>
            </div>
          </details>
          {#if detail.resume_history.length > 1}
            <details class="history">
              <summary>{tt("work.resumeHistory", { count: detail.resume_history.length })}</summary>
              {#each detail.resume_history as rp (rp.id)}
                <div class="hist-item">
                  <div class="muted">{fmtTime(rp.created_at)}</div>
                  <div>{rp.current_state} → {rp.next_step}</div>
                </div>
              {/each}
            </details>
          {/if}
        </section>

        <Modal bind:open={showEdit} title={tt("common.edit")} onclose={() => (showEdit = false)}>
          <form class="modal-form" onsubmit={(event) => { event.preventDefault(); saveEdit(); }}>
            <label for="edit-work-title">{tt("work.title")} *</label><input id="edit-work-title" bind:value={editTitle} />
          <label for="edit-work-summary">{tt("work.summary")}</label><textarea id="edit-work-summary" bind:value={editSummary} rows="3"></textarea>
            <label for="edit-work-status">{tt("common.status")}</label><select id="edit-work-status" bind:value={editStatus}>
              {#each ["active", "paused", "waiting", "done"] as s (s)}
                <option value={s}>{translateStatus(s, currentLocale)}</option>
              {/each}
            </select>
            <div class="modal-actions">
              <button type="button" onclick={() => (showEdit = false)}>{tt("common.cancel")}</button>
              <AppButton type="submit" loading={editLoading} label={tt("common.save")} />
            </div>
          </form>
        </Modal>

        <TypedDeleteDialog bind:open={showDelete} title={tt("work.deleteTitle")} name={deleteSnapshot?.title??w.title} busy={deleteLoading} error={deleteError} testid="work-delete" onconfirm={deleteWork}>
          <div class="delete-confirmation">
            <p>{tt("work.deleteMessage", { title: w.title })}</p>
            <div class="delete-impact">{tt("work.deleteImpact")}</div>
          </div>
        </TypedDeleteDialog>

        <!-- WAITING -->
        <section class="card">
          <div class="card-title">{tt("work.waiting")}</div>
          {#if detail.waiting.filter((x) => x.status === "open").length === 0}
            <div class="muted">{tt("work.noWaiting")}</div>
          {/if}
          <ListPager view={projectWaiting} onchange={(page)=>waitingListPage=page} testid="project-waiting-pagination"/>
          {#each projectWaiting.items as wq (wq.id)}
            {#if wq.status === "open"}
              <div class="row-item">
                <button class="entity-link" onclick={()=>navigateTo('waiting',wq.id)}>{wq.title}</button>
                <span class="muted">{tt("work.waitingDetail", { person: wq.waiting_for || "—" })}{wq.follow_up_at ? ` · ${tt("work.followUpDetail", { time: fmtTime(wq.follow_up_at) })}` : ""}</span>
                <button onclick={() => resolveWaiting(wq)}>{tt("common.resolve")}</button>
              </div>
            {/if}
          {/each}
        </section>

        <details class="card project-assistant" data-testid="work-folder-card">
          <summary>{currentLocale==='en-US'?'Project folders & knowledge':'项目资料与目录认知'}</summary>
          <div class="folder-toolbar"><button data-testid="attach-work-folder" onclick={attachFolder} disabled={folderBusy}><Icon name="folder" size={17}/>{tt("work.attachFolder")}</button>
            {#if availableFolders.length}<select data-testid="existing-work-folder" aria-label={tt("work.existingFolder")} bind:value={existingFolderId}><option value={null}>{tt("work.existingFolder")}</option>{#each availableFolders as folder(folder.id)}<option value={folder.id}>{folder.name}</option>{/each}</select><button onclick={linkExisting} disabled={!existingFolderId}>{tt("work.linkFolder")}</button>{/if}
          </div>
          {#each linkedFolders as folder(folder.id)}<div class="linked-folder"><Icon name="folder" size={17}/><span><strong>{folder.name}</strong><small>{folder.root_path}</small></span><button onclick={()=>detachFolder(folder)}>{tt("work.unlinkFolder")}</button></div>{/each}
          <p class="folder-note">{linkedFolders.length?tt("work.folderSafeHint"):tt("work.noFolderHint")}</p>
          <CognitionPanel scope="work" scopeId={w.id}/>
          {#if organizeBusy}<p role="status">{tt("work.backgroundHint")}</p>{/if}
        </details>

        <!-- FILES -->
        <details class="card project-details">
          <summary>{tt("work.aiFiles")}</summary>
          {#if detail.files.length === 0}
            <div class="muted ai-file-empty">{tt("work.noAiFiles")}</div>
          {/if}
          <ListPager view={projectFiles} onchange={(page)=>fileListPage=page} testid="project-files-pagination"/>
          {#each projectFiles.items as f (f.id)}
            <div class="row-item" class:file-pinned={f.pinned}>
              <button class="fname" onclick={() => openFile(f.path)} title={f.path}>
                {f.pinned ? "📌" : "📄"} {f.label || f.path.split(/[\\/]/).pop()}
              </button>
              <button onclick={() => togglePin(f)}>{f.pinned ? tt("common.unpin") : tt("common.pin")}</button>
              <button onclick={() => removeFile(f)}>{tt("common.remove")}</button>
            </div>
          {/each}
        </details>

        <!-- CALENDAR -->
        <details class="card project-details">
          <summary>{tt("work.calendar")}</summary>
          {#if detail.calendar.length === 0}
            <div class="muted">{tt("work.noCalendar")}</div>
          {/if}
          <ListPager view={projectCalendar} onchange={(page)=>calendarListPage=page} testid="project-calendar-pagination"/>
          {#each projectCalendar.items as ev (ev.id)}
            <div class="row-item">
              <span>{fmtTime(ev.start_at)}</span>
              <button class="entity-link" onclick={()=>navigateTo('calendar',ev.id)}>{ev.title}</button>
              <span class="muted">{translateKind(ev.kind, currentLocale)}</span>
            </div>
          {/each}
        </details>

        <!-- TASKS -->
        <section class="card">
          <div class="card-title">{tt("work.tasks")}</div>
          {#if detail.tasks.length === 0}
            <div class="muted">{tt("work.noTasks")}</div>
          {/if}
          <ListPager view={projectTasks} onchange={(page)=>taskListPage=page} testid="project-tasks-pagination"/>
          {#each projectTasks.items as t (t.id)}
            <div class="row-item" class:task-done={t.status === "done"}>
              <button class="entity-link" class:strike={t.status === "done"} onclick={()=>navigateTo('task',t.id)}>{t.title}</button>
              <span class="muted">{translateStatus(t.status, currentLocale)}{t.due_at ? ` · ${fmtTime(t.due_at)}` : ""}</span>
              {#if t.status !== "done"}
                <button onclick={() => completeTask(t)}>{tt("common.complete")}</button>
              {/if}
            </div>
          {/each}
        </section>

        <!-- RECENT ACTIVITY -->
        <details class="card project-details">
          <summary>{tt("work.activity")}</summary>
          {#if detail.recent_activity.length === 0}
            <div class="muted">{tt("work.noActivity")}</div>
          {/if}
          {#each detail.recent_activity as a (a.id)}
            <div class="row-item">
              <span class="muted">{fmtTime(a.timestamp)}</span>
              <span>{a.display_text}</span>
            </div>
          {/each}
        </details>
      {:else}
        <EmptyState compact title={currentLocale==='en-US'?'Start with a project':'从一个项目开始'} description={currentLocale==='en-US'?'Create a project with a name. Add progress and related matters as work unfolds.':'先给项目起个名字，进展与关联事项可以在工作中逐步补充。'}/>
      {/if}
    </div>

  </div>
</div>

<style>
  .project-blockers{display:flex;gap:12px;flex-wrap:wrap;font-size:15px;line-height:1.7;padding:12px;border-radius:10px;background:var(--color-warning-soft);color:var(--color-text);margin-bottom:12px}.project-blockers span{overflow-wrap:anywhere;min-width:0}
  .project-assistant{background:linear-gradient(140deg,var(--color-primary-soft),var(--color-surface));padding:22px!important}
  .folder-note{font-size:14px;line-height:1.65;color:var(--color-muted);margin:8px 0 0}.folder-toolbar{display:flex;align-items:center;flex-wrap:wrap;gap:10px;margin-top:20px}.folder-toolbar button{display:flex;align-items:center;gap:6px;min-height:40px;font-size:14px}.folder-toolbar select{min-width:0;max-width:100%;flex:1;padding:9px;border:1px solid var(--color-border);border-radius:10px;background:var(--color-surface);font-size:14px}.linked-folder{display:flex;align-items:center;gap:12px;padding:14px 0;border-bottom:1px solid var(--color-border)}.linked-folder>span{flex:1;min-width:0;display:grid;gap:4px}.linked-folder small{overflow-wrap:anywhere;color:var(--color-muted);font-size:13px}.linked-folder strong{font-size:15px}.linked-folder button{flex-shrink:0}
  h1 {
    font-size: 18px;
    margin: 0 0 12px;
  }
  .page-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 16px;
  }
  .page-head p { margin: -6px 0 0; }
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
  .modal-form { display: grid; gap: 8px; }
  .modal-form label { font-size: 12px; font-weight: 600; }
  .modal-form input,
  .modal-form textarea,
  .modal-form select { width: 100%; border: 1px solid var(--color-border); border-radius: var(--radius-sm); padding: 9px 10px; background: var(--color-surface); }
  .modal-actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 10px; }
  .delete-confirmation { display: grid; gap: 14px; }
  .delete-confirmation p { margin: 0; font-size: 15px; line-height: 1.65; }
  .delete-impact { padding: 12px 14px; border: 1px solid #ead7d2; border-radius: 10px; background: var(--color-danger-soft); color: #805f5f; font-size: 13px; line-height: 1.65; }
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
  .ai-file-empty { padding: 14px; border: 1px dashed var(--color-border); border-radius: 10px; background: var(--color-surface-muted); font-size: 12px; line-height: 1.55; }
  .status.error {
    color: #b3261e;
    font-size: 13px;
    margin: 6px 0;
  }

  /* C1 editorial workspace */
  .works { min-width: 0; display: grid; gap: 16px; }
  .page-head { min-height: 54px; align-items: center; margin: 0; }
  .page-head h1 { margin: 0 0 5px; font: 500 22px var(--font-serif); letter-spacing: .01em; }
  .page-head p { margin: 0; color: var(--color-muted); font-size: 10px; }
  .layout { min-height: 0; display: grid; grid-template-columns: 220px minmax(0, 1fr) 260px; gap: 12px; align-items: stretch; overflow: hidden; }
  .list-pane,.detail-pane{ min-width: 0; min-height: 0; border: 1px solid var(--color-border); border-radius: var(--radius-md); background: var(--color-surface); box-shadow: var(--shadow-sm); }
  .list-pane { width: auto; display: flex; flex-direction: column; align-self: start; overflow: hidden; }
  .pane-head { height: 47px; display: flex; align-items: center; justify-content: space-between; padding: 0 13px; border-bottom: 1px solid #e5ebed; }
  .pane-head strong { font-size: 11px; font-weight: 650; }
  .pane-head span { min-width: 22px; height: 22px; display: grid; place-items: center; border-radius: 8px; background: var(--color-primary-soft); color: var(--color-primary); font-size: 9px; font-weight: 700; }
  .work-list { flex: 1; min-height: 0; max-height: 650px; padding: 8px; overflow: auto; }
  .work-list li { margin: 2px 0; }
  .work-list li.active .work-item { background: var(--color-primary-soft); border-color: transparent; }
  .work-item { min-height: 52px; padding: 9px 10px; border: 1px solid transparent; border-radius: 10px; background: transparent; gap: 5px; }
  .work-item:hover { background: var(--color-surface-muted); }
  .wt { font-size: 11px; font-weight: 650; }
  .work-item .muted { font-size: 8px; color: #89989e; }
  .detail-pane { padding: 15px 16px; overflow: auto; }
  .detail-head { padding: 3px 2px 14px; border-bottom: 1px solid #e6ecee; margin-bottom: 12px; }
  .detail-head h2 { margin: 0 0 7px; font: 500 19px var(--font-serif); }
  .status-actions { gap: 5px; }
  .status-actions button, .row-item button, .rp-form button, .modal-actions button { min-height: 29px; padding: 5px 8px; border: 1px solid var(--color-border); border-radius: 8px; background: var(--color-surface-raised); color: #667b84; font-size: 9px; }
  .status-actions button.on { background: var(--color-primary-soft); border-color: #d2dfe3; color: #546e79; font-weight: 650; }
  .status-actions button.delete-action { border-color: #e4cccc; color: var(--color-danger); }
  .status-actions button.delete-action:hover { background: var(--color-danger-soft); }
  .card { margin-bottom: 10px; padding: 12px; border: 1px solid var(--color-border); border-radius: 12px; background: var(--color-surface); }
  .resume { background: linear-gradient(115deg, #f8fafa, #eef3f4); border-color: #d5e0e4; }
  .card-title { color: #566d77; font-size: 10px; letter-spacing: .04em; }
  .rp-current, .rp-next, .rp-remember { font-size: 10px; line-height: 1.5; }
  .rp-form { display: grid; grid-template-columns: repeat(3, minmax(110px, 1fr)) auto; gap: 6px; }
  .rp-form label { color: var(--color-muted); font-size: 8px; }
  .rp-form input { width: 100%; min-width: 0; margin-top: 4px; padding: 7px; border-color: var(--color-border); border-radius: 8px; background: #fbfcfc; font-size: 9px; }.progress-details{margin-top:10px}.progress-details summary{width:max-content;color:#71858d;font-size:10px;cursor:pointer}.progress-details[open] summary{margin-bottom:8px}
  .row-item { min-height: 34px; padding: 6px 0; border-bottom: 1px solid #edf1f2; font-size: 10px; }
  .row-item:last-child { border-bottom: 0; }
  .row-item .muted { font-size: 9px; }
  .row-item .fname { color: #5c7884; font-size: 10px; }
  .modal-form input, .modal-form textarea, .modal-form select { border-color: var(--color-border); border-radius: var(--radius-sm); background: var(--color-surface-raised); }
  @container (max-width: 1120px) { .works{height:auto}.layout{grid-template-columns:220px minmax(0,1fr);overflow:visible}.rp-form{grid-template-columns:1fr 1fr}.rp-form button{align-self:end} }
  @container (max-width: 760px) { .layout { grid-template-columns: minmax(0, 1fr); }.list-pane{max-height:280px} }
  @container (max-width: 560px) { .works{height:auto}.layout{grid-template-columns:1fr;overflow:visible}.list-pane{max-height:260px}.detail-pane{overflow:visible}.rp-form{grid-template-columns:1fr} }
  /* Readable compact work detail typography. */
  .page-head p{font-size:12px}
  .pane-head strong,.wt{font-size:13px}.pane-head span{font-size:11px}
  .work-item .muted,.rp-form label{font-size:10px}
  .status-actions button,.row-item button,.rp-form button,.modal-actions button,.rp-form input{font-size:11px}
  .card-title,.rp-current,.rp-next,.rp-remember,.row-item,.row-item .fname{font-size:12px}.row-item .muted{font-size:11px}
  .layout { overflow: visible; }
  .detail-pane { overflow: visible; }
  .detail-head h2,.wt{ overflow-wrap: anywhere; }
  .row-item { flex-wrap: wrap; gap: 8px 12px; align-items: start; line-height: 1.6; }
  .row-item>span { min-width: 0; flex: 1 1 180px; overflow-wrap: anywhere; }
  .row-item button { flex-shrink: 0; }
  .row-item .entity-link { flex: 1 1 200px; min-width: 0; max-width: 100%; white-space: normal; overflow-wrap: anywhere; }
  .row-item .fname { min-width: 0; flex: 1 1 200px; overflow-wrap: anywhere; }
  .page-head,.status-actions{ flex-wrap: wrap; }
  .rp-current,.rp-next,.rp-remember{ font-size: 13px; line-height: 1.65; }
  .pane-head { height: auto; min-height: 48px; padding-block: 10px; gap: 12px; }
  .pane-head span { flex-shrink: 0; }
  @container (max-width: 560px) { .row-item>span { flex-basis: 100%; } .rp-form { grid-template-columns: minmax(0,1fr); } }

  .layout{display:grid;grid-template-columns:230px minmax(0,1fr);align-items:start;gap:22px}.detail-pane{display:flex;flex-direction:column;min-width:0;gap:18px;overflow:visible}.detail-head{display:flex;flex-wrap:wrap;align-items:start;gap:14px}.detail-head h2{flex:1 1 250px;font-size:27px;line-height:1.4}.project-options{margin-left:auto}.project-options summary,.project-details>summary,.project-assistant>summary{cursor:pointer;font-size:15px;font-weight:600;line-height:1.7;padding:5px 0}.status-actions{padding:12px 0;display:flex;flex-wrap:wrap;gap:8px}.project-goal{flex-basis:100%;font-size:15px;line-height:1.65}.project-goal span{color:var(--color-muted);font-size:13px}.project-goal p{margin:6px 0}.project-primary-actions{display:flex;flex-wrap:wrap;gap:9px;width:100%}.project-primary-actions button{padding:10px 15px;font-size:14px;border:1px solid var(--color-border);border-radius:9px;background:var(--color-surface)}.detail-head :global(.natural-capture){width:100%}.entity-link{text-align:left;border:0!important;background:transparent!important;color:var(--color-primary);font-size:15px;text-decoration:underline;text-underline-offset:4px}.resume{padding:22px;font-size:15px;line-height:1.7}.project-assistant>div{margin-top:12px}.list-pane{max-height:none;overflow:visible}.work-list{max-height:none;overflow:visible}
  @container(max-width:850px){.layout{grid-template-columns:1fr}.work-list{display:flex;flex-wrap:wrap;gap:8px}.work-list li{flex:1 1 180px}.project-options{margin-left:0}}
</style>
