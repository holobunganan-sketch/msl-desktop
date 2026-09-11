// Critical settings regression against an isolated native application.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {createRequire} from 'node:module';
assert.ok(process.env.MSL_TEST_PROFILE);
const root=path.resolve(process.env.MSL_TEST_PROFILE);
for(const [key,part] of Object.entries({APPDATA:'appdata',LOCALAPPDATA:'localappdata',TEMP:'temp',TMP:'temp'})) assert.equal(path.resolve(process.env[key]),path.join(root,part));
const {chromium}=createRequire(path.join(process.env.MSL_NODE_MODULES,'runtime.cjs'))('playwright');
let browser;
for(const host of ['127.0.0.1','[::1]'])try{browser=await chromium.connectOverCDP(`http://${host}:${process.env.MSL_CDP_PORT}`);break}catch{}
assert.ok(browser);
const page=browser.contexts()[0].pages()[0],errors=[];
page.on('pageerror',e=>errors.push(e.message));
const call=(command,args={})=>page.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
try {
 assert.equal(path.resolve((await call('backup_status')).data_directory),path.join(root,'appdata','MSLDesktop'));
 await page.evaluate(()=>window.dispatchEvent(new CustomEvent('dashboard:navigate',{detail:{view:'settings'}})));
 await page.getByTestId('backup-directory').waitFor({state:'attached'});
 assert.equal(await page.getByTestId('backup-directory').isVisible(),false,'Secondary backup configuration must not compete with daily sync');
 const history=page.getByTestId('backup-history');
 await history.locator(':scope > summary').click();
 assert.equal(await page.getByTestId('backup-now').isVisible(),true);
 await page.getByTestId('backup-advanced').locator(':scope > summary').click();
 const original=(await call('backup_status')).config;
 await page.getByTestId('backup-retention').fill('6');
 await page.getByTestId('backup-save').click();
 await page.waitForFunction(()=>window.__TAURI_INTERNALS__.invoke('backup_status').then(s=>s.config.keep_count===6));
 await history.locator(':scope > summary').click();
 await history.locator(':scope > summary').click();
 assert.equal(await page.getByTestId('backup-retention').inputValue(),'6');
 await call('save_backup_settings',{directory:original.directory,enabled:original.enabled,intervalMinutes:original.interval_minutes,keepCount:original.keep_count});
 for(const [width,height,font] of [[1440,900,'standard'],[1024,768,'xlarge']]){
  await page.setViewportSize({width,height});
  await page.evaluate(font=>document.documentElement.dataset.fontSize=font,font);
  await page.locator('#backup').scrollIntoViewIfNeeded();
  for(const target of [page.getByTestId('backup-save'),page.getByTestId('backup-inspect'),page.locator('.sync-settings input:not([type=checkbox])').first()]){
   await target.scrollIntoViewIfNeeded();
   const b=await target.boundingBox();assert.ok(b&&b.x>=0&&b.x+b.width<=width+1&&b.y>=-1&&b.y+b.height<=height+1,'Settings controls must remain reachable');
  }
  await page.evaluate(()=>document.querySelector('[data-testid=backup-history]').open=false);
  await page.locator('#backup').scrollIntoViewIfNeeded();
  await page.screenshot({path:path.join(root,'artifacts',`data-settings-${width}.png`)});
  await history.locator(':scope > summary').click();
 }
 assert.deepEqual(errors,[]);
 fs.writeFileSync(path.join(root,'artifacts','data-settings-result.json'),JSON.stringify({pass:true,errors,checks:['default collapse','backup save persists','collapse preserves form','two window/font sizes','reachable controls']},null,2));
 console.log('PASS data settings: collapsed backup, persistent save, reachable controls at standard and large font, no page errors');
}finally{await browser.close();}
