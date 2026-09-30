import test,{before,after,beforeEach,afterEach} from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {createServer} from 'vite';
import path from 'node:path';

if(process.env.MSL_ISOLATED_TEST!=='1')throw new Error('Isolated execution required');
for(const key of ['APPDATA','LOCALAPPDATA','TEMP','TMP'])if(!process.env[key]?.includes('iteration-20260930'))throw new Error(`Isolated ${key} required`);
const require=createRequire(path.join(process.env.MSL_NODE_MODULES,'package.json'));
const {chromium}=require('playwright');
let server,browser,page,base;
before(async()=>{
  server=await createServer({server:{host:'127.0.0.1',port:0,strictPort:false,open:false,hmr:false,fs:{allow:[process.cwd()]}},logLevel:'error',plugins:[{
    name:'homepage-capture-regression',configureServer(vite){vite.middlewares.use(async(req,res,next)=>{
      if(!req.url?.startsWith('/__capture-test'))return next();
      const html=await vite.transformIndexHtml('/__capture-test','<!doctype html><html lang="zh-CN"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1"><link rel="icon" href="data:,"></head><body><div id="fixture-root"></div><script type="module" src="/tests/fixtures/homepageCapture.mjs"></script></body></html>');
      res.setHeader('Content-Type','text/html; charset=utf-8');res.end(html);
    });}
  }]});
  await server.listen();base=`http://127.0.0.1:${server.httpServer.address().port}`;
  browser=await chromium.launch({channel:'msedge',headless:true});
});
after(async()=>{await browser?.close();server?.httpServer?.closeAllConnections();await server?.close();});
beforeEach(async()=>{page=await browser.newPage({viewport:{width:1440,height:900}});page.setDefaultTimeout(15000);await page.goto(`${base}/__capture-test`,{waitUntil:'domcontentloaded',timeout:60000});await page.getByTestId('quick-capture').waitFor({timeout:60000});});
afterEach(async()=>page?.close());
const saved=()=>page.evaluate(()=>window.__CAPTURE_TEST__.calls.filter(call=>call.name==='capture_work_note'));
const submittedJobs=()=>page.evaluate(()=>window.__CAPTURE_TEST__.calls.filter(call=>call.name==='start_ai_job'));

