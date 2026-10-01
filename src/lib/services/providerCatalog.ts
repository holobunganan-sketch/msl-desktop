import type { ModelRouteOption, ProviderConnection, ProviderModel } from '../types/domain';

export type ProviderTemplate = {
  kind: string; vendor: string; display_name: string; service_tier: 'api' | 'plan';
  base_url: string; models_endpoint: string | null; auth_mode: string;
  protocol: string | null; endpoint_path: string | null; discovery: 'remote' | 'curated';
  docs_url: string; note: string; configurable?: boolean;
  models: { model_id: string; protocol: string; endpoint_path: string }[];
};
export type RefreshResult = {received: number; available: number; unknown: number; source?: 'remote' | 'curated'; message?: string};
export type FoldState = Record<string, boolean>;
type FoldStorage = Pick<Storage, 'getItem' | 'setItem'>;
export const protocols = [
  {id: 'responses', label: 'Responses', endpoint: '/responses'},
  {id: 'chat_completions', label: 'Chat Completions', endpoint: '/chat/completions'},
  {id: 'anthropic_messages', label: 'Anthropic Messages', endpoint: '/messages'}
] as const;

export function connectionVendor(connection: ProviderConnection, templates: ProviderTemplate[]): string {
  return templates.find(item => item.kind === connection.template_kind)?.vendor
    ?? (connection.template_kind.startsWith('opencode') ? 'OpenCode'
      : connection.template_kind.startsWith('deepseek') ? 'DeepSeek'
      : connection.template_kind === 'custom' ? 'custom' : connection.provider_type);
}

export function groupConnections(connections: ProviderConnection[], templates: ProviderTemplate[]) {
  const groups = new Map<string, ProviderConnection[]>();
  for (const connection of connections) {
    const vendor = connectionVendor(connection, templates);
    groups.set(vendor, [...(groups.get(vendor) ?? []), connection]);
  }
  return [...groups].map(([vendor, connections]) => ({vendor, connections}));
}

function capabilities(model?: ProviderModel): Record<string, unknown> {
  try {
    const parsed = JSON.parse(model?.capabilities_json ?? '{}');
    return parsed && typeof parsed === 'object' && !Array.isArray(parsed) ? parsed : {};
  } catch { return {}; }
}

export function supportedModel(model: ProviderModel) {
  const metadata = capabilities(model);
  return protocols.some(item => item.id === model.protocol) && !metadata.needs_protocol && !metadata.unsupported && !metadata.unsupported_protocol;
}

export function groupModels(models: ProviderModel[]) {
  return [...protocols.map(item => ({protocol: item.id as string, label: item.label as string})), {protocol: 'unknown', label: 'Unknown / unsupported'}]
    .map(group => ({...group, models: models.filter(model => (supportedModel(model) ? model.protocol : 'unknown') === group.protocol)}))
    .filter(group => group.models.length);
}

export function modelAccess(model: ProviderModel, locale: string) {
  const en = locale === 'en-US';
  const metadata = capabilities(model);
  if (metadata.unsupported || metadata.unsupported_protocol) return en ? 'Unsupported protocol' : '当前不支持此协议';
  if (!supportedModel(model)) return en ? 'Protocol required' : '待配置协议';
  if (!model.available) return en ? 'Not in current catalog' : '当前目录未列出';
  if (model.source === 'template') return en ? 'Catalog only · access unverified' : '内置目录 · 账户权限未验证';
  if (model.source === 'manual' || model.source === 'legacy') return en ? 'Manual · access unverified' : '手动配置 · 账户权限未验证';
  return en ? 'Listed in remote catalog' : '远程目录已列出';
}

export function connectionPayload(connection: ProviderConnection, apiKey: string = '') {
  return {id: connection.id, displayName: connection.display_name, providerType: connection.provider_type,
    baseUrl: connection.base_url, legacyModel: connection.legacy_model, templateKind: connection.template_kind,
    authMode: connection.auth_mode, modelsEndpoint: connection.models_endpoint, enabled: connection.enabled, apiKey};
}

export async function createConnection(
  invoke: <T>(name: string, payload: Record<string, unknown>) => Promise<T>,
  input: {template: ProviderTemplate; displayName: string; apiKey: string}
): Promise<ProviderConnection> {
  if (input.template.configurable === false) throw new Error('This provider plan does not permit this application. See the provider documentation.');
  return invoke<ProviderConnection>('create_provider_template', {
    templateKind: input.template.kind, displayName: input.displayName.trim() || input.template.display_name,
    apiKey: input.apiKey, enabled: true
  });
}

export type ModelForm = {modelId: string; displayName: string; protocol: string; endpointPath: string; enabled: boolean};
export function applyRouteOption(form: ModelForm, route: ModelRouteOption): ModelForm {
  if (!protocols.some(item => item.id === route.protocol)) throw new Error('Unknown protocol.');
  return {...form, protocol:route.protocol, endpointPath:route.endpoint_path};
}

// Only presented when the native core identifies an official OpenCode connection.
export function routePreview(baseUrl: string, endpointPath: string, protocol: string) {
  let base = baseUrl.replace(/\/+$/, '');
  if (/\/zen(?:\/go)?$/.test(base) && protocols.some(item => item.id === protocol && item.endpoint === endpointPath)) base += '/v1';
  return base + endpointPath;
}

export function routeAuthentication(baseUrl: string, endpointPath: string, protocol: string, authMode: string) {
  if (authMode === 'none') return 'none';
  if (authMode === 'api_key') return 'x-api-key';
  if (protocol === 'anthropic_messages' && /\/zen(?:\/go)?\/v1\/messages$/.test(routePreview(baseUrl,endpointPath,protocol))) return 'x-api-key';
  return 'Bearer';
}
export function modelSavePayload(providerId: number, form: ModelForm, original?: ProviderModel) {
  if (!form.modelId.trim() || !form.displayName.trim()) throw new Error('Model ID and display name are required.');
  if (!protocols.some(item => item.id === form.protocol)) throw new Error('Choose a supported protocol.');
  const endpoint = form.endpointPath.trim();
  if (!endpoint.startsWith('/') || endpoint.includes('..')) throw new Error('Model endpoint must start with / and cannot contain ..');
  const metadata = capabilities(original);
  delete metadata.needs_protocol;
  delete metadata.unsupported;
  delete metadata.unsupported_protocol;
  return {providerId, modelId: original?.model_id ?? form.modelId.trim(), displayName: form.displayName.trim(),
    protocol: form.protocol, endpointPath: endpoint, capabilitiesJson: JSON.stringify(metadata), source: 'manual',
    enabled: form.enabled, available: true, confirmAvailable: true};
}

export function readFolds(storage: FoldStorage): FoldState {
  try {
    const old = JSON.parse(storage.getItem('msl.provider-model-folds') ?? '{}');
    const current = JSON.parse(storage.getItem('msl.provider-folds.v2') ?? '{}');
    const entries = [...Object.entries(old ?? {}).map(([key, value]) => [`models:${key}`, value]), ...Object.entries(current ?? {})];
    return Object.fromEntries(entries.filter(([key, value]) => typeof key === 'string' && typeof value === 'boolean'));
  } catch { return {}; }
}

export function saveFold(storage: FoldStorage, folds: FoldState, key: string, open: boolean): FoldState {
  const next = {...folds, [key]: open};
  try { storage.setItem('msl.provider-folds.v2', JSON.stringify(next)); } catch { /* Session state still works when storage is unavailable. */ }
  return next;
}
