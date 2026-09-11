// Read/interaction stress only; records must have been generated in an isolated profile.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {createRequire} from 'node:module';
assert.ok(process.env.MSL_TEST_PROFILE);
const root=path.resolve(process.env.MSL_TEST_PROFILE);
for(const [key,part] of Object.entries({APPDATA:'appdata',LOCALAPPDATA:'localappdata',TEMP:'temp',TMP:'temp'}))assert.equal(path.resolve(process.env[key]),path.join(root,part));
const {chromium}=createRequire(path.join(process.env.MSL_NODE_MODULES,'runtime.cjs'))('playwright');
let browser;
for(const host of ['127.0.0.1','[::1]'])try{browser=await chromium.connectOverCDP(`http://${host}:${process.env.MSL_CDP_PORT}`);break}catch{}
assert.ok(browser);
const page=browser.contexts()[0].pages()[0];page.setDefaultTimeout(20000);
const errors=[];page.on('pageerror',e=>errors.push(e.message));
try {
 const status=await page.evaluate(()=>window.__TAURI_INTERNALS__.invoke('backup_status'));
 assert.equal(path.resolve(status.data_directory),path.join(root,'appdata','MSLDesktop'));
 await page.evaluate(()=>window.dispatchEvent(new CustomEvent('dashboard:navigate',{detail:{view:'today'}})));
 await page.locator('.dashboard-c1').waitFor();
 const start=Date.now();
 await page.evaluate(()=>window.dispatchEvent(new CustomEvent('dashboard:navigate',{detail:{view:'matters',section:'plan'}})));
 await page.locator('[data-testid^=task-row-]').first().waitFor();
 await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
 const loadMs=Date.now()-start,renderedRows=await page.locator('[data-testid^=task-row-]').count();
 const pager=page.getByTestId('task-pagination');
 const total=Number(await pager.getAttribute('data-total'));
 assert.ok(total>=10000,'Run the synthetic load fixture first');
 assert.ok(renderedRows<=50,'Large lists must bound rendered rows without discarding records');
 const firstId=await page.locator('[data-testid^=task-row-]').first().getAttribute('data-testid');
 await pager.getByRole('button').last().click();
 assert.notEqual(await page.locator('[data-testid^=task-row-]').first().getAttribute('data-testid'),firstId,'Next page did not advance records');
 await pager.locator('input').fill(String(Math.ceil(total/50)));await pager.locator('input').press('Tab');
 await page.getByText('Synthetic stress 10000',{exact:true}).waitFor();
 const interaction=Date.now();await page.getByTestId('task-create').click();await page.getByRole('dialog').waitFor();
 const dialogMs=Date.now()-interaction;await page.keyboard.press('Escape');await page.getByRole('dialog').waitFor({state:'hidden'});
 const result={loadMs,dialogMs,renderedRows,total,pageErrors:errors,pass:loadMs<=15000&&dialogMs<=3000&&errors.length===0};
 fs.writeFileSync(path.join(root,'artifacts','native-large-list.json'),JSON.stringify(result,null,2));console.log(JSON.stringify(result));
 assert.ok(result.pass,'Large-list responsiveness exceeded the stated stress budget');
 await page.evaluate(()=>window.dispatchEvent(new CustomEvent('dashboard:navigate',{detail:{view:'today'}})));
}finally{await browser.close();}
