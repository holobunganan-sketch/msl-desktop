import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {createRequire} from 'node:module';
import {inspectGeometry} from './ui-geometry.mjs';
import {DatabaseSync} from 'node:sqlite';
assert.ok(process.env.MSL_TEST_PROFILE,'An explicit isolated test profile is required');
const root=path.resolve(process.env.MSL_TEST_PROFILE);
for(const [key,part] of Object.entries({APPDATA:'appdata',LOCALAPPDATA:'localappdata',TEMP:'temp',TMP:'temp'}))assert.equal(path.resolve(process.env[key]),path.join(root,part));
const {chromium}=createRequire(path.join(process.env.MSL_NODE_MODULES,'runtime.cjs'))('playwright');
let browser;for(const host of ['127.0.0.1','[::1]'])try{browser=await chromium.connectOverCDP(`http://${host}:${process.env.MSL_CDP_PORT||9465}`);break}catch{}
assert.ok(browser);const page=browser.contexts()[0].pages()[0],errors=[],results=[];
page.on('pageerror',e=>errors.push(e.message));
const artifacts=path.join(root,'artifacts',process.env.MSL_UI_LABEL||'details');fs.mkdirSync(artifacts,{recursive:true});
const route=async(view)=>{const detail=['inbox','review','plan','waiting'].includes(view)?{view:'matters',section:view}:{view};await page.evaluate(detail=>window.dispatchEvent(new CustomEvent('dashboard:navigate',{detail})),detail);await page.waitForTimeout(250);};
async function check(name){const r=await page.evaluate(inspectGeometry),pass=r.controls>0&&!r.horizontal&&!r.failures.length;results.push({name,pass,...r});console.log(`${pass?'PASS':'FAIL'} ${name}${pass?'':' '+JSON.stringify(r)}`);if(!pass||name.includes('1024'))await page.screenshot({path:path.join(artifacts,name+'.png')});}
async function modal(view,id,name){await route(view);if(id==='work-delete')await page.locator('.project-options summary').click();if(id.startsWith('kol-')&&id!=='kol-add-expert')await page.locator('[data-testid^=kol-expert-]').first().click();await page.getByTestId(id).click();await page.getByRole('dialog').waitFor();await page.getByRole('dialog').locator('details').evaluateAll(items=>items.forEach(e=>e.open=true));await check(name);await page.keyboard.press('Escape');await page.getByRole('dialog').waitFor({state:'hidden'});}
try{
 for(const [width,height,font] of [[1440,900,'standard'],[1280,800,'large'],[1024,768,'xlarge']]){
  await page.setViewportSize({width,height});await page.evaluate(font=>document.documentElement.dataset.fontSize=font,font);
  for(const [view,id] of [['works','work-create'],['works','work-delete'],['plan','task-create'],['waiting','waiting-create'],['calendar','calendar-create'],['kol','kol-edit-profile'],['kol','kol-delete']])await modal(view,id,`${width}-${font}-${id}`);
  await route('kol');await page.locator('[data-testid^=kol-expert-]').first().click();
  for(const tab of ['record','materials','drafts','insights','followups']){await page.getByTestId('kol-tab-'+tab).click();await page.waitForTimeout(150);await check(`${width}-${font}-expert-${tab}`);}
  await route('inbox');await page.locator('.manual-routing summary').first().click();await page.locator('[data-testid^=inbox-task-]').first().click();await page.getByRole('dialog').waitFor();await check(`${width}-${font}-inbox-route`);await page.keyboard.press('Escape');
 }
 // A delayed scroll event from a removed transcript must not access a nulled bind:this.
 await route('qa');await page.getByTestId('qa-transcript').waitFor();
 await page.evaluate(()=>{const transcript=document.querySelector('[data-testid=qa-transcript]');transcript.scrollTop=100;window.dispatchEvent(new CustomEvent('dashboard:navigate',{detail:{view:'works'}}));setTimeout(()=>transcript.dispatchEvent(new Event('scroll')),0);});await page.waitForTimeout(200);
 assert.deepEqual(errors,[]);console.log('PASS scroll event after conversation unmount');
 const db=new DatabaseSync(path.join(root,'appdata','MSLDesktop','msl-desktop.db'));
 try{
  db.prepare("UPDATE inbox_items SET processed_at=1 WHERE content LIKE '合成原始记录%'").run();
  await route('inbox');await page.locator('.inbox .empty-state').waitFor();assert.equal(await page.locator('.in-list').count(),0);
  await page.locator('.history-switch input').check();await page.locator('.in-list li').first().waitFor();assert.equal(await page.locator('.inbox .empty-state').count(),0);
  console.log('PASS inbox filtered empty state and history toggle');
 }finally{db.prepare("UPDATE inbox_items SET processed_at=NULL WHERE content LIKE '合成原始记录%'").run();db.close();}
 fs.writeFileSync(path.join(artifacts,'results.json'),JSON.stringify({results,errors},null,2));assert.equal(results.filter(r=>!r.pass).length,0);
}finally{await browser.close();}
