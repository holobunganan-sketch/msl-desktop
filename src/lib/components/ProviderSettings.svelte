<script lang="ts">
  import {onMount, tick} from 'svelte';
  import Modal from './ui/Modal.svelte';
  import AppButton from './ui/AppButton.svelte';
  import ProviderConnectionCard from './ProviderConnectionCard.svelte';
  import {command, normalizeError} from '$lib/services/api';
  import type {ProviderConnection, ProviderModel} from '$lib/types/domain';
  import {locale} from '$lib/i18n';
  import {addToast} from '$lib/stores/toast';
  import {connectionPayload, connectionVendor, createConnection, groupConnections, modelSavePayload, protocols, readFolds, saveFold, supportedModel, type FoldState, type ModelForm, type ProviderTemplate, type RefreshResult} from '$lib/services/providerCatalog';

  let connections = $state<ProviderConnection[]>([]);
  let templates = $state<ProviderTemplate[]>([]);
  let models = $state<Record<number, ProviderModel[]>>({});
  let keyStatus = $state<Record<number, boolean>>({});
  let folds = $state<FoldState>({});
  let loading = $state(false);
  let busy = $state<string | null>(null);
  let error = $state('');
  let showError = $state(false);
  let formOpen = $state(false);
  let editing = $state<ProviderConnection | null>(null);
  let vendor = $state('custom');
  let kind = $state('custom');
  let displayName = $state('');
  let apiKey = $state('');
  let baseUrl = $state('');
  let authMode = $state('bearer');
  let modelsEndpoint = $state('');
  let connectionEnabled = $state(true);
  let formError = $state('');
  let modelOpen = $state(false);
  let modelConnection = $state<ProviderConnection | null>(null);
  let editingModel = $state<ProviderModel | undefined>(undefined);
  let modelForm = $state<ModelForm>({modelId:'', displayName:'', protocol:'', endpointPath:'', enabled:false});
  let modelError = $state('');
  let removal = $state<ProviderConnection | null>(null);
  let removeOpen = $state(false);
  let area: HTMLElement;
  const en = $derived($locale === 'en-US');
  const groups = $derived(groupConnections(connections, templates));
  const vendors = $derived([...new Set(templates.map(template => template.vendor))]);
  const vendorTemplates = $derived(templates.filter(template => template.vendor === vendor));
  const selectedTemplate = $derived(templates.find(template => template.kind === kind));
  const formTemplate = $derived(editing ? templates.find(template => template.kind === editing?.template_kind) : selectedTemplate);

  function fold(key: string, open: boolean) {
    if (folds[key] === open) return;
    folds = saveFold(localStorage, folds, key, open);
  }

  async function load() {
    loading = true;
    error = '';
    try {
      const [nextConnections, nextTemplates] = await Promise.all([
        command<ProviderConnection[]>('list_provider_connections'),
        command<ProviderTemplate[]>('list_provider_templates')
      ]);
      connections = nextConnections;
      templates = nextTemplates;
      await Promise.all(connections.map(connection => reloadConnection(connection.id)));
    } catch (value) { error = normalizeError(value); }
    finally { loading = false; }
  }

  async function reloadConnection(id: number) {
    const [nextModels, hasKey] = await Promise.all([
      command<ProviderModel[]>('list_provider_models', {providerId:id}), command<boolean>('provider_has_key', {id})
    ]);
    models = {...models, [id]:nextModels};
    keyStatus = {...keyStatus, [id]:hasKey};
  }

  // Keyed cards remain mounted; preserve the actual scroll container during local updates.
  async function retainScroll<T>(operation: () => Promise<T>): Promise<T> {
    const positions: {element: HTMLElement; top: number; left: number}[] = [];
    for (let element: HTMLElement | null = area; element; element = element.parentElement) {
      if (element.scrollHeight > element.clientHeight) positions.push({element, top:element.scrollTop, left:element.scrollLeft});
    }
    const windowPosition = {top:window.scrollY, left:window.scrollX};
    try { return await operation(); }
    finally {
      await tick();
      for (const position of positions) position.element.scrollTo({top:position.top, left:position.left, behavior:'instant'});
      window.scrollTo({...windowPosition, behavior:'instant'});
    }
  }

  async function perform(label: string, operation: () => Promise<void>, success?: string) {
    busy = label; error = '';
    try { await retainScroll(operation); if (success) addToast(success, 'success'); return true; }
    catch (value) { error = normalizeError(value); addToast(error, 'error'); return false; }
    finally { busy = null; }
  }

  function applyTemplate() {
    const template = templates.find(item => item.kind === kind);
    displayName = template?.display_name ?? '';
    baseUrl = template?.base_url ?? '';
    modelsEndpoint = template?.models_endpoint ?? '';
    authMode = template?.auth_mode ?? 'bearer';
  }
  function changeVendor() { kind = templates.find(template => template.vendor === vendor)?.kind ?? 'custom'; applyTemplate(); }
  function addConnection() {
    editing = null; vendor = vendors[0] ?? 'custom'; kind = templates.find(template => template.vendor === vendor)?.kind ?? 'custom';
    apiKey = ''; formError = ''; connectionEnabled = true; applyTemplate(); formOpen = true;
  }
  function editConnection(connection: ProviderConnection) {
    editing = connection; vendor = connectionVendor(connection, templates); kind = connection.template_kind;
    displayName = connection.display_name; baseUrl = connection.base_url; modelsEndpoint = connection.models_endpoint ?? '';
    authMode = connection.auth_mode; connectionEnabled = connection.enabled; apiKey = ''; formError = ''; formOpen = true;
  }
  function upsertConnection(connection: ProviderConnection) {
    connections = connections.some(item => item.id === connection.id)
      ? connections.map(item => item.id === connection.id ? connection : item) : [...connections, connection];
  }

  async function saveConnection() {
    if (!displayName.trim() || !baseUrl.trim()) { formError = en ? 'Nickname and base URL are required.' : '请填写连接昵称和基础地址。'; return; }
    if (!editing && selectedTemplate?.configurable === false) { formError = en ? 'This plan does not permit this application. See the provider documentation.' : '此套餐限制当前应用接入，请查看官方说明。'; return; }
    if (!editing && authMode !== 'none' && !apiKey.trim()) { formError = en ? 'Enter an API key.' : '请填写 API Key。'; return; }
    const saved = await perform('connection-save', async () => {
      let connection: ProviderConnection;
      if (!editing && selectedTemplate) {
        connection = await createConnection(command, {template:selectedTemplate, displayName, apiKey});
        upsertConnection(connection);
        editing = connection;
        if (baseUrl.trim() !== connection.base_url || authMode !== connection.auth_mode || (modelsEndpoint.trim() || null) !== connection.models_endpoint) {
          connection = await command<ProviderConnection>('save_provider_connection', {...connectionPayload(connection), baseUrl:baseUrl.trim(), authMode, modelsEndpoint:modelsEndpoint.trim() || null});
        }
      } else {
        connection = await command<ProviderConnection>('save_provider_connection', {
          ...(editing ? connectionPayload(editing) : {id:null, providerType:'custom', legacyModel:'', templateKind:'custom'}),
          displayName:displayName.trim(), baseUrl:baseUrl.trim(), authMode, modelsEndpoint:modelsEndpoint.trim() || null,
          enabled:connectionEnabled, apiKey
        });
      }
      upsertConnection(connection);
      // A catalog-read failure must retry the connection already created above.
      editing = connection;
      await reloadConnection(connection.id);
    }, en ? 'Connection saved' : '连接已保存');
    if (saved) { formOpen = false; apiKey = ''; }
    else formError = error;
  }

  async function saveKey(connection: ProviderConnection, key: string): Promise<boolean> {
    if (!key.trim()) return false;
    return perform(`key-${connection.id}`, async () => {
      const saved = await command<ProviderConnection>('save_provider_connection', connectionPayload(connection, key));
      upsertConnection(saved); keyStatus = {...keyStatus, [connection.id]:true};
    }, en ? 'API key saved' : 'API Key 已保存');
  }

  async function refresh(connection: ProviderConnection) {
    await perform(`refresh-${connection.id}`, async () => {
      const result = await command<RefreshResult>('refresh_provider_models', {providerId:connection.id});
      models = {...models, [connection.id]:await command<ProviderModel[]>('list_provider_models', {providerId:connection.id})};
      const message = result.source === 'curated'
        ? (en ? `Catalog updated · ${result.received} entries; account access unverified` : `已更新内置目录 · ${result.received} 个模型；账户权限未验证`)
        : (en ? `Remote catalog: ${result.received} entries · ${result.unknown} need protocol configuration` : `远程目录：${result.received} 个模型 · ${result.unknown} 个待配置协议`);
      addToast(result.message ? `${message} · ${result.message}` : message, 'success');
    });
  }

  async function toggleModel(model: ProviderModel) {
    if (!model.enabled && !supportedModel(model)) return;
    await perform(`toggle-${model.id}`, async () => {
      await command('set_provider_model_enabled', {id:model.id, enabled:!model.enabled});
      models = {...models, [model.provider_id]:models[model.provider_id].map(item => item.id === model.id ? {...item, enabled:!model.enabled} : item)};
    }, model.enabled ? (en ? 'Model disabled' : '模型已停用') : (en ? 'Model enabled' : '模型已启用'));
  }

  async function testModel(model: ProviderModel) {
    await perform(`test-${model.id}`, async () => { await command('test_provider_model', {providerModelId:model.id}); }, en ? 'Connection test succeeded' : '连接测试成功');
  }
  function editModel(connection: ProviderConnection, model?: ProviderModel) {
    modelConnection = connection; editingModel = model;
    modelForm = {modelId:model?.model_id ?? '', displayName:model?.display_name ?? '', protocol:model && supportedModel(model) ? model.protocol : '', endpointPath:model?.endpoint_path ?? '', enabled:model && supportedModel(model) ? model.enabled : false};
    modelError = ''; modelOpen = true;
  }
  function changeProtocol() { modelForm.endpointPath = protocols.find(protocol => protocol.id === modelForm.protocol)?.endpoint ?? ''; }
  async function saveModel() {
    if (!modelConnection) return;
    let payload: ReturnType<typeof modelSavePayload>;
    try { payload = modelSavePayload(modelConnection.id, modelForm, editingModel); }
    catch { modelError = en ? 'Enter a model ID and name, choose a protocol, and use an endpoint beginning with / without ..' : '请填写模型 ID 和名称、选择协议；端点需以 / 开头且不能包含 ..。'; return; }
    const saved = await perform('model-save', async () => {
      const model = await command<ProviderModel>('save_provider_model', payload);
      const current = models[model.provider_id] ?? [];
      models = {...models, [model.provider_id]:current.some(item => item.id === model.id) ? current.map(item => item.id === model.id ? model : item) : [...current, model]};
    }, en ? 'Model saved' : '模型已保存');
    if (saved) modelOpen = false; else modelError = error;
  }
  async function removeConnection() {
    if (!removal) return;
    const id = removal.id;
    if (await perform(`delete-${id}`, async () => {
      await command('delete_provider', {id}); connections = connections.filter(connection => connection.id !== id);
      const nextModels = {...models}; delete nextModels[id]; models = nextModels;
      const nextKeys = {...keyStatus}; delete nextKeys[id]; keyStatus = nextKeys;
    }, en ? 'Connection deleted' : '连接已删除')) { removeOpen = false; removal = null; }
  }

  onMount(() => { folds = readFolds(localStorage); void load(); });
