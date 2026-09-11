// Native layout regression: real application, isolated synthetic records only.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {createRequire} from 'node:module';
import {inspectGeometry} from './ui-geometry.mjs';
assert.ok(process.env.MSL_TEST_PROFILE,'An explicit isolated test profile is required');
const root=path.resolve(process.env.MSL_TEST_PROFILE);
for(const [key,part] of Object.entries({APPDATA:'appdata',LOCALAPPDATA:'localappdata',TEMP:'temp',TMP:'temp'}))assert.equal(path.resolve(process.env[key]),path.join(root,part));
const {chromium}=createRequire(path.join(process.env.MSL_NODE_MODULES,'runtime.cjs'))('playwright');
let browser;const port=process.env.MSL_CDP_PORT||9465;
for(let i=0;i<60&&!browser;i++){for(const host of ['127.0.0.1','[::1]'])try{browser=await chromium.connectOverCDP(`http://${host}:${port}`);break}catch{}if(!browser)await new Promise(r=>setTimeout(r,250));}
assert.ok(browser,'Native WebView unavailable');
const page=browser.contexts()[0].pages()[0],errors=[],results=[];let version='';
const label=process.env.MSL_UI_LABEL||'baseline',artifacts=path.join(root,'artifacts',label);fs.mkdirSync(artifacts,{recursive:true});
page.on('pageerror',e=>errors.push(e.message));
const views=[['today','.dashboard-c1'],['works','.works'],['kol','.kol-page'],['inbox','.inbox'],['review','.review-page'],['plan','.plan'],['waiting','.waiting'],['done','.plan'],['calendar','.calendar'],['reports','.reports-page'],['qa','.qa-chat'],['translation','.translation-page'],['workspace','.workspace-view'],['settings','.settings']];
const route=async(view)=>{const detail=['inbox','review','plan','waiting','done'].includes(view)?{view:'matters',section:view}:{view};await page.evaluate(detail=>window.dispatchEvent(new CustomEvent('dashboard:navigate',{detail})),detail);await page.waitForTimeout(220);};
try{
 await page.reload();await page.locator('.content-scroll').waitFor();
 version=await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('plugin:app|version'));
 if(process.env.MSL_EXPECTED_VERSION)assert.equal(version,process.env.MSL_EXPECTED_VERSION);
 const data=await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('backup_status'));assert.equal(path.resolve(data.data_directory),path.join(root,'appdata','MSLDesktop'));
 const expected=await page.evaluate(async()=>({models:await window.__TAURI_INTERNALS__.invoke('list_provider_models'),sessions:await window.__TAURI_INTERNALS__.invoke('list_qa_sessions'),connections:await window.__TAURI_INTERNALS__.invoke('list_provider_connections')}));
 if(((await page.locator('.locale-button').innerText()).trim()==='EN')!==process.argv.includes('--english'))await page.locator('.locale-button').click();
 const sizes=process.argv.includes('--quick')?[[1440,900,'standard'],[1024,768,'xlarge']]:[[1920,1080,'standard'],[1440,900,'standard'],[1440,900,'large'],[1280,800,'standard'],[1280,800,'xlarge'],[1024,768,'standard'],[1024,768,'xlarge']];
 for(const [width,height,font] of sizes){
  await page.setViewportSize({width,height});await page.evaluate(font=>document.documentElement.dataset.fontSize=font,font);
  for(const [view,selector] of views){
   await route(view);
   await page.locator(selector).waitFor();
   if(view==='qa'){
    await page.getByTestId('qa-loading').waitFor({state:'hidden'});
    assert.equal(await page.locator('.qa-status-error').count(),0,'Q&A must load successfully');
    if(expected.sessions.length){const session=page.locator('[data-testid^="qa-session-"]').first();if(!await session.isVisible())await page.locator('.qa-history-toggle').click();await session.click();await page.getByTestId('qa-loading').waitFor({state:'hidden'});await page.locator('.qa-turn').first().waitFor();}
   }
   if(view==='settings'){
    if(expected.models.length)await page.locator('[data-testid^="model-toggle-"]').first().waitFor({state:'attached'});
    assert.equal(await page.locator('.provider-status .error').count(),0,'Provider catalog must load successfully');
   }
   if(process.argv.includes('--expanded'))await page.locator('.content-scroll details').evaluateAll(items=>items.forEach(e=>e.open=true));
   const r=await page.evaluate(inspectGeometry);
   const bad=r.horizontal||r.failures.length||r.empty.some(e=>e.left<12||e.right<12);
   results.push({view,width,height,font,...r,pass:!bad});
   console.log(`${bad?'FAIL':'PASS'} ${view} ${width} ${font}${bad?' '+JSON.stringify(r):''}`);
   if((width===1440&&font==='standard')||bad){await page.evaluate(()=>{document.querySelector('.content-scroll').scrollTop=0;});await page.screenshot({path:path.join(artifacts,`${view}-${width}-${font}.png`)});}
  }
 }
 fs.writeFileSync(path.join(artifacts,'results.json'),JSON.stringify({version,results,errors},null,2));
 assert.deepEqual(errors,[]);assert.equal(results.filter(r=>!r.pass).length,0,'Native page geometry / empty state padding failures');
}finally{await browser.close();}
