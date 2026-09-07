type Proposal = {kind:string; operation?:string; work_id:number|null; payload_json:string};
export function proposalPresentation(item:Proposal,works:Array<{id:number;title:string}>,locale:string) {
  const en=locale==='en-US';
  let payload:Record<string,unknown>={};
  try { const parsed=JSON.parse(item.payload_json);if(parsed&&typeof parsed==='object'&&!Array.isArray(parsed))payload=parsed; } catch { /* Missing data requires editing. */ }
  const labels:Record<string,[string,string]>={work:['长期项目','project'],task:['任务','task'],waiting:['等待事项','waiting item'],calendar:['日程','event'],resume_point:['项目进展','project progress'],inbox:['记录','note']};
  const label=labels[item.kind]??['事项','item'];
  const action=en?`${item.operation==='update'?'Update existing':'Add'} ${label[1]}`:`${item.operation==='update'?'更新已有':'新增'}${label[0]}`;
  const project=works.find(work=>work.id===item.work_id);
  const scope=project?.title ?? (item.work_id ? (en?'Project unavailable — choose again':'项目不可用，请重新选择') : item.kind==='work' ? (en?'New long-term project':'新的长期项目') : (en?'Standalone item':'独立事项'));
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
  const fields:Array<[string,string,string]>=[['status','状态','Status'],['current_state','目前进展','Progress'],['next_step','下一步','Next step'],['waiting_for','等待谁','Waiting for'],['notes','说明','Notes'],['summary','目标','Goal'],['content','记录','Note'],['remember','提醒','Remember']];
  const statuses:Record<string,[string,string]>={done:['已完成','Completed'],resolved:['已结束等待','Resolved'],next:['待推进','Next'],scheduled:['已安排','Scheduled'],open:['等待中','Waiting'],active:['进行中','Active'],paused:['暂缓','Paused'],waiting:['等待中','Waiting']};
  const changes=fields.flatMap(([key,zh,eng])=>{const value=payload[key];if(typeof value!=='string'||!value.trim())return [];return [`${en?eng:zh}：${key==='status'?(statuses[value]?.[en?1:0]??value):value}`];});
  const timeBasis=payload.time_basis==='inferred'?(en?'AI suggested time':'AI 建议时间'):payload.time_basis==='explicit'?(en?'Specified time':'指定时间'):'';
  const timeReason=typeof payload.time_reason==='string'?payload.time_reason:'';
  const calendarHint=hasTime&&['task','waiting','calendar'].includes(item.kind)?(en?'Appears on the calendar after acceptance; editing keeps the original item in sync.':'采用后自动进入日历，修改时仍关联原事项。'):'';
  return {action,scope,time,timeDetails,changes,timeBasis,timeReason,calendarHint,needsAttention:missingDate||(item.kind==='resume_point'&&!item.work_id)||Boolean(item.work_id&&!project)};
}
export function actionTimeLabel(timestamp:number,now:number):string{
  const at=new Date(timestamp*1000),today=new Date(now*1000),pad=(n:number)=>String(n).padStart(2,'0');
  const time=`${pad(at.getHours())}:${pad(at.getMinutes())}`;
  return at.toDateString()===today.toDateString()?time:`${pad(at.getMonth()+1)}/${pad(at.getDate())}\n${time}`;
}
