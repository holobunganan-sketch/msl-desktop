<script lang="ts">
  import { untrack } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import CognitionPanel from "$lib/components/CognitionPanel.svelte";
  import AppButton from "$lib/components/ui/AppButton.svelte";
  import AppCard from "$lib/components/ui/AppCard.svelte";
  import Modal from "$lib/components/ui/Modal.svelte";
  import { locale, t } from "$lib/i18n";
  import { dataRevision, invalidate } from "$lib/stores/dataRevision";
  import { addToast } from "$lib/stores/toast";
  import { startWorkspaceWorkDraft } from "$lib/services/api";

  type Directory = { id: number; name: string; root_path: string; projects: { id: number; title: string; status: string }[] };
  type DirEntry = { name: string; path: string; is_dir: boolean; modified: number; size: number };
  type SyncStatus = { workspace_id: number | null; root: string | null; paused: boolean; watching: boolean; baseline_count: number; last_scan: number | null; last_warning: string | null };
  type DocumentIndex = { id: number; relative_path: string; extract_status: string; error_message: string | null };
  type DocumentStatus = { ready: number; unsupported: number; failed: number; needs_ocr: number; too_large: number };
  let directories = $state<Directory[]>([]);
  let selectedId = $state<number | null>(null);
  let selected = $derived(directories.find(d => d.id === selectedId) ?? null);
  let currentPath = $state("");
  let trail = $state<{ name: string; path: string }[]>([]);
  let entries = $state<DirEntry[]>([]);
  let sync = $state<SyncStatus | null>(null);
  let documents = $state<DocumentIndex[]>([]);
  let documentStatus = $state<DocumentStatus | null>(null);
  let error = $state("");
  let loading = $state(false);
  let scanning = $state(false);
  let indexing = $state(false);
  let draftBusy = $state(false);
  let removing = $state(false);
  let removalTarget = $state<Directory | null>(null);
  let removalOpen = $state(false);
  let removalError = $state("");
  let en = $derived($locale === "en-US");
  let selectionVersion = 0;
  let listVersion = 0;
  let pathVersion = 0;
  const tt = (key: Parameters<typeof t>[0], params: Record<string, string | number> = {}) => t(key, params, $locale);
  const displayPath = (path: string) => path.startsWith("\\\\?\\UNC\\") ? "\\\\" + path.slice(8) : path.replace(/^\\\\\?\\/, "");
  const protectedProjects = (directory: Directory) => directory.projects.filter(p => p.status !== "archived");
  function navigateProject(id?: number) { window.dispatchEvent(new CustomEvent("dashboard:navigate", { detail: { view: "works", id } })); }
  function fmtTime(ts: number | null) { return ts ? new Intl.DateTimeFormat($locale, { dateStyle: "medium", timeStyle: "short" }).format(new Date(ts * 1000)) : "—"; }
  function fmtSize(bytes: number) { return bytes < 1024 ? `${bytes} B` : bytes < 1024 ** 2 ? `${(bytes / 1024).toFixed(1)} KB` : `${(bytes / 1024 ** 2).toFixed(1)} MB`; }

  async function loadDir(path: string, nextTrail: typeof trail) {
    const token = ++pathVersion, id = selectedId;
    loading = true; entries = []; error = "";
    currentPath = path; trail = nextTrail;
    try { const result = await invoke<DirEntry[]>("list_dir", { path }); if (token === pathVersion && selectedId === id) entries = result; }
    catch (e) { if (token === pathVersion) error = String(e); }
    finally { if (token === pathVersion) loading = false; }
  }
  async function loadMetadata(id: number, token = selectionVersion) {
    const result = await Promise.all([
      invoke<SyncStatus>("workspace_sync_status", { workspaceId: id }),
      invoke<DocumentStatus>("workspace_document_status", { workspaceId: id }),
      invoke<DocumentIndex[]>("list_workspace_documents", { workspaceId: id })
    ]);
    if (selectedId === id && token === selectionVersion) [sync, documentStatus, documents] = result;
  }
  async function selectDirectory(directory: Directory) {
    const token = ++selectionVersion;
    selectedId = directory.id; sync = null; documentStatus = null; documents = [];
    try { await Promise.all([loadDir(directory.root_path, [{ name: directory.name, path: directory.root_path }]), loadMetadata(directory.id, token)]); }
    catch (e) { if (token === selectionVersion) error = String(e); }
  }
  function clearSelection() {
    ++selectionVersion; ++pathVersion;
    selectedId = null; currentPath = ""; trail = []; entries = []; documents = []; sync = null; documentStatus = null; loading = false;
  }
  async function load() {
    const token = ++listVersion;
    try {
      const items = await invoke<Directory[]>("get_project_directories");
      if (token !== listVersion) return;
      directories = items;
      if (!items.length) { clearSelection(); return; }
      const current = items.find(d => d.id === selectedId);
      if (!current) await selectDirectory(items[0]);
      else await loadMetadata(current.id);
    } catch (e) { if (token === listVersion) error = String(e); }
  }
  async function scanNow() {
    if (selectedId === null) return;
    const id = selectedId; scanning = true; error = "";
    try { await invoke("workspace_rescan", { workspaceId: id }); await loadMetadata(id); invalidate("workspace"); addToast(tt("feedback.scanComplete"), "success"); }
    catch (e) { error = String(e); } finally { scanning = false; }
  }
  async function reindex() {
    if (selectedId === null) return;
    const id = selectedId; indexing = true; error = "";
    try { await invoke("reindex_workspace_documents", { workspaceId: id }); await loadMetadata(id); addToast(tt("workspace.documentsIndexed"), "success"); }
    catch (e) { error = String(e); } finally { indexing = false; }
  }
  async function generateDraft() {
    if (selectedId === null) return;
    draftBusy = true;
    try { await startWorkspaceWorkDraft(selectedId); addToast(tt("workspace.draftQueued"), "success"); }
    catch (e) { error = String(e); } finally { draftBusy = false; }
  }
  function requestRemoval(directory: Directory) { if (protectedProjects(directory).length) return; removalTarget = directory; removalError = ""; removalOpen = true; }
  async function removeDirectory() {
    if (!removalTarget || removing) return;
    removing = true; removalError = "";
    try {
      await invoke("remove_workspace", { workspaceId: removalTarget.id });
      removalOpen = false; removalTarget = null;
      await load(); invalidate("workspace", "works", "documents");
      addToast(en ? "Directory removed from the workbench. Source files are unchanged." : "目录已从工作台移除，源文件保持不变。", "success");
    } catch (e) { removalError = String(e).replace(/^migration error:\s*/, ""); await load(); }
    finally { removing = false; }
  }
  async function openPath(path: string, reveal = false) { try { await invoke(reveal ? "reveal_in_explorer" : "open_file", { path }); } catch (e) { error = String(e); } }
  function enter(item: DirEntry) { if (item.is_dir) void loadDir(item.path, [...trail, { name: item.name, path: item.path }]); else void openPath(item.path); }
  $effect(() => { $dataRevision.workspace; $dataRevision.works; $dataRevision.proposals; untrack(() => { void load(); }); });
