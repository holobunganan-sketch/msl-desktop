// Native regression: loopback-only models and synthetic records in an isolated profile.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import {createRequire} from 'node:module';
assert.equal(process.env.MSL_ISOLATED_TEST,'1');
const profile=path.resolve(process.env.MSL_TEST_PROFILE);
assert.ok(profile.split(path.sep).includes('.test-runtime'));
for(const [key,part] of Object.entries({APPDATA:'appdata',LOCALAPPDATA:'localappdata',TEMP:'temp',TMP:'temp'}))assert.equal(path.resolve(process.env[key]??''),path.join(profile,part));
const {chromium}=createRequire(path.join(process.env.MSL_NODE_MODULES,'runtime.cjs'))('playwright');
let browser;
for(const host of ['127.0.0.1','[::1]'])try{browser=await chromium.connectOverCDP(`http://${host}:${process.env.MSL_CDP_PORT}`);break;}catch{}
assert.ok(browser);
const page=browser.contexts()[0].pages()[0],errors=[];
page.on('pageerror',e=>errors.push(e.message));
const call=(name,args={})=>page.evaluate(({name,args})=>window.__TAURI_INTERNALS__.invoke(name,args),{name,args});
const go=detail=>page.evaluate(detail=>window.dispatchEvent(new CustomEvent('dashboard:navigate',{detail})),detail);
const artifacts=path.join(profile,'artifacts');fs.mkdirSync(artifacts,{recursive:true});
const receipt=path.join(artifacts,'retained-output.json');
let server,mode='mixed',requests=0;
const raw='先核对交流背景，再决定下一步。\n**模型原话完整保留** <script>window.syntheticInjection=true</script>';
async function start(command,args={}){return call('start_ai_job',{request:{command,args}});}
async function finish(job,status='completed'){
  for(let n=0;n<400;n++){const value=await call('get_ai_job',{id:job.id});if(value.status!=='running'){assert.equal(value.status,status,value.error);return value;}await page.waitForTimeout(150);}
  throw new Error('Synthetic job did not finish');
}
try{
  await page.locator('.content-scroll').waitFor();
  assert.equal(await call('plugin:app|version'),process.env.MSL_EXPECTED_VERSION);
  assert.equal(path.resolve((await call('backup_status')).data_directory),path.join(profile,'appdata','MSLDesktop'));
  const schedule=await call('get_analysis_schedule');await call('save_analysis_schedule',{schedule:{...schedule,enabled:false,daily_enabled:false}});
  if(!process.argv.includes('--reopened')){
    assert.ok(!fs.existsSync(receipt));
    const capture=await call('create_inbox_item',{content:'合成原话：请核对长期随访资料。'});
    const mockErrors=[];
    server=http.createServer(async(req,res)=>{
      try{
        const chunks=[];for await(const chunk of req)chunks.push(chunk);
        const body=JSON.parse(Buffer.concat(chunks));assert.equal(body.model,'synthetic-independent-model');requests++;
        if(mode==='auth'){res.writeHead(401,{'content-type':'application/json'});res.end('{"error":"synthetic auth failure"}');return;}
        const content=mode==='mixed'?JSON.stringify({summary:'一条正常建议与一条归属待核对的观点。',proposals:[{kind:'task',operation:'create',title:'核对长期随访资料',payload:{},source_refs:[{source_type:'inbox',entity_id:capture.id}]},{kind:'kol_insight',operation:'create',title:'专家归属待核对的观点',payload:{expert_id:9999,observation:'待核对观察'},source_refs:[{source_type:'inbox',entity_id:capture.id}]}]}):raw;
        await new Promise(resolve=>setTimeout(resolve,1000));
        res.writeHead(200,{'content-type':'application/json'});res.end(JSON.stringify({choices:[{message:{role:'assistant',content},finish_reason:'stop'}]}));
      }catch(e){mockErrors.push(String(e));res.writeHead(500);res.end('{"error":"synthetic contract failure"}');}
    });
    await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
    const provider=await call('save_provider_connection',{id:null,displayName:'本地合成测试',providerType:'custom',baseUrl:`http://127.0.0.1:${server.address().port}/v1`,legacyModel:'',templateKind:'custom',authMode:'bearer',modelsEndpoint:null,enabled:true,apiKey:'synthetic-not-a-secret'});
    const model=await call('save_provider_model',{providerId:provider.id,modelId:'synthetic-independent-model',displayName:'合成独立模型',protocol:'chat_completions',endpointPath:'/chat/completions',capabilitiesJson:'{}',source:'manual',enabled:true,available:true});
    await call('save_ai_task_route',{taskKind:'general',providerModelId:model.id});
    const pendingJob=await start('run_analysis_now',{trigger:'manual'});
    await go({view:'matters',section:'review'});
    const mixed=await finish(pendingJob);assert.match(mixed.error,/已保留/);assert.equal(requests,1);
    await page.getByTestId('retained-analysis-output').waitFor();
    assert.match(await page.getByTestId('retained-analysis-output').innerText(),/专家归属待核对的观点/);
    const proposals=await call('list_ai_proposals',{status:'pending',limit:200});assert.equal(proposals.length,1);assert.equal(proposals[0].title,'核对长期随访资料');
    assert.equal((await call('list_tasks',{status:null,workId:null})).length,0);
    mode='plain';await call('create_inbox_item',{content:'合成新增原话：请核对本周交流背景。'});
    const plain=await finish(await start('run_analysis_now',{trigger:'manual'}));assert.match(plain.error,/已保留/);assert.equal(requests,2);
    assert.equal((await call('get_analysis_output',{runId:plain.result})).raw_output,raw);
    await finish(await start('run_analysis_now',{trigger:'manual'}));assert.equal(requests,2,'Unchanged input must not trigger another analysis');
    const session=await call('create_qa_session',{title:'合成回答保留测试',scope:[],expertId:null});
    const turn=await call('queue_qa_question',{sessionId:session.id,question:'请概括已有记录',scope:[],expertId:null});
    const answer=await finish(await start('ask_workbench',{turnId:turn.id,locale:'zh-CN'}));assert.equal(answer.result.status,'completed');
    assert.match(answer.result.answer_json,/先核对交流背景/);
    const now=Math.floor(Date.now()/1000);
    const report=await finish(await start('generate_report',{kind:'weekly',periodStart:now-7*86400,periodEnd:now+60}));
    const reports=await call('list_reports',{kind:null,limit:20});const savedReport=reports.find(r=>r.id===report.result);assert.match(savedReport.content,/报告格式或依据待核对/);assert.equal(JSON.parse(savedReport.structured_json).review_required,true);
    mode='auth';await call('create_inbox_item',{content:'合成认证故障测试：这条不应产生回答。'});
    const failed=await finish(await start('run_analysis_now',{trigger:'manual'}),'failed');assert.match(failed.error,/401|认证/);
    fs.writeFileSync(receipt,JSON.stringify({mixed:mixed.result,plain:plain.result,raw,sessionId:session.id,reportId:report.result}));
    assert.deepEqual(mockErrors,[]);
    console.log('PASS native: mixed proposals, plain output, no repeat generation, Q&A/report preservation, real auth failure remains failure');
  }
  const saved=JSON.parse(fs.readFileSync(receipt,'utf8'));
  assert.equal((await call('get_analysis_output',{runId:saved.plain})).raw_output,saved.raw);
  for(const [width,height,font] of [[1440,900,'standard'],[1280,800,'large'],[1024,768,'xlarge']]){
    await page.setViewportSize({width,height});await page.evaluate(font=>document.documentElement.dataset.fontSize=font,font);
    await go({view:'matters',section:'review',runId:saved.plain});
    const panel=page.getByTestId('retained-analysis-output');await panel.waitFor();
    await panel.getByText('先核对交流背景，再决定下一步。',{exact:false}).first().waitFor();
    assert.equal(await page.evaluate(()=>window.syntheticInjection),undefined);
    assert.equal(await panel.locator('script').count(),0);
    assert.ok(await panel.evaluate(el=>el.scrollWidth<=el.clientWidth+1));
    await panel.scrollIntoViewIfNeeded();await page.screenshot({path:path.join(artifacts,`retained-answer-${width}.png`)});
  }
  assert.deepEqual(errors,[]);
  console.log('PASS saved raw output, safe text rendering, responsive review; '+(process.argv.includes('--reopened')?'restart persistence':'initial run'));
}finally{if(server)await new Promise(resolve=>server.close(resolve));await browser.close();}
