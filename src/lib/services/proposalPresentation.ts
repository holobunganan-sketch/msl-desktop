type Proposal = {kind:string; operation?:string; target_id?:number|null; work_id:number|null; payload_json:string};
export type ProposalExpert = {id:number;name:string;institution:string;department:string;revision:number;archived:number};
export const proposalTimeFields=['scheduled_start','scheduled_end','due_at','follow_up_at','start_at','end_at'] as const;
export function needsTimeConfirmation(payload:Record<string,unknown>):boolean {
  return payload.time_basis==='inferred'&&proposalTimeFields.some(key=>typeof payload[key]==='number'&&Number.isFinite(payload[key])&&Number(payload[key])>0);
}
export function proposalPresentation(item:Proposal,works:Array<{id:number;title:string;category?:string|null}>,locale:string,experts?:ProposalExpert[]) {
  const en=locale==='en-US';
  let payload:Record<string,unknown>={};
  try { const parsed=JSON.parse(item.payload_json);if(parsed&&typeof parsed==='object'&&!Array.isArray(parsed))payload=parsed; } catch { /* Missing data requires editing. */ }
  const labels:Record<string,[string,string]>={work:['长期项目','project'],task:['任务','task'],waiting:['等待事项','waiting item'],calendar:['日程','event'],resume_point:['项目进展','project progress'],inbox:['记录','note'],kol_insight:['专家洞察','expert insight']};
  const label=labels[item.kind]??['事项','item'];
  const action=en?`${item.operation==='update'?'Update existing':'Add'} ${label[1]}`:`${item.operation==='update'?'更新已有':'新增'}${label[0]}`;
  const updatingProject=item.kind==='work'&&item.operation==='update';
  const project=works.find(work=>work.id===(updatingProject?item.target_id:item.work_id));
  const missingProject=(updatingProject||!!item.work_id)&&!project;
  const expert=experts?.find(expert=>expert.id===payload.expert_id);
  const invalidExpert=item.kind==='kol_insight'&&(!expert||!!expert.archived||expert.revision!==payload.expert_revision);
  const scope=item.kind==='kol_insight'?(expert?[expert.name,expert.institution,expert.department].filter(Boolean).join(' · '):(en?'Review expert identity in suggestions':'请到建议页核对专家归属')):project?.title ?? (missingProject ? (en?'Project unavailable — refresh review':'项目不可用，请刷新审阅') : item.kind==='work' ? (en?'New long-term project':'新的长期项目') : (en?'Standalone item':'独立事项'));
  const timestamp=payload.start_at??payload.scheduled_start??payload.due_at??payload.follow_up_at;
  const hasTime=typeof timestamp==='number'&&Number.isFinite(timestamp)&&timestamp>0;
  const missingDate=item.kind==='calendar'&&!hasTime&&(item.operation!=='update'||Object.hasOwn(payload,'start_at'));
  const time=hasTime?new Date(timestamp*1000).toLocaleString(locale,{year:'numeric',month:'short',day:'numeric',hour:'2-digit',minute:'2-digit',hour12:false}):missingDate?(en?'Confirm a time':'请确认安排时间'):item.kind==='calendar'?(en?'Keep existing time':'保持原有时间'):'';
  const timeFields:Array<[string,string,string]>=[['scheduled_start','安排开始','Start'],['scheduled_end','安排结束','End'],['due_at','截止时间','Deadline'],['follow_up_at','跟进时间','Follow-up'],['start_at','日程开始','Event start'],['end_at','日程结束','Event end']];
  const timeDetails=timeFields.flatMap(([key,zh,eng])=>{
    if(!Object.hasOwn(payload,key))return [];
    const value=payload[key];
    if(value===null&&item.operation==='update')return [(en?eng:zh)+'：'+(en?'Clear the existing time':'清除原时间')];
    if(typeof value!=='number'||!Number.isFinite(value)||value<=0)return [];
    return [(en?eng:zh)+'：'+new Date(value*1000).toLocaleString(locale,{year:'numeric',month:'short',day:'numeric',hour:'2-digit',minute:'2-digit',hour12:false})];
  });
  const fields:Array<[string,string,string]>=[['status','状态','Status'],['current_state','目前进展','Progress'],['next_step','下一步','Next step'],['waiting_for','等待谁','Waiting for'],['notes','说明','Notes'],['summary','目标','Goal'],['content','记录','Note'],['remember','提醒','Remember'],['observation','观察 / 专家表达','Observation / expert statement'],['implication','可能的意义','Possible significance'],['uncertainty','待核实','Uncertainty'],['next_question','下次追问','Next question']];
  const statuses:Record<string,[string,string]>={done:['已完成','Completed'],resolved:['已结束等待','Resolved'],next:['待推进','Next'],scheduled:['已安排','Scheduled'],open:['等待中','Waiting'],active:['进行中','Active'],paused:['暂缓','Paused'],waiting:['等待中','Waiting']};
  const changes=fields.flatMap(([key,zh,eng])=>{const value=payload[key];if(typeof value!=='string'||!value.trim())return [];return [`${en?eng:zh}：${key==='status'?(statuses[value]?.[en?1:0]??value):value}`];});
  if(item.kind==='kol_insight'&&Array.isArray(payload.categories)){
    const kinds:Record<string,string>={practice_barrier:en?'Practice barriers':'临床实践障碍',evidence_need:en?'Evidence needs':'证据需求',research_opportunity:en?'Research opportunities':'研究合作机会'};
    changes.unshift(`${en?'Insight categories':'洞察分类'}：${payload.categories.filter((v):v is string=>typeof v==='string').map(v=>kinds[v]??v).join('、')}`);
  }
  if(item.kind==='kol_insight'&&project)changes.push(`${en?'Link expert to project':'关联专家与项目'}：${project.title}`);
  const categories:Record<string,string>={clinical:en?'Clinical research':'临床研究',non_clinical:en?'Non-clinical work':'非临床研究'};
  if(typeof payload.category==='string'&&categories[payload.category])changes.unshift(`${en?'Project category':'项目分类'}：${categories[payload.category]}`);
  const clinical=works.find(work=>work.id===payload.clinical_work_id);
  const invalidClinical=typeof payload.clinical_work_id==='number'&&(!clinical||clinical.category!=='clinical');
  if(Object.hasOwn(payload,'clinical_work_id'))changes.push(`${en?'Related clinical research':'关联临床研究'}：${payload.clinical_work_id===null?(en?'No additional research link':'不额外关联研究'):clinical?.title??(en?'Unavailable — choose again':'项目不可用，请重新选择')}`);
  if(typeof payload.priority==='string'){
    const priorities:Record<string,string>={low:en?'Low':'低',normal:en?'Normal':'普通',high:en?'High':'高'};
    if(priorities[payload.priority])changes.push(`${en?'Priority':'优先级'}：${priorities[payload.priority]}`);
  }
  const timeBasis=payload.time_basis==='inferred'?(en?'AI suggested time':'AI 建议时间'):payload.time_basis==='explicit'?(en?'Specified time':'指定时间'):'';
  const timeReason=typeof payload.time_reason==='string'?payload.time_reason:'';
  const confirmTime=needsTimeConfirmation(payload);
  const calendarHint=confirmTime?(en?'Confirm the suggested time separately before adding it to the calendar.':'建议时间需要单独确认，确认后才进入日历。'):hasTime&&['task','waiting','calendar'].includes(item.kind)?(en?'Appears on the calendar after acceptance; editing keeps the original item in sync.':'采用后自动进入日历，修改时仍关联原事项。'):'';
  return {action,scope,time,timeDetails,changes,timeBasis,timeReason,calendarHint,needsTimeConfirmation:confirmTime,needsAttention:invalidExpert||confirmTime||invalidClinical||missingDate||(item.kind==='resume_point'&&!item.work_id)||missingProject};
}
export function actionTimeLabel(timestamp:number,now:number):string{
  const at=new Date(timestamp*1000),today=new Date(now*1000),pad=(n:number)=>String(n).padStart(2,'0');
  const time=`${pad(at.getHours())}:${pad(at.getMinutes())}`;
  return at.toDateString()===today.toDateString()?time:`${pad(at.getMonth()+1)}/${pad(at.getDate())}\n${time}`;
}

