// Synthetic native IPC regression: no production data or external provider.
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
let requests=0,briefRequests=0,delay=250,currentProject=0,empty=false;
const server=http.createServer(async(req,res)=>{
 try{
  const chunks=[];for await(const c of req)chunks.push(c);
  const body=JSON.parse(Buffer.concat(chunks).toString());
  const input=JSON.parse(body.messages.find(m=>m.role==='user').content);
  if(input.date&&input.tasks_open&&!input.round_tickets){
   briefRequests++;
   const bullets=Array.from({length:11},(_,i)=>`• 合成简报第 ${i+1} 条：仅供布局测试。`).join('\n');
   res.writeHead(200,{'content-type':'application/json'});res.end(JSON.stringify({choices:[{message:{role:'assistant',content:bullets},finish_reason:'stop'}]}));
   return;
  }
  assert.ok(input.round_tickets?.length,'Every secretary request has a reserved round');
  assert.ok(Array.isArray(input.project_catalog),'Project catalog reaches model');
  assert.ok(Array.isArray(input.user_directions),'User directions reach model');
  if(currentProject&&requests===0){
   assert.ok(input.project_catalog.some(project=>project.id===currentProject),'Current project identity reaches model');
   assert.ok(input.user_directions.some(direction=>direction.content?.includes('整理现有证据')),'User-authored direction reaches model');
  }
  requests++;const n=requests;
  const work=input.focused_work?.work?.id??input.round_tickets.find(t=>t.scope.startsWith('work:'))?.scope.split(':')[1];
  if(currentProject)assert.equal(Number(work),currentProject,'Only eligible project sent to model');
  const refs=input.focused_inbox?[input.source_refs.find(r=>r.source_type==='inbox'&&r.entity_id===input.focused_inbox.id)]:[];
  const output={summary:'已核对演示项目进展。',proposals:empty?[]:[{kind:'task',operation:'create',work_id:Number(work),title:`演示安排 · 第 ${n} 轮`,payload:{notes:'合成证据，无真实工作内容'},reason:'根据本轮合成信息整理',source_refs:refs,confidence:.9}]};
  await new Promise(r=>setTimeout(r,delay));
  res.writeHead(200,{'content-type':'application/json'});res.end(JSON.stringify({choices:[{message:{role:'assistant',content:JSON.stringify(output)},finish_reason:'stop'}]}));
 }catch(e){res.writeHead(500);res.end(JSON.stringify({error:'Synthetic mock contract failure'}));console.error(e.message);}
});
await new Promise(r=>server.listen(9597,'127.0.0.1',r));
let browser;
for(const host of ['127.0.0.1','[::1]'])try{browser=await chromium.connectOverCDP(`http://${host}:${process.env.MSL_CDP_PORT}`);break}catch{}
assert.ok(browser,'Native CDP');
const page=browser.contexts()[0].pages()[0];
const errors=[];page.on('pageerror',e=>errors.push(e.message));
const call=(command,args={})=>page.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
const until=async(fn)=>{for(let i=0;i<180;i++){if(await fn())return;await page.waitForTimeout(150);}throw Error('State did not settle');};
const start=(command,args)=>call('start_ai_job',{request:{command,args}});
const done=async job=>{let j;await until(async()=>{j=await call('get_ai_job',{id:job.id});return j.status!=='running'});assert.equal(j.status,'completed',j.error);return j.result;};
const run=async(command,args)=>done(await start(command,args));
const status=async w=>(await call('secretary_round_status',{workId:w,proposalId:null}))[0];
const route=async detail=>{await page.evaluate(detail=>window.dispatchEvent(new CustomEvent('dashboard:navigate',{detail})),detail);await page.waitForTimeout(350);};
try{
 await page.locator('.content-scroll').waitFor();
 assert.equal(path.resolve((await call('backup_status')).data_directory),path.join(profile,'appdata','MSLDesktop'));
 const schedule=await call('get_analysis_schedule');await call('save_analysis_schedule',{schedule:{...schedule,enabled:false,daily_enabled:false}});
 if(process.env.MSL_RESTART_CHECK==='1'){
  const saved=JSON.parse(fs.readFileSync(path.join(artifacts,'persistence.json'),'utf8'));
  await route({view:'today'});
  await page.getByTestId('dashboard-brief-hero').waitFor();
  await page.screenshot({path:path.join(artifacts,'today-default.png')});
  assert.deepEqual(await call('list_works',{status:null}),saved.works);
  assert.deepEqual(await call('list_ai_proposals',{status:null,limit:200}),saved.proposals);
  for(const s of saved.states)assert.equal((await status(Number(s.scope.slice(5)))).state,s.state);
  await run('run_analysis_now',{trigger:'manual'});assert.equal(requests,0);
  await route({view:'matters',section:'review'});
  await page.getByTestId('secretary-round').waitFor();
  await page.screenshot({path:path.join(artifacts,'review-round.png')});
  const visible=await page.locator('.editor-panel').evaluate(el=>({client:el.clientWidth,scroll:el.scrollWidth}));
  assert.ok(visible.scroll<=visible.client+1,'Review round fits its panel');
  console.log('PASS restart: records, opinions, waiting gates persist; no model request');
 }else{
  const w=await call('create_work',{title:'证据协作项目 · 演示',status:'active'});currentProject=w.id;
  const p=await call('save_provider_connection',{id:null,displayName:'Synthetic secretary round',providerType:'custom',baseUrl:'http://127.0.0.1:9597/v1',legacyModel:'',templateKind:'custom',authMode:'bearer',modelsEndpoint:null,enabled:true,apiKey:'synthetic-round-key'});
  const m=await call('save_provider_model',{providerId:p.id,modelId:'mock-rounds',displayName:'合成测试模型',protocol:'chat_completions',endpointPath:'/chat/completions',capabilitiesJson:'{}',source:'manual',enabled:true,available:true});
  for(const taskKind of ['work_draft','global_analysis','daily_brief'])await call('save_ai_task_route',{taskKind,providerModelId:m.id});
  const note=await call('capture_work_note',{content:'整理现有证据，确定下一步沟通安排。',workId:w.id,entityKind:'work',entityId:w.id});
  const now=new Date(),dayStart=new Date(now.getFullYear(),now.getMonth(),now.getDate()),day=Math.floor(dayStart.getTime()/1000);
  await call('generate_brief',{date:`${now.getFullYear()}-${String(now.getMonth()+1).padStart(2,'0')}-${String(now.getDate()).padStart(2,'0')}`,periodStart:day-86400,periodEnd:day,todayStart:day,todayEnd:day+86400,locale:'zh-CN',force:true});
  assert.equal(briefRequests,1);
  await route({view:'works',id:w.id});
  await route({view:'today'});
  await page.getByTestId('dashboard-brief-hero').waitFor();
  await page.locator('.brief-details > summary').click();
  await page.getByTestId('dashboard-brief-highlights').locator('li').first().waitFor();
  assert.equal(await page.getByTestId('dashboard-brief-highlights').locator('li').count(),3);
  await page.locator('.brief-full > summary').click();
  assert.equal(await page.getByTestId('dashboard-brief-summary').locator('li').count(),11);
  for(const [width,height] of [[1440,900],[1024,768]]){
   const cdp=await page.context().newCDPSession(page);await cdp.send('Emulation.setDeviceMetricsOverride',{width,height,deviceScaleFactor:1,mobile:false});
   const geometry=await page.getByTestId('dashboard-brief-hero').evaluate(el=>({client:el.clientWidth,scroll:el.scrollWidth,visible:[...el.querySelectorAll('.today-focus-row')].every(row=>{const r=row.getBoundingClientRect();return r.right<=innerWidth+1&&r.left>=0})}));
   assert.ok(geometry.scroll<=geometry.client+1&&geometry.visible,'Home brief fits viewport');
   await page.screenshot({path:path.join(artifacts,`today-structured-${width}.png`)});await cdp.detach();
  }
  console.log('PASS home shows three brief highlights and preserves all eleven details on demand');
  delay=1400;const a=await start('start_workspace_work_draft',{workspaceId:null,workId:w.id});await until(()=>requests===1);
  await run('organize_inbox_item',{inboxId:note.id});await done(a);assert.equal(requests,1);
  assert.equal((await status(w.id)).state,'waiting_review');
  const original=await call('list_ai_proposals',{status:'pending',limit:200});assert.equal(original.length,1);
  await run('run_analysis_now',{trigger:'interval'});await run('start_workspace_work_draft',{workspaceId:null,workId:w.id});assert.equal(requests,1);
  assert.deepEqual(await call('list_ai_proposals',{status:'pending',limit:200}),original);
  console.log('PASS project/inbox concurrency; held timer/manual runs make zero extra requests and preserve opinion');
  const accepted=await call('confirm_ai_proposal',{id:original[0].id,expectedUpdatedAt:original[0].updated_at,editedPayload:null});
  assert.equal((await status(w.id)).state,'waiting_progress');await run('run_analysis_now',{trigger:'manual'});assert.equal(requests,1);
  await call('complete_task',{id:accepted.target_id});assert.equal((await status(w.id)).state,'ready');
  delay=300;await run('start_workspace_work_draft',{workspaceId:null,workId:w.id});assert.equal(requests,2);
  console.log('PASS confirmation waits; real task completion releases one next round');
  await route({view:'works',id:w.id});
  await page.getByTestId('secretary-complete-round').waitFor();
  for(const [width,height] of [[1440,900],[1280,800],[1024,768]]){
   const cdp=await page.context().newCDPSession(page);await cdp.send('Emulation.setDeviceMetricsOverride',{width,height,deviceScaleFactor:1,mobile:false});
   await page.getByTestId('secretary-round').scrollIntoViewIfNeeded();
   const geometry=await page.getByTestId('secretary-round').evaluate(el=>({width:el.clientWidth,scroll:el.scrollWidth,buttons:[...el.querySelectorAll('button')].map(b=>{const r=b.getBoundingClientRect();return {x:r.x,right:r.right,y:r.y,bottom:r.bottom}})}));
   assert.ok(geometry.scroll<=geometry.width+1);assert.ok(geometry.buttons.every(b=>b.x>=0&&b.right<=width));
   await page.screenshot({path:path.join(artifacts,`secretary-round-${width}.png`)});await cdp.detach();
  }
  await page.getByTestId('secretary-complete-round').click();await page.getByRole('dialog').waitFor();
  await page.screenshot({path:path.join(artifacts,'complete-round-dialog.png')});
  await page.getByTestId('secretary-complete-confirm').click();
  await until(()=>requests===3);await until(async()=>(await status(w.id)).state==='waiting_review');
  assert.ok((await call('list_ai_proposals',{status:null,limit:200})).some(p=>p.status==='completed'));
  console.log('PASS completion UI, retained history, three viewport layouts');
  const b=await call('create_work',{title:'资料准备项目 · 演示',status:'active'});currentProject=b.id;
  delay=1200;const late=await start('run_analysis_now',{trigger:'manual'});await until(()=>requests===4);
  await call('create_resume_point',{workId:b.id,currentState:'已补充新的沟通进展',nextStep:'核对安排',remember:''});
  await route({view:'calendar'});const lateRun=await done(late);
  assert.equal((await call('list_analysis_runs',{limit:30})).find(r=>r.id===lateRun).status,'reused');
  assert.equal((await call('list_ai_proposals',{status:null,limit:200})).filter(p=>p.work_id===b.id).length,0);
  delay=200;empty=true;await run('start_workspace_work_draft',{workspaceId:null,workId:b.id});assert.equal(requests,5);
  await run('run_analysis_now',{trigger:'manual'});assert.equal(requests,5);
  console.log('PASS independent projects, late result rejected, empty round remembered, background survives navigation');
  const saved={works:await call('list_works',{status:null}),proposals:await call('list_ai_proposals',{status:null,limit:200}),states:[await status(w.id),await status(b.id)]};
  fs.writeFileSync(path.join(artifacts,'persistence.json'),JSON.stringify(saved));
 }
 assert.deepEqual(errors,[]);console.log('PASS native secretary flow; no unhandled UI error');
}finally{server.close();await browser.close();}
