<script lang="ts">
  import type {ModelRouteOption, ModelRoutingGuidance} from '$lib/types/domain';
  import {protocols, routePreview, routeAuthentication} from '$lib/services/providerCatalog';
  let {routing, protocol, endpointPath, baseUrl, authMode, locale, onselect}: {
    routing: ModelRoutingGuidance; protocol: string; endpointPath: string; baseUrl: string;
    authMode: string; locale: string; onselect: (option: ModelRouteOption) => void;
  } = $props();
  const en = $derived(locale === 'en-US');
  const authentication = $derived(routeAuthentication(baseUrl,endpointPath,protocol,authMode));
</script>

<section class="routing-guide" aria-label={en ? 'OpenCode protocol routing' : 'OpenCode 协议分流'} data-testid="model-routing-guide">
  <div><strong>{en ? 'Choose a request route' : '选择接入入口'}</strong><p>{en ? 'These routes share the current connection’s key. Only the selected route is used; requests are not sent to all routes.' : '各入口共用当前连接的 Key，只使用选定入口，不会向多个入口重复请求。'}</p></div>
  <div class="route-options">
    {#each routing.options as option (option.protocol)}
      <button type="button" data-testid={`model-route-${option.protocol}`} aria-pressed={protocol === option.protocol && endpointPath === option.endpoint_path} onclick={() => onselect(option)}>
        <span>{protocols.find(item => item.id === option.protocol)?.label ?? option.protocol}</span>
        <small>{option.recommended ? (en ? 'Recommended' : '推荐入口') : (en ? 'Compatibility route' : '兼容入口')}</small>
      </button>
    {/each}
  </div>
  <p class="routing-note">{routing.note}</p>
  <div class="route-preview"><span>{en ? 'Request URL' : '请求地址'}</span><code data-testid="model-route-url">{routePreview(baseUrl,endpointPath,protocol)}</code><span>{en ? 'Authentication' : '认证方式'}</span><strong>{authentication === 'none' ? (en ? 'No authentication' : '无需认证') : authentication}</strong></div>
  <p>{en ? 'Manual choices are retained after refresh. Optional routes may vary with upstream service support.' : '手动选择会在刷新后保留。兼容入口的可用情况取决于上游服务。'}</p>
</section>

<style>
  .routing-guide{min-width:0;display:grid;gap:10px;padding:14px;background:var(--color-surface-muted);border:1px solid var(--color-border);border-radius:var(--radius-sm)}
  strong{font-size:14px}p{margin:4px 0 0;font-size:13px;line-height:1.55;color:var(--color-muted);overflow-wrap:anywhere}
  .route-options{display:grid;grid-template-columns:repeat(auto-fit,minmax(140px,1fr));gap:8px}button{min-width:0;min-height:60px;display:grid;gap:4px;align-content:center;text-align:left;padding:10px;border:1px solid var(--color-border);border-radius:var(--radius-sm);background:var(--color-surface);color:var(--color-text);font-size:13px;cursor:pointer;overflow-wrap:anywhere}button[aria-pressed=true]{border-color:var(--color-primary);background:var(--color-primary-soft);color:var(--color-primary)}button:focus-visible{outline:2px solid var(--color-primary);outline-offset:2px}small{font-size:12px;color:var(--color-muted)}
  .route-preview{min-width:0;display:grid;grid-template-columns:auto minmax(0,1fr);gap:5px 12px;font-size:12px;line-height:1.5}.route-preview>span{color:var(--color-muted)}code{font-size:12px;overflow-wrap:anywhere;white-space:normal}.route-preview strong{font-size:12px}
</style>
