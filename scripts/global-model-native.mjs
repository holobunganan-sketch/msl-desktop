// Real native routing and layout regression with loopback AI and synthetic data only.
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
const receiptPath=path.join(artifacts,'global-model.json');
const kinds=['workspace_analysis','work_draft','global_analysis','daily_brief','weekly_report','monthly_report','translation','workbench_qa','kol_analysis'];
const uiErrors=[];page.on('pageerror',error=>uiErrors.push(error.message));
let server;
async function runJob(command,args){
  const job=await call('start_ai_job',{request:{command,args}});
  for(let n=0;n<400;n++){
    const result=await call('get_ai_job',{id:job.id});
    if(result.status!=='running'){assert.equal(result.status,'completed',result.error);return result.result;}
    await page.waitForTimeout(150);
  }
  throw new Error(`Timed out: ${command}`);
}
async function saveSelect(kind,value){
  const select=page.getByTestId(`route-${kind}`);
  await select.selectOption(String(value??''));
  await page.waitForFunction(()=>!document.querySelector('[data-testid="route-general"]').disabled);
  assert.equal((await call('list_ai_task_routes')).find(r=>r.task_kind===kind)?.provider_model_id,value);
}
try{
  await page.locator('.content-scroll').waitFor();
  if(process.env.MSL_EXPECTED_VERSION)assert.equal(await call('plugin:app|version'),process.env.MSL_EXPECTED_VERSION);
  assert.equal(path.resolve((await call('backup_status')).data_directory),path.join(profile,'appdata','MSLDesktop'));
  const schedule=await call('get_analysis_schedule');await call('save_analysis_schedule',{schedule:{...schedule,enabled:false,daily_enabled:false}});
  if(process.argv.includes('--reopened')){
    const saved=JSON.parse(fs.readFileSync(receiptPath,'utf8'));
    const routes=await call('list_ai_task_routes');
    assert.equal(routes.find(r=>r.task_kind==='general').provider_model_id,saved.defaultId);
    assert.equal(routes.find(r=>r.task_kind==='translation').provider_model_id,saved.otherId);
    for(const kind of kinds.filter(k=>k!=='translation'))assert.equal(routes.find(r=>r.task_kind===kind).provider_model_id,null);
    const reports=await call('list_reports',{kind:null,limit:100});
    for(const id of saved.reportIds)assert.ok(reports.some(r=>r.id===id&&r.structured_json));
    console.log('PASS restart: global default, explicit override, inherited tasks and both reports persisted');
  }else{
    assert.ok(!fs.existsSync(receiptPath),'Use a fresh synthetic profile');
    const work=await call('create_work',{title:'合成项目 · 模型路由校验',status:'active',category:'clinical'});
    const task=await call('create_task',{workId:work.id,title:'整理合成会前问题',priority:'normal',dueAt:null,notes:'Synthetic task for model routing verification'});
    const requests=[],mockErrors=[];
    server=http.createServer(async(req,res)=>{
      try{
        const chunks=[];for await(const chunk of req)chunks.push(chunk);
        const body=JSON.parse(Buffer.concat(chunks));requests.push(body.model);
        const input=JSON.parse(body.messages.find(m=>m.role==='user').content);
        const source=input.analysis.source_refs.find(ref=>ref.source_type==='task_open'&&ref.entity_id===task.id);
        assert.ok(source,'Report input must include the current task');
        const output={items:[{category:'progress',project_id:work.id,headline:'合成项目跟进',change:'会前问题尚待整理。',impact:'保留当前跟进状态。',next_action:'核对会前问题。',certainty:'observed',horizon:'current',evidence_refs:[source]}]};
        res.writeHead(200,{'content-type':'application/json'});res.end(JSON.stringify({choices:[{message:{role:'assistant',content:JSON.stringify(output)},finish_reason:'stop'}]}));
      }catch(error){mockErrors.push(error.message);res.writeHead(500);res.end('{"error":"synthetic-contract"}');}
    });
    await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
    const provider=await call('save_provider_connection',{id:null,displayName:'本地合成测试',providerType:'custom',baseUrl:`http://127.0.0.1:${server.address().port}/v1`,legacyModel:'',templateKind:'custom',authMode:'none',modelsEndpoint:null,enabled:true,apiKey:null});
    const createModel=(id,name)=>call('save_provider_model',{providerId:provider.id,modelId:id,displayName:name,protocol:'chat_completions',endpointPath:'/chat/completions',capabilitiesJson:'{}',source:'manual',enabled:true,available:true});
    const defaultModel=await createModel('synthetic-default','合成默认模型');
    const other=await createModel('synthetic-override','合成专用模型');
    for(const kind of kinds)await call('save_ai_task_route',{taskKind:kind,providerModelId:null});
    await go({view:'settings'});await page.getByTestId('route-general').waitFor();
    await saveSelect('general',defaultModel.id);
    await saveSelect('translation',other.id);
    for(const kind of kinds.filter(k=>k!=='translation'))assert.equal(await page.getByTestId(`route-${kind}`).inputValue(),'');
    await saveSelect('general',other.id);
    await page.getByTestId('route-weekly_report').locator('..').getByText('跟随全局 · 本地合成测试 · 合成专用模型',{exact:true}).waitFor();
    assert.equal(await page.getByTestId('route-translation').inputValue(),String(other.id));
    await saveSelect('general',defaultModel.id);
    await saveSelect('daily_brief',other.id);await saveSelect('daily_brief',null);
    await go({view:'qa'});await page.getByTestId('qa-model').waitFor();
    assert.equal(await page.getByTestId('qa-model').inputValue(),'');
    assert.match(await page.getByTestId('qa-model').locator('option:checked').innerText(),/合成默认模型/);
    await page.getByTestId('qa-model').selectOption(String(other.id));
    await page.waitForFunction(()=>!document.querySelector('[data-testid="qa-model"]').disabled);
    await page.getByTestId('qa-model').selectOption('');
    await page.waitForFunction(()=>!document.querySelector('[data-testid="qa-model"]').disabled);
    assert.equal((await call('list_ai_task_routes')).find(r=>r.task_kind==='workbench_qa').provider_model_id,null);
    const now=Math.floor(Date.now()/1000),reportIds=[];
    for(const kind of ['weekly','monthly'])reportIds.push(await runJob('generate_report',{kind,periodStart:now-7*86400,periodEnd:now+60}));
    assert.deepEqual(mockErrors,[]);assert.deepEqual(requests,['synthetic-default','synthetic-default']);
    fs.writeFileSync(receiptPath,JSON.stringify({defaultId:defaultModel.id,otherId:other.id,reportIds}));
    console.log('PASS native routing: default changes preserve overrides; clearing restores inheritance; weekly/monthly requests use global default');
  }
  const measurements=[];
  for(const [width,height,font] of [[1440,900,'standard'],[1280,800,'large'],[1024,768,'xlarge']]){
    await page.setViewportSize({width,height});
    await page.evaluate(font=>document.documentElement.dataset.fontSize=font,font);
    await go({view:'today'});await page.locator('.focus-grid').waitFor();
    await page.waitForTimeout(250);
    const gap=await page.evaluate(()=>document.querySelector('.attention-strip').getBoundingClientRect().top-document.querySelector('.focus-grid').getBoundingClientRect().bottom);
    assert.ok(gap>=0&&gap<=24,`Homepage gap ${gap} at ${width}`);
    assert.equal(await page.getByTestId('manual-completion-feedback').count(),0);
    await page.locator('.focus-grid').screenshot({path:path.join(artifacts,`home-cards-${width}.png`)});
    await page.locator('.attention-strip').scrollIntoViewIfNeeded();
    await page.screenshot({path:path.join(artifacts,`home-gap-${width}.png`)});
    await go({view:'settings'});await page.getByTestId('ai-routing').waitFor();
    await page.getByTestId('ai-routing').scrollIntoViewIfNeeded();
    const layout=await page.getByTestId('ai-routing').evaluate(el=>{
      const header=el.querySelector('header').getBoundingClientRect(),row=el.querySelector('[data-testid="global-model-row"]').getBoundingClientRect();
      return {gap:row.top-header.bottom,overflow:el.scrollWidth-el.clientWidth,selects:[...el.querySelectorAll('select')].map(s=>({width:s.getBoundingClientRect().width,overflow:s.getBoundingClientRect().right-el.getBoundingClientRect().right}))};
    });
    assert.ok(layout.gap<=20,JSON.stringify(layout));assert.ok(layout.overflow<=1);assert.ok(layout.selects.every(s=>s.width>100&&s.overflow<=1));
    await page.locator('.routing-head').evaluate(el=>el.scrollIntoView({block:'start'}));
    await page.screenshot({path:path.join(artifacts,`global-model-top-${width}.png`)});
    await page.getByTestId('route-kol_analysis').scrollIntoViewIfNeeded();
    await page.screenshot({path:path.join(artifacts,`global-model-bottom-${width}.png`)});
    measurements.push({width,height,font,homepageGap:gap,routing:layout});
  }
  assert.deepEqual(uiErrors,[]);
  fs.writeFileSync(path.join(artifacts,'global-model-layout.json'),JSON.stringify(measurements,null,2));
  console.log('PASS native layout: compact gaps and accessible controls at 1440/1280/1024, including larger fonts');
}finally{server?.close();await browser.close();}
