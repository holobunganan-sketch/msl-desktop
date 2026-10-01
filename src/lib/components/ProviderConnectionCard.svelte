<script lang="ts">
  import AppButton from './ui/AppButton.svelte';
  import type { ProviderConnection, ProviderModel } from '$lib/types/domain';
  import {groupModels, modelAccess, supportedModel, type FoldState, type ProviderTemplate} from '$lib/services/providerCatalog';
  let {connection, models, template, hasKey, folds, locale, busy, onfold, onkey, onrefresh, onedit, onremove, onmodel, ontoggle, ontest}: {
    connection: ProviderConnection; models: ProviderModel[]; template?: ProviderTemplate; hasKey: boolean;
    folds: FoldState; locale: string; busy: string | null;
    onfold: (key: string, open: boolean) => void; onkey: (key: string) => Promise<boolean> | void;
    onrefresh: () => void; onedit: () => void; onremove: () => void; onmodel: (model?: ProviderModel) => void;
    ontoggle: (model: ProviderModel) => void; ontest: (model: ProviderModel) => void;
  } = $props();
  let apiKey = $state('');
  const en = $derived(locale === 'en-US');
  const groups = $derived(groupModels(models));
  async function saveKey() { if (await onkey(apiKey)) apiKey = ''; }
</script>

