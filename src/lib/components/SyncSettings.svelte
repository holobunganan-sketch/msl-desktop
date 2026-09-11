<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { open as choose } from '@tauri-apps/plugin-dialog';
  import { command, normalizeError } from '$lib/services/api';
  import { connectionActions, syncStatusLabel, type ConnectMode, type SyncProbeKind } from '$lib/services/sync';
  import { locale } from '$lib/i18n';
  import AppButton from './ui/AppButton.svelte';
  import Modal from './ui/Modal.svelte';
  import StatusLine from './ui/StatusLine.svelte';

  type SyncStatus = {
    config: { directory: string; enabled: boolean; interval_minutes: number; dataset_id: string; generation: string; ai_primary: boolean };
    state: { phase: string; last_success: number; last_error: string; last_uploaded: number; last_applied: number; conflicts: number };
    running: boolean;
  };
  type Probe = { kind: SyncProbeKind; entries: number; dataset_id: string; generation: string };
  type Conflict = { id: string; table_name: string; row_key: string; field: string; local_value: unknown; remote_value: unknown; created_at: number };

  let status = $state<SyncStatus | null>(null);
  let directory = $state('');
  let enabled = $state(true);
  let interval = $state(180);
  let probe = $state<Probe | null>(null);
  let modal = $state(false);
  let busy = $state(false);
  let message = $state('');
  let error = $state('');
  let initialized = false;
  let timer: ReturnType<typeof setInterval> | undefined;
  let conflicts = $state<Conflict[]>([]);
  let resolving = $state('');
  let en = $derived($locale === 'en-US');
  const L = (zh: string, english: string) => en ? english : zh;
  const formatTime = (value: number) => value ? new Date(value * 1000).toLocaleString(en ? 'en-US' : 'zh-CN') : L('尚未同步', 'Not synced yet');

  async function load() {
    try {
      status = await command<SyncStatus>('sync_status');
      conflicts = status.state.conflicts ? await command<Conflict[]>('list_sync_conflicts') : [];
      if (!initialized) {
        directory = status.config.directory;
        enabled = status.config.enabled;
        interval = status.config.interval_minutes;
        initialized = true;
      }
    } catch (value) {
      error = normalizeError(value);
    }
  }

  async function inspect(path = directory) {
    if (!path.trim()) return;
    busy = true; error = ''; message = '';
    try {
      probe = await command<Probe>('probe_sync_folder', { directory: path.trim() });
      modal = true;
    } catch (value) {
      error = normalizeError(value);
    } finally {
      busy = false;
    }
  }

  async function selectDirectory() {
    const path = await choose({ directory: true, multiple: false });
    if (typeof path === 'string') {
      directory = path;
      await inspect(path);
    }
  }

  async function preparePrivate() {
    busy = true; error = '';
    try {
      directory = await command<string>('prepare_private_cloud_folder', { directory });
      await inspect(directory);
    } catch (value) { error = normalizeError(value); }
    finally { busy = false; }
  }

  async function connect(mode: ConnectMode) {
    busy = true; error = '';
    try {
      await command('connect_sync_folder', { directory: directory.trim(), mode });
      modal = false;
      message = L('同步文件夹已连接，第一份数据已经写入或读取。', 'Sync folder connected and the initial data exchange completed.');
      initialized = false;
      await load();
    } catch (value) {
      error = normalizeError(value);
    } finally {
      busy = false;
    }
  }

  async function saveSchedule() {
    busy = true; error = '';
    try {
      await command('save_sync_schedule', { enabled, intervalMinutes: Number(interval) });
      message = L('自动同步周期已保存', 'Automatic sync schedule saved');
      await load();
    } catch (value) { error = normalizeError(value); }
    finally { busy = false; }
  }

  async function syncNow() {
    busy = true; error = '';
    try {
      await command('run_sync_now');
      message = L('已完成本次文件夹数据交换', 'Folder data exchange completed');
      await load();
    } catch (value) { error = normalizeError(value); }
    finally { busy = false; }
  }

  function readable(value: unknown) {
    if (value === null) return L('空', 'Empty');
    if (typeof value === 'string') return value;
    return JSON.stringify(value);
  }

  async function resolve(id: string, choice: 'local' | 'remote') {
    resolving = id; error = '';
    try {
      await command('resolve_sync_conflict', { id, choice });
      message = L('冲突选择已保存，将在下次同步传播到其他设备。', 'The choice was saved and will be shared on the next sync.');
      await load();
    } catch (value) { error = normalizeError(value); }
    finally { resolving = ''; }
  }

  async function makePrimary() {
    busy = true; error = '';
    try {
      await command('make_sync_ai_primary');
      message = L('本机已负责周期简报和自动报告。其他设备将在同步后停止重复生成。', 'This device now runs scheduled briefs and reports. Other devices will stop duplicate generation after syncing.');
      await load();
    } catch (value) { error = normalizeError(value); }
    finally { busy = false; }
  }

  onMount(() => { void load(); timer = setInterval(() => void load(), 3000); });
  onDestroy(() => { if (timer) clearInterval(timer); });
