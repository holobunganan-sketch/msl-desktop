// Deliberately synthetic stress data; no provider calls or credential access.
import assert from 'node:assert/strict';
import path from 'node:path';
import fs from 'node:fs';
import {DatabaseSync} from 'node:sqlite';
assert.ok(process.env.MSL_TEST_PROFILE,'An explicit isolated test profile is required');
const root=path.resolve(process.env.MSL_TEST_PROFILE);
for(const [key,part] of Object.entries({APPDATA:'appdata',LOCALAPPDATA:'localappdata',TEMP:'temp',TMP:'temp'}))assert.equal(path.resolve(process.env[key]),path.join(root,part));
const db=new DatabaseSync(path.join(root,'appdata','MSLDesktop','msl-desktop.db'));
db.exec('PRAGMA foreign_keys=ON; BEGIN IMMEDIATE');
const now=Math.floor(Date.now()/1000),insert=(q,...args)=>Number(db.prepare(q).run(...args).lastInsertRowid);
const pack=JSON.stringify({sources:[],as_of:now,scope_ids:[],counts:{},omitted:0,notes:['合成布局资料，无真实证据。']});
try {
 if(db.prepare("SELECT id FROM works WHERE title LIKE '界面合成项目%' LIMIT 1").get()){
  db.prepare("UPDATE provider_settings SET credential_ref='synthetic-ui-spacing-no-key' WHERE display_name LIKE '合成供应商%' AND base_url='http://127.0.0.1:9' AND credential_ref IS NULL").run();
  for(const turn of db.prepare("SELECT id,answer_json FROM qa_turns WHERE question='合成问题：请梳理进展。'").all()){
   const answer=JSON.parse(turn.answer_json);for(const claim of answer.claims){claim.citations??=[];delete claim.source_ids;}
   db.prepare('UPDATE qa_turns SET answer_json=?,evidence_json=? WHERE id=?').run(JSON.stringify(answer),pack,turn.id);
  }
  db.exec('COMMIT');console.log('PASS existing synthetic fixture schema repaired');process.exit(0);
 }
 db.exec('UPDATE analysis_schedule_state SET enabled=0,daily_enabled=0; UPDATE report_schedule_state SET weekly_enabled=0,monthly_enabled=0');
 let first;
 for(let i=0;i<8;i++){
  const id=insert('INSERT INTO works(title,summary,created_at,updated_at) VALUES(?,?,?,?)',`界面合成项目 ${i+1} · 跨部门合作与长期学术交流的资料整理和计划跟进`,'用于验证长标题与段落换行。'.repeat(12),now,now);first??=id;
  insert('INSERT INTO resume_points(work_id,current_state,next_step,remember,created_at) VALUES(?,?,?,?,?)',id,'合成进展：资料已准备完成。'.repeat(15),'下一步核对日程并协调回应。'.repeat(7),'仅用于界面验收。',now);
  for(let n=0;n<3;n++)insert('INSERT INTO tasks(work_id,title,status,priority,due_at,notes,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?)',id,'合成事项：需要在跨部门讨论前完成研究材料核对与后续行动协调。'.repeat(n+1),n===2?'done':'next',n===0?'high':'normal',now-60,'合成备注。',now,now);
  insert('INSERT INTO waiting_items(work_id,title,waiting_for,started_at,follow_up_at,created_at,updated_at) VALUES(?,?,?,?,?,?,?)',id,'合成等待：确认学术交流日程及跨团队材料反馈。'.repeat(3),'合成协作方',now-86400,now,now,now);
  insert('INSERT INTO inbox_items(content,created_at) VALUES(?,?)','合成原始记录：准备项目材料，确认下一轮时间与沟通安排。'.repeat(10),now);
  insert('INSERT INTO calendar_events(work_id,title,start_at,end_at,location,kind,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?)',id,'合成日程：项目进展核对与学术材料讨论。'.repeat(3),now+i*3600,now+(i+1)*3600,'合成地点','meeting',now,now);
  const expert=insert('INSERT INTO kol_experts(name,institution,department,specialty,summary,created_at,updated_at) VALUES(?,?,?,?,?,?,?)',`合成专家 ${i+1}`,'示例医疗机构 · 跨学科学术研究中心与区域协作项目组','示例临床研究科室','合成研究方向','合成摘要。'.repeat(30),now,now);
  insert('INSERT INTO kol_projects(expert_id,work_id) VALUES(?,?)',expert,id);
  insert('INSERT INTO kol_notes(expert_id,work_id,content,occurred_at,created_at) VALUES(?,?,?,?,?)',expert,id,'合成交流内容，仅用于测试行距与布局。'.repeat(20),now,now);
 }
 const folder=path.join(root,'workspace','synthetic-project-folder');fs.mkdirSync(folder,{recursive:true});
 const workspace=insert('INSERT INTO workspaces(name,root_path,created_at,updated_at) VALUES(?,?,?,?)','合成项目目录',folder,now,now);
 insert('INSERT INTO work_workspace_links(work_id,workspace_id,created_at) VALUES(?,?,?)',first,workspace,now);
 const brief=insert('INSERT INTO daily_briefs(brief_date,generated_at,content) VALUES(?,?,?)',new Date().toISOString().slice(0,10),now,Array.from({length:6},(_,i)=>`• 合成进展 ${i+1}：项目资料与待办事项需要进一步核对。`).join('\n'));
 const run=insert("INSERT INTO analysis_runs(trigger,status,started_at,finished_at,summary,brief_id,created_at) VALUES('manual','completed',?,?,?,?,?)",now,now,'合成分析：查看待确认的下一步安排。',brief,now);
 for(let i=0;i<4;i++){const title=`合成建议 ${i+1}：整理项目材料并确认跟进时间。`.repeat(3);insert("INSERT INTO ai_proposals(analysis_run_id,kind,operation,work_id,dedupe_key,title,payload_json,reason,created_at,updated_at) VALUES(?,'task','create',?,?,?,?,?,?,?)",run,first,`spacing-${i}`,title,JSON.stringify({title,work_id:first,priority:'normal',notes:'合成备注。'.repeat(10)}),'合成依据：需要在确认后继续推进。'.repeat(10),now,now);}
 for(const kind of ['weekly','monthly'])insert("INSERT INTO reports(kind,period_start,period_end,status,content,generated_at,created_at,updated_at) VALUES(?,?,?,'completed',?,?,?,?)",kind,now-7*86400,now,Array.from({length:16},(_,i)=>`${i+1}. 合成报告：项目进展、等待反馈与下一阶段安排。${'这是用于界面测试的段落。'.repeat(20)}`).join('\n\n'),now,now,now);
 for(let i=0;i<4;i++){const session=insert('INSERT INTO qa_sessions(title,created_at,updated_at) VALUES(?,?,?)',`合成问答 ${i+1} · 梳理项目证据和待办安排`,now,now);insert("INSERT INTO qa_turns(session_id,question,scope_json,status,answer_json,evidence_json,created_at,finished_at) VALUES(?,?,'[]','completed',?,?,?,?)",session,'合成问题：请梳理进展。',JSON.stringify({claims:Array.from({length:18},()=>({text:'合成回答：项目资料与下一阶段安排仍需核对。'.repeat(10),basis:'inference',citations:[]})),gaps:['以上均为合成数据，仅用于布局检查。']}),pack,now,now);}
 const provider=insert("INSERT INTO provider_settings(display_name,provider_type,base_url,auth_mode,credential_ref,enabled,created_at,updated_at) VALUES(?,'custom','http://127.0.0.1:9','none','synthetic-ui-spacing-no-key',1,?,?)",'合成供应商 · 仅用于模型列表布局，禁止网络调用',now,now);
 for(let i=0;i<12;i++)insert("INSERT INTO provider_models(provider_id,model_id,display_name,protocol,endpoint_path,source,enabled,available,created_at,updated_at) VALUES(?,?,?,'chat_completions','/chat/completions','manual',1,1,?,?)",provider,`synthetic-${i}`,`合成长模型名称 ${i+1} · Interface Layout Check With An Intentionally Long Name`,now,now);
 db.exec('COMMIT');console.log('PASS synthetic stress fixture saved');
}catch(e){db.exec('ROLLBACK');throw e;}finally{db.close();}
