// Synthetic local display data. No credentials, native files, or network calls.
const now = Math.floor(Date.now()/1000);
const empty = new URLSearchParams(location.search).has('empty');
const base = {created_at:now-86400,updated_at:now};
const works = empty ? [] : [
  {id:1,title:'演示 · 长期随访医学交流',status:'active',summary:'梳理随访中的实践障碍，让下一次沟通更有针对性。',...base,archived_at:null},
  {id:2,title:'演示 · 区域学术沟通计划',status:'active',summary:'围绕专家的证据需求，安排资料准备与持续跟进。',...base,archived_at:null},
  {id:3,title:'演示 · 真实世界研究讨论',status:'paused',summary:'等待协作方核对数据可用性。',...base,archived_at:null}
];
const tasks = empty ? [] : [
  {id:1,work_id:1,title:'整理随访中断的常见障碍',status:'next',priority:'high',due_at:now+7200,scheduled_start:null,scheduled_end:null,notes:'演示：先汇总交流原话，再确认待核实的问题。',...base,completed_at:null},
  {id:2,work_id:2,title:'准备下次专家交流的证据资料',status:'next',priority:'normal',due_at:now+86400,scheduled_start:null,scheduled_end:null,notes:'演示：明确研究人群与终点的适用范围。',...base,completed_at:null},
  {id:3,work_id:1,title:'确认本周跟进安排',status:'next',priority:'normal',due_at:null,scheduled_start:null,scheduled_end:null,notes:'演示事项',...base,completed_at:null}
];
const waiting = empty ? [] : [{id:1,work_id:3,title:'等待研究团队确认资料可用性',waiting_for:'演示协作方',started_at:now-86400,follow_up_at:now,status:'open',notes:'演示记录',...base,resolved_at:null}];
const calendar = empty ? [] : [{id:1,work_id:2,title:'区域医学团队进展沟通',start_at:now+3600,end_at:now+7200,all_day:false,location:'线上会议 · 演示',notes:'核对下一阶段的学术沟通重点。',kind:'meeting',...base}];
const inbox = empty ? [] : [{id:1,content:'演示：下次交流前，先整理专家提出的长期随访问题。',created_at:now,processed_at:null,converted_to_type:null,converted_to_id:null}];
const proposals = empty ? [] : [
  {id:1,analysis_run_id:1,kind:'task',operation:'create',work_id:1,target_id:null,title:'把随访障碍整理为下一次讨论提纲',payload_json:JSON.stringify({title:'把随访障碍整理为下一次讨论提纲',work_id:1,priority:'normal',notes:'演示：保留专家原话，分清已经确认的事实与待核实问题。'}),reason:'您提出先了解中断原因，再安排下一步。',status:'pending',dedupe_key:'demo-one',...base},
  {id:2,analysis_run_id:1,kind:'waiting',operation:'create',work_id:3,target_id:null,title:'跟进研究资料的可用性反馈',payload_json:JSON.stringify({title:'跟进研究资料的可用性反馈',work_id:3,waiting_for:'演示研究团队',notes:'确认反馈后，再讨论研究问题。'}),reason:'演示：研究讨论需等待数据可用性确认。',status:'pending',dedupe_key:'demo-two',...base}
];
const resume = {id:1,work_id:1,current_state:'交流要点已经留存，正在梳理实践中的共同障碍。',next_step:'准备下一次讨论提纲，保留需要进一步核实的问题。',remember:'先了解障碍，再安排后续工作。',source:'user',created_at:now};
if (new URLSearchParams(location.search).has('long')) {
  for (const entry of [...works,...tasks,...proposals]) entry.title += ' · 跨区域医学沟通与长期研究资料核对'.repeat(7);
  for (const item of inbox) item.content += ' 保留完整交流原话和后续方向调整。'.repeat(20);
}
const settings = new Map([['appearance_font_scale',new URLSearchParams(location.search).get('font')||'standard'],['appearance_theme','mist']]);
let callbackId=0;
export const invoke = async (name,args={}) => {
  if (name==='app_settings_get') return settings.get(args.key)??null;
  if (name==='app_settings_set') {settings.set(args.key,args.value);return null;}
  if (name==='list_works') return works;
  if (name==='get_work') return works.find(w=>w.id===args.id)??works[0]??null;
  if (name==='get_work_detail') return {work:works.find(w=>w.id===args.id),latest_resume:resume,resume_history:[resume],files:[],tasks:tasks.filter(t=>t.work_id===args.id),waiting:waiting.filter(t=>t.work_id===args.id),calendar:calendar.filter(t=>t.work_id===args.id),recent_activity:[]};
  if (['get_workspaces','get_project_directories','secretary_round_status'].includes(name)) return [];
  if (name==='get_today') return {continue_works:works.map(work=>({work,latest_resume:resume,last_activity_at:now,files:[]})),today_tasks:tasks,today_calendar:calendar,waiting_followups:waiting,inbox_pending:inbox};
  if (name==='list_tasks') return tasks.filter(t=>!args.workId||t.work_id===args.workId);
  if (name==='list_waiting_items'||name==='list_waiting') return waiting;
  if (name==='list_calendar_events') return calendar;
  if (name==='list_inbox_items'||name==='list_inbox') return inbox;
  if (name==='list_latest_analysis_proposals'||name==='list_ai_proposals'||name==='list_recent_ai_proposals') return proposals;
  if (name==='list_resume_points') return empty?[]:[resume];
  if (name==='get_latest_resume_point') return empty?null:resume;
  if (name==='workspace_sync_status') return {root:null,paused:true,baseline_count:0,last_scan:now,last_warning:null};
  if (name==='recent_files') return empty?[]:[{path:'演示资料/随访问题讨论提纲.docx',event_type:'更新',display_text:'演示资料已更新',timestamp:now-1800},{path:'演示资料/证据资料摘要.pdf',event_type:'更新',display_text:'演示资料已更新',timestamp:now-7200}];
  if (name==='get_morning_brief') return empty?null:{content:'• 长期随访项目：交流原话已整理，下一步核实共同障碍。\n• 研究讨论：等待资料可用性反馈，暂不安排方案讨论。\n• 您的意见：先明确证据需求，再准备沟通材料。'};
  if (name==='list_analysis_runs') return empty?[]:[{id:1,trigger:'manual',status:'completed',started_at:now-600,finished_at:now-570,summary:'已结合项目与您的调整整理建议。',error_code:null,error_message:null}];
  if (name==='get_analysis_schedule') return {id:1,enabled:false,interval_minutes:180,daily_enabled:false,daily_hour:6,daily_minute:0,last_interval_run_at:null,last_daily_local_date:null,updated_at:now};
  if (name==='sync_status') return {config:{directory:'',enabled:false,interval_minutes:180,dataset_id:'',generation:'',ai_primary:true},state:{phase:'disconnected',last_success:0,last_error:'',last_uploaded:0,last_applied:0,conflicts:0},running:false};
  if (name==='backup_status') return {data_directory:'演示 / 正式数据',cache_directory:'演示 / 可重建缓存',config:{directory:'',enabled:false,interval_minutes:180,keep_count:10},state:{last_success:0,last_file:'',error:'',warning:'',files:[]},running:false,restore_pending:false,has_rollback:false};
  if (name==='get_report_schedule') return {weekly_enabled:false,monthly_enabled:false,weekly_weekday:0,weekly_hour:15,weekly_minute:0,monthly_day:1,monthly_hour:9,monthly_minute:0};
  if (name==='list_kol_experts') return empty?[]:[{id:1,name:'演示专家',institution:'示例医疗机构',department:'临床研究科室',specialty:'长期随访',summary:'演示：关注随访实践与证据需求。',projects:[1],archived_at:null,...base}];
  if (name==='plugin:event|listen') return ++callbackId;
  if (name.startsWith('plugin:')) return null;
  if (name==='start_ai_job') throw new Error('合成预览不调用模型。');
  if (name.startsWith('list_')||name==='search_all'||name==='get_work_workspace_links') return [];
  return null;
};
window.__TAURI_INTERNALS__={invoke,transformCallback:()=>++callbackId,unregisterCallback:()=>{},convertFileSrc:p=>p,metadata:{currentWindow:{label:'main'},currentWebview:{label:'main'}}};
window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener:()=>{}};
