import { test } from 'node:test';
import assert from 'node:assert/strict';
const nav = await import('../src/lib/services/navigation.ts').catch(() => ({}));
const presentation = await import('../src/lib/services/proposalPresentation.ts').catch(() => ({}));

test('navigation retains the exact task and project instead of sending people to an unrelated list', () => {
  assert.equal(typeof nav.resolveDestination, 'function');
  assert.deepEqual(nav.resolveDestination('task', 42), {view:'matters',section:'plan',id:42});
  assert.deepEqual(nav.resolveDestination('work', 7), {view:'works',id:7});
  assert.deepEqual(nav.resolveDestination('resume_point', 21, 7), {view:'works',id:7});
});
test('legacy review and waiting links still lead to their corresponding matter view', () => {
  assert.equal(typeof nav.resolveDestination, 'function');
  assert.deepEqual(nav.resolveDestination('review', 9), {view:'matters',section:'review',id:9});
  assert.deepEqual(nav.resolveDestination('waiting', 4), {view:'matters',section:'waiting',id:4});
  assert.equal(nav.resolveDestination('invalid-route'), null);
});
test('update preview describes the existing target and does not invent a missing date', () => {
  assert.equal(typeof presentation.proposalPresentation, 'function');
  const item={kind:'calendar',operation:'update',work_id:7,payload_json:'{}'};
  const result=presentation.proposalPresentation(item,[{id:7,title:'研究沟通'}],'zh-CN');
  assert.equal(result.action,'更新已有日程');
  assert.equal(result.scope,'研究沟通');
  assert.equal(result.time,'保持原有时间');
  assert.equal(result.needsAttention,false);
  const create=presentation.proposalPresentation({...item,operation:'create'},[],'zh-CN');
  assert.equal(create.needsAttention,true);
  assert.equal(create.time,'请确认安排时间');
});
test('progress without a project requires a choice; standalone task does not', () => {
  assert.equal(typeof presentation.proposalPresentation, 'function');
  assert.equal(presentation.proposalPresentation({kind:'resume_point',work_id:null,payload_json:'{}'},[],'zh-CN').needsAttention,true);
  assert.equal(presentation.proposalPresentation({kind:'task',work_id:null,payload_json:'{}'},[],'zh-CN').scope,'独立事项');
});

test('Decision preview exposes the actual changes before accepting',()=>{
  const preview=presentation.proposalPresentation({kind:'resume_point',work_id:7,payload_json:JSON.stringify({current_state:'已完成沟通',next_step:'等待材料'})},[{id:7,title:'长期项目'}],'zh-CN');
  assert.deepEqual(preview.changes,['目前进展：已完成沟通','下一步：等待材料']);
  const task=presentation.proposalPresentation({kind:'task',operation:'update',work_id:null,payload_json:'{"status":"done"}'},[],'zh-CN');
  assert.deepEqual(task.changes,['状态：已完成']);
});

test('Future appointments show their date on Today instead of looking like today',()=>{
  const now=new Date(2026,8,5,10).getTime()/1000;
  const later=new Date(2026,8,6,15).getTime()/1000;
  assert.equal(presentation.actionTimeLabel(later,now),'09/06\n15:00');
  assert.equal(presentation.actionTimeLabel(now,now),'10:00');
});

test('inferred schedule is visibly tentative and accepting it explains calendar placement',()=>{
  const preview=presentation.proposalPresentation({kind:'task',work_id:null,payload_json:JSON.stringify({scheduled_start:2000000000,time_basis:'inferred',time_reason:'在截止前预留准备时间'})},[],'zh-CN');
  assert.equal(preview.timeBasis,'AI 建议时间');assert.match(preview.calendarHint,/日历/);assert.equal(preview.timeReason,'在截止前预留准备时间');
});

test('Decision preview shows every proposed time node, including a separate deadline and clearing',()=>{
 const p=presentation.proposalPresentation({kind:'task',operation:'update',work_id:null,payload_json:JSON.stringify({scheduled_start:2000000000,scheduled_end:2000003600,due_at:2000086400})},[],'zh-CN');
 assert.equal(p.timeDetails.length,3);assert.match(p.timeDetails[0],/安排开始/);assert.match(p.timeDetails[2],/截止/);
 const clear=presentation.proposalPresentation({kind:'waiting',operation:'update',work_id:null,payload_json:'{"follow_up_at":null}'},[],'zh-CN');
 assert.deepEqual(clear.timeDetails,['跟进时间：清除原时间']);assert.equal(clear.calendarHint,'');
});