</script>

<div class="workspace-view">
  <header class="page-head"><div><div class="eyebrow">MSL Desktop</div><h1>{tt("workspace.title")}</h1><p>{en ? "Folders linked to your projects, kept in sync automatically." : "项目关联的目录集中在这里，随项目绑定关系自动同步。"}</p></div><AppButton variant="secondary" onclick={() => navigateProject()}>{en ? "Link a folder in Projects" : "前往项目绑定目录"}</AppButton></header>
  {#if error}<div class="error" role="alert">{error}</div>{/if}
  <section class="directory-list" data-testid="project-directory-list" aria-label={en ? "Project directories" : "项目关联目录"}>
    {#each directories as directory (directory.id)}
      {@const blockers = protectedProjects(directory)}
      <article class="directory-card" class:selected={selectedId === directory.id} data-testid={`directory-card-${directory.id}`}>
        <button class="directory-select" data-testid={`directory-select-${directory.id}`} aria-pressed={selectedId === directory.id} onclick={() => selectDirectory(directory)}><span class="directory-icon" aria-hidden="true">▤</span><span><strong>{directory.name}</strong><span class="path">{displayPath(directory.root_path)}</span></span></button>
        <div class="project-links">{#each directory.projects as project (project.id)}<button onclick={() => navigateProject(project.id)}>{project.title}<span>{project.status === "archived" ? (en ? "Archived" : "已归档") : (en ? "Linked" : "关联中")}</span></button>{/each}</div>
        <div class="directory-footer"><p>{blockers.length ? (en ? "Linked to an unarchived project. Unlink it in the project first." : "仍关联未归档项目，请先在项目中解除关联。") : (en ? "All linked projects are archived; this directory can be removed." : "关联项目均已归档，可从工作台移除。")}</p><AppButton variant="ghost" testid={`directory-remove-${directory.id}`} disabled={blockers.length > 0 || removing} onclick={() => requestRemoval(directory)}>{en ? "Remove" : "移除"}</AppButton></div>
      </article>
    {:else}
      <div class="empty-directory"><h2>{en ? "No project folders yet" : "还没有项目关联目录"}</h2><p>{en ? "Open a project and link its folder. Unlinking removes it here when no other project uses it. Your files stay untouched." : "在项目中关联文件夹后，它会自动出现在这里。解除最后一条关联后同步移出，不改动磁盘文件。"}</p><AppButton onclick={() => navigateProject()}>{en ? "Go to Projects" : "前往项目"}</AppButton></div>
    {/each}
  </section>
  {#if selected}
    <div class="selected-heading"><div><h2>{selected.name}</h2><p class="path">{displayPath(currentPath)}</p></div><AppButton variant="secondary" testid="workspace-scan-selected" loading={scanning} onclick={scanNow}>{tt("workspace.scanNow")}</AppButton></div>
    {#if sync}<div class="sync-card"><AppCard><div class="sync-head"><strong>{tt("workspace.syncStatus")}</strong><span>{sync.watching ? (sync.paused ? tt("workspace.paused") : tt("workspace.watching")) : (en ? "Included in scheduled analysis" : "随秘书周期分析扫描")}</span></div><div class="sync-meta"><span>{tt("workspace.fileCount", { count: sync.baseline_count })}</span><span>{sync.last_scan ? tt("workspace.lastScan", { time: fmtTime(sync.last_scan) }) : tt("workspace.neverScanned")}</span></div>{#if sync.last_warning}<p class="warning">{sync.last_warning}</p>{/if}</AppCard></div>{/if}
    <CognitionPanel scope="workspace" scopeId={selected.id}/>
    <div data-testid="workspace-browser"><AppCard>
      <nav class="breadcrumb" aria-label={en ? "Current folder" : "当前文件夹"}>{#each trail as segment, i (segment.path)}<button onclick={() => loadDir(segment.path, trail.slice(0, i + 1))}>{segment.name}</button>{#if i < trail.length - 1}<span>›</span>{/if}{/each}</nav>
      {#if loading}<p class="muted">{tt("common.loading")}</p>{:else if !entries.length}<p class="muted">{tt("workspace.empty")}</p>{:else}
        <div class="file-table-wrap"><table><thead><tr><th>{tt("workspace.name")}</th><th>{tt("workspace.modified")}</th><th>{tt("workspace.size")}</th><th><span class="muted">{en ? "Actions" : "操作"}</span></th></tr></thead><tbody>{#each entries as item (item.path)}<tr><td><button class="file-name" onclick={() => enter(item)}>{item.is_dir ? "📁" : "📄"} {item.name}</button></td><td>{fmtTime(item.modified)}</td><td>{item.is_dir ? "—" : fmtSize(item.size)}</td><td><div class="file-actions"><button onclick={() => openPath(item.path)}>{tt("common.open")}</button><button onclick={() => openPath(item.path, true)}>{tt("common.reveal")}</button></div></td></tr>{/each}</tbody></table></div>
      {/if}
    </AppCard></div>
    <div data-testid="workspace-documents"><AppCard><div class="doc-head"><div><h2>{tt("workspace.documents")}</h2><p class="muted">{documentStatus ? `${tt("workspace.readyCount", { count: documentStatus.ready })} · ${tt("workspace.skippedCount", { count: documentStatus.unsupported + documentStatus.failed + documentStatus.needs_ocr + documentStatus.too_large })}` : tt("common.loading")}</p></div><div class="doc-actions"><AppButton variant="secondary" loading={indexing} onclick={reindex}>{tt("workspace.reindex")}</AppButton><AppButton variant="ghost" loading={draftBusy} onclick={generateDraft}>{tt("workspace.generateDraft")}</AppButton></div></div><div class="doc-list">{#each documents.slice(0, 8) as doc (doc.id)}<div class="doc-row"><span>{doc.relative_path}</span><span class="muted">{doc.extract_status}{doc.error_message ? ` · ${doc.error_message}` : ""}</span></div>{/each}{#if documents.length > 8}<p class="muted">{tt("workspace.moreDocuments", { count: documents.length - 8 })}</p>{/if}{#if !documents.length}<p class="muted">{tt("workspace.noSupportedDocuments")}</p>{/if}</div></AppCard></div>
    <details class="global-cognition"><summary>{en ? "All-project context" : "查看整体项目认知"}</summary><CognitionPanel scope="global"/></details>
  {/if}
</div>
<Modal bind:open={removalOpen} title={en ? "Remove directory from workbench" : "从工作台移除目录"}>
  <p class="remove-message">{en ? `Remove “${removalTarget?.name ?? ""}” and its archived-project links? Files and folders on disk will not be changed. Existing reports and historical evidence remain available.` : `确定移除“${removalTarget?.name ?? ""}”及其已归档项目关联？磁盘上的文件夹和文件保持不变，已有报告和历史证据继续保留。`}</p>
  {#if removalError}<p class="error" role="alert">{removalError}</p>{/if}
  <div class="modal-actions"><AppButton variant="secondary" testid="directory-remove-cancel" disabled={removing} onclick={() => removalOpen = false}>{tt("common.cancel")}</AppButton><AppButton variant="danger" testid="directory-remove-confirm" loading={removing} onclick={removeDirectory}>{en ? "Remove from workbench" : "确认移除"}</AppButton></div>
</Modal>

<style>
  .workspace-view{display:grid;gap:20px;min-width:0;padding-bottom:12px}
  .page-head,.selected-heading,.doc-head{display:flex;align-items:center;justify-content:space-between;gap:18px;flex-wrap:wrap}
  h1{margin:6px 0 8px;font:500 30px var(--font-serif)}h2{font-size:20px;margin:0 0 6px}.eyebrow{color:var(--color-muted);font-size:14px;letter-spacing:.1em}
  p{margin:0;line-height:1.65;overflow-wrap:anywhere}.page-head p,.muted,.path,.directory-footer p,.sync-meta{color:var(--color-muted)}
  .directory-list{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,340px),1fr));gap:16px}
  .directory-card{min-width:0;display:flex;flex-direction:column;gap:14px;border:1px solid var(--color-border);border-radius:18px;padding:20px;background:var(--color-surface)}
  .directory-card.selected{border-color:var(--color-primary);background:linear-gradient(140deg,var(--color-primary-soft),var(--color-surface) 75%)}
  button{font:inherit;cursor:pointer;color:var(--color-text)}.directory-select{display:flex;gap:12px;align-items:flex-start;min-width:0;text-align:left;background:none;border:0;padding:0;width:100%}
  .directory-select>span:last-child{min-width:0}.directory-select strong{font-size:20px;line-height:1.5;overflow-wrap:anywhere}.directory-icon{font-size:26px;color:var(--color-primary);flex-shrink:0}
  .path{display:block;font-size:14px;line-height:1.6;overflow-wrap:anywhere;margin-top:4px}.project-links{display:flex;gap:8px;flex-wrap:wrap}
  .project-links button{display:flex;align-items:center;flex-wrap:wrap;gap:8px;max-width:100%;padding:7px 10px;background:var(--color-surface);border:1px solid var(--color-border);border-radius:8px;text-align:left;overflow-wrap:anywhere;font-size:15px}
  .project-links span{color:var(--color-muted);font-size:14px}.directory-footer{display:flex;align-items:center;gap:12px;margin-top:auto}.directory-footer p{font-size:14px;flex:1;min-width:0}
  .empty-directory{padding:30px;border:1px dashed var(--color-border-strong);border-radius:18px;background:var(--color-surface)}.empty-directory p{margin-bottom:18px;max-width:720px;color:var(--color-muted)}
  .sync-head,.sync-meta{display:flex;justify-content:space-between;gap:12px;flex-wrap:wrap;font-size:15px}.sync-meta{margin-top:8px}.sync-head span{color:var(--color-primary)}
  .doc-actions,.file-actions,.modal-actions{display:flex;gap:8px;flex-wrap:wrap}.modal-actions{justify-content:flex-end;margin-top:22px}.remove-message{line-height:1.8}
  .breadcrumb{display:flex;gap:8px;align-items:center;flex-wrap:wrap;margin-bottom:14px}.breadcrumb button,.file-actions button{padding:7px 10px;border:1px solid var(--color-border);background:var(--color-surface);border-radius:9px;overflow-wrap:anywhere;text-align:left}
  .file-table-wrap{max-width:100%;overflow:auto;border:1px solid var(--color-border);border-radius:12px}table{width:100%;border-collapse:collapse;font-size:15px;table-layout:fixed}th,td{padding:12px;vertical-align:top;text-align:left;border-bottom:1px solid var(--color-border);overflow-wrap:anywhere}th{color:var(--color-muted);font-weight:600}th:first-child{width:34%}th:nth-child(2){width:23%}th:nth-child(3){width:12%}.file-name{padding:0;border:0;background:none;font:inherit;text-align:left;overflow-wrap:anywhere}
  .doc-list{display:grid;gap:10px;margin-top:16px}.doc-row{display:flex;justify-content:space-between;gap:16px;border-top:1px solid var(--color-border);padding-top:10px;font-size:15px}.doc-row>span{min-width:0;overflow-wrap:anywhere}.doc-row>span:last-child{flex-shrink:0;max-width:45%}
  .error{color:var(--color-danger);background:var(--color-danger-soft);border:1px solid var(--color-border);border-radius:12px;padding:14px;overflow-wrap:anywhere}.warning{color:var(--color-warning);margin-top:10px}.global-cognition summary{color:var(--color-muted);cursor:pointer}
  @media(max-width:900px){.directory-list{grid-template-columns:1fr}th,td{padding:8px}.file-actions{flex-direction:column}.doc-head{align-items:flex-start}}
</style>
