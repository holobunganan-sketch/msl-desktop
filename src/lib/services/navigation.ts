export type MatterSection = 'inbox' | 'review' | 'plan' | 'waiting' | 'done';
export type View = 'today' | 'works' | 'matters' | 'calendar' | 'reports' | 'translation' | 'workspace' | 'settings' | 'qa' | 'kol';
export type Destination = {view: View; section?: MatterSection; id?: number; workId?: number; resumeId?: number; runId?:number; expertId?:number; noteId?:number; insightId?:number; draftId?:number; materialId?:number; workspaceId?:number; relativePath?:string; filePath?:string; activityId?:number};

export function resolveDestination(kind: string, id?: number, workId?: number | null): Destination | null {
  const focus = typeof id === 'number' && Number.isSafeInteger(id) && id > 0 ? {id} : {};
  if (kind === 'work' || kind === 'works') return {view:'works',...focus};
  if (kind === 'resume' || kind === 'resume_point') return {view:'works',...(workId?{id:workId}:{}),...(focus.id?{resumeId:focus.id}:{})};
  const section = ({task:'plan',plan:'plan',waiting:'waiting',inbox:'inbox',review:'review',proposal:'review',done:'done'} as Record<string,MatterSection>)[kind];
  if (section) return {view:'matters',section,...focus,...(workId?{workId}:{})};
  if (['today','matters','calendar','reports','translation','workspace','settings','qa','kol'].includes(kind)) return {view:kind as View,...focus};
  if (kind === 'file') return {view:'workspace'};
  if (kind === 'expert') return {view:'kol',...focus};
  if (kind === 'report' || kind === 'weekly_report') return {view:'reports',...focus};
  if (kind === 'activity') return {view:'workspace',activityId:id};
  return null;
}

export function navigateBack():void { window.dispatchEvent(new CustomEvent('dashboard:back')); }

export function navigateTo(target: Destination | string, id?: number, workId?: number | null): void {
  const destination = typeof target === 'string' ? resolveDestination(target,id,workId) : target;
  if (destination) window.dispatchEvent(new CustomEvent('dashboard:navigate',{detail:destination}));
}
