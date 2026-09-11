// Bounded concurrent IPC stress. All records are synthetic and stay outside source.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {createRequire} from 'node:module';
import {DatabaseSync} from 'node:sqlite';
assert.ok(process.env.MSL_TEST_PROFILE);
const root=path.resolve(process.env.MSL_TEST_PROFILE);
for(const [key,part] of Object.entries({APPDATA:'appdata',LOCALAPPDATA:'localappdata',TEMP:'temp',TMP:'temp'}))assert.equal(path.resolve(process.env[key]),path.join(root,part));
const {chromium}=createRequire(path.join(process.env.MSL_NODE_MODULES,'runtime.cjs'))('playwright');
let browser;for(const host of ['127.0.0.1','[::1]'])try{browser=await chromium.connectOverCDP(`http://${host}:${process.env.MSL_CDP_PORT}`);break}catch{}
assert.ok(browser);const page=browser.contexts()[0].pages()[0];
const call=(command,args={})=>page.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
try {
 const status=await call('backup_status');assert.equal(path.resolve(status.data_directory),path.join(root,'appdata','MSLDesktop'));
 const project=await call('create_work',{title:'Synthetic bounded stress project',status:'active'});
 const count=10000,start=Date.now();
 for(let offset=0;offset<count;offset+=8){
  await Promise.all(Array.from({length:Math.min(8,count-offset)},(_,i)=>call('create_task',{workId:project.id,title:`Synthetic stress ${offset+i+1}`,priority:'normal',notes:'Synthetic record for concurrency and snapshot validation.',dueAt:null})));
  if((offset+8)%1000===0)console.log(`Committed ${offset+8}/${count} synthetic tasks`);
 }
 const writeMs=Date.now()-start,syncStart=Date.now();await call('run_sync_now');const syncMs=Date.now()-syncStart;
 const db=new DatabaseSync(path.join(status.data_directory,'msl-desktop.db'),{readOnly:true});
 try {
  assert.equal(db.prepare('SELECT count(*) n FROM tasks WHERE work_id=?').get(project.id).n,count);
  assert.equal(db.prepare('PRAGMA integrity_check').get().integrity_check,'ok');
  assert.equal(db.prepare('PRAGMA foreign_key_check').all().length,0);
 } finally {db.close();}
 const result={pass:true,tasks:count,concurrentRequests:8,writeMs,syncMs,integrity:'ok',foreignKeyErrors:0};
 fs.writeFileSync(path.join(root,'artifacts','native-load.json'),JSON.stringify(result,null,2));console.log(JSON.stringify(result));
}finally{await browser.close();}
