import {resolveDestination,type Destination} from './navigation';
export type SourceLocation={entity_kind:string;entity_id:number;work_id?:number|null;expert_id?:number|null;workspace_id?:number|null;relative_path?:string|null;available:boolean;remote_only?:boolean;material_id?:number|null;blob_hash?:string|null;locator?:string|null;revision?:number|null};
export function sourceDestination(location?:SourceLocation|null):Destination|null {
 if(!location?.available||location.remote_only||location.entity_id<=0)return null;
 const {entity_kind:kind,entity_id:id,expert_id:expert,workspace_id:workspace}=location;
 if(kind==='expert')return {view:'kol',id};
 if(['kol_note','kol_insight','kol_draft','kol_material'].includes(kind))return expert?{view:'kol',id:expert,...(kind==='kol_note'?{noteId:id}:kind==='kol_insight'?{insightId:id}:kind==='kol_draft'?{draftId:id}:location.material_id?{materialId:location.material_id}:{})}:null;
 if(kind==='document')return workspace&&location.relative_path?{view:'workspace',workspaceId:workspace,relativePath:location.relative_path}:null;
 if(kind==='workspace')return {view:'workspace',workspaceId:workspace??id};
 if(kind==='activity'||kind==='file_change')return null;
 return resolveDestination(kind,id,location.work_id);
}
type ActionTask={id:number;work_id:number|null;status:string;due_at?:number|null;scheduled_start?:number|null;priority?:string};
export function actionableTasks<T extends ActionTask>(tasks:T[],works:{id:number;status:string}[]):T[]{
 const active=new Set(works.filter(w=>w.status==='active').map(w=>w.id));
 return tasks.filter(t=>['next','doing','scheduled'].includes(t.status)&&(t.work_id===null||active.has(t.work_id))).sort((a,b)=>(a.scheduled_start??a.due_at??Infinity)-(b.scheduled_start??b.due_at??Infinity)||(b.priority==='high'?1:0)-(a.priority==='high'?1:0)||a.id-b.id);
}
export function taskReason(task:ActionTask,now:number,en:boolean):string {return task.due_at&&task.due_at<=now?(en?'Due now':'已到截止时间'):task.scheduled_start?(en?'Scheduled':'已安排时间'):task.due_at?(en?'Has a deadline':'有截止时间'):(en?'Next action':'下一步行动');}
export type ExpertDraft={content:string;at:string;workId:number|null;inboxId:number|null};
export function expertFocus(target:{noteId?:number|null;insightId?:number|null;draftId?:number|null;materialId?:number|null}) {
 if(target.materialId)return {tab:'materials',materialId:target.materialId};
 if(target.draftId)return {tab:'drafts',draftId:target.draftId};
 if(target.insightId)return {tab:'insights',insightId:target.insightId};
 return {tab:'record',...(target.noteId?{noteId:target.noteId}:{})};
}
const expertDrafts=new Map<number,ExpertDraft>();
export function writeExpertDraft(id:number|null,draft:ExpertDraft){if(id!==null)expertDrafts.set(id,{...draft});}
export function readExpertDraft(id:number|null):ExpertDraft|null{return id===null?null:expertDrafts.has(id)?{...expertDrafts.get(id)!}:null;}
export function clearSavedExpertDraft(id:number,draft:ExpertDraft){if(JSON.stringify(expertDrafts.get(id))===JSON.stringify(draft))expertDrafts.delete(id);}
export function expertSession<T extends {expert_id?:number|null;expert_scoped?:number;scope_json?:string}>(sessions:T[],id:number,scope?:number[]):T|null{return sessions.find(s=>{if(s.expert_scoped!==1||s.expert_id!==id)return false;if(!scope)return true;try{const saved=JSON.parse(s.scope_json??'[]');return Array.isArray(saved)&&JSON.stringify([...new Set(saved)].sort())===JSON.stringify([...new Set(scope)].sort());}catch{return false;}})??null;}
export function jobDestination(job:{command:string;args:Record<string,unknown>;result:unknown}):Destination {
 const args=job.args,r=job.result as {session_id?:number;expert_id?:number;draft_id?:number}|null;
 if(job.command==='ask_workbench')return {view:'qa',...(r?.session_id?{id:r.session_id}:{})};
 if(job.command==='analyze_kol')return {view:'kol',...(r?.expert_id??args.expertId?{id:Number(r?.expert_id??args.expertId)}:{}),...(r?.draft_id?{draftId:r.draft_id}:{})};
 if(job.command.includes('report'))return {view:'reports',...(typeof job.result==='number'?{id:job.result}:{})};
 if(job.command==='generate_brief')return {view:'today'};
 if(job.command==='translate_text')return {view:'translation'};
 if(['import_kol_materials','read_kol_materials'].includes(job.command))return {view:'kol',...(args.expertId?{id:Number(args.expertId)}:{})};
 if(job.command==='refresh_project_cognition')return args.scope==='work'?{view:'works',id:Number(args.scopeId)}:{view:'workspace',...(args.scopeId?{workspaceId:Number(args.scopeId)}:{})};
 return {view:'matters',section:'review',...(args.workId?{workId:Number(args.workId)}:{}),...(typeof job.result==='number'?{runId:job.result}:{})};
}
export type ReportSource={source_type:string;entity_id?:number;timestamp?:number;location?:SourceLocation;[key:string]:unknown};
export type ReportItem={category:string;project_id:number|null;headline:string;change:string;impact:string;next_action:string;certainty:string;horizon:string;evidence_refs:ReportSource[];sources:ReportSource[]};
export function reportItems(report:{structured_json?:string|null;evidence_json?:string|null}):ReportItem[]{
 try{const value=JSON.parse(report.structured_json??'null'),pack=JSON.parse(report.evidence_json??'{}');if(value?.version!=='report-spec-v2'||!Array.isArray(value.items))return [];
 return value.items.map((item:ReportItem)=>({...item,sources:(item.evidence_refs??[]).flatMap(ref=>(pack.sources??[]).filter((source:ReportSource)=>source.source_type===ref.source_type&&source.entity_id===ref.entity_id&&(!ref.content_hash||source.content_hash===ref.content_hash)))}));}catch{return [];}
}
