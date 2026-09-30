import test, {after} from 'node:test';
import assert from 'node:assert/strict';
import {createServer} from 'vite';
import {readFileSync} from 'node:fs';
const server=await createServer({server:{middlewareMode:true,hmr:false},logLevel:'error'});
after(()=>server.close());
const {render}=await server.ssrLoadModule('svelte/server');
const {proposalPresentation}=await server.ssrLoadModule('/src/lib/services/proposalPresentation.ts');
const {default:ProposalPreview}=await server.ssrLoadModule('/src/lib/components/ProposalPreview.svelte');
const reviewModule=await server.ssrLoadModule('/src/lib/services/proposalPayload.ts');
const works=[{id:1,title:'学术交流',category:'non_clinical'},{id:2,title:'合成研究 A',category:'clinical'}];

test('compact-input policy cannot instruct the model to erase fields through output nulls',()=>{
  const source=readFileSync(new URL('../src-tauri/src/ai/efficiency.rs',import.meta.url),'utf8');
  const contract=source.match(/pub const INPUT_CONTRACT: &str = "([^\n]+)";/)[1];
  assert.doesNotMatch(contract,/preserve explicit null in OUTPUT patches/);
  assert.match(contract,/model-generated.*unknown/i);
  assert.match(contract,/user.*editor/i);
});

test('draft save cannot preserve a time-approval token supplied by a model or previous action',()=>{
  const item={kind:'task',title:'合成安排',reason:'',payload_json:JSON.stringify({scheduled_start:1801267200,time_basis:'inferred',time_confirmation:'user_confirmed'})};
  assert.equal(reviewModule.decisionPayload(item,'task').time_confirmation,undefined);
});

test('only a separate current user action adds a time approval to the confirmation request',()=>{
  const payload={scheduled_start:1801267200,time_basis:'inferred',notes:'保持原话'};
  let accepted,plain;
  assert.doesNotThrow(()=>{accepted=reviewModule.confirmationPayload(payload,true);plain=reviewModule.confirmationPayload(payload,false);});
  assert.equal(accepted.time_confirmation,'user_confirmed');
  assert.equal(plain.time_confirmation,undefined);
  assert.equal(payload.time_confirmation,undefined);
  assert.equal(accepted.notes,'保持原话');
});

test('tentative dates cannot look ready for one-click acceptance',()=>{
  const item={kind:'task',work_id:1,payload_json:JSON.stringify({scheduled_start:1801267200,time_basis:'inferred',time_reason:'拟议时段'})};
  const preview=proposalPresentation(item,works,'zh-CN');
  assert.equal(preview.needsAttention,true);
  assert.match(preview.calendarHint,/单独确认/);
  const explicit=proposalPresentation({...item,payload_json:JSON.stringify({scheduled_start:1801267200,time_basis:'explicit'})},works,'zh-CN');
  assert.equal(explicit.needsAttention,false);
});

test('changing a suggestion type preserves clinical association and tentative-time safeguards',()=>{
  const item={kind:'waiting',title:'合成安排',reason:'待核对',payload_json:JSON.stringify({clinical_work_id:2,due_at:1801267200,time_basis:'inferred',time_reason:'拟议日期',field_evidence:{due_at:{basis:'suggestion',snapshot_path:'/focused_inbox/content',quote:'安排一次交流'}},unknowns:['确切日期待确认']})};
  const changed=reviewModule.decisionPayload(item,'task');
  assert.equal(changed.clinical_work_id,2);
  assert.equal(changed.time_basis,'inferred');
  assert.equal(changed.field_evidence.due_at.basis,'suggestion');
  assert.deepEqual(changed.unknowns,['确切日期待确认']);
  assert.ok(proposalPresentation({...item,kind:'task',payload_json:JSON.stringify(changed)},works,'zh-CN').needsTimeConfirmation);
});

test('clinical association is visible without replacing the owning project',()=>{
  const item={kind:'task',work_id:1,payload_json:JSON.stringify({clinical_work_id:2})};
  const preview=proposalPresentation(item,works,'zh-CN');
  assert.equal(preview.scope,'学术交流');
  assert.ok(preview.changes.includes('关联临床研究：合成研究 A'));
  assert.ok(proposalPresentation({...item,payload_json:'{"clinical_work_id":999}'},works,'zh-CN').needsAttention);
});

test('classification and supplied evidence remain readable with explicit unknown information',()=>{
  const item={kind:'work',work_id:null,reason:'待用户校准',payload_json:JSON.stringify({category:'clinical',summary:'观察性研究',field_evidence:{summary:{basis:'explicit',snapshot_path:'/focused_inbox/content',quote:'计划开展**观察性研究** <img src=x>'}},unknowns:['截止日期未提供']})};
  const {body}=render(ProposalPreview,{props:{item,works}});
  assert.match(body,/临床研究/);
  assert.match(body,/计划开展观察性研究/);
  assert.match(body,/截止日期未提供/);
  assert.doesNotMatch(body,/<img|\*\*|snapshot_path|focused_inbox/);
});
