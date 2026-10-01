// Native catalog regression. Synthetic credentials and loopback traffic only.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import http from 'node:http';
import {createRequire} from 'node:module';

assert.equal(process.env.MSL_ISOLATED_TEST, '1');
const profile = path.resolve(process.env.MSL_TEST_PROFILE ?? '');
assert.ok(profile.split(path.sep).includes('.test-runtime'));
for (const [key, part] of Object.entries({APPDATA:'appdata', LOCALAPPDATA:'localappdata', TEMP:'temp', TMP:'temp'})) {
  assert.equal(path.resolve(process.env[key] ?? ''), path.join(profile, part));
}
const {chromium} = createRequire(path.join(process.env.MSL_NODE_MODULES, 'runtime.cjs'))('playwright');
let browser;
for (const host of ['127.0.0.1', '[::1]']) {
  try {browser = await chromium.connectOverCDP(`http://${host}:${process.env.MSL_CDP_PORT}`); break;} catch {}
}
assert.ok(browser);
const page = browser.contexts()[0].pages()[0];
const call = (name, args = {}) => page.evaluate(({name,args}) => window.__TAURI_INTERNALS__.invoke(name,args), {name,args});
const go = view => page.evaluate(view => window.dispatchEvent(new CustomEvent('dashboard:navigate', {detail:{view}})), view);
const artifacts = path.join(profile,'artifacts'); fs.mkdirSync(artifacts,{recursive:true});
const receipt = path.join(artifacts,'provider-catalog.json');
const errors = []; page.on('pageerror', e => errors.push(e.message));
let server;
const connectionPayload = connection => ({id:connection.id,displayName:connection.display_name,providerType:connection.provider_type,baseUrl:connection.base_url,legacyModel:connection.legacy_model,templateKind:connection.template_kind,authMode:connection.auth_mode,modelsEndpoint:connection.models_endpoint,enabled:connection.enabled,apiKey:null});
const open = async id => { const summary = page.getByTestId(id); await summary.waitFor({state:'attached'}); if (!await summary.evaluate(el => el.parentElement.open)) await summary.click(); };
const idle = () => page.waitForFunction(() => !document.querySelector('[data-testid="add-provider-connection"]')?.disabled);

