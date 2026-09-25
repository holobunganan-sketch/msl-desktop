// Real Tauri IPC and UI regression; only synthetic isolated data and loopback AI.
import assert from 'node:assert/strict';
import path from 'node:path';
import fs from 'node:fs';
import http from 'node:http';
import {createRequire} from 'node:module';
const profile=path.resolve(process.env.MSL_TEST_PROFILE);
assert.ok(profile.includes('.test-runtime'));
for(const [key,part] of Object.entries({APPDATA:'appdata',LOCALAPPDATA:'localappdata',TEMP:'temp',TMP:'temp'}))assert.equal(path.resolve(process.env[key]),path.join(profile,part));
const {chromium}=createRequire(path.join(process.env.MSL_NODE_MODULES,'runtime.cjs'))('playwright');
const artifacts=path.join(profile,'artifacts');fs.mkdirSync(artifacts,{recursive:true});
let requests=0,lastInput,replyMode='new',closedId=null,relatedTask=null;
const server=http.createServer(async(req,res)=>{
 try{
  const chunks=[];for await(const c of req)chunks.push(c);
  const body=JSON.parse(Buffer.concat(chunks).toString());
  const input=JSON.parse(body.messages.find(m=>m.role==='user').content);lastInput=input;requests++;
  const work=Number(input.focused_work?.work?.id??input.round_tickets.find(t=>t.scope.startsWith('work:')).scope.split(':')[1]);
  const proposal={kind:'task',operation:'create',work_id:work,title:`演示洞察 · 第 ${requests} 轮`,payload:{notes:'合成资料，无真实工作数据。'},reason:'根据合成项目记录提出下一步',source_refs:[],confidence:.9};
  if(replyMode==='repeat'){
   assert.ok(!input.focused_work.tasks.some(t=>t.id===relatedTask));
   assert.ok(input.round_history.some(r=>r.closed_opinions.some(p=>p.id===closedId)));
   assert.ok(input.round_history.every(r=>r.opinions.every(p=>p.id!==closedId)));
   proposal.related_proposal_id=closedId;proposal.title='换个说法重复已解决问题';
  }
  const output={summary:'已核对合成项目的当前记录。',proposals:replyMode==='empty'?[]:[proposal]};
  res.writeHead(200,{'content-type':'application/json'});res.end(JSON.stringify({choices:[{message:{role:'assistant',content:JSON.stringify(output)},finish_reason:'stop'}]}));
 }catch(error){console.error(error.message);res.writeHead(500);res.end('{"error":"synthetic assertion failed"}');}
});
await new Promise(r=>server.listen(9598,'127.0.0.1',r));
let browser;
for(const host of ['127.0.0.1','[::1]'])try{browser=await chromium.connectOverCDP(`http://${host}:${process.env.MSL_CDP_PORT}`);break}catch{}
assert.ok(browser);const page=browser.contexts()[0].pages()[0];
const errors=[];page.on('pageerror',e=>errors.push(e.message));
const call=(command,args={})=>page.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
const until=async fn=>{for(let i=0;i<160;i++){if(await fn())return;await page.waitForTimeout(200);}throw Error('State did not settle');};
const run=async(command,args)=>{const job=await call('start_ai_job',{request:{command,args}});let done;await until(async()=>{done=await call('get_ai_job',{id:job.id});return done.status!=='running'});assert.equal(done.status,'completed',done.error);return done.result;};
const route=async detail=>{await page.evaluate(detail=>window.dispatchEvent(new CustomEvent('dashboard:navigate',{detail})),detail);await page.waitForTimeout(400);};
const list=()=>call('list_ai_proposals',{status:null,limit:200});
const round=async id=>(await call('secretary_round_status',{workId:id,proposalId:null}))[0];
try{
 await page.locator('.content-scroll').waitFor();
 assert.equal(path.resolve((await call('backup_status')).data_directory),path.join(profile,'appdata','MSLDesktop'));
 const schedule=await call('get_analysis_schedule');await call('save_analysis_schedule',{schedule:{...schedule,enabled:false,daily_enabled:false}});
 if(process.env.MSL_RESTART_CHECK==='1'){
  const saved=JSON.parse(fs.readFileSync(path.join(artifacts,'lifecycle-persistence.json'),'utf8'));
  assert.deepEqual(await list(),saved.proposals);
  assert.deepEqual(await call('get_work_detail',{id:saved.workId}),saved.detail);
  assert.deepEqual(await call('list_works',{status:null}),saved.works);
  console.log('PASS restart: project records, resolved state and deleted review stay persisted');
 }else{
  const a=await call('create_work',{title:'资料归属项目 · 演示',status:'active'});
  const b=await call('create_work',{title:'后续跟进项目 · 演示',status:'active'});
  const provider=await call('save_provider_connection',{id:null,displayName:'Synthetic lifecycle',providerType:'custom',baseUrl:'http://127.0.0.1:9598/v1',legacyModel:'',templateKind:'custom',authMode:'bearer',modelsEndpoint:null,enabled:true,apiKey:'synthetic-lifecycle-key'});
  const model=await call('save_provider_model',{providerId:provider.id,modelId:'mock-lifecycle',displayName:'合成测试模型',protocol:'chat_completions',endpointPath:'/chat/completions',capabilitiesJson:'{}',source:'manual',enabled:true,available:true});
  for(const taskKind of ['work_draft','global_analysis'])await call('save_ai_task_route',{taskKind,providerModelId:model.id});
  replyMode='empty';await run('start_workspace_work_draft',{workspaceId:null,workId:b.id});assert.equal((await round(b.id)).state,'waiting_progress');
  replyMode='new';await run('start_workspace_work_draft',{workspaceId:null,workId:a.id});
  let p=(await list()).find(p=>p.status==='pending'&&p.work_id===a.id);
  p=await call('update_ai_proposal_classification',{id:p.id,expectedUpdatedAt:p.updated_at,kind:'task',workId:b.id,title:p.title,payload:JSON.parse(p.payload_json)});
  const linked=await call('confirm_ai_proposal',{id:p.id,expectedUpdatedAt:p.updated_at,editedPayload:null});
  assert.equal((await round(b.id)).state,'ready');
  await run('start_workspace_work_draft',{workspaceId:null,workId:b.id});
  assert.ok(lastInput.focused_work.tasks.some(t=>t.id===linked.target_id));
  console.log('PASS reassigned secretary content included in next project model input');
  let insight=(await list()).find(p=>p.status==='pending'&&p.work_id===b.id);
  const adopted=await call('confirm_ai_proposal',{id:insight.id,expectedUpdatedAt:insight.updated_at,editedPayload:null});
  closedId=insight.id;relatedTask=adopted.target_id;
  await route({view:'matters',section:'review',id:insight.id});
  await page.locator('.proposal-item').filter({hasText:insight.title}).click();
  await page.getByTestId('review-resolve').click();await page.getByRole('dialog').waitFor();
  await page.getByTestId('proposal-lifecycle-cancel').click();
  assert.equal((await list()).find(p=>p.id===insight.id).status,'confirmed');
  await page.getByTestId('review-resolve').click();await page.getByTestId('proposal-lifecycle-confirm').click();
  await until(async()=>(await list()).find(p=>p.id===insight.id).status==='resolved');
  await page.locator('.filter-options summary').click();
  await page.locator('.review-filters select').first().selectOption('resolved');
  await page.getByTestId('review-resolved-notice').waitFor();
  assert.equal((await call('get_work_detail',{id:b.id})).tasks.find(t=>t.id===adopted.target_id).status,'done');
  await page.screenshot({path:path.join(artifacts,'resolved-review.png')});
  replyMode='repeat';await run('start_workspace_work_draft',{workspaceId:null,workId:b.id});
  assert.equal((await list()).filter(p=>p.status==='pending'&&p.work_id===b.id).length,0);
  const count=requests;await run('start_workspace_work_draft',{workspaceId:null,workId:b.id});assert.equal(requests,count);
  console.log('PASS checkbox completes linked task; closed insight absent from active input; reworded continuation blocked; idle run uses no model');
  await route({view:'matters',section:'review',id:insight.id});
  await page.locator('.proposal-item').filter({hasText:insight.title}).click();
  await page.getByTestId('review-delete').click();await page.getByRole('dialog').waitFor();
  await page.screenshot({path:path.join(artifacts,'delete-review-dialog.png')});
  await page.getByTestId('proposal-lifecycle-confirm').click();
  await until(async()=>!(await list()).some(p=>p.id===insight.id));
  assert.ok((await call('get_work_detail',{id:b.id})).tasks.some(t=>t.id===adopted.target_id));
  console.log('PASS deleting review keeps adopted item');
  const waiting=await call('create_waiting',{workId:b.id,title:'等待演示回复',waitingFor:'测试联系人',followUpAt:null,notes:null});
  const event=await call('create_calendar_event',{workId:b.id,title:'演示沟通安排',startAt:Math.floor(Date.now()/1000)+3600,endAt:null,allDay:false,kind:'meeting',location:null,notes:null});
  const progress=await call('create_resume_point',{workId:b.id,currentState:'演示进展记录',nextStep:'核对后续事项',remember:''});
  await route({view:'works',id:b.id});await page.getByTestId('work-delete').waitFor();
  for(const [kind,id,key] of [['task',linked.target_id,'tasks'],['waiting',waiting.id,'waiting'],['resume',progress.id,'resume_history'],['calendar',event.id,'calendar']]){
   const button=page.getByTestId(`project-${kind}-delete-${id}`);
   if(kind==='calendar')await button.locator('xpath=ancestor::details').locator('summary').click();
   await button.click();await page.getByRole('dialog').waitFor();
   await page.getByTestId('project-item-delete-cancel').click();
   assert.ok((await call('get_work_detail',{id:b.id}))[key].some(item=>item.id===id));
   await button.click();await page.getByTestId('project-item-delete-confirm').click();
   await until(async()=>!(await call('get_work_detail',{id:b.id}))[key].some(item=>item.id===id));
  }
  console.log('PASS project task, waiting, calendar and single progress deletion, including cancellation');
  for(const [width,height] of [[1440,900],[1024,768]]){
   const cdp=await page.context().newCDPSession(page);await cdp.send('Emulation.setDeviceMetricsOverride',{width,height,deviceScaleFactor:1,mobile:false});
   await page.getByTestId('work-delete').scrollIntoViewIfNeeded();
   await page.screenshot({path:path.join(artifacts,`project-actions-${width}.png`)});
   await page.getByTestId(`project-task-delete-${adopted.target_id}`).click();
   const bounds=await page.getByRole('dialog').evaluate(el=>{const r=el.getBoundingClientRect();return{x:r.x,y:r.y,right:r.right,bottom:r.bottom,client:el.clientWidth,scroll:el.scrollWidth}});
   assert.ok(bounds.x>=0&&bounds.y>=0&&bounds.right<=width+1&&bounds.bottom<=height+1&&bounds.scroll<=bounds.client+1);
   await page.screenshot({path:path.join(artifacts,`delete-item-${width}.png`)});
   await page.getByTestId('project-item-delete-cancel').click();await cdp.detach();
  }
  const detail=await call('get_work_detail',{id:b.id});
  fs.writeFileSync(path.join(artifacts,'lifecycle-persistence.json'),JSON.stringify({workId:b.id,detail,proposals:await list(),works:await call('list_works',{status:null})}));
 }
 assert.deepEqual(errors,[]);console.log('PASS native lifecycle UI and IPC; no unhandled UI errors');
}finally{server.close();await browser.close();}
