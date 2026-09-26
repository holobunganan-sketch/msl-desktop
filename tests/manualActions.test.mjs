import test from 'node:test';
import assert from 'node:assert/strict';
const actions=await import('../src/lib/services/manualActions.ts').catch(()=>({}));
test('delete carries the full immutable confirmation snapshot',()=>{
 assert.equal(typeof actions.deletionRequest,'function');
 const record={id:3,title:'Synthetic',updated_at:10,status:'next',notes:'keep me'};
 const request=actions.deletionRequest(record);record.notes='new';
 assert.deepEqual(request,{id:3,confirmed:true,expectedUpdatedAt:10,expectedRecord:{id:3,title:'Synthetic',updated_at:10,status:'next',notes:'keep me'}});
});
test('manual completion suppresses duplicate clicks and undo rejection retains the receipt',async()=>{
 assert.equal(typeof actions.createManualActions,'function');
 let calls=0,changed=0,release;
 const result={id:'receipt',entity_kind:'task',entity_id:3,title:'Synthetic',created_at:1,undone_at:null};
 const service=actions.createManualActions(async(name)=>{calls++;if(name==='complete_task'){await new Promise(r=>release=r);return result;}throw Error('Later edit');},()=>changed++);
 const one=service.complete('task',3),two=service.complete('task',3);release();
 assert.deepEqual(await one,result);assert.deepEqual(await two,result);assert.equal(calls,1);assert.equal(changed,1);
 await assert.rejects(service.undo(result),/Later edit/);assert.equal(result.undone_at,null);assert.equal(changed,1);
});
