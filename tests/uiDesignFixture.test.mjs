import test from 'node:test';
import assert from 'node:assert/strict';
test('review page required parallel reads resolve with typed memory statistics and matching runs',async()=>{
  const {invoke}=await fixture();
  const [recent,pending,works,runs,memory]=await Promise.all([
    invoke('list_recent_ai_proposals',{cutoffCreatedAt:0,status:null,limit:500}),invoke('list_ai_proposals',{status:'pending',limit:500}),
    invoke('list_works',{status:null}),invoke('list_analysis_runs',{limit:20}),invoke('get_classification_memory_stats')
  ]);
  assert.deepEqual(memory,{pattern_count:0,feedback_count:0,accepted_count:0,corrected_count:0,rejected_count:0,updated_at:null});
  assert.ok(works.length>0&&pending.length>0);
  for(const proposal of recent)assert.ok(runs.some(run=>run.id===proposal.analysis_run_id));
});
let serial=0;
async function fixture(search='') {
  globalThis.location={search};globalThis.window={};
  const module=await import(`../scripts/ui-design-fixture.mjs?test=${++serial}`);
  return {invoke:module.invoke,controls:window.__MSL_FIXTURE__};
}
test('completion receipts restore exact prior status and stale full snapshots cannot delete',async()=>{
  const {invoke}=await fixture();
  const before=(await invoke('list_tasks'))[0];
  const receipt=await invoke('complete_task',{id:before.id});
  assert.equal(typeof receipt?.id,'string');
  assert.equal((await invoke('list_tasks')).find(t=>t.id===before.id).status,'done');
  assert.equal((await invoke('list_manual_completions')).length,1);
  await invoke('undo_manual_completion',{receiptId:receipt.id});
  const restored=(await invoke('list_tasks')).find(t=>t.id===before.id);
  assert.equal(restored.status,before.status);
  await assert.rejects(invoke('delete_task',{id:before.id,confirmed:true,expectedUpdatedAt:before.updated_at,expectedRecord:before}));
  await invoke('delete_task',{id:restored.id,confirmed:true,expectedUpdatedAt:restored.updated_at,expectedRecord:restored});
  assert.equal((await invoke('list_tasks')).some(t=>t.id===before.id),false);
});
test('expert-only sessions stay distinct from global and linked expert scopes resolve',async()=>{
  const {invoke}=await fixture();
  const experts=await invoke('list_kol_experts');assert.equal(experts.length,2);
  assert.equal(experts[1].project_ids_json,'[]');assert.equal(experts[1].archived,0);
  const linked=await invoke('create_qa_session',{scope:[],expertId:1});
  assert.equal(linked.scope_json,'[1]');assert.equal(linked.expert_scoped,1);
  const independent=await invoke('create_qa_session',{scope:[],expertId:2});
  assert.equal(independent.scope_json,'[]');assert.equal(independent.expert_scoped,1);
  await invoke('delete_kol_expert',{id:2,confirmationName:experts[1].name,expectedRevision:1});
  await assert.rejects(invoke('queue_qa_question',{sessionId:independent.id,question:'test',scope:[],expertId:2}));
});
test('delayed capture preserves submitted expert and report/search sources carry exact identity',async()=>{
  const {invoke,controls}=await fixture();
  assert.ok(controls,'preview controls available');
  controls.hold('capture_kol_note');
  const pending=invoke('capture_kol_note',{expertId:2,content:'independent note',workId:null});
  assert.equal((await invoke('list_kol_notes',{expertId:2})).length,0);
  controls.release('capture_kol_note');await pending;
  assert.equal((await invoke('list_kol_notes',{expertId:2}))[0].content,'independent note');
  const reports=await invoke('list_reports');assert.equal(reports.length,2);
  assert.equal(JSON.parse(reports[1].structured_json).items[0].evidence_refs[0].source_type,'task_open');
  const refs=JSON.parse(reports[1].structured_json).items[0].evidence_refs;
  const reportSources=JSON.parse(reports[1].evidence_json).sources;
  assert.deepEqual(refs.map(ref=>`${ref.source_type}:${ref.entity_id}`),['task_open:1','task_open:999']);
  assert.deepEqual(refs.map(ref=>reportSources.find(s=>s.source_type===ref.source_type&&s.entity_id===ref.entity_id)?.location.available),[true,false]);
  assert.ok(JSON.parse(reports[1].evidence_json).sources.some(s=>s.location.available===false));
  const results=await invoke('search',{query:'演示'});
  assert.ok(results.resume_points.some(r=>r.id!==r.work_id));
  assert.match(results.files[0].path,/随访问题讨论提纲\.docx$/);
  await assert.rejects(invoke('unsupported_mutation'));
});
test('QA background answer waits for release and cannot land after expert deletion',async()=>{
  const {invoke,controls}=await fixture();
  const models=await invoke('list_provider_models');assert.equal(models[0]?.available,true);
  const session=await invoke('create_qa_session',{scope:[],expertId:2});
  const turn=await invoke('queue_qa_question',{sessionId:session.id,expertId:2,question:'独立专家问题',scope:[]});
  assert.equal(turn.expert_scoped,1);
  controls.hold('ask_workbench');
  const job=await invoke('start_ai_job',{request:{command:'ask_workbench',args:{turnId:turn.id}}});
  assert.equal(job.status,'running');
  const expert=(await invoke('list_kol_experts'))[1];
  await invoke('delete_kol_expert',{id:2,confirmationName:expert.name,expectedRevision:1});
  controls.release('ask_workbench');await new Promise(r=>setTimeout(r,0));
  assert.equal((await invoke('get_ai_job',{id:job.id})).status,'failed');
  assert.equal((await invoke('list_qa_turns',{sessionId:session.id}))[0].answer_json,null);
});
test('review decisions mutate only selected work and generated reports get persistent job results',async()=>{
  const {invoke}=await fixture();
  const proposal=(await invoke('list_ai_proposals',{workId:1}))[0];
  await invoke('reject_ai_proposal',{id:proposal.id,expectedUpdatedAt:proposal.updated_at,reason:'not_now'});
  assert.equal((await invoke('list_ai_proposals',{workId:1,status:'pending'})).length,0);
  assert.equal((await invoke('list_ai_proposals',{workId:3,status:'pending'})).length,1);
  const job=await invoke('start_ai_job',{request:{command:'generate_report',args:{kind:'weekly'}}});
  await new Promise(r=>setTimeout(r,0));
  const finished=await invoke('get_ai_job',{id:job.id});assert.equal(finished.status,'completed');
  assert.equal((await invoke('get_report',{id:finished.result})).status,'completed');
});
test('same timestamp concurrent deletion conflict changes row and waiting recovery mutates state',async()=>{
  const {invoke}=await fixture('?delete-conflict');
  const row=(await invoke('list_tasks'))[0];
  await assert.rejects(invoke('delete_task',{id:row.id,confirmed:true,expectedUpdatedAt:row.updated_at,expectedRecord:row}));
  const current=(await invoke('list_tasks'))[0];assert.equal(current.updated_at,row.updated_at);assert.notEqual(current.notes,row.notes);
  const receipt=await invoke('resolve_waiting',{id:1});assert.equal((await invoke('list_waiting_items'))[0].status,'resolved');
  await invoke('undo_manual_completion',{receiptId:receipt.id});assert.equal((await invoke('list_waiting_items'))[0].status,'open');
});
test('expert QA cites only selected saved notes with trusted record location',async()=>{
  const {invoke}=await fixture();
  await invoke('capture_kol_note',{expertId:1,content:'other expert secret'});
  const note=await invoke('capture_kol_note',{expertId:2,content:'selected original'});
  const session=await invoke('create_qa_session',{expertId:2,scope:[]});
  const turn=await invoke('queue_qa_question',{expertId:2,scope:[],sessionId:session.id,question:'next?'});
  await invoke('ask_workbench',{turnId:turn.id});
  const saved=(await invoke('list_qa_turns',{sessionId:session.id}))[0];
  const sources=JSON.parse(saved.evidence_json).sources;
  assert.equal(sources.length,1);assert.equal(sources[0].location.entity_id,note.id);assert.equal(sources[0].location.expert_id,2);
  assert.equal(saved.evidence_json.includes('other expert secret'),false);
});
