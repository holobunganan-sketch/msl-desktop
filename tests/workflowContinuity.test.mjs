import test,{after} from 'node:test';
import assert from 'node:assert/strict';
import {createServer} from 'vite';
const nav=await import('../src/lib/services/navigation.ts');
const server=await createServer({server:{middlewareMode:true,hmr:false},logLevel:'error'});
after(()=>server.close());
const flow=await server.ssrLoadModule('/src/lib/services/workflowContinuity.ts');
test('task and review routes preserve their project scope',()=>{
 assert.deepEqual(nav.resolveDestination('task',9,3),{view:'matters',section:'plan',id:9,workId:3});
 assert.deepEqual(nav.resolveDestination('review',undefined,3),{view:'matters',section:'review',workId:3});
});
test('resume search keeps the resume identity when its project is known',()=>{
 assert.deepEqual(nav.resolveDestination('resume_point',41,3),{view:'works',id:3,resumeId:41});
});
test('material locations retain the exact material identity',()=>{
 assert.deepEqual(flow.sourceDestination({entity_kind:'kol_material',entity_id:90,material_id:7,expert_id:2,available:true}),{view:'kol',id:2,materialId:7});
 assert.deepEqual(flow.expertFocus({materialId:7}),{tab:'materials',materialId:7});
 assert.deepEqual(flow.expertFocus({noteId:8}),{tab:'record',noteId:8});
});
test('trusted source locations open exact expert notes and cannot open remote or deleted files',()=>{
 assert.equal(typeof flow.sourceDestination,'function');
 assert.deepEqual(flow.sourceDestination({entity_kind:'kol_note',entity_id:8,expert_id:4,available:true}),{view:'kol',id:4,noteId:8});
 assert.deepEqual(flow.sourceDestination({entity_kind:'document',entity_id:8,workspace_id:2,relative_path:'sub/a.pdf',available:true}),{view:'workspace',workspaceId:2,relativePath:'sub/a.pdf'});
 assert.equal(flow.sourceDestination({entity_kind:'document',entity_id:8,workspace_id:2,available:false}),null);
 assert.equal(flow.sourceDestination({entity_kind:'work',entity_id:3,available:true,remote_only:true}),null);
});
test('Today excludes paused work while preserving independent tasks and upcoming schedules',()=>{
 assert.equal(typeof flow.actionableTasks,'function');
 const tasks=[{id:1,work_id:1,status:'next',due_at:90},{id:2,work_id:2,status:'next'},{id:3,work_id:null,status:'paused'},{id:4,work_id:1,status:'waiting'},{id:5,work_id:null,status:'next'},{id:6,work_id:1,status:'scheduled',scheduled_start:200},{id:7,work_id:99,status:'next'}];
 assert.deepEqual(flow.actionableTasks(tasks,[{id:1,status:'active'},{id:2,status:'archived'}]).map(t=>t.id),[1,6,5]);
 assert.equal(flow.taskReason(tasks[0],100,false),'已到截止时间');
 assert.equal(flow.taskReason(tasks[5],100,false),'已安排时间');
});
test('expert drafts survive switches and saving an older snapshot preserves newer edits',()=>{
 assert.equal(typeof flow.writeExpertDraft,'function');
 const draft={content:'Original',at:'2026-09-27T10:00',workId:3,inboxId:null};
 flow.writeExpertDraft(1,draft);flow.writeExpertDraft(2,{...draft,content:'Other'});
 assert.equal(flow.readExpertDraft(1).content,'Original');
 flow.writeExpertDraft(1,{...draft,content:'Newer'});flow.clearSavedExpertDraft(1,draft);
 assert.equal(flow.readExpertDraft(1).content,'Newer');
 flow.clearSavedExpertDraft(2,{...draft,content:'Other'});assert.equal(flow.readExpertDraft(2),null);
});
test('explicit expert entry never selects unrelated conversation and survives deleted markers',()=>{
 assert.equal(typeof flow.expertSession,'function');
 const sessions=[{id:1,expert_id:null,expert_scoped:0},{id:2,expert_id:5,expert_scoped:1}];
 assert.equal(flow.expertSession(sessions,3),null);assert.equal(flow.expertSession(sessions,5).id,2);
 assert.equal(flow.expertSession([{id:3,expert_id:null,expert_scoped:1}],5),null);
 assert.equal(flow.expertSession([{id:4,expert_id:5,expert_scoped:1,scope_json:'[1,2]'}],5,[1]),null);
 assert.equal(flow.expertSession([{id:5,expert_id:5,expert_scoped:1,scope_json:'[2,1]'}],5,[1,2]).id,5);
});
test('background results keep exact reports, projects and expert drafts',()=>{
 assert.deepEqual(flow.jobDestination({command:'generate_report',args:{},result:12}),{view:'reports',id:12});
 assert.deepEqual(flow.jobDestination({command:'start_workspace_work_draft',args:{workId:7},result:21}),{view:'matters',section:'review',workId:7,runId:21});
 assert.deepEqual(flow.jobDestination({command:'analyze_kol',args:{expertId:2},result:{expert_id:2,draft_id:8}}),{view:'kol',id:2,draftId:8});
});
test('report citations use canonical metadata and legacy reports remain readable',()=>{
 const report={structured_json:JSON.stringify({version:'report-spec-v2',items:[{headline:'Progress',change:'Done',evidence_refs:[{source_type:'task_completed',entity_id:2}]}]}),evidence_json:JSON.stringify({sources:[{source_type:'task_completed',entity_id:2,location:{entity_kind:'task',entity_id:2,available:true}}]})};
 assert.equal(flow.reportItems(report)[0].sources[0].location.entity_id,2);
 assert.deepEqual(flow.reportItems({content:'1. Legacy',structured_json:null}),[]);
});