export type ProposalChange={key:string;label:string;before:string;after:string};
/** Compare only submitted fields. An omitted field is preserved, never cleared. */
export function proposalChanges(item:Proposal,current:Record<string,unknown>|null,works:Array<{id:number;title:string}>,locale:string):ProposalChange[]{
  if(item.operation!=='update'||!current)return [];
  let payload:Record<string,unknown>;
  try{payload=JSON.parse(item.payload_json);if(!payload||Array.isArray(payload)||typeof payload!=='object')return [];}catch{return [];}
  if(['task','waiting','calendar','resume_point'].includes(item.kind))payload={...payload,work_id:item.work_id};
  const en=locale==='en-US';
  const fields:Array<[string,string,string]>=[['title','标题','Title'],['work_id','所属项目','Project'],['status','状态','Status'],['priority','优先级','Priority'],['category','项目分类','Project category'],['summary','目标','Goal'],['current_state','目前进展','Progress'],['next_step','下一步','Next step'],['remember','提醒','Remember'],['notes','说明','Notes'],['content','原话','Original note'],['waiting_for','等待谁','Waiting for'],['due_at','截止时间','Deadline'],['follow_up_at','跟进时间','Follow-up'],['scheduled_start','安排开始','Start'],['scheduled_end','安排结束','End'],['start_at','日程开始','Event start'],['end_at','日程结束','Event end'],['all_day','全天','All day'],['location','地点','Location'],['observation','观察 / 专家表达','Observation / expert statement'],['implication','可能的意义','Possible significance'],['uncertainty','待核实','Uncertainty'],['next_question','下次追问','Next question'],['categories','洞察分类','Insight categories']];
  const labels:Record<string,[string,string]>={low:['低','Low'],normal:['普通','Normal'],high:['高','High'],active:['进行中','Active'],paused:['暂缓','Paused'],archived:['已归档','Archived'],next:['待推进','Next'],doing:['推进中','In progress'],scheduled:['已安排','Scheduled'],done:['已完成','Completed'],open:['等待中','Waiting'],resolved:['已结束等待','Resolved'],clinical:['临床研究','Clinical research'],non_clinical:['非临床研究','Non-clinical work'],practice_barrier:['临床实践障碍','Practice barriers'],evidence_need:['证据需求','Evidence needs'],research_opportunity:['研究合作机会','Research opportunities']};
  const format=(key:string,value:unknown,after:boolean):string=>{
    if(value===null||value===undefined||value==='')return after?(en?'Clear':'清除'):(en?'Not set':'未设置');
    if(key==='work_id')return `${works.find(work=>work.id===value)?.title??(en?'Unavailable project':'项目不可用')} · #${value}`;
    if(proposalTimeFields.includes(key as typeof proposalTimeFields[number])&&typeof value==='number')return new Date(value*1000).toLocaleString(locale,{hour12:false});
    if(typeof value==='boolean')return value?(en?'Yes':'是'):(en?'No':'否');
    if(Array.isArray(value))return value.map(v=>labels[String(v)]?.[en?1:0]??String(v)).join('、');
    if(['status','priority','category'].includes(key))return labels[String(value)]?.[en?1:0]??String(value);
    return String(value);
  };
  return fields.flatMap(([key,zh,english])=>{
    if(!Object.hasOwn(payload,key))return [];
    const before=current[key]??null,after=payload[key]??null;
    // Match application semantics: null text and empty note/goal fallbacks keep
    // their previous values. Only supported nullable fields actually clear.
    if(after===null&&!['work_id','category','due_at','follow_up_at','scheduled_start','scheduled_end','end_at'].includes(key))return [];
    if(['notes','summary','content'].includes(key)&&typeof after==='string'&&!after.trim())return [];
    if(JSON.stringify(before)===JSON.stringify(after))return [];
    return [{key,label:en?english:zh,before:format(key,before,false),after:format(key,after,true)}];
  });
}
