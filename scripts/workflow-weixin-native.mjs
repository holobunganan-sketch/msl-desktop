// Synthetic native IPC regression; never runs against an installed user profile.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import {createRequire} from 'node:module';
assert.equal(process.env.MSL_ISOLATED_TEST,'1');
assert.ok(process.env.MSL_TEST_PROFILE);
const profile=path.resolve(process.env.MSL_TEST_PROFILE);
assert.ok(profile.split(path.sep).includes('.test-runtime'));
for(const [key,part] of Object.entries({APPDATA:'appdata',LOCALAPPDATA:'localappdata',TEMP:'temp',TMP:'temp'}))assert.equal(path.resolve(process.env[key]??''),path.join(profile,part));
const {chromium}=createRequire(path.join(process.env.MSL_NODE_MODULES,'runtime.cjs'))('playwright');
let browser;
for(const host of ['127.0.0.1','[::1]'])try{browser=await chromium.connectOverCDP(`http://${host}:${process.env.MSL_CDP_PORT}`);break;}catch{}
assert.ok(browser);
const page=browser.contexts()[0].pages()[0];
const call=(name,args={})=>page.evaluate(({name,args})=>window.__TAURI_INTERNALS__.invoke(name,args),{name,args});
const go=target=>page.evaluate(detail=>window.dispatchEvent(new CustomEvent('dashboard:navigate',{detail})),target);
const artifacts=path.join(profile,'artifacts');fs.mkdirSync(artifacts,{recursive:true});
const receiptPath=path.join(artifacts,'workflow-weixin.json');
const rejected=async(name,args,pattern)=>assert.rejects(call(name,args),error=>pattern.test(String(error)));
let server,requests=0;
try{
  await page.locator('.content-scroll').waitFor();
  await page.setViewportSize({width:1440,height:900});
  assert.equal(path.resolve((await call('backup_status')).data_directory),path.join(profile,'appdata','MSLDesktop'));
  const schedule=await call('get_analysis_schedule');await call('save_analysis_schedule',{schedule:{...schedule,enabled:false,daily_enabled:false}});
  const connection=await call('get_weixin_status');assert.equal(connection.bound,false);assert.equal(connection.enabled,false);
  await rejected('begin_weixin_login',{},/隔离测试环境/);
  assert.equal((await call('get_weixin_status')).bound,false);
  if(process.argv.includes('--reopened')){
    const saved=JSON.parse(fs.readFileSync(receiptPath,'utf8'));
    const task=(await call('list_tasks',{status:null,workId:saved.workId})).find(row=>row.id===saved.taskId);
    assert.equal(task.notes,'Synthetic later edit preserved');
    const proposal=(await call('list_ai_proposals',{status:'pending',limit:200})).find(row=>row.id===saved.proposalId);
    assert.ok(proposal);assert.equal(proposal.deferred_at,null);
    const continuity=(await call('list_inbox_continuity')).find(row=>row.inbox_id===saved.noteId);
    assert.equal(continuity.work_id,saved.workId);assert.ok(continuity.pending_ids.includes(saved.proposalId));
    console.log('PASS restart: exact project, original proposal, undo and device-local unbound WeChat preserved');
  }else{
    assert.ok(!fs.existsSync(receiptPath),'Use a fresh synthetic profile');
    const work=await call('create_work',{title:'合成研究项目 · 工作流校验',status:'active',category:'clinical'});
    const task=await call('create_task',{workId:work.id,title:'合成任务 · 准备会前问题',priority:'normal',dueAt:null,notes:'Synthetic original notes'});
    const text='请把会前问题的备注改为：先核对研究范围，再准备专家交流。';
    const note=await call('capture_work_note',{content:text,workId:work.id,entityKind:'task',entityId:task.id});
    server=http.createServer(async(req,res)=>{
      try{
        const chunks=[];for await(const chunk of req)chunks.push(chunk);
        const body=JSON.parse(Buffer.concat(chunks));
        const snapshot=JSON.parse(body.messages.find(message=>message.role==='user').content);
        const index=snapshot.user_directions.findIndex(direction=>direction.content===text);assert.ok(index>=0);
        const sources=snapshot.source_refs.filter(ref=>ref.source_type==='inbox'&&ref.entity_id===note.id);assert.ok(sources.length);
        requests++;
        const output={summary:'依据原话调整已有任务，保留项目归属。',proposals:[{kind:'task',operation:'update',target_id:task.id,work_id:work.id,title:task.title,reason:'用户明确要求调整已有任务',payload:{notes:text,field_evidence:{notes:{snapshot_path:`/user_directions/${index}/content`,quote:text,basis:'explicit'}}},source_refs:sources}]};
        res.writeHead(200,{'content-type':'application/json'});res.end(JSON.stringify({choices:[{message:{role:'assistant',content:JSON.stringify(output)},finish_reason:'stop'}]}));
      }catch(error){console.error(error.message);res.writeHead(500);res.end('{"error":"synthetic-contract"}');}
    });
    await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
    const provider=await call('save_provider_connection',{id:null,displayName:'Synthetic workflow',providerType:'custom',baseUrl:`http://127.0.0.1:${server.address().port}/v1`,legacyModel:'',templateKind:'custom',authMode:'bearer',modelsEndpoint:null,enabled:true,apiKey:'synthetic-not-a-secret'});
    const model=await call('save_provider_model',{providerId:provider.id,modelId:'synthetic',displayName:'合成测试模型',protocol:'chat_completions',endpointPath:'/chat/completions',capabilitiesJson:'{}',source:'manual',enabled:true,available:true});
    await call('save_ai_task_route',{taskKind:'work_draft',providerModelId:model.id});
    const job=await call('start_ai_job',{request:{command:'start_workspace_work_draft',args:{workspaceId:null,workId:work.id}}});
    let result;for(let n=0;n<300;n++){result=await call('get_ai_job',{id:job.id});if(result.status!=='running')break;await page.waitForTimeout(150);}
    assert.equal(result.status,'completed',result.error);assert.equal(requests,1);
    let proposal=(await call('list_ai_proposals',{status:'pending',limit:200})).find(row=>row.analysis_run_id===result.result);
    assert.ok(proposal);assert.equal(proposal.target_id,task.id);
    let continuity=(await call('list_inbox_continuity')).find(row=>row.inbox_id===note.id);
    assert.equal(continuity.work_id,work.id);assert.ok(continuity.pending_ids.includes(proposal.id));
    await call('defer_ai_proposal',{id:proposal.id,expectedUpdatedAt:proposal.updated_at,correctionNote:'Synthetic direction: confirm the research scope first.'});
    continuity=(await call('list_inbox_continuity')).find(row=>row.inbox_id===note.id);assert.ok(continuity.deferred_ids.includes(proposal.id));
    proposal=(await call('list_ai_proposals',{status:'pending',limit:200})).find(row=>row.id===proposal.id);
    proposal=await call('restore_deferred_proposal',{id:proposal.id,expectedUpdatedAt:proposal.updated_at});
    assert.equal(proposal.deferred_at,null);assert.equal(requests,1,'Restoring must not call AI');
    const before=await call('get_proposal_current_record',{id:proposal.id});assert.equal(before.record.notes,'Synthetic original notes');
    await call('update_task',{id:task.id,workId:work.id,title:task.title,priority:task.priority,dueAt:null,notes:'Synthetic later edit preserved'});
    await rejected('confirm_ai_proposal',{id:proposal.id,expectedUpdatedAt:proposal.updated_at,editedPayload:null,currentRecordToken:before.token,correctionNote:'Synthetic rejected stale decision'},/变化|刷新|更新/);
    const current=await call('get_proposal_current_record',{id:proposal.id});assert.notEqual(current.token,before.token);
    await go({view:'matters',section:'review',id:proposal.id,workId:work.id});
    await page.getByTestId('review-correction-note').waitFor();
    await page.screenshot({path:path.join(artifacts,'review-current-proposed.png'),fullPage:true});
    const accepted=await call('confirm_ai_proposal',{id:proposal.id,expectedUpdatedAt:proposal.updated_at,editedPayload:null,currentRecordToken:current.token,correctionNote:'Synthetic confirmed direction retained'});
    assert.equal((await call('list_tasks',{status:null,workId:work.id})).find(row=>row.id===task.id).notes,text);
    continuity=(await call('list_inbox_continuity')).find(row=>row.inbox_id===note.id);assert.ok(continuity.destinations.some(row=>row.entity_kind==='task'&&row.entity_id===task.id));
    await call('undo_ai_confirmation',{receiptId:accepted.receipt_id});
    assert.equal((await call('list_tasks',{status:null,workId:work.id})).find(row=>row.id===task.id).notes,'Synthetic later edit preserved');
    fs.writeFileSync(receiptPath,JSON.stringify({workId:work.id,taskId:task.id,noteId:note.id,proposalId:proposal.id}));
    console.log('PASS native review: project retained, defer/restore without AI, old/new race guard, confirm/undo, durable destinations');
  }
  const saved=JSON.parse(fs.readFileSync(receiptPath,'utf8'));
  await go({view:'matters',section:'review',id:saved.proposalId,workId:saved.workId});
  await page.getByTestId('review-change-preview').scrollIntoViewIfNeeded();
  await page.getByTestId('review-change-preview').locator('.before').filter({hasText:'Synthetic later edit preserved'}).waitFor();
  await page.screenshot({path:path.join(artifacts,'review-current-proposed.png'),fullPage:true});
  await go({view:'settings'});
  await page.getByTestId('weixin-settings').waitFor();
  await page.locator('a[href="#weixin"]').click();
  await page.getByTestId('weixin-settings').scrollIntoViewIfNeeded();
  const uiErrors=[];page.on('pageerror',error=>uiErrors.push(error.message));
  for(const [width,height,scale] of [[1440,900,1],[1280,800,1],[1024,768,1.15]]){
    await page.setViewportSize({width,height});
    await page.evaluate(scale=>document.documentElement.dataset.fontSize=scale>1?'xlarge':'standard',scale);
    await page.getByTestId('weixin-settings').scrollIntoViewIfNeeded();
    const overflow=await page.getByTestId('weixin-settings').evaluate(el=>el.scrollWidth-el.clientWidth);assert.ok(overflow<=1,`WeChat card overflow ${overflow}`);
    await page.screenshot({path:path.join(artifacts,`weixin-settings-${width}.png`),fullPage:true});
  }
  await page.getByRole('button',{name:'扫码绑定并启用',exact:true}).click();
  await page.getByRole('alert').filter({hasText:'隔离测试环境'}).waitFor();
  assert.equal(await page.locator('.qr img').count(),0,'Test must not create a real login QR');
  assert.deepEqual(uiErrors,[]);
  console.log('PASS native WeChat settings: IPC registered, isolation blocks real login, three widths readable');
}finally{server?.close();await browser.close();}
