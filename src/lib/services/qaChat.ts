import type { AiTaskRoute, ProviderConnection, ProviderModel } from '../types/domain';
export function chatModels(providers: ProviderConnection[], models: ProviderModel[]) {
  const enabled = new Set(providers.filter(p => p.enabled).map(p => p.id));
  return models.filter(m => enabled.has(m.provider_id) && m.enabled && m.available);
}
export function chatModelId(routes: AiTaskRoute[]): number | null {
  return taskModelId(routes, 'workbench_qa');
}
export function taskModelId(routes: AiTaskRoute[], taskKind: string): number | null {
  return routes.find(r => r.task_kind === taskKind)?.provider_model_id
    ?? routes.find(r => r.task_kind === 'general')?.provider_model_id ?? null;
}
export function chatEnter(event: {key: string; shiftKey?: boolean; isComposing?: boolean; keyCode?: number}) {
  return event.key === 'Enter' && !event.shiftKey && !event.isComposing && event.keyCode !== 229;
}
// Unsent drafts survive page navigation, stay local, and are never sent to AI.
export const chatDrafts = new Map<string, {question: string; scope: number[]; expertId?:number|null}>();
export const chatSelection: {sessionId: number | null | undefined} = {sessionId: undefined};
// Page-local loading must never replace a remembered reading position.
export const chatScrollPositions=new Map<number,{top:number;follow:boolean}>();
