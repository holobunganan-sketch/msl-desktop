import type { AiTaskRoute, ProviderConnection, ProviderModel } from '../types/domain';
export function chatModels(providers: ProviderConnection[], models: ProviderModel[]) {
  const enabled = new Set(providers.filter(p => p.enabled).map(p => p.id));
  return models.filter(m => enabled.has(m.provider_id) && m.enabled && m.available);
}
export function chatModelId(routes: AiTaskRoute[]): number | null {
  return (routes.find(r => r.task_kind === 'workbench_qa') ?? routes.find(r => r.task_kind === 'general'))?.provider_model_id ?? null;
}
export function chatEnter(event: {key: string; shiftKey?: boolean; isComposing?: boolean; keyCode?: number}) {
  return event.key === 'Enter' && !event.shiftKey && !event.isComposing && event.keyCode !== 229;
}
// Unsent drafts survive page navigation, stay local, and are never sent to AI.
export const chatDrafts = new Map<string, {question: string; scope: number[]}>();
export const chatSelection: {sessionId: number | null | undefined} = {sessionId: undefined};
