import test, {after} from 'node:test';
import assert from 'node:assert/strict';
import {createServer} from 'vite';
const server=await createServer({server:{middlewareMode:true,hmr:false},logLevel:'error'});
after(()=>server.close());
const {render}=await server.ssrLoadModule('svelte/server');
const {proposalPresentation}=await server.ssrLoadModule('/src/lib/services/proposalPresentation.ts');
const {decisionPayload}=await server.ssrLoadModule('/src/lib/services/proposalPayload.ts');
const {sourceDestination}=await server.ssrLoadModule('/src/lib/services/workflowContinuity.ts');
const {default:ProposalPreview}=await server.ssrLoadModule('/src/lib/components/ProposalPreview.svelte');
const experts=[{id:11,name:'合成专家甲',institution:'合成医院',department:'研究科',revision:3,archived:0}];
const item=(patch={})=>({kind:'kol_insight',operation:'create',work_id:null,title:'交流中的证据需求',reason:'根据本次原话',payload_json:JSON.stringify({expert_id:11,expert_revision:3,categories:['evidence_need'],observation:'专家询问长期随访资料',implication:'可能需要补充随访证据',uncertainty:'尚未确认具体人群',next_question:'希望关注哪类患者？',...patch})});
test('insight preview identifies its actual expert and separates observation from interpretation',()=>{
  const result=proposalPresentation(item(),[],'zh-CN',experts);
  assert.equal(result.action,'新增专家洞察');
  assert.equal(result.scope,'合成专家甲 · 合成医院 · 研究科');
  assert.ok(result.changes.includes('观察 / 专家表达：专家询问长期随访资料'));
  assert.ok(result.changes.includes('可能的意义：可能需要补充随访证据'));
  assert.ok(result.changes.includes('待核实：尚未确认具体人群'));
  assert.equal(result.needsAttention,false);
  const {body}=render(ProposalPreview,{props:{item:item(),works:[],experts}});
  assert.match(body,/合成专家甲/);
  assert.match(body,/可能的意义/);
  assert.doesNotMatch(body,/独立事项/);
});
test('missing, deleted or changed expert identity prevents one-click acceptance',()=>{
  for(const [draft,catalog] of [[item(),undefined],[item({expert_id:99}),experts],[item({expert_revision:2}),experts],[item(),[{...experts[0],archived:1}]]]){
    assert.equal(proposalPresentation(draft,[],'zh-CN',catalog).needsAttention,true);
  }
});
test('partial insight update keeps its revision and only explicitly edited fields',()=>{
  const draft={...item(),operation:'update',target_id:17,payload_json:JSON.stringify({expert_id:11,expert_revision:3,insight_revision:'saved-hash',implication:'修正后的理解'})};
  const payload=decisionPayload(draft,'kol_insight');
  assert.equal(payload.insight_revision,'saved-hash');
  assert.equal(payload.implication,'修正后的理解');
  assert.equal(Object.hasOwn(payload,'observation'),false);
  assert.equal(Object.hasOwn(payload,'categories'),false);
});
test('rerouting to an insight preserves the note but requires expert selection',()=>{
  const payload=decisionPayload({kind:'inbox',title:'原话',reason:'',payload_json:'{"content":"合成专家提出随访问题"}'},'kol_insight');
  assert.equal(payload.observation,'合成专家提出随访问题');
  assert.equal(payload.expert_id,undefined);
  assert.equal(proposalPresentation({...item(),payload_json:JSON.stringify(payload)},[],'zh-CN',experts).needsAttention,true);
});
test('confirmed insight receipt opens the exact expert and insight together',()=>{
  assert.deepEqual(sourceDestination({entity_kind:'kol_insight',entity_id:17,expert_id:11,available:true}),{view:'kol',id:11,insightId:17});
});
test('project update names its existing target and blocks an unavailable target',()=>{
  const draft={kind:'work',operation:'update',target_id:7,work_id:null,payload_json:'{"summary":"调整后的目标"}'};
  assert.equal(proposalPresentation(draft,[{id:7,title:'合成研究甲'}],'zh-CN').scope,'合成研究甲');
  assert.equal(proposalPresentation(draft,[],'zh-CN').needsAttention,true);
});
test('insight editor exposes expert identity and editable fact/interpretation fields',async()=>{
  const {default:Fields}=await server.ssrLoadModule('/src/lib/components/ProposalInsightFields.svelte');
  const {body}=render(Fields,{props:{payload:JSON.parse(item().payload_json),experts,updating:true,disabled:false,onchange:()=>{}}});
  assert.match(body,/合成专家甲 · 合成医院 · 研究科/);
  assert.match(body,/<select[^>]*disabled/);
  assert.match(body,/专家询问长期随访资料/);
  assert.match(body,/可能需要补充随访证据/);
  assert.match(body,/尚未确认具体人群/);
});