</script>

<section class="provider-area" bind:this={area}>
  <div class="section-head"><div><h2>{en ? 'Model connections' : '模型连接'}</h2><p>{en ? 'Manage separate accounts and model access by provider. Keys stay in Windows Credential Manager.' : '按厂商管理独立账户与模型。API Key 保存在 Windows 凭据管理器。'}</p></div><AppButton testid="add-provider-connection" disabled={loading || busy !== null} onclick={addConnection}>{en ? 'Add connection' : '添加连接'}</AppButton></div>
  {#if error || loading || connections.length === 0}<div class="provider-status" aria-live="polite">{#if error}<button class="error error-summary" onclick={() => showError = true} title={error}>{error}</button>{:else if loading}<span>{en ? 'Loading connections…' : '正在加载连接…'}</span>{:else}<span>{en ? 'No connections yet. Add a provider to get started.' : '尚未添加连接。选择厂商后即可配置。'}</span>{/if}</div>{/if}
  {#each groups as group (group.vendor)}
    <details class="vendor-folder" open={folds[`vendor:${group.vendor}`] ?? false} ontoggle={event => fold(`vendor:${group.vendor}`, event.currentTarget.open)}>
      <summary data-testid={`vendor-fold-${group.vendor}`}><span>{group.vendor === 'custom' ? (en ? 'Custom providers' : '自定义厂商') : group.vendor}</span><span class="vendor-count">{group.connections.length} {en ? 'connections' : '个连接'}</span></summary>
      <div class="connection-list">{#each group.connections as connection (connection.id)}<ProviderConnectionCard {connection} models={models[connection.id] ?? []} template={templates.find(template => template.kind === connection.template_kind)} hasKey={keyStatus[connection.id] ?? false} {folds} locale={$locale} {busy} onfold={fold} onkey={key => saveKey(connection, key)} onrefresh={() => refresh(connection)} onedit={() => editConnection(connection)} onremove={() => {removal = connection; removeOpen = true;}} onmodel={model => editModel(connection, model)} ontoggle={toggleModel} ontest={testModel} />{/each}</div>
    </details>
  {/each}
</section>

<Modal bind:open={formOpen} title={editing ? (en ? 'Connection settings' : '连接设置') : (en ? 'Add connection' : '添加连接')} dismissible={busy !== 'connection-save'} onclose={() => apiKey = ''}>
  <form onsubmit={event => {event.preventDefault(); void saveConnection();}}>
    {#if !editing}<div class="form-grid"><label>{en ? 'Provider' : '厂商'}<select data-testid="provider-vendor-select" bind:value={vendor} onchange={changeVendor}>{#each vendors as item}<option value={item}>{item}</option>{/each}<option value="custom">{en ? 'Custom' : '自定义'}</option></select></label><label>{en ? 'Access mode' : '接入方式'}<select data-testid="provider-access-select" bind:value={kind} onchange={applyTemplate} disabled={vendor === 'custom'}>{#if vendor === 'custom'}<option value="custom">{en ? 'Custom API' : '自定义 API'}</option>{:else}{#each vendorTemplates as template}<option value={template.kind}>{template.service_tier === 'plan' ? (en ? 'Plan' : '套餐') : 'API'} · {template.display_name}</option>{/each}{/if}</select></label></div>{/if}
    {#if formTemplate?.note}<div class:restricted={formTemplate.service_tier === 'plan'} class="provider-note"><p>{formTemplate.note}</p>{#if formTemplate.docs_url}<a href={formTemplate.docs_url} target="_blank" rel="noreferrer">{en ? 'Read provider requirements' : '查看官方使用要求'} ↗</a>{/if}{#if !editing && formTemplate.configurable === false}<strong>{en ? 'This plan does not allow this application.' : '此套餐不允许当前应用接入。'}</strong>{/if}</div>{/if}
    <label>{en ? 'Connection nickname' : '连接昵称'}<input data-testid="provider-connection-name" bind:value={displayName} required placeholder={en ? 'e.g. Research account' : '例如：研究专用账户'} /></label>
    <label>{editing ? (en ? 'New API key (leave blank to keep current)' : '新 API Key（留空保留现有 Key）') : 'API Key'}<input data-testid="provider-connection-api-key" type="password" bind:value={apiKey} autocomplete="new-password" /></label>
    <details class="endpoint-settings" open={kind === 'custom' || !!editing}><summary>{en ? 'Endpoint and authentication' : '端点与认证设置'}</summary><label>{en ? 'Base URL' : '基础地址'}<input data-testid="provider-base-url" type="url" bind:value={baseUrl} required placeholder="https://example.com/v1" /></label><label>{en ? 'Model discovery URL (optional)' : '模型发现地址（可选）'}<input data-testid="provider-models-endpoint" bind:value={modelsEndpoint} placeholder="https://example.com/v1/models" /></label><label>{en ? 'Authentication' : '认证方式'}<select data-testid="provider-auth-mode" bind:value={authMode}><option value="bearer">Bearer</option><option value="api_key">API Key</option><option value="none">{en ? 'None' : '无需认证'}</option></select></label><p>{en ? 'Model protocol and request endpoint can be edited inside each connection’s model list.' : '在连接内展开模型，可编辑各模型的协议和请求端点。'}</p>{#if selectedTemplate?.protocol}<p>{en ? 'Preset protocol: ' : '预设协议：'}{protocols.find(protocol => protocol.id === selectedTemplate.protocol)?.label ?? selectedTemplate.protocol} · {selectedTemplate.endpoint_path}</p>{/if}</details>
    {#if editing}<label class="check"><input type="checkbox" bind:checked={connectionEnabled} />{en ? 'Enable connection' : '启用连接'}</label>{/if}
    {#if formError}<p class="error" role="alert">{formError}</p>{/if}
    <div class="form-actions"><AppButton type="submit" testid="provider-save-connection" loading={busy === 'connection-save'} disabled={!editing && selectedTemplate?.configurable === false}>{en ? 'Save connection' : '保存连接'}</AppButton><AppButton variant="ghost" disabled={busy !== null} onclick={() => {formOpen = false; apiKey = '';}}>{en ? 'Cancel' : '取消'}</AppButton></div>
  </form>
</Modal>

<Modal bind:open={modelOpen} title={editingModel ? (en ? 'Edit model' : '编辑模型') : (en ? 'Add model' : '手动添加模型')} dismissible={busy !== 'model-save'}>
  <form onsubmit={event => {event.preventDefault(); void saveModel();}}>
    <p class="form-intro">{modelConnection?.display_name} · {en ? 'Confirm the protocol and endpoint in the provider documentation.' : '请根据厂商文档确认协议与端点。'}</p>
    {#if editingModel && !editingModel.available}<p>{en ? 'Saving restores this as a manually configured model. Test the connection to confirm actual access.' : '保存后将恢复为手动配置，实际调用权限可通过测试确认。'}</p>{/if}
    <label>{en ? 'Model ID' : '模型 ID'}<input data-testid="provider-model-id" bind:value={modelForm.modelId} readonly={!!editingModel} required /></label>
    <label>{en ? 'Display name' : '显示名称'}<input data-testid="provider-model-name" bind:value={modelForm.displayName} required /></label>
    <div class="form-grid"><label>{en ? 'Protocol' : '协议'}<select data-testid="provider-model-protocol" bind:value={modelForm.protocol} onchange={changeProtocol} required><option value="" disabled>{en ? 'Choose protocol' : '请选择协议'}</option>{#each protocols as protocol}<option value={protocol.id}>{protocol.label}</option>{/each}</select></label><label>{en ? 'Endpoint path' : '请求端点'}<input data-testid="provider-model-endpoint" bind:value={modelForm.endpointPath} required placeholder="/responses" /></label></div>
    <label class="check"><input data-testid="provider-model-enabled" type="checkbox" bind:checked={modelForm.enabled} />{en ? 'Enable this model' : '启用此模型'}</label>
    {#if modelError}<p class="error" role="alert">{modelError}</p>{/if}
    <div class="form-actions"><AppButton testid="provider-save-model" type="submit" loading={busy === 'model-save'}>{en ? 'Save model' : '保存模型'}</AppButton><AppButton variant="ghost" disabled={busy !== null} onclick={() => modelOpen = false}>{en ? 'Cancel' : '取消'}</AppButton></div>
  </form>
</Modal>
<Modal bind:open={removeOpen} title={en ? 'Delete connection' : '删除连接'} dismissible={!busy?.startsWith('delete-')}><p>{en ? `Delete “${removal?.display_name ?? ''}” and its models? Routes using this connection will need another model.` : `确定删除“${removal?.display_name ?? ''}”及其模型？使用此连接的任务路由需要重新选择模型。`}</p><div class="form-actions"><AppButton testid="provider-confirm-delete" variant="danger" loading={busy?.startsWith('delete-') ?? false} onclick={removeConnection}>{en ? 'Delete connection' : '删除连接'}</AppButton><AppButton variant="ghost" disabled={busy !== null} onclick={() => removeOpen = false}>{en ? 'Cancel' : '取消'}</AppButton></div></Modal>
<Modal bind:open={showError} title={en ? 'Error details' : '错误详情'}><p class="error-detail">{error}</p></Modal>

<style>
  .provider-area{display:grid;gap:12px;min-width:0}.section-head{display:flex;justify-content:space-between;align-items:flex-start;gap:16px}.section-head>div{min-width:0}h2{margin:0 0 5px;font-size:18px}p{margin:0;color:var(--color-muted);font-size:14px;line-height:1.55;overflow-wrap:anywhere}.provider-status{min-width:0;font-size:14px;color:var(--color-muted)}.vendor-folder{min-width:0;border:1px solid var(--color-border);border-radius:var(--radius-md);background:var(--color-surface-muted)}summary{cursor:pointer;user-select:none;min-height:44px;font-size:14px}.vendor-folder>summary{padding:15px 16px;font-size:16px;font-weight:550}.vendor-count{margin-left:14px;color:var(--color-muted);font-size:13px;font-weight:400}.connection-list{display:grid;gap:12px;padding:0 12px 12px;min-width:0}
  form{display:grid;gap:14px}.form-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:12px}label{display:grid;gap:6px;color:var(--color-muted);font-size:14px;min-width:0}input,select{width:100%;min-width:0;min-height:42px;border:1px solid var(--color-border);border-radius:var(--radius-sm);padding:8px 10px;color:var(--color-text);background:var(--color-surface);font-size:14px;box-sizing:border-box}.check{display:flex;align-items:center;gap:8px}.check input{width:18px;min-height:18px}.form-actions{display:flex;flex-wrap:wrap;gap:8px;margin-top:4px}.endpoint-settings{display:grid;border-top:1px solid var(--color-border);border-bottom:1px solid var(--color-border);padding-bottom:4px}.endpoint-settings summary{padding:12px 0}.endpoint-settings label,.endpoint-settings p{margin-bottom:12px}.provider-note{padding:12px;border-radius:var(--radius-sm);background:var(--color-surface-muted);display:grid;gap:8px}.provider-note.restricted{border-left:3px solid var(--color-warning)}.provider-note strong{font-size:14px}.provider-note a{color:var(--color-primary);font-size:14px;text-underline-offset:3px}.error{border:1px solid var(--color-border);border-radius:var(--radius-sm);padding:9px 11px;background:var(--color-danger-soft);color:var(--color-danger);font-size:14px}.error-summary{max-width:100%;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;cursor:pointer}.error-detail{white-space:pre-wrap;overflow-wrap:anywhere}.form-intro{font-size:13px}
  @media(max-width:650px){.section-head{flex-wrap:wrap}.form-grid{grid-template-columns:1fr}.connection-list{padding:0 8px 8px}.vendor-folder>summary{padding:13px 12px}}
</style>
