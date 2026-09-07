export type MatterSection = 'inbox' | 'review' | 'plan' | 'waiting' | 'done';
export type View = 'today' | 'works' | 'matters' | 'calendar' | 'reports' | 'translation' | 'workspace' | 'settings' | 'qa' | 'kol';
export type Destination = {view: View; section?: MatterSection; id?: number; workId?: number; resumeId?: number};

export function resolveDestination(kind: string, id?: number, workId?: number | null): Destination | null {
  const focus = typeof id === 'number' && Number.isSafeInteger(id) && id > 0 ? {id} : {};
  if (kind === 'work' || kind === 'works') return {view:'works',...focus};
  if (kind === 'resume' || kind === 'resume_point') return workId ? {view:'works',id:workId} : {view:'works',...(focus.id ? {resumeId:focus.id} : {})};
  const section = ({task:'plan',plan:'plan',waiting:'waiting',inbox:'inbox',review:'review',proposal:'review',done:'done'} as Record<string,MatterSection>)[kind];
  if (section) return {view:'matters',section,...focus};
  if (['today','matters','calendar','reports','translation','workspace','settings','qa','kol'].includes(kind)) return {view:kind as View,...focus};
  if (kind === 'file') return {view:'workspace'};
  return null;
}

export function navigateTo(target: Destination | string, id?: number, workId?: number | null): void {
  const destination = typeof target === 'string' ? resolveDestination(target,id,workId) : target;
  if (destination) window.dispatchEvent(new CustomEvent('dashboard:navigate',{detail:destination}));
}
