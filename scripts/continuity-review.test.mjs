import test, {after} from 'node:test';
import assert from 'node:assert/strict';
import {createServer} from 'vite';

const server=await createServer({server:{middlewareMode:true,hmr:false},logLevel:'error'});
after(()=>server.close());
const presentation=await server.ssrLoadModule('/src/lib/services/proposalPresentation.ts');

test('update preview compares only explicitly proposed fields and shows cleared values',()=>{
  assert.equal(typeof presentation.proposalChanges,'function');
  const changes=presentation.proposalChanges({kind:'task',operation:'update',work_id:2,payload_json:JSON.stringify({title:'Follow up',priority:'high',due_at:null,notes:'Original'})},{title:'Follow up',priority:'normal',due_at:1800000000,notes:'Original',work_id:1},[{id:1,title:'Same title'},{id:2,title:'Same title'}],'en-US');
  assert.deepEqual(changes.map(c=>c.key),['work_id','priority','due_at']);
  assert.equal(changes[0].before,'Same title · #1');
  assert.equal(changes[0].after,'Same title · #2');
  assert.equal(changes[1].before,'Normal');
  assert.equal(changes[1].after,'High');
  assert.equal(changes[2].after,'Clear');
});

test('new suggestions and unavailable source records never manufacture previous values',()=>{
  assert.equal(typeof presentation.proposalChanges,'function');
  const item={kind:'work',operation:'create',work_id:null,payload_json:'{"title":"New project"}'};
  assert.deepEqual(presentation.proposalChanges(item,{title:'Unrelated'},[],'zh-CN'),[]);
  assert.deepEqual(presentation.proposalChanges({...item,operation:'update'},null,[],'zh-CN'),[]);
});

test('preview preserves omitted or non-clearing text and names unavailable projects explicitly',()=>{
  const item={kind:'task',operation:'update',work_id:44,payload_json:'{"notes":null}'};
  const changes=presentation.proposalChanges(item,{work_id:null,notes:'Keep original'},[],'en-US');
  assert.deepEqual(changes.map(c=>c.key),['work_id']);
  assert.equal(changes[0].after,'Unavailable project · #44');
});

test('inbox action prefers reviewing existing advice over another analysis, including deferred advice',async()=>{
  const continuity=await server.ssrLoadModule('/src/lib/services/inboxContinuity.ts').catch(()=>({}));
  assert.equal(typeof continuity.inboxAction,'function');
  assert.deepEqual(continuity.inboxAction({pending_ids:[4],deferred_ids:[8],destinations:[],last_job:null},false),{kind:'review',proposalId:4});
  assert.deepEqual(continuity.inboxAction({pending_ids:[],deferred_ids:[8],destinations:[],last_job:null},false),{kind:'review',proposalId:8});
  assert.deepEqual(continuity.inboxAction({pending_ids:[],deferred_ids:[],destinations:[],last_job:{status:'running'}},false),{kind:'running'});
  assert.deepEqual(continuity.inboxAction({pending_ids:[],deferred_ids:[],destinations:[{entity_id:2}],last_job:null},true),{kind:'complete'});
});

test('project prefill uses explicit valid IDs and never names or ambiguous candidates',async()=>{
  const continuity=await server.ssrLoadModule('/src/lib/services/inboxContinuity.ts').catch(()=>({}));
  assert.equal(typeof continuity.inboxProjectPrefill,'function');
  const works=[{id:1,title:'Same',status:'active'},{id:2,title:'Same',status:'active'},{id:3,title:'Old',status:'archived'}];
  assert.equal(continuity.inboxProjectPrefill({work_id:2},works),2);
  assert.equal(continuity.inboxProjectPrefill({work_id:3},works),null);
  assert.equal(continuity.inboxProjectPrefill({work_id:null,candidate_work_ids:[1,2]},works),null);
  assert.equal(continuity.inboxProjectPrefill({work_id:999,title:'Same'},works),null);
});
