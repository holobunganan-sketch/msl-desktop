import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {createRequire} from 'node:module';
import {DatabaseSync} from 'node:sqlite';
assert.ok(process.env.MSL_TEST_PROFILE);
const root=path.resolve(process.env.MSL_TEST_PROFILE);
for(const [key,part] of Object.entries({APPDATA:'appdata',LOCALAPPDATA:'localappdata',TEMP:'temp',TMP:'temp'}))assert.equal(path.resolve(process.env[key]),path.join(root,part));
const {chromium}=createRequire(path.join(process.env.MSL_NODE_MODULES,'runtime.cjs'))('playwright');
let browser;
for(const host of ['127.0.0.1','[::1]'])try{browser=await chromium.connectOverCDP(`http://${host}:${process.env.MSL_CDP_PORT}`);break}catch{}
assert.ok(browser,'Isolated native WebView is required');
const page=browser.contexts()[0].pages()[0];
const call=(command,args={})=>page.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
const artifact=path.join(root,'artifacts','persistence.json');
try {
 const status=await call('backup_status');assert.equal(path.resolve(status.data_directory),path.join(root,'appdata','MSLDesktop'));
 if(!process.argv.includes('--reopened')){
  const project=await call('create_work',{title:'Synthetic crash recovery project',status:'active'});
  const task=await call('create_task',{workId:project.id,title:'Synthetic crash recovery task',priority:'normal',dueAt:null,notes:'Only a synthetic test record'});
  await call('app_settings_set',{key:'synthetic_persistence_marker',value:'committed before forced exit'});
  const cloud=path.join(root,'sync-folder');fs.mkdirSync(cloud,{recursive:true});
  assert.equal((await call('probe_sync_folder',{directory:cloud})).kind,'empty');
  await call('connect_sync_folder',{directory:cloud,mode:'initialize_from_local'});
  await call('save_sync_schedule',{enabled:false,intervalMinutes:180});
  const countFiles=()=>fs.readdirSync(cloud,{recursive:true}).filter(p=>/^state-.*\.json$/.test(path.basename(p))).length;
  const before=countFiles();await call('run_sync_now');await call('run_sync_now');assert.equal(countFiles(),before,'No-change native sync grew snapshot count');
  const backups=path.join(root,'backups');fs.mkdirSync(backups,{recursive:true});
  await call('save_backup_settings',{directory:backups,enabled:false,intervalMinutes:180,keepCount:3});
  const backup=await call('create_backup');assert.ok(fs.existsSync(backup));
  const preview=await call('preview_backup_restore',{path:backup});assert.ok(preview.token);
  await call('discard_backup_preview',{token:preview.token});
  fs.writeFileSync(artifact,JSON.stringify({projectId:project.id,taskId:task.id,backup,initialSnapshotCount:before},null,2));
  console.log('PASS native commands, sync connection/no-change reuse, verified backup and preview cancellation');
 } else {
  const expected=JSON.parse(fs.readFileSync(artifact,'utf8'));
  assert.equal(await call('app_settings_get',{key:'synthetic_persistence_marker'}),'committed before forced exit');
  const db=new DatabaseSync(path.join(status.data_directory,'msl-desktop.db'),{readOnly:true});
  try {
   assert.equal(db.prepare('PRAGMA integrity_check').get().integrity_check,'ok');
   assert.equal(db.prepare('PRAGMA foreign_key_check').all().length,0);
   assert.equal(db.prepare('SELECT work_id FROM tasks WHERE id=?').get(expected.taskId).work_id,expected.projectId);
   assert.equal(db.prepare('SELECT title FROM works WHERE id=?').get(expected.projectId).title,'Synthetic crash recovery project');
   const loadFile=path.join(root,'artifacts','native-load.json');
   if(fs.existsSync(loadFile)){
    const load=JSON.parse(fs.readFileSync(loadFile,'utf8'));
    assert.equal(db.prepare("SELECT count(*) n FROM tasks t JOIN works w ON w.id=t.work_id WHERE w.title='Synthetic bounded stress project'").get().n,load.tasks,'Stress records or project links were lost after restart');
   }
  } finally {db.close();}
  const sync=await call('sync_status');assert.equal(sync.config.interval_minutes,180);assert.equal(sync.config.enabled,false);
  fs.writeFileSync(path.join(root,'artifacts','persistence-reopened.json'),JSON.stringify({pass:true,integrity:'ok',foreignKeyErrors:0},null,2));
  console.log('PASS forced-exit restart: committed records, project links, sync preferences, SQLite integrity');
 }
}finally{await browser.close();}
