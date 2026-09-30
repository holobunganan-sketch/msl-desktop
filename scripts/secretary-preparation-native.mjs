// Native IPC + loopback AI contract. Runs only against synthetic isolated data.
import assert from 'node:assert/strict';
import path from 'node:path';
import fs from 'node:fs';
import http from 'node:http';
import {createRequire} from 'node:module';
const profile=path.resolve(process.env.MSL_TEST_PROFILE);
assert.ok(profile.includes('.test-runtime'));
assert.equal(process.env.MSL_ISOLATED_TEST,'1');
for(const [key,part] of Object.entries({APPDATA:'appdata',LOCALAPPDATA:'localappdata',TEMP:'temp',TMP:'temp'}))assert.equal(path.resolve(process.env[key]),path.join(profile,part));
const {chromium}=createRequire(path.join(process.env.MSL_NODE_MODULES,'runtime.cjs'))('playwright');
const artifacts=path.join(profile,'artifacts');fs.mkdirSync(artifacts,{recursive:true});
let browser;
for(const host of ['127.0.0.1','[::1]'])try{browser=await chromium.connectOverCDP(`http://${host}:${process.env.MSL_CDP_PORT}`);break;}catch{}
assert.ok(browser,'Native CDP must be available');
const page=browser.contexts()[0].pages()[0];
const call=(name,args={})=>page.evaluate(({name,args})=>window.__TAURI_INTERNALS__.invoke(name,args),{name,args});
let requests=0,owner,study,server;
const noteText='安排一次交流，准备交流提纲。将本次交流关联到合成临床研究 Alpha。';
try{
  await page.locator('.content-scroll').waitFor();
  assert.equal(path.resolve((await call('backup_status')).data_directory),path.join(profile,'appdata','MSLDesktop'));
  const schedule=await call('get_analysis_schedule');await call('save_analysis_schedule',{schedule:{...schedule,enabled:false,daily_enabled:false}});
  if(process.env.MSL_RESTART_CHECK==='1'){
    const expected=JSON.parse(fs.readFileSync(path.join(artifacts,'preparation-persistence.json'),'utf8'));
    const detail=await call('get_work_detail',{id:expected.studyId});
    assert.equal(detail.work.category,'clinical');
    const linked=detail.tasks.find(task=>task.id===expected.taskId);
    assert.equal(linked.work_id,expected.ownerId);
    assert.equal(linked.scheduled_start,expected.time);
    assert.equal(detail.linked_inbox[0].content,'合成记录：补充会前问题');
    console.log('PASS preparation restart: categories, original ownership, clinical links, times, and unprocessed notes persist');
  }else{
    study=await call('create_work',{title:'合成临床研究 Alpha',status:'active',category:'clinical'});
    owner=await call('create_work',{title:'合成专家交流计划',status:'active',category:'non_clinical'});
    await call('capture_work_note',{content:noteText,workId:owner.id,entityKind:'work',entityId:owner.id});
    const time=Math.floor(Date.now()/1000)+86400;
    server=http.createServer(async(req,res)=>{
      try{
        const chunks=[];for await(const chunk of req)chunks.push(chunk);
        const body=JSON.parse(Buffer.concat(chunks));
        const input=JSON.parse(body.messages.find(message=>message.role==='user').content);
        const index=input.user_directions.findIndex(direction=>direction.content===noteText);assert.ok(index>=0);
        requests++;
        const path=`/user_directions/${index}/content`;
        const evidence=(quote,basis='explicit')=>({snapshot_path:path,quote,basis});
        const output={summary:'根据合成原话准备安排。',proposals:[{kind:'task',operation:'create',work_id:owner.id,title:'准备交流提纲',reason:'依据合成原话，时间仅供确认',payload:{notes:noteText,clinical_work_id:study.id,scheduled_start:time,time_basis:'inferred',time_reason:'待确认的合成时段',field_evidence:{notes:evidence(noteText),clinical_work_id:evidence('将本次交流关联到合成临床研究 Alpha'),scheduled_start:evidence('安排一次交流','suggestion')},unknowns:['具体交流地点尚未提供']},source_refs:[]}]};
        res.writeHead(200,{'content-type':'application/json'});res.end(JSON.stringify({choices:[{message:{role:'assistant',content:JSON.stringify(output)},finish_reason:'stop'}]}));
      }catch(error){res.writeHead(500);res.end('{"error":"synthetic-contract"}');console.error(error.message);}
    });
    await new Promise(resolve=>server.listen(9598,'127.0.0.1',resolve));
    const provider=await call('save_provider_connection',{id:null,displayName:'Synthetic preparation',providerType:'custom',baseUrl:'http://127.0.0.1:9598/v1',legacyModel:'',templateKind:'custom',authMode:'bearer',modelsEndpoint:null,enabled:true,apiKey:'synthetic-not-a-secret'});
    const model=await call('save_provider_model',{providerId:provider.id,modelId:'synthetic',displayName:'合成测试',protocol:'chat_completions',endpointPath:'/chat/completions',capabilitiesJson:'{}',source:'manual',enabled:true,available:true});
    await call('save_ai_task_route',{taskKind:'work_draft',providerModelId:model.id});
    const job=await call('start_ai_job',{request:{command:'start_workspace_work_draft',args:{workspaceId:null,workId:owner.id}}});
    let result;for(let n=0;n<200;n++){result=await call('get_ai_job',{id:job.id});if(result.status!=='running')break;await page.waitForTimeout(150);}
    assert.equal(result.status,'completed',result.error);assert.equal(requests,1,'Valid evidence must need no repair request');
    const proposal=(await call('list_ai_proposals',{status:'pending',limit:200})).find(item=>item.work_id===owner.id);
    assert.ok(proposal,'Evidence-backed suggestion must reach editable review');
    const payload=JSON.parse(proposal.payload_json);
    assert.equal(payload.clinical_work_id,study.id);assert.equal(payload.unknowns.length,1);
    await assert.rejects(call('confirm_ai_proposal',{id:proposal.id,expectedUpdatedAt:proposal.updated_at,editedPayload:null}),/时间/);
    assert.equal((await call('get_work_detail',{id:owner.id})).tasks.length,0,'Unconfirmed time writes no task');
    const accepted=await call('confirm_ai_proposal',{id:proposal.id,expectedUpdatedAt:proposal.updated_at,editedPayload:{...payload,time_confirmation:'user_confirmed'}});
    const detail=await call('get_work_detail',{id:study.id});
    const task=detail.tasks.find(item=>item.id===accepted.target_id);
    assert.equal(task.work_id,owner.id);assert.equal(task.scheduled_start,time);assert.equal(task.notes,noteText);
    const original=await call('get_work_detail',{id:owner.id});assert.equal(original.tasks[0].id,task.id,'Both projects point to one task');
    const note=await call('capture_work_note',{content:'合成记录：补充会前问题',workId:owner.id,entityKind:'work',entityId:owner.id});
    const link=await call('set_clinical_link',{entityKind:'inbox',entityId:note.id,clinicalWorkId:study.id,expectedRevision:0});
    assert.ok(link.revision>0);
    await assert.rejects(call('set_clinical_link',{entityKind:'inbox',entityId:note.id,clinicalWorkId:null,expectedRevision:0}),/刷新/);
    assert.equal((await call('get_work_detail',{id:study.id})).linked_inbox[0].id,note.id);
    await page.evaluate(id=>window.dispatchEvent(new CustomEvent('dashboard:navigate',{detail:{view:'works',id}})),study.id);
    await page.locator('.project-classification').waitFor();
    await page.screenshot({path:path.join(artifacts,'native-clinical-project.png'),fullPage:true});
    fs.writeFileSync(path.join(artifacts,'preparation-persistence.json'),JSON.stringify({studyId:study.id,ownerId:owner.id,taskId:task.id,time}));
    console.log('PASS native preparation: evidence reaches review, generic time adoption rejected, explicit consent saves one linked task, stale linking rejected');
  }
}finally{server?.close();await browser.close();}
