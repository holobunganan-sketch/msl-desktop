import {needsTimeConfirmation} from './proposalPresentation';
type Draft = { kind: string; title: string; payload_json: string; reason: string; suggested_kind?: string; user_edited?: boolean };
export function confirmationPayload(payload:Record<string,unknown>,acceptSuggestedTime=false):Record<string,unknown>{
  const {time_confirmation:_ignored,...result}=payload;
  return acceptSuggestedTime&&needsTimeConfirmation(result)?{...result,time_confirmation:'user_confirmed'}:result;
}
export const proposalStatuses: Record<string, string[]> = {
  work: ['active', 'paused', 'waiting', 'done', 'archived'],
  task: ['next', 'doing', 'scheduled', 'waiting', 'paused', 'done'],
  waiting: ['open', 'resolved'],
};
export function decisionPayload(item: Draft, kind: string): Record<string, unknown> {
  let current: Record<string, unknown>;
  try { const value=JSON.parse(item.payload_json);current=value&&typeof value==='object'&&!Array.isArray(value)?value:{}; } catch { current={}; }
  delete current.time_confirmation;
  const title=item.title.trim();
  // Preserve the approved patch exactly when its destination is unchanged.
  // Adding defaults here could clear dates/notes or reopen completed work.
  if(kind===item.kind){
    // Recover drafts saved by the old editor after a type change. Only discard
    // a status proven to belong to the original type; unknown values need review.
    const allowed=proposalStatuses[kind];
    if(item.user_edited && item.suggested_kind!==kind && typeof current.status==='string'
      && allowed && !allowed.includes(current.status)
      && proposalStatuses[item.suggested_kind??'']?.includes(current.status)) delete current.status;
    return kind==='inbox'?{...current,content:current.content??title}:{...current,title};
  }
  let converted:Record<string,unknown>;
  if(kind==='work')converted={title,status:'active',summary:current.summary??current.notes??current.content??item.reason};
  else if(kind==='task')converted={title,priority:current.priority??'normal',due_at:current.due_at??null,notes:current.notes??item.reason};
  else if(kind==='waiting')converted={title,waiting_for:current.waiting_for??'',follow_up_at:current.follow_up_at??null,notes:current.notes??item.reason};
  else if(kind==='calendar')converted={title,start_at:current.start_at??null,end_at:current.end_at??null,all_day:current.all_day??false,kind:'other',notes:current.notes??item.reason};
  else if(kind==='resume_point')converted={title,current_state:current.current_state??current.notes??current.summary??current.content??item.reason,next_step:current.next_step??'',remember:current.remember??''};
  else converted={content:current.content??title};
  // A destination change must not silently turn a proposed time into a fact.
  for(const field of ['time_basis','time_reason','unknowns'])if(Object.hasOwn(current,field))converted[field]=current[field];
  if(['task','waiting','calendar','inbox'].includes(kind)&&Object.hasOwn(current,'clinical_work_id'))converted.clinical_work_id=current.clinical_work_id;
  if(current.field_evidence&&typeof current.field_evidence==='object'&&!Array.isArray(current.field_evidence)){
    converted.field_evidence=Object.fromEntries(Object.entries(current.field_evidence).filter(([key])=>Object.hasOwn(converted,key)&&converted[key]===current[key]));
  }
  return converted;
}

export function reviewPayload(item:Draft):Record<string,unknown>{
  const payload=decisionPayload(item,item.kind);
  if(['task','waiting','calendar'].includes(item.kind)&&!Object.hasOwn(payload,'notes')){
    const notes=[payload.summary,payload.next_step,payload.content].filter(value=>typeof value==='string'&&value.trim()).join('\n');
    if(notes)payload.notes=notes;
  }
  return payload;
}