test('homepage provides one prominent multiline capture with two explicit save choices',async()=>{
  const input=page.getByTestId('quick-capture');
  assert.equal(await input.count(),1);
  assert.equal(await input.evaluate(el=>el.tagName),'TEXTAREA');
  assert.ok((await input.boundingBox()).height>=64);
  assert.equal(await page.getByTestId('quick-capture-save').isVisible(),true);
  assert.equal(await page.getByTestId('quick-capture-organize').isVisible(),true);
  assert.equal(await input.evaluate(el=>Boolean(el.closest('.topbar'))),false);
});
test('Shift Enter creates a line, composing Enter does not save, ordinary Enter saves the complete note once',async()=>{
  const input=page.getByTestId('quick-capture');
  await input.fill('演示：讨论记录');await input.press('Shift+Enter');await input.press('a');
  assert.equal(await input.inputValue(),'演示：讨论记录\na');
  await input.dispatchEvent('keydown',{key:'Enter',isComposing:true});
  assert.equal((await saved()).length,0);
  await input.press('Enter');
  await page.waitForFunction(()=>window.__CAPTURE_TEST__.calls.some(call=>call.name==='capture_work_note'));
  assert.equal((await saved())[0].args.content,'演示：讨论记录\na');
  assert.equal((await submittedJobs()).length,0);
  await page.waitForFunction(()=>document.querySelector('[data-testid="quick-capture"]').value==='');
});
test('save locks repeat submissions and save plus organize schedules the saved note only',async()=>{
  await page.evaluate(()=>window.__CAPTURE_TEST__.holdCapture());
  await page.getByTestId('quick-capture').fill('演示：请整理这条原话');
  await page.getByTestId('quick-capture-organize').click();
  assert.equal(await page.getByTestId('quick-capture-save').isDisabled(),true);
  assert.equal(await page.getByTestId('quick-capture-organize').isDisabled(),true);
  assert.equal((await saved()).length,1);
  await page.evaluate(()=>window.__CAPTURE_TEST__.releaseCapture());
  await page.waitForFunction(()=>window.__CAPTURE_TEST__.jobs.length===1);
  const [job]=await submittedJobs();
  assert.equal(job.args.request.command,'organize_inbox_item');
  assert.equal(typeof job.args.request.args.inboxId,'number');
});
test('draft and one capture registration survive page changes with no duplicate fields',async()=>{
  await page.getByTestId('quick-capture').fill('演示：尚未发送的原话');
  await page.getByTestId('nav-calendar').click();
  assert.equal(await page.getByTestId('quick-capture').count(),1);
  assert.equal(await page.getByTestId('quick-capture').inputValue(),'演示：尚未发送的原话');
  await page.getByTestId('nav-today').click();
  assert.equal(await page.getByTestId('quick-capture').inputValue(),'演示：尚未发送的原话');
  const state=await page.evaluate(()=>({registrations:window.__CAPTURE_TEST__.calls.filter(c=>c.name==='plugin:global-shortcut|register').length,shortcuts:window.__CAPTURE_TEST__.shortcutCount(),listeners:window.__CAPTURE_TEST__.captureListenerCount()}));
  assert.deepEqual(state,{registrations:1,shortcuts:1,listeners:1});
});
test('saving an older capture after navigation preserves the newer unsent draft',async()=>{
  await page.evaluate(()=>window.__CAPTURE_TEST__.holdCapture());
  await page.getByTestId('quick-capture').fill('演示：第一条已发送');
  await page.getByTestId('quick-capture-save').click();
  await page.getByTestId('nav-calendar').click();
  await page.getByTestId('quick-capture').fill('演示：第二条尚未发送');
  await page.evaluate(()=>window.__CAPTURE_TEST__.releaseCapture());
  await page.getByText('已记在“事项 → 待整理”，需要时再交给秘书。',{exact:true}).waitFor();
  await page.getByTestId('nav-today').click();
  assert.equal(await page.getByTestId('quick-capture').inputValue(),'演示：第二条尚未发送');
});
test('organize everything remains visible beside pending advice and owns a background job after navigation',async()=>{
  const button=page.getByTestId('organize-workspace');
  assert.equal(await button.count(),1);
  assert.equal(await button.isVisible(),true);
  const before=await button.boundingBox();
  await button.click();
  assert.equal(await button.isDisabled(),true);
  const during=await button.boundingBox();
  assert.ok(Math.abs(during.width-before.width)<=1);
  assert.equal((await submittedJobs()).filter(call=>call.args.request.command==='run_analysis_now').length,1);
  await page.getByTestId('nav-calendar').click();await page.getByTestId('nav-today').click();
  assert.equal(await page.getByTestId('organize-workspace').isDisabled(),true);
  await page.evaluate(()=>window.__CAPTURE_TEST__.finishJobs());
  await page.waitForFunction(()=>!document.querySelector('[data-testid="organize-workspace"]').disabled);
  assert.equal(await page.getByTestId('generate-daily-brief').isVisible(),true);
});
test('homepage routes inferred times to explicit review without confirming or writing the suggestion',async()=>{
  await page.goto(`${base}/__capture-test?inferred-time`,{waitUntil:'domcontentloaded'});
  const suggestion=page.getByTestId('latest-decision-1');
  await suggestion.waitFor();
  const primary=suggestion.locator('.decision-actions > button').first();
  assert.equal(await primary.isEnabled(),true,'Review must remain actionable instead of a dead disabled button');
  await primary.click();
  await page.getByTestId('matters-tabs').waitFor();
  assert.equal(await page.getByText('请先核对并单独确认建议时间，确认后才会写入正式安排。',{exact:true}).isVisible(),true);
  assert.equal(await page.evaluate(()=>window.__CAPTURE_TEST__.calls.filter(call=>['confirm_ai_proposal','update_ai_proposal_classification'].includes(call.name)).length),0);
});
test('capture and primary actions fit supported widths and font settings without clipping',async()=>{
  for(const [width,height] of [[1440,900],[1280,800],[1024,768]])for(const font of ['standard','large']){
    await page.setViewportSize({width,height});
    await page.evaluate(value=>document.documentElement.dataset.fontSize=value,font);
    await page.getByTestId('quick-capture').fill('演示：记录一条交流后的方向调整。');
    const bounds=await page.evaluate(()=>{
      const ids=['quick-capture','quick-capture-save','quick-capture-organize','organize-workspace','generate-daily-brief'];
      return ids.map(id=>{const el=document.querySelector(`[data-testid="${id}"]`),rect=el.getBoundingClientRect();return {id,left:rect.left,right:rect.right,width:rect.width,scrollWidth:el.scrollWidth,clientWidth:el.clientWidth};});
    });
    for(const box of bounds){assert.ok(box.left>=0&&box.right<=width+1,`${box.id} escaped ${width}/${font}`);assert.ok(box.width>0);assert.ok(box.scrollWidth<=box.clientWidth+1,`${box.id} clipped ${width}/${font}`);}
    await page.screenshot({path:path.resolve(process.env.APPDATA,'..',`homepage-${width}-${font}.png`)});
  }
});

