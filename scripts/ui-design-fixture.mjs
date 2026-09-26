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
const proposals = empty || new URLSearchParams(location.search).has('no-decisions') ? [] : [
  {id:1,analysis_run_id:1,kind:'task',operation:'create',work_id:1,target_id:null,title:'把随访障碍整理为下一次讨论提纲',payload_json:JSON.stringify({title:'把随访障碍整理为下一次讨论提纲',work_id:1,priority:'normal',notes:'演示：保留专家原话，分清已经确认的事实与待核实问题。'}),reason:'您提出先了解中断原因，再安排下一步。',status:'pending',dedupe_key:'demo-one',...base},
  {id:2,analysis_run_id:1,kind:'waiting',operation:'create',work_id:3,target_id:null,title:'跟进研究资料的可用性反馈',payload_json:JSON.stringify({title:'跟进研究资料的可用性反馈',work_id:3,waiting_for:'演示研究团队',notes:'确认反馈后，再讨论研究问题。'}),reason:'演示：研究讨论需等待数据可用性确认。',status:'pending',dedupe_key:'demo-two',...base}
];
const resume = {id:1,work_id:1,current_state:'交流要点已经留存，正在梳理实践中的共同障碍。',next_step:'准备下一次讨论提纲，保留需要进一步核实的问题。',remember:'先了解障碍，再安排后续工作。',source:'user',created_at:now};
if (new URLSearchParams(location.search).has('long')) {
  for (const entry of [...works,...tasks,...proposals]) entry.title += ' · 跨区域医学沟通与长期研究资料核对'.repeat(7);
  for (const item of inbox) item.content += ' 保留完整交流原话和后续方向调整。'.repeat(20);
}
const settings = new Map([['appearance_font_scale',new URLSearchParams(location.search).get('font')||'standard'],['appearance_theme','mist']]);
const showDocuments=new URLSearchParams(location.search).has('documents');
const documentStates=['ready','pending','unsupported','needs_ocr','too_large','failed_parse','failed_encoding','ready','ready','ready','ready','failed_parse'];
const documentFixtures=documentStates.map((extract_status,index)=>({id:index+1,relative_path:`演示资料-${index+1}${index===11?' · 较长的研究资料说明及后续讨论记录'.repeat(3):''}.pdf`,extract_status,error_message:extract_status==='failed_parse'?'演示文件暂时无法读取，原文件保持完整，可重新读取。':null}));
let callbackId=0;
const jobs=[];
let generatedBrief=null;
const finishJobs=()=>{
  for(const job of jobs)if(job.status==='running'&&Date.now()-job.started>9000){
    job.status=new URLSearchParams(location.search).has('brief-error')?'failed':'completed';
    job.finished_at=Math.floor(Date.now()/1000);
    job.error=job.status==='failed'?'演示服务暂时不可用，请重试。':null;
    if(job.status==='completed')generatedBrief=job.result;
  }
};
// In-memory browser fixture only. Controls deliberately model races without native IO.
const params=new URLSearchParams(location.search);
const clone=value=>value==null?value:structuredClone(value);
const held=new Map();
const controls={hold(name){if(!held.has(name)){let release;const promise=new Promise(r=>release=r);held.set(name,{promise,release});}},release(name){held.get(name)?.release();held.delete(name);},editTask(id,patch){Object.assign(tasks.find(t=>t.id===id),patch);}};
window.__MSL_FIXTURE__=controls;
resume.id=41;
if(!empty){
  works.push({...works[0],id:4,title:'演示 · 已归档项目',status:'archived',archived_at:now});
  for(const [index,status] of ['waiting','paused','done'].entries())tasks.push({...tasks[0],id:4+index,title:`演示 · ${status}事项`,status,completed_at:status==='done'?now:null});
  tasks.push({...tasks[0],id:7,work_id:3,title:'演示 · 暂停项目中的事项'}, {...tasks[0],id:8,work_id:4,title:'演示 · 归档项目中的事项'});
  proposals[1] && (proposals[1].analysis_run_id=2);
}
const experts=empty?[]:[1,2].map(id=>({id,revision:1,name:id===1?'演示专家 · 项目关联':'演示专家 · 独立交流',institution:'示例医疗机构',department:'临床研究科室',specialty:'长期随访',summary:'演示：关注实践与证据需求。',archived:0,project_ids_json:id===1?'[1]':'[]',note_count:0,last_contact:null,...base}));
const notes=[],sessions=[],turns=[],receipts=[];
let nextReceipt=1,deleteConflict=params.has('delete-conflict');
const source={id:'task:1',kind:'task',entity_id:1,title:tasks[0]?.title??'演示任务',text:'保留专家原话',timestamp:now,trust:'user',hash:'synthetic',location:{entity_kind:'task',entity_id:1,work_id:1,available:true}};
const reports=empty?[]:[1,2].map(id=>({
  id,kind:'weekly',period_start:now-604800,period_end:now,status:'completed',provider_model_id:null,
  content:id===1?'历史周报：已记录交流要点。':'本周交流要点与待核实问题。',snapshot_hash:'synthetic',source_counts_json:'{"task":2}',source_report_ids_json:'[]',error_code:null,error_message:null,retention_state:'kept',generated_at:now,...base,
  structured_json:id===1?null:JSON.stringify({version:'report-spec-v2',items:[{
    category:'progress',project_id:1,headline:'整理交流要点',change:'已保留原话',impact:'便于下次跟进',next_action:'核实实践障碍',certainty:'observed',horizon:'current',
    evidence_refs:[
      {source_type:'task_open',entity_id:1,workspace_id:null,relative_path:null,content_hash:null,timestamp:now},
      {source_type:'task_open',entity_id:999,workspace_id:null,relative_path:null,content_hash:null,timestamp:now}
    ]
  }]}),
  evidence_json:id===1?null:JSON.stringify({sources:[
    {source_type:'task_open',entity_id:1,workspace_id:null,relative_path:null,content_hash:null,timestamp:now,location:source.location},
    {source_type:'task_open',entity_id:999,workspace_id:null,relative_path:null,content_hash:null,timestamp:now,trust:'deleted',location:{entity_kind:'task',entity_id:999,available:false}}
  ],coverage_notes:[],source_counts:{task:2}})
}));
const routes=[{task_kind:'workbench_qa',provider_model_id:1,updated_at:now}];
const find=(rows,id)=>{const row=rows.find(r=>r.id===id);if(!row)throw new Error('演示记录不存在');return row;};
const same=(a,b)=>JSON.stringify(Object.entries(a??{}).sort())===JSON.stringify(Object.entries(b??{}).sort());
const scoped=(rows,args)=>rows.filter(r=>(args.workId==null||r.work_id===args.workId)&&(args.status==null||r.status===args.status));
async function dispatch(name,args={}) {
  if(name==='list_provider_connections')return [{id:1,display_name:'合成预览 · 无网络',provider_type:'openai_compatible',base_url:'https://synthetic.invalid',legacy_model:'synthetic',enabled:true,template_kind:'custom',auth_mode:'none',models_endpoint:null,last_models_refresh_at:null,...base}];
  if(name==='list_provider_models')return [{id:1,provider_id:1,model_id:'synthetic',display_name:'合成响应',protocol:'chat_completions',endpoint_path:'/never-called',capabilities_json:'{}',source:'fixture',enabled:true,available:true,...base}];
  if(name==='list_ai_task_routes')return routes;
  if(name==='save_ai_task_route'){if(args.providerModelId!==1)throw new Error('仅支持合成模型');const existing=routes.find(r=>r.task_kind===args.taskKind);if(existing)existing.provider_model_id=1;else routes.push({task_kind:args.taskKind,provider_model_id:1,updated_at:now});return null;}
  if(name==='provider_has_key')return false;
  if(['complete_task','resolve_waiting'].includes(name)){
    const task=name==='complete_task',row=find(task?tasks:waiting,args.id),before=clone(row);
    if(row.status===(task?'done':'resolved'))throw new Error('已经完成');
    row.status=task?'done':'resolved';row[task?'completed_at':'resolved_at']=now;row.updated_at++;
    const receipt={id:`manual-${nextReceipt++}`,entity_kind:task?'task':'waiting',entity_id:row.id,title:row.title,created_at:now,undone_at:null};
    receipts.unshift({receipt,before,after:clone(row)});return receipt;
  }
  if(name==='list_manual_completions')return receipts.map(r=>r.receipt);
  if(name==='undo_manual_completion'){
    const record=receipts.find(r=>r.receipt.id===args.receiptId);if(!record||record.receipt.undone_at)throw new Error('无法重复撤销');
    const row=find(record.receipt.entity_kind==='task'?tasks:waiting,record.receipt.entity_id);
    if(!same(row,record.after))throw new Error('记录已经变化，请刷新');
    const version=row.updated_at;Object.assign(row,record.before,{updated_at:version+1});record.receipt.undone_at=now;return null;
  }
  if(['delete_task','delete_waiting'].includes(name)){
    const rows=name==='delete_task'?tasks:waiting,row=find(rows,args.id);
    if(deleteConflict){deleteConflict=false;row.notes+=' · 另一窗口补充的内容';}
    if(args.confirmed!==true||args.expectedUpdatedAt!==row.updated_at||!same(args.expectedRecord,row))throw new Error('记录已变化，请刷新后重新确认');
    rows.splice(rows.indexOf(row),1);return null;
  }
  if(name==='list_kol_experts')return experts;
  if(name==='list_kol_notes')return notes.filter(n=>args.expertId==null||n.expert_id===args.expertId);
  if(name==='capture_kol_note'){
    const expert=find(experts,args.expertId);if(!args.content?.trim())throw new Error('请输入交流原话');
    const note={id:notes.length+1,expert_id:expert.id,work_id:args.workId??null,inbox_id:args.inboxId??null,content:args.content,occurred_at:args.occurredAt??now,created_at:now,name:expert.name,institution:expert.institution,department:expert.department,project_title:works.find(w=>w.id===args.workId)?.title??null};notes.push(note);expert.note_count++;expert.last_contact=note.occurred_at;return note;
  }
  if(name==='save_kol_expert'){
    const row=args.id==null?{id:Math.max(0,...experts.map(e=>e.id))+1,revision:0,summary:'',note_count:0,last_contact:null,...base}:find(experts,args.id);
    if(args.id!=null&&args.revision!==row.revision)throw new Error('专家记录已经变化');
    Object.assign(row,{name:args.name,institution:args.institution,department:args.department,specialty:args.specialty,project_ids_json:JSON.stringify(args.projects??[]),archived:args.archived?1:0,revision:row.revision+1});if(args.id==null)experts.push(row);return row;
  }
  if(name==='delete_kol_expert'){
    const row=find(experts,args.id);if(args.confirmationName!==row.name||args.expectedRevision!==row.revision)throw new Error('删除确认无效');
    experts.splice(experts.indexOf(row),1);for(const s of sessions)if(s.expert_id===row.id)s.expert_id=null;return {pending_cleanup:0};
  }
  if(name==='create_qa_session'){
    const expert=args.expertId==null?null:find(experts,args.expertId);
    const scope=[...new Set([...(args.scope??[]),...JSON.parse(expert?.project_ids_json??'[]')])].sort((a,b)=>a-b);
    const session={id:Math.max(0,...sessions.map(s=>s.id))+1,title:args.title||'演示问答',scope_json:JSON.stringify(scope),expert_id:expert?.id??null,expert_scoped:expert?1:0,expert_label:expert?.name??null,...base};sessions.push(session);return session;
  }
  if(name==='list_qa_sessions')return sessions;
  if(name==='list_qa_turns')return turns.filter(t=>t.session_id===args.sessionId);
  if(name==='queue_qa_question'){
    const session=find(sessions,args.sessionId);if(session.expert_scoped&&!experts.some(e=>e.id===session.expert_id))throw new Error('此会话的专家已删除');
    if((args.expertId??null)!==session.expert_id)throw new Error('专家范围不匹配');
    if(turns.some(t=>t.session_id===session.id&&['pending','running'].includes(t.status)))throw new Error('当前会话还有一轮处理中');
    const turn={id:turns.length+1,session_id:session.id,question:args.question,scope_json:session.scope_json,expert_id:session.expert_id,expert_scoped:session.expert_scoped,expert_label:session.expert_label,status:'pending',answer_json:null,evidence_json:null,error:null,created_at:now,updated_at:now};turns.push(turn);return turn;
  }
  if(name==='delete_qa_session'){const row=find(sessions,args.id);sessions.splice(sessions.indexOf(row),1);return null;}
  if(name==='ask_workbench'){
    const turn=find(turns,args.turnId),session=find(sessions,turn.session_id);if(session.expert_scoped&&!session.expert_id)throw new Error('专家已删除');
    const sources=session.expert_scoped?notes.filter(n=>n.expert_id===session.expert_id).map(n=>({id:`kol_note:${n.id}`,kind:'kol_note',entity_id:n.id,title:'交流原话',text:n.content,timestamp:n.occurred_at,trust:'user',hash:'synthetic',location:{entity_kind:'kol_note',entity_id:n.id,expert_id:n.expert_id,work_id:n.work_id,available:true}})):[source];
    turn.status='completed';turn.answer_json=JSON.stringify({claims:sources.length?[{text:'建议保留原话并核实实践障碍。',basis:'inference',citations:sources.map(s=>({source_id:s.id,quote:s.text}))}]:[],gaps:sources.length?[]:['尚无交流原话，请先记录。']});turn.evidence_json=JSON.stringify({sources,as_of:now,scope_ids:JSON.parse(session.scope_json),counts:{},omitted:0,notes:[]});return turn;
  }
  if(name==='list_reports')return reports;
  if(name==='generate_report'){
    if(!['weekly','monthly'].includes(args.kind))throw new Error('报告类型无效');
    const row={...clone(reports.find(r=>r.structured_json)),id:Math.max(0,...reports.map(r=>r.id))+1,kind:args.kind,period_start:args.periodStart??now-604800,period_end:args.periodEnd??now};
    if(!row.structured_json)throw new Error('空白预览未配置报告证据');reports.unshift(row);return row.id;
  }
  if(name==='get_report')return find(reports,args.id);
  if(name==='search'){
    const matches=row=>JSON.stringify(row).toLowerCase().includes((args.query??'').toLowerCase());
    return {works:works.filter(matches),tasks:tasks.filter(matches),waiting:waiting.filter(matches),calendar:calendar.filter(matches),inbox:inbox.filter(matches),resume_points:empty?[]:[resume],activity:[],files:empty?[]:[{path:'X:\\SyntheticOnly\\资料目录\\演示资料\\随访问题讨论提纲.docx',label:'演示 · 随访问题讨论提纲',work_id:1}]};
  }
  if(name==='search_all')return empty?[]:[{kind:'resume',id:41,work_id:1,title:'演示推进记录',snippet:resume.current_state},{kind:'file',id:51,work_id:1,title:'演示资料/随访问题讨论提纲.docx',snippet:'演示资料',path:'X:\\SyntheticOnly\\资料目录\\演示资料\\随访问题讨论提纲.docx',relative_path:'演示资料/随访问题讨论提纲.docx',workspace_id:1}];
  if(name==='get_work')return works.find(w=>w.id===args.id)??null;
  if(name==='list_tasks')return scoped(tasks,args);
  if(name==='list_waiting_items'||name==='list_waiting')return scoped(waiting,args);
  if(name==='list_works')return works.filter(w=>args.status==null||w.status===args.status);
  if(name==='get_work_detail'){const work=find(works,args.id),point=work.id===1?resume:null;return {work,latest_resume:point,resume_history:point?[point]:[],files:[],tasks:scoped(tasks,{workId:work.id}),waiting:scoped(waiting,{workId:work.id}),calendar:scoped(calendar,{workId:work.id}),recent_activity:[]};}
  if(name==='get_latest_resume_point')return !empty&&(args.workId==null||args.workId===1)?resume:null;
  if(name==='list_resume_points')return !empty&&(args.workId==null||args.workId===1)?[resume]:[];
  if(['list_latest_analysis_proposals','list_ai_proposals','list_recent_ai_proposals'].includes(name))return scoped(proposals,args).filter(p=>args.runId==null||p.analysis_run_id===args.runId);
  if(['reject_ai_proposal','defer_ai_proposal','update_ai_proposal_draft','update_ai_proposal_classification','confirm_ai_proposal'].includes(name)){
    const row=find(proposals,args.id);if(row.updated_at!==args.expectedUpdatedAt||row.status!=='pending')throw new Error('建议已经变化');
    if(name==='confirm_ai_proposal')throw new Error('合成预览未实现建议确认，请使用手动新增事项');
    if(name==='reject_ai_proposal'){row.status='rejected';row.reason=args.reason??row.reason;}
    else if(name==='defer_ai_proposal')row.status='deferred';
    else {row.title=args.title;row.payload_json=JSON.stringify(args.payload);if(name==='update_ai_proposal_classification'){row.kind=args.kind;row.work_id=args.workId;}}
    row.updated_at++;return name==='reject_ai_proposal'?null:row;
  }
  if(name==='create_task'){const row={id:Math.max(0,...tasks.map(t=>t.id))+1,work_id:args.workId??null,title:args.title,status:args.status??'next',priority:args.priority??'normal',notes:args.notes??'',due_at:args.dueAt??null,scheduled_start:null,scheduled_end:null,completed_at:null,...base};tasks.push(row);return row;}
  if(name==='create_inbox_item'){const row={id:inbox.length+1,content:args.content,created_at:now,processed_at:null,converted_to_type:null,converted_to_id:null};inbox.push(row);return row;}
  if(name==='start_ai_job'&&['ask_workbench','generate_report'].includes(args.request.command)){
    const job={id:jobs.length+1,command:args.request.command,args:args.request.args,status:'running',result:null,error:null,created_at:now,finished_at:null};jobs.push(job);
    void invoke(job.command,job.args).then(result=>Object.assign(job,{status:'completed',result,finished_at:now}),error=>Object.assign(job,{status:'failed',error:error.message,finished_at:now}));return job;
  }
  if (name==='app_settings_get') return settings.get(args.key)??null;
  if (name==='app_settings_set') {settings.set(args.key,args.value);return null;}
  if (name==='list_works') return works;
  if (name==='get_work') return works.find(w=>w.id===args.id)??works[0]??null;
  if (name==='get_work_detail') return {work:works.find(w=>w.id===args.id),latest_resume:resume,resume_history:[resume],files:[],tasks:tasks.filter(t=>t.work_id===args.id),waiting:waiting.filter(t=>t.work_id===args.id),calendar:calendar.filter(t=>t.work_id===args.id),recent_activity:[]};
  if (name==='get_project_directories') return showDocuments?[{id:1,name:'合成资料目录',root_path:'X:\\SyntheticOnly\\资料目录',projects:[{id:1,title:'演示 · 长期随访医学交流',status:'active'}]}]:[];
  if (name==='list_workspace_documents') return showDocuments?documentFixtures:[];
  if (name==='workspace_document_status') return {ready:5,unsupported:1,failed:3,needs_ocr:1,too_large:1};
  if (['get_workspaces','secretary_round_status'].includes(name)) return [];
  if (name==='get_today') return {continue_works:works.map(work=>({work,latest_resume:resume,last_activity_at:now,files:[]})),today_tasks:tasks,today_calendar:calendar,waiting_followups:waiting,inbox_pending:inbox};
  if (name==='list_tasks') return tasks.filter(t=>!args.workId||t.work_id===args.workId);
  if (name==='list_waiting_items'||name==='list_waiting') return waiting;
  if (name==='list_calendar_events') return calendar;
  if (name==='list_inbox_items'||name==='list_inbox') return inbox;
  if(name==='capture_work_note'){
    if(new URLSearchParams(location.search).has('capture-error'))throw new Error('演示保存失败：'+ '可展开查看完整错误。'.repeat(30));
    const note={id:inbox.length+1,content:args.content,created_at:now,processed_at:null,converted_to_type:null,converted_to_id:null};
    inbox.push(note);return note;
  }
  if (name==='list_latest_analysis_proposals'||name==='list_ai_proposals'||name==='list_recent_ai_proposals') return proposals;
  if (name==='list_resume_points') return empty?[]:[resume];
  if (name==='get_latest_resume_point') return empty?null:resume;
  if (name==='workspace_sync_status') return {root:null,paused:true,baseline_count:0,last_scan:now,last_warning:null};
  if (name==='recent_files') return empty?[]:[{path:'演示资料/随访问题讨论提纲.docx',event_type:'file.modified',display_text:'演示资料已更新',timestamp:now-1800},{path:'演示资料/证据资料摘要.pdf',event_type:'file.created',display_text:'演示资料已新增',timestamp:now-7200}];
  if (name==='get_morning_brief') return generatedBrief??(empty?null:{content:'• 长期随访项目：交流原话已整理，下一步核实共同障碍。\n• 研究讨论：等待资料可用性反馈，暂不安排方案讨论。\n• 您的意见：先明确证据需求，再准备沟通材料。'});
  if (name==='get_classification_memory_stats') return {pattern_count:0,feedback_count:0,accepted_count:0,corrected_count:0,rejected_count:0,updated_at:null};
  if (name==='list_analysis_runs') return empty?[]:[1,2].map(id=>({id,trigger:'manual',status:'completed',period_start:null,period_end:null,provider_model_id:null,started_at:now-600,finished_at:now-570,source_counts_json:'{}',snapshot_hash:'synthetic',summary:'已结合项目与您的调整整理建议。',brief_id:null,error_code:null,error_message:null,created_at:now-600}));
  if (name==='get_analysis_schedule') return {id:1,enabled:false,interval_minutes:180,daily_enabled:false,daily_hour:6,daily_minute:0,last_interval_run_at:null,last_daily_local_date:null,updated_at:now};
  if (name==='sync_status') return {config:{directory:'',enabled:false,interval_minutes:180,dataset_id:'',generation:'',ai_primary:true},state:{phase:'disconnected',last_success:0,last_error:'',last_uploaded:0,last_applied:0,conflicts:0},running:false};
  if (name==='backup_status') return {data_directory:'演示 / 正式数据',cache_directory:'演示 / 可重建缓存',config:{directory:'',enabled:false,interval_minutes:180,keep_count:10},state:{last_success:0,last_file:'',error:'',warning:'',files:[]},running:false,restore_pending:false,has_rollback:false};
  if (name==='get_report_schedule') return {weekly_enabled:false,monthly_enabled:false,weekly_weekday:0,weekly_hour:15,weekly_minute:0,monthly_day:1,monthly_hour:9,monthly_minute:0};
  if (name==='list_kol_experts') return empty?[]:[{id:1,name:'演示专家',institution:'示例医疗机构',department:'临床研究科室',specialty:'长期随访',summary:'演示：关注随访实践与证据需求。',projects:[1],archived_at:null,...base}];
  if (name==='plugin:event|listen') return ++callbackId;
  if (name==='plugin:event|unlisten') return null;
  if (name==='plugin:dialog|open') return null; // Explicitly cancelled; never opens native files.
  if (name==='plugin:opener|open_path'||name==='plugin:opener|reveal_item_in_dir'){controls.lastOpened=clone(args);return null;}
  if (name==='start_ai_job'&&args.request.command==='generate_brief') {
    const {command,args:request}=args.request;
    if(!/^\d{4}-\d{2}-\d{2}$/.test(request.date)||request.periodStart>=request.periodEnd)throw new Error('合成简报请求缺少有效日期。');
    const job={id:jobs.length+1,command,args:request,status:'running',started:Date.now(),created_at:now,finished_at:null,error:null,result:{content:'• 演示简报已更新：先核实随访障碍，再准备下一次交流。\n• 研究讨论：保留等待反馈的安排。',ai_used:true,warning:null,source_counts:{work:3},source_preview:[{type:'work',title:'演示 · 长期随访医学交流'}],period_start:request.periodStart,period_end:request.periodEnd,locale:request.locale}};
    jobs.push(job);return {...job};
  }
  if(name==='list_ai_jobs'){finishJobs();return jobs.map(job=>({...job}));}
  if(name==='get_ai_job'){finishJobs();return {...jobs.find(job=>job.id===args.id)};}
  if (name==='start_ai_job') throw new Error('合成预览不调用模型。');
  if (name.startsWith('list_')||name==='search_all'||name==='get_work_workspace_links') return [];
  throw new Error(`合成预览尚未实现命令：${name}`);
};
export const invoke=async(name,args={})=>{
  const submitted=clone(args);
  if(held.has(name))await held.get(name).promise;
  const delay=Number(params.get('delay')??(params.has('slow')?3000:0));
  if(delay>0&&['capture_kol_note','ask_workbench','list_qa_turns'].includes(name))await new Promise(r=>setTimeout(r,delay));
  return clone(await dispatch(name,submitted));
};
window.__TAURI_INTERNALS__={invoke,transformCallback:()=>++callbackId,unregisterCallback:()=>{},convertFileSrc:p=>p,metadata:{currentWindow:{label:'main'},currentWebview:{label:'main'}}};
window.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener:()=>{}};
