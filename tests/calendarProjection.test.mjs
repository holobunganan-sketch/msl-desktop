import {test} from 'node:test';import assert from 'node:assert/strict';
const mod=await import('../src/lib/services/calendarProjection.ts').catch(()=>({}));
const task={id:7,work_id:3,title:'准备拜访',status:'next',scheduled_start:120,scheduled_end:150,due_at:190,notes:'保留',created_at:1,updated_at:2};
test('calendar preserves task identity across appointments and separate deadlines',()=>{
  assert.equal(typeof mod.projectCalendar,'function');
  const events=mod.projectCalendar([task],[],100,200);
  assert.deepEqual(events.map(e=>[e.task_id,e.work_id,e.time_kind,e.start_at]),[[7,3,'appointment',120],[7,3,'deadline',190]]);
  assert.equal(new Set(events.map(e=>e.id)).size,2);
});
test('same-time deadline does not duplicate the appointment; original task is untouched',()=>{
  assert.equal(typeof mod.projectCalendar,'function');
  const item={...task,due_at:120};const before=JSON.stringify(item);
  assert.equal(mod.projectCalendar([item],[],100,200).length,1);
  assert.equal(JSON.stringify(item),before);
});
test('waiting followups and deadline-only tasks need no copied calendar entity',()=>{
  assert.equal(typeof mod.projectCalendar,'function');
  const waiting={id:7,work_id:null,title:'等材料',status:'open',follow_up_at:180,notes:null,created_at:1,updated_at:1};
  const events=mod.projectCalendar([{...task,scheduled_start:null,scheduled_end:null}], [waiting],100,200);
  assert.deepEqual(events.map(e=>[e.task_id??null,e.waiting_id??null,e.time_kind]),[[7,null,'deadline'],[null,7,'followup']]);
  assert.equal(new Set(events.map(e=>e.id)).size,2);
  assert.equal(mod.projectCalendar([task],[waiting],200,300).length,0);
});
test('completion remains visible as completion, and missing dates never create events',()=>{
  assert.equal(typeof mod.projectCalendar,'function');
  assert.ok(mod.projectCalendar([{...task,status:'done'}],[],100,200).every(e=>e.completed));
  assert.deepEqual(mod.projectCalendar([{...task,due_at:null,scheduled_start:null}],[],100,200),[]);
});