test('review keeps a proposed time unapproved until a separate checked action and never saves that approval in the draft',async()=>{
  await page.goto(`${base}/__capture-test?inferred-time`,{waitUntil:'domcontentloaded'});
  await page.getByTestId('latest-decision-1').locator('.decision-actions > button').first().click();
  const confirm=page.getByTestId('review-confirm');
  await confirm.waitFor();
  await confirm.click();
  assert.equal(await page.evaluate(()=>window.__CAPTURE_TEST__.calls.filter(call=>call.name==='confirm_ai_proposal').length),0);
  const checkbox=page.getByTestId('suggested-time-confirm').locator('input');
  await checkbox.check();
  // Editing the dates after ticking consent must require a fresh confirmation.
  await page.getByTestId('review-scheduled-start').fill('2026-10-02T14:00');
  await page.getByTestId('review-scheduled-start').dispatchEvent('change');
  assert.equal(await checkbox.isChecked(),false);
  await checkbox.check();
  await confirm.click();
  await page.getByTestId('review-receipt-view').waitFor();
  const calls=await page.evaluate(()=>window.__CAPTURE_TEST__.calls);
  const saved=calls.filter(call=>call.name==='update_ai_proposal_classification').at(-1);
  const accepted=calls.filter(call=>call.name==='confirm_ai_proposal');
  assert.equal(saved.args.payload.time_confirmation,undefined);
  assert.equal(saved.args.payload.time_basis,'inferred');
  assert.equal(accepted.length,1);
  assert.equal(accepted[0].args.editedPayload.time_confirmation,'user_confirmed');
});

test('invalid advanced review JSON cannot trigger saving or crash the confirmation action',async()=>{
  await page.goto(`${base}/__capture-test?inferred-time`,{waitUntil:'domcontentloaded'});
  await page.getByTestId('latest-decision-1').locator('.decision-actions > button').first().click();
  await page.getByTestId('review-adjust').click();
  await page.locator('.advanced-panel > summary').click();
  await page.locator('textarea.payload').fill('{ invalid');
  const errors=[];page.on('pageerror',error=>errors.push(error.message));
  await page.getByTestId('review-confirm').click();
  assert.equal(await page.evaluate(()=>window.__CAPTURE_TEST__.calls.filter(call=>['confirm_ai_proposal','update_ai_proposal_classification'].includes(call.name)).length),0);
  assert.deepEqual(errors,[]);
  assert.ok((await page.locator('.error.stable-feedback').innerText()).length>0);
});