<article class="connection-card" data-testid={`connection-card-${connection.id}`}>
  <div class="connection-head">
    <div class="connection-name"><strong>{connection.display_name}</strong><span class="tag">{template?.display_name ?? (connection.template_kind === 'custom' ? (en ? 'Custom' : '自定义') : connection.template_kind)}</span><div class="muted endpoint">{connection.base_url}</div></div>
    <div class="actions">
      <AppButton testid={`connection-refresh-${connection.id}`} variant="secondary" loading={busy === `refresh-${connection.id}`} disabled={busy !== null} onclick={onrefresh}>{en ? 'Refresh models' : '刷新模型'}</AppButton>
      <AppButton testid={`connection-edit-${connection.id}`} variant="secondary" disabled={busy !== null} onclick={onedit}>{en ? 'Settings' : '连接设置'}</AppButton>
      <AppButton testid={`connection-delete-${connection.id}`} variant="ghost" disabled={busy !== null} onclick={onremove}>{en ? 'Delete' : '删除'}</AppButton>
    </div>
  </div>
  {#if template?.note}<div class:plan-note={template.service_tier === 'plan'} class="provider-note"><span>{template.note}</span>{#if template.docs_url}<a href={template.docs_url} target="_blank" rel="noreferrer">{en ? 'Provider documentation' : '官方说明'} ↗</a>{/if}</div>{/if}
  <div class="connection-key">
    <label for={`key-${connection.id}`}>{en ? 'Replace API key' : '更换 API Key'}<input id={`key-${connection.id}`} data-testid={`connection-${connection.id}-api-key`} type="password" bind:value={apiKey} autocomplete="new-password" placeholder={en ? 'Enter a new key' : '输入新 Key'} /></label>
    <AppButton variant="secondary" disabled={!apiKey.trim() || busy !== null} loading={busy === `key-${connection.id}`} onclick={saveKey}>{en ? 'Save key' : '保存 Key'}</AppButton>
    <span class="key-state" class:key-ok={hasKey}>{hasKey ? (en ? 'Key configured' : '已配置 Key') : (en ? 'No key configured' : '未配置 Key')}</span>
  </div>
  <details class="model-list" open={folds[`models:${connection.id}`] ?? false} ontoggle={event => onfold(`models:${connection.id}`, event.currentTarget.open)}>
    <summary data-testid={`models-fold-${connection.id}`}>{en ? 'Models' : '模型'} · {models.length} <span class="muted">{en ? 'Enabled' : '启用'} {models.filter(model => model.enabled).length}</span></summary>
    <div class="models-toolbar"><span class="muted">{en ? 'A catalog entry does not confirm successful model access.' : '目录条目仅表示发现记录；实际调用权限以连接测试为准。'}</span><AppButton testid={`model-add-${connection.id}`} variant="secondary" disabled={busy !== null} onclick={() => onmodel()}>{en ? 'Add model' : '手动添加模型'}</AppButton></div>
    {#if models.length === 0}<p class="empty">{en ? 'Refresh the catalog or add a model with its protocol and endpoint.' : '刷新目录，或手动填写模型 ID、协议与端点。'}</p>{/if}
    {#each groups as group (group.protocol)}
      <details class="protocol-group" open={folds[`protocol:${connection.id}:${group.protocol}`] ?? false} ontoggle={event => onfold(`protocol:${connection.id}:${group.protocol}`, event.currentTarget.open)}>
        <summary data-testid={`protocol-fold-${connection.id}-${group.protocol}`}>{group.protocol === 'unknown' ? (en ? 'Unknown / unsupported' : '未知 / 不支持的协议') : group.label} <span class="count">{group.models.length}</span></summary>
        {#each group.models as model (model.id)}
          <div class="model-row" data-testid={`model-row-${model.id}`}>
            <div class="model-info"><strong>{model.display_name}</strong><div class="muted">{model.model_id} · {model.endpoint_path || '—'}</div><div class="model-access" class:attention={!supportedModel(model)}>{modelAccess(model, locale)}</div>{#if model.model_id.toLowerCase().includes('contributor')}<p class="model-note">{en ? 'Contributor model: prompts and completions may be used to train future models.' : 'Contributor 模型：提示词与补全结果可能用于训练未来模型。'}</p>{/if}</div>
            <div class="model-actions"><span class="state">{model.enabled ? (en ? 'Enabled' : '已启用') : (en ? 'Disabled' : '已停用')}</span><AppButton testid={`model-toggle-${model.id}`} variant="secondary" loading={busy === `toggle-${model.id}`} disabled={busy !== null || (!model.enabled && !supportedModel(model))} onclick={() => ontoggle(model)}>{model.enabled ? (en ? 'Disable' : '停用') : (en ? 'Enable' : '启用')}</AppButton><AppButton testid={`model-edit-${model.id}`} variant="secondary" disabled={busy !== null} onclick={() => onmodel(model)}>{en ? 'Edit' : '编辑'}</AppButton><AppButton testid={`model-test-${model.id}`} variant="ghost" loading={busy === `test-${model.id}`} disabled={busy !== null || !model.enabled || !supportedModel(model)} onclick={() => ontest(model)}>{en ? 'Test' : '测试'}</AppButton></div>
          </div>
        {/each}
      </details>
    {/each}
  </details>
</article>

<style>
  .connection-card{min-width:0;padding:16px;border:1px solid var(--color-border);border-radius:var(--radius-md);background:var(--color-surface)}
  .connection-head,.actions,.models-toolbar{display:flex;align-items:center;justify-content:space-between;gap:10px}.connection-head{align-items:flex-start}.connection-name,.model-info{min-width:0;overflow-wrap:anywhere}.connection-name strong{font-size:16px}.actions{flex-wrap:wrap;justify-content:flex-end;flex-shrink:0}.tag{display:inline-block;margin-left:8px;padding:3px 8px;background:var(--color-surface-muted);border-radius:999px;font-size:12px;color:var(--color-muted)}
  .muted,.key-state,.state{font-size:13px;color:var(--color-muted);line-height:1.5}.endpoint{margin-top:5px;overflow-wrap:anywhere}.provider-note{display:flex;flex-wrap:wrap;gap:4px 12px;margin:12px 0;padding:10px 12px;background:var(--color-surface-muted);border-radius:8px;font-size:13px;line-height:1.55;overflow-wrap:anywhere}.provider-note.plan-note{border-left:3px solid var(--color-warning)}a{color:var(--color-primary);text-underline-offset:3px}.connection-key{display:grid;grid-template-columns:minmax(160px,1fr) auto auto;align-items:end;gap:10px;margin-top:12px}.connection-key label{display:grid;gap:5px;font-size:13px;color:var(--color-muted)}input{min-width:0;min-height:40px;padding:8px 10px;border:1px solid var(--color-border);border-radius:var(--radius-sm);color:var(--color-text);background:var(--color-surface);font-size:14px}.key-state{align-self:center;padding-top:20px}.key-ok{color:var(--color-success)}
  .model-list{margin-top:12px;border-top:1px solid var(--color-border)}summary{min-height:44px;padding:12px 2px;cursor:pointer;font-size:14px;font-weight:550;user-select:none}.model-list>summary .muted{margin-left:10px;font-weight:400}.models-toolbar{align-items:flex-start;margin:2px 0 12px}.models-toolbar>span{max-width:65%}.protocol-group{border-top:1px solid var(--color-border)}.count{color:var(--color-muted);margin-left:8px;font-weight:400}.model-row{display:grid;grid-template-columns:minmax(0,1fr) auto;gap:12px;align-items:center;padding:12px 0;border-top:1px solid var(--color-border)}.model-info strong{font-size:14px}.model-access{font-size:12px;color:var(--color-muted);margin-top:3px}.attention{color:var(--color-warning)}.model-actions{display:flex;align-items:center;gap:6px;flex-wrap:wrap;justify-content:flex-end}.state{min-width:54px;text-align:center}.model-note{font-size:12px;line-height:1.5;color:var(--color-warning);margin:5px 0 0}.empty{font-size:14px;line-height:1.5;color:var(--color-muted);margin:10px 0}
  .model-actions :global(.app-button){min-width:84px}.model-actions .state{width:72px;flex:0 0 72px}
  @media(max-width:1150px){.connection-head{flex-wrap:wrap}.model-row{grid-template-columns:1fr}.model-actions{justify-content:flex-start}.state{text-align:left}.actions{justify-content:flex-start}}
  @media(max-width:650px){.connection-card{padding:12px}.connection-key{grid-template-columns:minmax(0,1fr) auto}.key-state{grid-column:1/-1;padding:0}.models-toolbar{flex-direction:column}.models-toolbar>span{max-width:100%}}
</style>
