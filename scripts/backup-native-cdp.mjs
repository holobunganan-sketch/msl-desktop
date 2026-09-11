import assert from 'node:assert/strict';
import fs from 'node:fs';import path from 'node:path';import {createRequire} from 'node:module';
import {DatabaseSync} from 'node:sqlite';
const root=path.resolve(import.meta.dirname,'..'),profile=path.join(root,'.test-runtime','backup-ui');
for(const [key,part] of Object.entries({APPDATA:'appdata',LOCALAPPDATA:'localappdata',TEMP:'temp',TMP:'temp'}))assert.equal(path.resolve(process.env[key]),path.join(profile,part));
const req=createRequire(path.join(process.env.MSL_NODE_MODULES,'runtime.cjs')), {chromium}=req('playwright');
let browser;for(let attempt=0;attempt<60&&!browser;attempt++){for(const url of ['http://127.0.0.1:9463','http://[::1]:9463']){try{browser=await chromium.connectOverCDP(url);break}catch{}}if(!browser)await new Promise(r=>setTimeout(r,250));}assert.ok(browser,'Native WebView did not start');const page=browser.contexts()[0].pages()[0];
const call=(command,args={})=>page.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
const route=async view=>{await page.evaluate(view=>window.dispatchEvent(new CustomEvent('dashboard:navigate',{detail:{view}})),view);};
const until=async(fn)=>{for(let i=0;i<200;i++){const value=await fn();if(value)return value;await new Promise(r=>setTimeout(r,250));}throw Error('State did not settle');};
const artifacts=path.join(profile,'artifacts'),cloud=path.join(profile,'cloud-folder');fs.mkdirSync(artifacts,{recursive:true});fs.mkdirSync(cloud,{recursive:true});
const errors=[];page.on('pageerror',e=>errors.push(e.message));
try {
 await page.getByRole('button',{name:'设置',exact:true}).click();await page.getByRole('link',{name:'数据与同步',exact:true}).waitFor({timeout:5000});
 await page.getByTestId('backup-history').locator(':scope > summary').click();
 await page.getByTestId('backup-advanced').locator(':scope > summary').click();
 console.log('PASS backup settings navigation');
 let state=await call('backup_status');assert.equal(path.resolve(state.data_directory),path.join(profile,'appdata','MSLDesktop'));
 if(process.argv.includes('--reopened')){
  assert.equal(await call('app_settings_get',{key:'synthetic_backup_marker'}),'before backup');
  assert.equal(state.config.directory,cloud);assert.equal(state.config.enabled,false);assert.equal(state.has_rollback,true);assert.equal(state.restore_pending,false);
  const materials=await call('list_kol_materials',{expertId:null});assert.equal(materials.length,1);const details=await call('get_kol_material_preview',{id:materials[0].id});assert.ok(details.segments.length>0);
  const source=path.join(profile,'workspace','synthetic-evidence.txt');assert.equal(fs.readFileSync(source,'utf8'),'Synthetic reference for backup restoration.');
  const checkDb=new DatabaseSync(path.join(profile,'appdata','MSLDesktop','msl-desktop.db'),{readOnly:true});assert.ok(checkDb.prepare('SELECT COUNT(*) n FROM work_workspace_links').get().n>0);assert.equal(checkDb.prepare("SELECT value FROM app_settings WHERE key='main_workspace'").get(),undefined);checkDb.close();
  console.log('PASS restored SQLite, expert attachment/index, source unchanged, machine folder retained, auto backup paused');
 }else{
  const seed=new DatabaseSync(path.join(profile,'appdata','MSLDesktop','msl-desktop.db'));seed.exec('PRAGMA foreign_keys=ON; BEGIN IMMEDIATE');
  let project=seed.prepare("SELECT id FROM works WHERE title='备份演示项目'").get();if(!project){const r=seed.prepare("INSERT INTO works(title,created_at,updated_at) VALUES('备份演示项目',0,0)").run();project={id:r.lastInsertRowid};}
  let workspace=seed.prepare('SELECT id FROM workspaces WHERE root_path=?').get(path.join(profile,'workspace'));if(!workspace){const r=seed.prepare("INSERT INTO workspaces(name,root_path,created_at,updated_at) VALUES('合成工作目录',?,0,0)").run(path.join(profile,'workspace'));workspace={id:r.lastInsertRowid};}
  seed.prepare('INSERT OR IGNORE INTO work_workspace_links(work_id,workspace_id,created_at) VALUES(?,?,0)').run(project.id,workspace.id);seed.exec('COMMIT');seed.close();
  await call('app_settings_set',{key:'main_workspace',value:path.join(profile,'workspace')});
  const expert=(await call('list_kol_experts')).find(e=>e.name==='备份演示专家')??await call('save_kol_expert',{id:null,revision:null,name:'备份演示专家',institution:'示例机构',department:'示例科室',specialty:'合成资料',projects:[],archived:false});
  const source=path.join(profile,'workspace','synthetic-evidence.txt');fs.writeFileSync(source,'Synthetic reference for backup restoration.');
  const job=await call('start_ai_job',{request:{command:'import_kol_materials',args:{expertId:expert.id,paths:[source]}}});
  await until(async()=>{const item=await call('get_ai_job',{id:job.id});if(item.status==='failed')throw Error(item.error);return item.status==='completed';});
  await call('app_settings_set',{key:'synthetic_backup_marker',value:'before backup'});
  await page.getByTestId('backup-directory').fill(cloud);await page.getByTestId('backup-interval').fill('1440');await page.getByTestId('backup-retention').fill('3');await page.getByLabel('自动备份',{exact:true}).check();await page.getByTestId('backup-save').click();
  await until(async()=> (await call('backup_status')).config.directory===cloud);
  await page.getByTestId('backup-now').click();await route('today');
  state=await until(async()=>{const s=await call('backup_status');return s.state.last_success&&!s.running?s:false});assert.equal(state.state.error,'');assert.ok(fs.existsSync(state.state.last_file));
  console.log('PASS native backup completed after leaving settings');
  // Simulate elapsed time in this synthetic profile without changing the system clock.
  const stateFile=path.join(profile,'appdata','MSLDesktop','backup-state.json');const elapsed=JSON.parse(fs.readFileSync(stateFile,'utf8'));elapsed.last_attempt=0;fs.writeFileSync(stateFile,JSON.stringify(elapsed));const previous=state.state.last_file;
  state=await until(async()=>{const s=await call('backup_status');return s.state.last_file!==previous&&!s.running?s:false});assert.equal(state.state.error,'');console.log('PASS automatic folder backup after scheduled interval');
  await call('app_settings_set',{key:'synthetic_backup_marker',value:'after backup'});
  await route('settings');await page.getByTestId('backup-file').fill(state.state.last_file);await page.getByTestId('backup-inspect').click();await page.getByRole('dialog').waitFor();
  assert.ok(await page.getByTestId('restore-confirm').isDisabled());await page.getByTestId('restore-name').fill('错误确认');assert.ok(await page.getByTestId('restore-confirm').isDisabled());await page.getByTestId('restore-name').fill('恢复备份');assert.ok(await page.getByTestId('restore-confirm').isEnabled());
  await page.screenshot({path:path.join(artifacts,'backup-restore-confirm.png')});await page.keyboard.press('Escape');
  assert.equal(await call('app_settings_get',{key:'synthetic_backup_marker'}),'after backup');console.log('PASS restore requires typed confirmation; cancel leaves data unchanged');
 }
 for(const [width,height] of [[1440,900],[1280,800],[1024,768]])for(const scale of ['standard','large']){
  await page.setViewportSize({width,height});await page.evaluate(scale=>document.documentElement.dataset.fontSize=scale,scale);await page.locator('#backup').scrollIntoViewIfNeeded();
  assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));
  const controls=page.locator('#backup input,#backup button');for(let i=0;i<await controls.count();i++){const item=controls.nth(i);await item.scrollIntoViewIfNeeded();const b=await item.boundingBox();assert.ok(b.x>=0&&b.x+b.width<=width+1,`control overflow ${width} ${scale}`);}
 }
 await page.setViewportSize({width:1440,height:1050});await page.evaluate(()=>document.documentElement.dataset.fontSize='standard');await page.locator('#backup').scrollIntoViewIfNeeded();await page.locator('#backup').screenshot({path:path.join(artifacts,'backup-settings.png')});
 console.log('PASS backup layout: three sizes, standard and large text');assert.deepEqual(errors,[]);
 if(!process.argv.includes('--reopened')){
  state=await call('backup_status');await page.getByTestId('backup-file').fill(state.state.last_file);await page.getByTestId('backup-inspect').click();await page.getByRole('dialog').waitFor();await page.getByTestId('restore-name').fill('恢复备份');
  await page.getByTestId('restore-confirm').click();console.log('PASS restore confirmed through UI; restart dispatched');
 }
 fs.writeFileSync(path.join(artifacts,process.argv.includes('--reopened')?'backup-reopened.json':'backup-ui.json'),JSON.stringify({pass:true,viewportCases:6,errors},null,2));
}finally{await browser.close().catch(()=>{});}
