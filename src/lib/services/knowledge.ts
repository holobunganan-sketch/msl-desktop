export type Project={id:number;title:string;status:string};
export type Citation={source_id:string;quote:string};
export type Evidence={id:string;kind:string;entity_id:number;title:string;text:string;timestamp:number|null;trust:string;hash:string};
export type Pack={sources:Evidence[];as_of:number;scope_ids:number[];counts:Record<string,number>;omitted:number;notes:string[]};
export type Answer={claims:{text:string;basis:string;citations:Citation[]}[];gaps:string[];document?:import('$lib/types/aiDocument').AiDocumentValue};
export type Session={id:number;title:string;scope_json:string;updated_at:number};
export type Turn={id:number;session_id:number;question:string;scope_json:string;status:string;answer_json:string|null;evidence_json:string|null;error:string|null;created_at:number;document?:import('$lib/types/aiDocument').AiDocumentValue};
export type Expert={id:number;revision:number;name:string;institution:string;department:string;specialty:string;summary:string;archived:number;project_ids_json:string;note_count:number;last_contact:number|null};
export type Insight={title:string;categories:string[];observation:string;implication:string;uncertainty:string;next_question:string;citations:Citation[]};
export type KolAction={enabled:boolean;kind:string;title:string;work_id:number|null;notes:string;waiting_for:string;at:number|null;time_basis:string;time_reason:string;citations:Citation[]};
export type KolOutput={summary:string;insights:Insight[];actions:KolAction[];citations:Citation[]};
export type KolDraft={id:number;expert_id:number|null;purpose:string;status:string;payload_json:string;evidence_json:string;revision:number;created_at:number;name?:string};
export const categories=[['practice_barrier','临床实践障碍','Practice barriers'],['evidence_need','证据需求','Evidence needs'],['research_opportunity','研究合作机会','Research opportunities']] as const;
export function json<T>(value:string|null|undefined,fallback:T):T {try{return value?JSON.parse(value):fallback;}catch{return fallback;}}
export function mentionAt(text:string,caret:number) {const prefix=text.slice(0,caret);const match=/(?:^|\s)@([^@\n]{0,80})$/.exec(prefix);return match?{start:prefix.lastIndexOf('@'),end:caret,query:match[1]}:null;}
export function addScope(ids:number[],id:number) {if(!Number.isSafeInteger(id)||id<1)throw new Error('无效项目');return [...new Set([...ids,id])].sort((a,b)=>a-b);}
export function localTime(at:number|null):string {if(at==null)return '';const d=new Date(at*1000);return new Date(d.getTime()-d.getTimezoneOffset()*60000).toISOString().slice(0,16);}
export function parseLocalTime(value:string):number|null {if(!value.trim())return null;const at=Math.floor(new Date(value).getTime()/1000);if(!Number.isFinite(at))throw new Error('日期无效');return at;}
export function sourceLabel(kind:string,en:boolean) {return ({work:['项目','Project'],task:['任务','Task'],waiting:['等待','Waiting'],calendar:['日历','Calendar'],inbox:['收件箱','Inbox'],resume:['推进记录','Progress'],activity:['变动','Activity'],kol_material:['专家资料','Expert material'],document:['文档','Document'],proposal:['AI 建议','AI proposal'],report:['报告','Report'],brief:['简报','Brief'],kol_note:['交流原话','Interaction'],expert:['专家资料','Expert'],kol_insight:['洞察假设','Insight hypothesis'],kol_followup:['专家跟进','Expert follow-up'],coverage:['覆盖范围','Coverage']} as Record<string,string[]>)[kind]?.[en?1:0]??(en?'Record':'工作记录');}
