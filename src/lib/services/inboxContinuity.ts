import type {SourceLocation} from './workflowContinuity';

export type InboxContinuity = {
  inbox_id:number;
  work_id:number|null;
  source:SourceLocation|null;
  pending_ids:number[];
  deferred_ids:number[];
  destinations:Array<SourceLocation & {title:string}>;
  last_job:{id:number;status:string;error:string|null}|null;
};
export function inboxProjectPrefill(context:Pick<InboxContinuity,'work_id'>|undefined,works:Array<{id:number;status:string}>):number|null {
  return works.some(work=>work.id===context?.work_id&&work.status!=='archived')?context!.work_id:null;
}
export function inboxAction(context:Pick<InboxContinuity,'pending_ids'|'deferred_ids'|'destinations'|'last_job'>|undefined,processed:boolean):{kind:'review';proposalId:number}|{kind:'running'|'complete'|'organize'} {
  const proposalId=context?.pending_ids[0]??context?.deferred_ids[0];
  if(proposalId)return {kind:'review',proposalId};
  if(context?.last_job?.status==='running')return {kind:'running'};
  return {kind:processed?'complete':'organize'};
}
