import { test } from 'node:test';
import assert from 'node:assert/strict';
import { decisionPayload } from '../src/lib/services/proposalPayload.ts';
const review=await import('../src/lib/services/proposalPayload.ts');
const draft=(kind,payload)=>({kind,title:'Synthetic',payload_json:JSON.stringify(payload),reason:'Evidence'});
test('home confirmation preserves partial updates and completion states',()=>{
  assert.deepEqual(decisionPayload(draft('task',{status:'done'}),'task'),{title:'Synthetic',status:'done'});
  assert.deepEqual(decisionPayload(draft('waiting',{status:'resolved'}),'waiting'),{title:'Synthetic',status:'resolved'});
  assert.deepEqual(decisionPayload(draft('work',{status:'archived'}),'work'),{title:'Synthetic',status:'archived'});
});
test('home confirmation preserves project progress fields',()=>{
  const fields={current_state:'Received',next_step:'Review',remember:'Keep context'};
  assert.deepEqual(decisionPayload(draft('resume_point',fields),'resume_point'),{...fields,title:'Synthetic'});
});
test('changing destination does not invent a calendar date or carry incompatible status',()=>{
  const original=draft('waiting',{status:'resolved'});
  assert.equal(decisionPayload(original,'calendar').start_at,null);
  assert.equal(decisionPayload(original,'task').status,undefined);
});
test('legacy reclassified project drafts drop task-only status without losing edits',()=>{
  const item={...draft('work',{status:'next',priority:'normal',summary:'Edited summary'}),suggested_kind:'task',user_edited:true};
  const payload=decisionPayload(item,'work');
  assert.equal(payload.status,undefined);
  assert.equal(payload.summary,'Edited summary');
});
test('classification conversions keep the latest edited content as project progress',()=>{
  const payload=decisionPayload(draft('task',{notes:'Edited progress',status:'done'}),'resume_point');
  assert.equal(payload.current_state,'Edited progress');
  assert.equal(payload.status,undefined);
});

test('unrecognized statuses remain visible for explicit review instead of being guessed',()=>{
  const item={...draft('work',{status:'unknown'}),suggested_kind:'task',user_edited:true};
  assert.equal(decisionPayload(item,'work').status,'unknown');
  assert.equal(decisionPayload({...item,payload_json:'{"status":"done"}'},'work').status,'done');
});

test('Review forms must never turn omitted fields into clearing patches',()=>{
  for(const kind of ['task','waiting','calendar']){
    const fields=kind==='calendar'?{location:'Room A'}:{status:kind==='task'?'done':'resolved'};
    assert.equal(Object.hasOwn(review.reviewPayload(draft(kind,fields)),'notes'),false);
    assert.equal(review.reviewPayload(draft(kind,{notes:''})).notes,'');
  }
  assert.equal(review.reviewPayload(draft('task',{summary:'Changed context'})).notes,'Changed context');
});