</script>

<section class="sync-settings" data-testid="sync-settings">
  <header>
    <div><h3>{L('日常同步', 'Everyday sync')}</h3><p>{L('连接云盘文件夹，让多台电脑交换最新工作记录。', 'Connect a cloud-drive folder to exchange the latest work records across devices.')}</p></div>
    <span class:active={!!status?.config.dataset_id} class="connection-state">{status ? syncStatusLabel(status, en ? 'en-US' : 'zh-CN') : L('读取状态…', 'Loading…')}</span>
  </header>
  <StatusLine message={error || message || status?.state.last_error || ''} error={!!(error || status?.state.last_error)} />
  {#if error.includes('GIT_PRIVATE_FOLDER_REQUIRED')}<div class="folder-actions"><AppButton variant="secondary" loading={busy} onclick={preparePrivate}>{L('创建私有云盘子文件夹','Create private cloud subfolder')}</AppButton><p>{L('创建 MSLDesktop-private 并排除 Git，现有文件保持原样。请勿强制添加此目录到 Git。','Creates MSLDesktop-private excluded from Git; existing files stay unchanged. Do not force-add this folder.')}</p></div>{/if}

  <div class="folder-row">
    <label>{L('同步文件夹', 'Sync folder')}<input data-testid="sync-directory" bind:value={directory} placeholder={L('选择 WPS、OneDrive 或其他云盘中的文件夹', 'Choose a folder inside your cloud drive')} /></label>
    <div class="folder-actions">
      <AppButton variant="secondary" disabled={busy} onclick={selectDirectory}>{L('选择文件夹', 'Choose folder')}</AppButton>
      <AppButton testid="sync-connect" loading={busy} disabled={!directory.trim()} onclick={() => inspect()}>{status?.config.directory === directory ? L('检查连接', 'Review connection') : L('连接并同步', 'Connect & sync')}</AppButton>
    </div>
  </div>

  <div class="schedule-row">
    <label class="toggle"><input type="checkbox" bind:checked={enabled} />{L('自动同步', 'Automatic sync')}</label>
    <label>{L('间隔（分钟）', 'Interval (minutes)')}<input data-testid="sync-interval" type="number" min="5" max="10080" bind:value={interval} /></label>
    <div class="schedule-actions"><AppButton variant="secondary" loading={busy} onclick={saveSchedule}>{L('保存周期', 'Save schedule')}</AppButton><AppButton loading={busy || status?.running} disabled={!status?.config.dataset_id} onclick={syncNow}>{L('立即同步', 'Sync now')}</AppButton></div>
  </div>

  <div class="sync-summary">
    <div><span>{L('最近成功', 'Last success')}</span><strong>{formatTime(status?.state.last_success ?? 0)}</strong></div>
    <div><span>{L('最近读取', 'Last received')}</span><strong>{status?.state.last_applied ?? 0}</strong></div>
    <div><span>{L('待处理冲突', 'Conflicts')}</span><strong>{status?.state.conflicts ?? 0}</strong></div>
  </div>
  {#if status?.config.dataset_id}
    <div class="ai-owner"><div><strong>{L('自动 AI 分析设备', 'Scheduled AI device')}</strong><span>{status.config.ai_primary ? L('本机负责周期简报和报告，手动分析在所有设备都可使用。', 'This device runs scheduled briefs and reports. Manual analysis remains available everywhere.') : L('根据当前同步信息，由另一台设备负责周期分析。', 'According to the latest local sync information, another device runs scheduled analysis.')}</span><span>{L('切换前请暂停原设备的自动分析，并等待云盘传输完成；离线时无法确认另一台设备的状态。', 'Before switching, pause scheduled analysis on the previous device and wait for cloud transfers. Another device’s offline state cannot be verified.')}</span></div>{#if !status.config.ai_primary}<AppButton variant="secondary" disabled={busy} onclick={makePrimary}>{L('改由本机负责', 'Make this device primary')}</AppButton>{/if}</div>
  {/if}
  {#if conflicts.length}
    <details class="conflicts" open>
      <summary>{L('需要选择的冲突', 'Conflicts requiring a choice')} · {conflicts.length}</summary>
      <div class="conflict-list">
        {#each conflicts as conflict (conflict.id)}
          <article>
            <div class="conflict-title"><strong>{conflict.table_name} · {conflict.field}</strong><span>{new Date(conflict.created_at * 1000).toLocaleString()}</span></div>
            <div class="conflict-options">
              <button disabled={!!resolving} onclick={() => resolve(conflict.id, 'local')}><span>{L('保留本机', 'Keep this device')}</span><strong title={readable(conflict.local_value)}>{readable(conflict.local_value)}</strong></button>
              <button disabled={!!resolving} onclick={() => resolve(conflict.id, 'remote')}><span>{L('采用同步文件夹版本', 'Use synced version')}</span><strong title={readable(conflict.remote_value)}>{readable(conflict.remote_value)}</strong></button>
            </div>
          </article>
        {/each}
      </div>
    </details>
  {/if}
  <p class="boundary">{L('“同步完成”表示数据已安全写入或读取本机的同步文件夹。云盘上传进度请在对应云盘客户端查看。API Key、本机路径、缓存和源工作文件不会进入同步文件夹。', '“Sync completed” means data was safely written to or read from the local sync folder. Check the cloud client for upload progress. API keys, device paths, caches, and source work files stay local.')}</p>
</section>

<Modal bind:open={modal} title={L('连接同步文件夹', 'Connect sync folder')} dismissible={!busy}>
  {#if probe}
    {#if probe.kind === 'empty'}<p>{L('这里还没有工作台数据。确认后会建立可识别的数据集，并写入本机现有数据。', 'No workbench data was found. Confirm to create a recognizable dataset and publish this device’s current data.')}</p>{/if}
    {#if probe.kind === 'unrelated'}<p>{L(`文件夹中已有 ${probe.entries} 个其他项目。工作台只会创建并管理自己的 MSLDesktop.sync 子目录。`, `This folder contains ${probe.entries} other entries. The app will only create and manage its own MSLDesktop.sync subfolder.`)}</p>{/if}
    {#if probe.kind === 'existing'}<p>{L('检测到 MSL Desktop 数据。请选择首次连接的数据起点。完成后进入双向同步。', 'MSL Desktop data was found. Choose the initial source. Bidirectional sync begins after this choice.')}</p>{/if}
    {#if probe.kind === 'incomplete'}<p class="warning">{L('同步数据仍在传输或无法验证。请等待云盘完成后重新检查。', 'Sync data is incomplete or cannot be verified. Wait for the cloud client and check again.')}</p>{/if}
  {/if}
  <p class="path">{directory}</p>
  {#snippet footer()}
    <AppButton variant="secondary" disabled={busy} onclick={() => modal = false}>{L('取消', 'Cancel')}</AppButton>
    {#if probe && connectionActions(probe).includes('initialize_from_local')}<AppButton loading={busy} onclick={() => connect('initialize_from_local')}>{L('建立数据集并上传本机数据', 'Create dataset from this device')}</AppButton>{/if}
    {#if probe && connectionActions(probe).includes('use_folder')}<AppButton variant="secondary" loading={busy} onclick={() => connect('use_folder')}>{L('使用文件夹中的数据', 'Use folder data')}</AppButton>{/if}
    {#if probe && connectionActions(probe).includes('replace_with_local')}<AppButton loading={busy} onclick={() => connect('replace_with_local')}>{L('以本机数据建立新基线', 'Create new baseline from this device')}</AppButton>{/if}
  {/snippet}
</Modal>

<style>
  .sync-settings{display:grid;gap:12px;min-width:0;align-content:start}
  header{display:flex;align-items:flex-start;justify-content:space-between;gap:18px;min-width:0;min-height:0;height:auto;padding:0}h3{margin:0;font-size:16px}p{margin:6px 0 0;color:var(--color-muted);line-height:1.7}.connection-state{flex:0 1 auto;max-width:280px;padding:7px 10px;border:1px solid var(--color-border);border-radius:10px;color:var(--color-muted);background:var(--color-surface-muted);font-size:12px;overflow-wrap:anywhere}.connection-state.active{border-color:#cfdfd9;background:var(--color-success-soft);color:var(--color-success)}
  .folder-row{display:grid;grid-template-columns:minmax(0,1fr) auto;align-items:end;gap:12px}.folder-row label,.schedule-row label{display:grid;gap:8px;min-width:0}.folder-actions,.schedule-actions{display:flex;gap:8px;align-items:center;min-height:40px;flex-wrap:wrap}input:not([type=checkbox]){width:100%;min-width:0;padding:8px 12px}.schedule-row{display:grid;grid-template-columns:minmax(140px,1fr) 180px auto;align-items:end;gap:16px}.toggle{display:flex!important;align-items:center;gap:8px;min-height:40px}.sync-summary{display:grid;grid-template-columns:2fr 1fr 1fr;gap:10px}.sync-summary>div{display:grid;gap:4px;min-height:62px;padding:12px 14px;border:1px solid var(--color-border);border-radius:10px;background:var(--color-surface-muted)}.sync-summary span{font-size:11px;color:var(--color-muted)}.sync-summary strong{font-size:13px;overflow-wrap:anywhere}.boundary{padding:11px 13px;border-left:3px solid #cadbd6;background:var(--color-surface-muted);font-size:12px}.path{overflow-wrap:anywhere;padding:10px;border-radius:9px;background:var(--color-surface-muted)}.warning{color:var(--color-warning)}
  .ai-owner{display:flex;align-items:center;justify-content:space-between;gap:16px;min-height:66px;padding:12px 14px;border:1px solid var(--color-border);border-radius:10px}.ai-owner>div{display:grid;gap:4px;min-width:0}.ai-owner span{color:var(--color-muted);font-size:12px;line-height:1.6}.conflicts{border:1px solid var(--color-warning);border-radius:10px;padding:0 14px}.conflicts>summary{min-height:44px;display:flex;align-items:center;cursor:pointer;font-weight:650}.conflict-list{display:grid;gap:10px;padding:0 0 14px}.conflict-list article{display:grid;gap:9px;padding:12px;border-radius:9px;background:var(--color-surface-muted)}.conflict-title{display:flex;justify-content:space-between;gap:12px}.conflict-title span{color:var(--color-muted);font-size:11px}.conflict-options{display:grid;grid-template-columns:1fr 1fr;gap:8px}.conflict-options button{display:grid;gap:4px;min-width:0;min-height:56px;padding:9px 11px;border:1px solid var(--color-border);border-radius:8px;background:var(--color-surface-raised);text-align:left}.conflict-options button span{color:var(--color-muted);font-size:11px}.conflict-options button strong{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
  .conflicts{border:1px solid var(--color-border);border-radius:11px;background:var(--color-surface)}.conflicts>summary{cursor:pointer;padding:12px 14px;font-weight:650}.conflict-list{display:grid;gap:10px;padding:0 12px 12px}.conflict-list article{display:grid;gap:9px;padding:12px;border-radius:9px;background:var(--color-surface-muted)}.conflict-title{display:flex;justify-content:space-between;gap:12px;font-size:12px}.conflict-title span{color:var(--color-muted)}.conflict-options{display:grid;grid-template-columns:1fr 1fr;gap:8px}.conflict-options button{display:grid;gap:4px;min-width:0;text-align:left;padding:10px 12px;border:1px solid var(--color-border);border-radius:9px;background:var(--color-surface);color:var(--color-text);cursor:pointer}.conflict-options button:hover{border-color:var(--color-primary)}.conflict-options span{font-size:11px;color:var(--color-muted)}.conflict-options strong{white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
  @container(max-width:760px){header,.folder-row,.schedule-row{grid-template-columns:1fr;display:grid}.connection-state{max-width:100%}.sync-summary{grid-template-columns:1fr 1fr}.sync-summary>div:first-child{grid-column:1/-1}.folder-actions,.schedule-actions{justify-content:flex-start}}
</style>
