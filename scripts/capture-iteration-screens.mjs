// Screenshot-only review capture of the known isolated native instance.
import {createRequire} from 'node:module';
import path from 'node:path';
import assert from 'node:assert/strict';
const root=path.resolve(import.meta.dirname,'..','.test-runtime','materials-ui');
for(const [key,part] of Object.entries({APPDATA:'appdata',LOCALAPPDATA:'localappdata',TEMP:'temp',TMP:'temp'}))assert.equal(path.resolve(process.env[key]),path.join(root,part));
const {chromium}=createRequire(path.join(process.env.MSL_NODE_MODULES,'runtime.cjs'))('playwright');
let browser;
for(const address of ['http://127.0.0.1:9459','http://[::1]:9459'])try{browser=await chromium.connectOverCDP(address,{timeout:3000});break;}catch{}
assert.ok(browser);
try{
 const page=browser.contexts()[0].pages()[0],cdp=await page.context().newCDPSession(page);
 await cdp.send('Emulation.setDeviceMetricsOverride',{width:1440,height:900,deviceScaleFactor:1,mobile:false});
 await page.evaluate(()=>document.documentElement.dataset.fontSize='standard');
 const call=(name,args={})=>page.evaluate(({name,args})=>window.__TAURI_INTERNALS__.invoke(name,args),{name,args});
 const route=async(view,id)=>{await page.evaluate(detail=>window.dispatchEvent(new CustomEvent('dashboard:navigate',{detail})),{view,id});await page.waitForTimeout(500);};
 const snap=name=>page.screenshot({path:path.join(root,'artifacts',name+'.png')});
 const session=(await call('list_qa_sessions'))[0],expert=(await call('list_kol_experts'))[0],provider=(await call('list_provider_connections'))[0];
 await route('qa',session.id);await page.getByTestId('qa-answer').first().waitFor();
 await page.getByTestId('qa-transcript').evaluate(el=>{const last=el.querySelector('.qa-turn:last-child');el.scrollTop+=last.getBoundingClientRect().top-el.getBoundingClientRect().top-16;});await page.waitForTimeout(250);await snap('01-question-answer');
 await page.locator('.sources summary').last().click();
 await page.getByRole('button',{name:'查看资料',exact:true}).last().click();
 await page.getByRole('dialog').waitFor();await snap('06-material-evidence');await page.keyboard.press('Escape');
 await route('kol',expert.id);await page.getByTestId('kol-tab-materials').click();await page.getByTestId('kol-materials').waitFor();
 await page.evaluate(()=>document.querySelector('.content-scroll').scrollTop=0);await snap('02-expert-materials');
 await page.getByTestId('kol-delete').click();await snap('03-safe-delete');await page.keyboard.press('Escape');
 await route('settings');const fold=page.getByTestId('models-fold-'+provider.id);
 if(await fold.evaluate(e=>e.parentElement.open))await fold.click();
 await fold.evaluate(el=>{const scroll=document.querySelector('.content-scroll'),card=el.closest('.app-card');scroll.scrollTop+=card.getBoundingClientRect().top-scroll.getBoundingClientRect().top-18;});await snap('04-folded-models');
 await fold.click();await snap('05-expanded-models');
 console.log('Six native screenshots refreshed, synthetic information only.');
}finally{await browser.close();}