try {
  await page.locator('.content-scroll').waitFor();
  assert.equal(path.resolve((await call('backup_status')).data_directory), path.join(profile,'appdata','MSLDesktop'));
  assert.equal(await call('plugin:app|version'), process.env.MSL_EXPECTED_VERSION);
  const schedule = await call('get_analysis_schedule');
  await call('save_analysis_schedule',{schedule:{...schedule,enabled:false,daily_enabled:false}});
  await go('settings'); await page.getByTestId('add-provider-connection').waitFor(); await idle();

  if (process.argv.includes('--reopened')) {
    const saved = JSON.parse(fs.readFileSync(receipt,'utf8'));
    const connections = await call('list_provider_connections');
    for (const id of saved.connections) assert.ok(connections.some(item => item.id === id));
    const models = await call('list_provider_models',{providerId:saved.connections[0]});
    assert.equal(models.find(item => item.id === saved.disabled).enabled,false);
    assert.equal(models.find(item => item.id === saved.manual).protocol,'chat_completions');
    assert.equal((await call('list_ai_task_routes')).find(item => item.task_kind === 'general').provider_model_id,saved.route);
    assert.equal(await page.getByTestId('vendor-fold-OpenCode').evaluate(el=>el.parentElement.open),true);
    assert.equal(await page.getByTestId(`models-fold-${saved.connections[0]}`).evaluate(el=>el.parentElement.open),true);
    assert.equal(await page.getByTestId(`protocol-fold-${saved.connections[0]}-responses`).evaluate(el=>el.parentElement.open),true);
    assert.deepEqual(errors,[]);
    console.log('PASS restart: independent connections, model flags, manual protocol, routes and nested folds persisted');
  } else {
    assert.ok(!fs.existsSync(receipt),'Use a fresh isolated profile');
    const templates = await call('list_provider_templates');
    for (const kind of ['deepseek','opencode_go','opencode_zen','aliyun','volcengine','zhipu','kimi','minimax','tencent','baidu','mimo','siliconflow']) assert.ok(templates.some(t => t.kind === kind),kind);
    assert.ok(templates.length >= 20);
    await assert.rejects(call('create_provider_template',{templateKind:'aliyun_coding',apiKey:null,enabled:true,displayName:'restricted synthetic'}));
    let catalogMode = 'valid';
    const requests = [];
    server = http.createServer(async(req,res) => {
      const chunks=[]; for await (const chunk of req) chunks.push(chunk);
      const input = chunks.length ? JSON.parse(Buffer.concat(chunks)) : null;
      requests.push({method:req.method,url:req.url,headers:req.headers,model:input?.model});
      res.setHeader('content-type','application/json');
      if (req.method === 'GET') {
        if (catalogMode === 'unauthorized') {res.statusCode=401; res.end('{"error":"synthetic-key-must-not-appear"}'); return;}
        if (catalogMode === 'malformed') {res.end('{"data":[{"id":"partial-new"},{}]}'); return;}
        res.end(JSON.stringify({data:['gpt-5.6-luna','glm-5.3','minimax-m2.7','qwen3.8-max','synthetic-future-model'].map(id => ({id,object:'model'}))})); return;
      }
      res.end(JSON.stringify({choices:[{message:{role:'assistant',content:'synthetic response'},finish_reason:'stop'}],output:[{content:[{type:'output_text',text:'synthetic response'}]}],content:[{type:'text',text:'synthetic response'}],stop_reason:'end_turn'}));
    });
    await new Promise(resolve => server.listen(0,'127.0.0.1',resolve));
    const base = `http://127.0.0.1:${server.address().port}/v1`;
    async function add(nickname, key, kind='opencode_go') {
      await page.getByTestId('add-provider-connection').click();
      await page.getByTestId('provider-vendor-select').selectOption('OpenCode');
      await page.getByTestId('provider-access-select').selectOption(kind);
      await page.getByTestId('provider-connection-name').fill(nickname);
      await page.getByTestId('provider-connection-api-key').fill(key);
      await page.locator('.endpoint-settings>summary').click();
      await page.getByTestId('provider-base-url').fill(base);
      await page.getByTestId('provider-models-endpoint').fill(`${base}/models`);
      await page.getByTestId('provider-save-connection').click();
      await page.locator('[role="dialog"]').waitFor({state:'hidden'});
      const saved = (await call('list_provider_connections')).find(item=>item.display_name === nickname);
      assert.ok(saved); assert.equal(saved.base_url,base); return saved;
    }
    const first = await add('合成账户 A','synthetic-first');
    const second = await add('合成账户 B','synthetic-second');
    const zen = await add('合成 Zen','synthetic-zen','opencode_zen');
    assert.notEqual(first.id, second.id); assert.notEqual(first.credential_ref,second.credential_ref);
    assert.equal(await page.getByTestId('vendor-fold-OpenCode').evaluate(el=>el.parentElement.open),false);
    await page.setViewportSize({width:1440,height:900});
    await page.getByTestId('add-provider-connection').scrollIntoViewIfNeeded();
    await page.screenshot({path:path.join(artifacts,'providers-collapsed.png')});
    await open('vendor-fold-OpenCode');
    await open(`models-fold-${first.id}`);
    const allModels = await call('list_provider_models',{providerId:first.id});
    const disabled = allModels.find(m=>m.model_id === 'gpt-5.6-luna');
    const routed = allModels.find(m=>m.model_id === 'glm-5.3');
    await open(`protocol-fold-${first.id}-responses`);
    await page.getByTestId(`model-toggle-${disabled.id}`).click(); await idle();
    await call('save_ai_task_route',{taskKind:'general',providerModelId:routed.id});
    await page.getByTestId(`connection-refresh-${first.id}`).click(); await idle();
    let models = await call('list_provider_models',{providerId:first.id});
    assert.equal(models.find(m=>m.id === disabled.id).enabled,false);
    assert.equal(models.find(m=>m.id === routed.id).enabled,true);
    assert.equal((await call('list_ai_task_routes')).find(r=>r.task_kind === 'general').provider_model_id,routed.id);
    const unknown = models.find(m=>m.model_id === 'synthetic-future-model');
    assert.ok(unknown && !unknown.enabled);
    await open(`protocol-fold-${first.id}-unknown`);
    assert.ok(await page.getByTestId(`model-toggle-${unknown.id}`).isDisabled());
    await page.getByTestId(`model-edit-${unknown.id}`).click();
    await page.getByTestId('provider-model-protocol').selectOption('chat_completions');
    await page.getByTestId('provider-save-model').click();
    await page.locator('[role="dialog"]').waitFor({state:'hidden'});
    await call('refresh_provider_models',{providerId:first.id});
    models = await call('list_provider_models',{providerId:first.id});
    assert.equal(models.find(m=>m.id === unknown.id).protocol,'chat_completions');
    assert.equal(models.find(m=>m.id === unknown.id).source,'manual');
    assert.equal(models.find(m=>m.id === unknown.id).available,true,'Explicit manual confirmation restores a protocol-pending model');
    for (const mode of ['malformed','unauthorized']) {
      catalogMode=mode;
      const before = JSON.stringify(await call('list_provider_models',{providerId:first.id}));
      const message = await call('refresh_provider_models',{providerId:first.id}).then(()=>{throw new Error('Expected catalog error');},e=>String(e));
      assert.ok(!message.includes('synthetic-key-must-not-appear'));
      assert.equal(JSON.stringify(await call('list_provider_models',{providerId:first.id})),before);
    }
    catalogMode='valid';
    await call('refresh_provider_models',{providerId:second.id});
    await call('refresh_provider_models',{providerId:zen.id});
    assert.equal((await call('list_provider_models',{providerId:zen.id})).find(m=>m.model_id === 'qwen3.8-max').protocol,'chat_completions');
    assert.equal(models.find(m=>m.model_id === 'qwen3.8-max').protocol,'anthropic_messages');
    for (const id of ['glm-5.3','minimax-m2.7']) {
      const model = models.find(m=>m.model_id === id);
      assert.match(await call('test_provider_model',{providerModelId:model.id}),/连接成功/);
    }
    const secondModels = await call('list_provider_models',{providerId:second.id});
    await call('test_provider_model',{providerModelId:secondModels.find(m=>m.model_id === 'gpt-5.6-luna').id});
    for (const endpoint of ['/v1/chat/completions','/v1/messages','/v1/responses']) assert.ok(requests.some(r=>r.method==='POST' && r.url===endpoint),endpoint);
    assert.ok(requests.some(r=>r.headers.authorization==='Bearer synthetic-first'));
    assert.ok(requests.some(r=>r.headers.authorization==='Bearer synthetic-second'));
    await call('save_provider_connection',{...connectionPayload(second),authMode:'none'});
    await call('test_provider_model',{providerModelId:secondModels.find(m=>m.model_id === 'gpt-5.6-luna').id});
    assert.equal(requests.at(-1).headers.authorization,undefined);
    assert.equal(requests.at(-1).headers['x-api-key'],undefined);
    for (const [width,height,font] of [[1440,900,'standard'],[1024,768,'xlarge']]) {
      await page.setViewportSize({width,height});
      await page.evaluate(font=>document.documentElement.dataset.fontSize=font,font);
      await page.getByTestId('vendor-fold-OpenCode').scrollIntoViewIfNeeded();
      assert.ok(await page.locator('.provider-area').evaluate(el=>el.scrollWidth-el.clientWidth<=1));
      await page.screenshot({path:path.join(artifacts,`providers-${width}.png`)});
      await page.getByTestId('add-provider-connection').click();
      await page.getByTestId('provider-vendor-select').selectOption('阿里云百炼');
      await page.getByTestId('provider-access-select').selectOption('aliyun_coding');
      assert.ok(await page.getByTestId('provider-save-connection').isDisabled());
      assert.ok(await page.locator('[role="dialog"]').evaluate(el=>el.scrollWidth-el.clientWidth<=1));
      await page.screenshot({path:path.join(artifacts,`provider-picker-${width}.png`)});
      await page.keyboard.press('Escape');
    }
    assert.deepEqual(errors,[]);
    fs.writeFileSync(receipt,JSON.stringify({connections:[first.id,second.id,zen.id],disabled:disabled.id,manual:unknown.id,route:routed.id,templates:templates.length,protocols:3},null,2));
    console.log('PASS native: dropdown + independent accounts, protocol groups, live refresh, failure preservation, manual correction, routes, auth modes and responsive screenshots');
  }
} finally {server?.close(); await browser.close();}
