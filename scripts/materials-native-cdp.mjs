// Native release IPC + UI, synthetic profile + loopback server only.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import {createRequire} from 'node:module';
import {createHash} from 'node:crypto';
import {DatabaseSync} from 'node:sqlite';
const root=path.resolve(import.meta.dirname,'..'), profile=path.join(root,'.test-runtime','materials-ui');
for(const [key,part] of Object.entries({APPDATA:'appdata',LOCALAPPDATA:'localappdata',TEMP:'temp',TMP:'temp'}))assert.equal(path.resolve(process.env[key]),path.join(profile,part));
const req=createRequire(path.join(process.env.MSL_NODE_MODULES,'runtime.cjs'));
const {chromium}=req('playwright');
const artifacts=path.join(profile,'artifacts');fs.mkdirSync(artifacts,{recursive:true});
const db=new DatabaseSync(path.join(profile,'appdata','MSLDesktop','msl-desktop.db'));
const rows=(q,...args)=>db.prepare(q).all(...args);
const count=t=>Number(rows('SELECT COUNT(*) AS n FROM '+t)[0].n);
const logs=[],metrics={files:0,protocols:new Set(),history:false,materialEvidence:false,requests:0};
let delay=350;
const server=http.createServer(async(request,response)=>{
 try{
  const chunks=[];for await(const chunk of request)chunks.push(chunk);
  const data=JSON.parse(Buffer.concat(chunks)),proto=data.input?'responses':request.url.includes('/messages')?'anthropic_messages':'chat_completions';
  metrics.protocols.add(proto);metrics.requests++;
  const messages=data.input??data.messages;
  const user=messages.find(m=>m.role==='user'), content=user.content;
  const input=JSON.parse(typeof content==='string'?content:content.find(c=>c.text)?.text??'{}');
  let output;
  if(input.files_in_attachment_order){
   const fileParts=content.filter(c=>!['text','input_text'].includes(c.type));
   assert.equal(fileParts.length,input.files_in_attachment_order.length);
   fileParts.forEach((p,i)=>{const encoded=p.file_data??p.file?.file_data??p.source?.data??p.image_url?.url??p.image_url??p.input_audio?.data;assert.equal(typeof encoded,'string');const bytes=Buffer.from(encoded.includes('base64,')?encoded.split('base64,')[1]:encoded,'base64');assert.equal(createHash('sha256').update(bytes).digest('hex'),input.files_in_attachment_order[i].file_hash);metrics.files++;});
   output={files:input.files_in_attachment_order.map(f=>({id:f.id,segments:['合成模型解读：材料涉及随访证据需求，适用人群与合作条件仍需核实。'],limitations:['这是测试模型解读，未经原文精确定位验证。']}))};
  }else{
   const sources=input.evidence.sources, material=sources.find(s=>s.kind==='kol_material');
   assert.ok(material,'Material evidence must reach the model');metrics.materialEvidence=true;
   const source=JSON.parse(material.text),citations=[{source_id:material.id,quote:source.text.slice(0,80)}];
   if(input.purpose){
    const work=sources.find(s=>s.kind==='work');
    output={summary:'现有资料提示随访证据需求。建议先澄清适用人群，再讨论合作条件。',citations,
     insights:[{title:'把证据需求转为下一次交流的问题',categories:['practice_barrier','evidence_need','research_opportunity'],observation:source.text,implication:'可能需要更贴近实际情境的证据材料。',uncertainty:'这些线索尚需专家确认。',next_question:'哪些随访终点和人群最值得优先讨论？',citations}],
     actions:['prepare','synthesize'].includes(input.purpose)?[]:[{enabled:true,kind:'task',title:'整理下一次交流的证据问题',work_id:work?.entity_id??null,notes:'由用户确认后推进',waiting_for:'',at:null,time_basis:'unknown',time_reason:'',citations}]};
   }else{
    metrics.history ||= Boolean(input.history_context_only?.length);
    output={claims:[{text:'资料中记录了随访证据需求，适用人群和合作条件仍需要进一步核对。',basis:material.trust==='model_reading'?'inference':'fact',citations},{text:'建议下一次交流先澄清关键终点，再决定材料准备与后续合作。',basis:'inference',citations}],gaps:['现有资料尚不能确认专家的最终意向。']};
   }
  }
  await new Promise(r=>setTimeout(r,delay));const text=JSON.stringify(output);
  const body=proto==='responses'?{output:[{type:'message',role:'assistant',content:[{type:'output_text',text}]}]}:proto==='anthropic_messages'?{content:[{type:'text',text}],stop_reason:'end_turn'}:{choices:[{message:{role:'assistant',content:text},finish_reason:'stop'}]};
  response.writeHead(200,{'content-type':'application/json'});response.end(JSON.stringify(body));
 }catch(e){logs.push('Mock contract: '+e.message);response.writeHead(500);response.end('Synthetic request failed');}
});
await new Promise(r=>server.listen(9497,'127.0.0.1',r));
let browser;
const check=(label,ok=true)=>{assert.ok(ok,label);console.log('PASS '+label);};
try{
 for(const endpoint of ['http://127.0.0.1:9459','http://[::1]:9459']){
  try{browser=await chromium.connectOverCDP(endpoint,{timeout:5000});break;}catch{}
 }
 assert.ok(browser,'Native isolated CDP endpoint');
 const page=browser.contexts()[0].pages().find(p=>p.url().includes('tauri'))??browser.contexts()[0].pages()[0];
 page.on('pageerror',e=>logs.push(e.message));page.on('console',m=>{if(m.type()==='error')logs.push(m.text());});
 const cdp=await page.context().newCDPSession(page);
 const call=(command,args={})=>page.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
 const route=async(view,id=null)=>{const detail=['plan','waiting','inbox','review'].includes(view)?{view:'matters',section:view,id}:{view:view==='translate'?'translation':view,id};await page.evaluate(detail=>window.dispatchEvent(new CustomEvent('dashboard:navigate',{detail})),detail);await page.waitForTimeout(300);};
 const start=(command,args)=>call('start_ai_job',{request:{command,args}});
 const done=async(job)=>{for(let i=0;i<200;i++){const j=await call('get_ai_job',{id:job.id});if(j.status!=='running'){assert.equal(j.status,'completed',j.error??JSON.stringify(j));return j;}await page.waitForTimeout(100);}throw Error('Background job timeout');};
 const run=async(command,args)=>(await done(await start(command,args))).result;
 const snap=async(name)=>{await page.screenshot({path:path.join(artifacts,name+'.png')});};
 const size=async(width,height,font='standard')=>{await cdp.send('Emulation.setDeviceMetricsOverride',{width,height,deviceScaleFactor:1,mobile:false});await page.evaluate(f=>document.documentElement.dataset.fontSize=f,font);await page.waitForTimeout(180);};
 await page.getByTestId('nav-qa').waitFor();
 let expert,project,session,provider;
 if(process.argv.includes('--reopen')||process.argv.includes('--layout')){
  expert=(await call('list_kol_experts')).find(e=>e.name==='林知远（演示）');project=(await call('list_works',{status:null})).find(w=>w.title==='随访证据交流（演示项目）');session=(await call('list_qa_sessions'))[0];provider=(await call('list_provider_connections'))[0];
  check('Durable attachments and indices survive restart',count('kol_materials')>=7&&count('material_segments')>=6);
  check('SQLite integrity',rows('PRAGMA integrity_check')[0].integrity_check==='ok'&&!rows('PRAGMA foreign_key_check').length);
 }else{
  assert.equal(count('kol_experts'),0,'Fresh isolated database required');
  let schedule=await call('get_analysis_schedule');await call('save_analysis_schedule',{schedule:{...schedule,enabled:false,daily_enabled:false}});
  project=await call('create_work',{title:'随访证据交流（演示项目）'});
  const b=await call('create_work',{title:'独立合作项目（演示）'});
  expert=await call('save_kol_expert',{id:null,revision:null,name:'林知远（演示）',institution:'示例医学中心',department:'研究协作科',specialty:'证据交流与随访研究',projects:[project.id],archived:false});
  provider=await call('save_provider_connection',{id:null,displayName:'本地合成模型 · 隔离验证',providerType:'custom',baseUrl:'http://127.0.0.1:9497/v1',legacyModel:'',templateKind:'custom',authMode:'none',modelsEndpoint:null,enabled:true,apiKey:null});
  const models={};
  for(const protocol of ['responses','chat_completions','anthropic_messages']){
   models[protocol]=await call('save_provider_model',{providerId:provider.id,modelId:'mock-'+protocol,displayName:'演示模型 · '+protocol,protocol,endpointPath:protocol==='responses'?'/responses':protocol==='anthropic_messages'?'/messages':'/chat/completions',capabilitiesJson:'{}',source:'manual',enabled:true,available:true});
  }
  for(const taskKind of ['kol_analysis','workbench_qa'])await call('save_ai_task_route',{taskKind,providerModelId:models.responses.id});
  const files=fs.readdirSync(path.join(profile,'workspace')).map(f=>path.join(profile,'workspace',f)),hashes=files.map(f=>createHash('sha256').update(fs.readFileSync(f)).digest('hex'));
  const importJob=await start('import_kol_materials',{expertId:expert.id,paths:files});await route('calendar');await done(importJob);
  let materials=await call('list_kol_materials',{expertId:expert.id});
  check('Seven files durable, including unsupported format',materials.length===7);
  check('Plain text ready; unsupported file retained',materials.find(m=>m.filename.endsWith('.txt')).status==='ready'&&materials.find(m=>m.filename.endsWith('.custom')).status==='unsupported');
  check('Images, scan PDF and Office content actually sent',metrics.files>=5);
  check('Background import finishes after page switch',materials.every(m=>!['reading','queued'].includes(m.status)));
  const image=materials.find(m=>m.filename.endsWith('.png'));
  for(const protocol of ['chat_completions','anthropic_messages']){
   await call('save_ai_task_route',{taskKind:'kol_analysis',providerModelId:models[protocol].id});
   await run('read_kol_materials',{expertId:expert.id,ids:[image.id],force:true});
  }
  check('All three real adapters transmit exact source bytes',metrics.protocols.size===3);
  await call('save_ai_task_route',{taskKind:'kol_analysis',providerModelId:models.responses.id});
  const segmentIds=rows('SELECT id FROM material_segments WHERE material_id=?',materials.find(m=>m.filename.endsWith('.txt')).id).map(r=>r.id);
  await run('import_kol_materials',{expertId:expert.id,paths:files});
  check('Repeated upload deduplicates without replacing text evidence',count('kol_materials')===7&&JSON.stringify(segmentIds)===JSON.stringify(rows('SELECT id FROM material_segments WHERE material_id=?',materials.find(m=>m.filename.endsWith('.txt')).id).map(r=>r.id)));
  check('Source files never modified',files.every((f,i)=>hashes[i]===createHash('sha256').update(fs.readFileSync(f)).digest('hex')));
  const before=count('tasks');const analyzed=await run('analyze_kol',{expertId:expert.id,purpose:'organize',locale:'zh-CN'});
  let draft=(await call('list_kol_drafts',{expertId:expert.id})).find(d=>d.id===analyzed.draft_id);
  check('Attachments alone produce an editable draft, no auto-created task',draft?.status==='pending'&&count('tasks')===before);
  await call('review_kol_draft',{id:draft.id,revision:draft.revision,decision:'confirm',payload:JSON.parse(draft.payload_json)});
  check('Explicit confirmation creates follow-up once',count('tasks')===before+1);
  await run('analyze_kol',{expertId:expert.id,purpose:'prepare',locale:'zh-CN'});
  await run('analyze_kol',{expertId:null,purpose:'synthesize',locale:'zh-CN'});
  check('Preparation and synthesis use attachment evidence',metrics.materialEvidence);
  session=await call('create_qa_session',{title:'把现有线索梳理清楚',scope:[project.id]});
  for(const question of ['现有资料有哪些可推进的线索？','下一次交流优先核对什么？']){
   const turn=await call('queue_qa_question',{sessionId:session.id,question,scope:[project.id]});
   const job=await start('ask_workbench',{turnId:turn.id,locale:'zh-CN'});await route('kol',expert.id);await done(job);
  }
  check('Continuous grounded Q&A receives prior context',metrics.history&&count('qa_turns')===2);
  const pack=JSON.parse((await call('list_qa_turns',{sessionId:session.id}))[0].evidence_json);
  check('@project includes explicitly linked expert materials',pack.sources.some(s=>s.kind==='kol_material')&&!pack.sources.some(s=>s.kind==='work'&&s.entity_id===b.id));
  const copy=await call('save_kol_expert',{id:null,revision:null,name:expert.name,institution:'另一家演示机构',department:'另一科室',specialty:'',projects:[],archived:false});
  await run('import_kol_materials',{expertId:copy.id,paths:[files.find(f=>f.endsWith('.png'))]});
  let refused=false;try{await call('delete_kol_expert',{id:copy.id,confirmationName:'错误名字',expectedRevision:copy.revision});}catch{refused=true;}
  check('Backend refuses wrong typed name',refused);
  const updated=await call('save_kol_expert',{id:copy.id,revision:copy.revision,name:copy.name,institution:copy.institution,department:'已更新的科室',specialty:'',projects:[],archived:false});
  refused=false;try{await call('delete_kol_expert',{id:copy.id,confirmationName:copy.name,expectedRevision:copy.revision});}catch{refused=true;}check('Concurrent edit invalidates deletion confirmation',refused);
  const copyMaterial=(await call('list_kol_materials',{expertId:copy.id}))[0];delay=1400;
  const late=await start('read_kol_materials',{expertId:copy.id,ids:[copyMaterial.id],force:true});await page.waitForTimeout(250);
  await call('delete_kol_expert',{id:copy.id,confirmationName:' '+updated.name+' ',expectedRevision:updated.revision});await done(late);delay=350;
  check('Late model result never resurrects a deleted expert',!(await call('list_kol_experts')).some(e=>e.id===copy.id)&&count('kol_materials')===7);
  check('Shared file remains with its other expert',fs.existsSync(path.join(profile,'appdata','MSLDesktop','attachments','blobs',rows('SELECT relative_path FROM material_blobs WHERE hash=?',image.blob_hash)[0].relative_path)));
  refused=false;try{await call('delete_work',{id:b.id,confirmationName:'错误',expectedRevision:b.revision});}catch{refused=true;}check('Project backend requires exact title',refused);
  await call('delete_work',{id:b.id,confirmationName:b.title,expectedRevision:b.revision});
  const cleanup=await call('preview_storage_cleanup',{categories:null});await call('execute_storage_cleanup',{planId:cleanup.id??cleanup.plan_id});
  check('Cache cleanup retains all durable materials and task',count('kol_materials')===7&&count('tasks')===before+1);
  fs.writeFileSync(path.join(artifacts,'mechanical.json'),JSON.stringify({files:metrics.files,protocols:[...metrics.protocols],history:metrics.history,materialEvidence:metrics.materialEvidence},null,2));
 }
 if(!count('waiting_items'))await call('create_waiting',{workId:project.id,title:'等待确认随访证据的适用范围（演示）',waitingFor:'示例研究团队',followUpAt:null,notes:'用于界面验证的合成事项'});
 if(!count('calendar_events'))await call('create_calendar_event',{workId:project.id,title:'证据交流准备（演示）',startAt:Math.floor(Date.now()/1000)+3600,endAt:null,allDay:false,kind:'meeting',location:'示例地点',notes:'合成日程'});
 if(!count('inbox_items'))await call('create_inbox_item',{content:'合成记录：下次讨论需要核对随访终点、适用人群与合作条件，先保留线索，待确认后再推进。'});
 for(const [width,height,font] of [[1440,900,'standard'],[1024,768,'large']]){
  await size(width,height,font);
  for(const [view,prefix] of [['works','work'],['plan','task'],['waiting','waiting'],['calendar','calendar']]){
   await route(view);await page.getByTestId(prefix+'-create').click();const save=page.getByTestId(prefix+'-save');await save.scrollIntoViewIfNeeded();
   const before=await save.boundingBox();await save.click();await page.waitForTimeout(120);const after=await save.boundingBox();
   check(prefix+' validation retains button position '+width,Math.abs(before.x-after.x)<=1&&Math.abs(before.y-after.y)<=1&&Math.abs(before.width-after.width)<=1);
   await page.keyboard.press('Escape');
  }
 }
 await size(1440,900);await route('qa',session.id);await page.getByTestId('qa-answer').first().waitFor();
 await page.getByTestId('qa-turn-'+(await call('list_qa_turns',{sessionId:session.id})).at(-1).id).scrollIntoViewIfNeeded();
 await snap('01-question-answer');
 await route('kol',expert.id);await page.getByTestId('kol-tab-materials').click();await page.getByTestId('kol-materials').waitFor();await snap('02-expert-materials');
 await page.getByTestId('kol-delete').click();await snap('03-safe-delete');await page.keyboard.press('Escape');
 await route('settings');const fold=page.getByTestId('models-fold-'+provider.id);await fold.scrollIntoViewIfNeeded();
 if(process.argv.includes('--reopen'))check('Model fold preference survived restart',await fold.evaluate(el=>el.parentElement.open));
 else if(!process.argv.includes('--layout')){check('Model catalog initially folded',!await fold.evaluate(el=>el.parentElement.open));await snap('04-folded-models');await fold.click();}
 await snap('05-expanded-models');
 const views=['today','works','plan','waiting','calendar','inbox','review','reports','workspace','qa','kol','translate','settings'];
 const matrix=[];
 for(const [width,height] of [[1440,900],[1280,800],[1024,768]])for(const font of ['standard','large']){
  await size(width,height,font);
  for(const view of views){
   await route(view,view==='qa'?session.id:view==='kol'?expert.id:null);
   const overflow=await page.evaluate(()=>{const s=document.querySelector('.content-scroll');return {document:document.documentElement.scrollWidth-innerWidth,content:s?s.scrollWidth-s.clientWidth:0};});
   const unreachable=await page.evaluate(()=>{
    const bad=[];
    for(const el of document.querySelectorAll('.view-body button,.view-body input,.view-body select,.view-body textarea,.view-body summary')){
     if(!el.checkVisibility({visibilityProperty:true})||el.closest('details:not([open])')&&!el.matches('summary'))continue;
     el.scrollIntoView({block:'nearest',inline:'nearest'});const r=el.getBoundingClientRect();
     if(r.width<1||r.height<1)continue;
     let clipped=r.top<0||r.left<0||r.right>innerWidth+2||r.bottom>innerHeight+2;
     for(let a=el.parentElement;a&&a!==document.body;a=a.parentElement){
      const s=getComputedStyle(a),b=a.getBoundingClientRect();
      if(/hidden|clip|auto|scroll/.test(s.overflowY)&&r.height<=b.height+2&&(r.top<b.top-3||r.bottom>b.bottom+3))clipped=true;
      if(/hidden|clip|auto|scroll/.test(s.overflowX)&&r.width<=b.width+2&&(r.left<b.left-3||r.right>b.right+3))clipped=true;
     }
     const s=getComputedStyle(el),textHeight=el.clientHeight-parseFloat(s.paddingTop)-parseFloat(s.paddingBottom);
     if((el.matches('select')||el.matches('input:not([type=checkbox]):not([type=radio]):not([type=color]):not([type=range])'))&&textHeight<parseFloat(s.fontSize)*.9)clipped=true;
     if(clipped)bad.push({tag:el.tagName,id:el.getAttribute('data-testid'),label:(el.textContent??el.getAttribute('aria-label')??'').trim().slice(0,45)});
    }
    return bad.slice(0,10);
   });
   matrix.push({view,width,height,font,...overflow,unreachable});
   if(unreachable.length)console.log('LAYOUT_UNREACHABLE '+JSON.stringify(matrix.at(-1)));
   if(overflow.document>2||overflow.content>2)console.log('LAYOUT_OVERFLOW '+JSON.stringify(matrix.at(-1)));
   if(['qa','kol'].includes(view))await snap(view+'-'+width+'-'+font);
  }
 }
 fs.writeFileSync(path.join(artifacts,'layout-matrix.json'),JSON.stringify(matrix,null,2));
 check('78 page / window / font combinations without horizontal overflow',matrix.every(r=>r.document<=2&&r.content<=2));
 check('Every rendered control reachable, with legible single-line input height',matrix.every(r=>!r.unreachable.length));
 check('No native UI or mock exceptions',logs.length===0);
 console.log('ARTIFACTS '+artifacts);
}finally{db.close();if(browser)await browser.close();await new Promise(r=>server.close(r));if(logs.length)console.log(JSON.stringify(logs));}
