import test, {after} from 'node:test';
import assert from 'node:assert/strict';
import {createServer} from 'vite';
const server=await createServer({server:{middlewareMode:true,hmr:false},logLevel:'error'});
after(()=>server.close());
const {render}=await server.ssrLoadModule('svelte/server');
const {default:AiDocument}=await server.ssrLoadModule('/src/lib/components/AiDocument.svelte');
const {default:ProposalPreview}=await server.ssrLoadModule('/src/lib/components/ProposalPreview.svelte');
test('structured answers render reading text safely without Markdown or active model HTML',()=>{
  const statement={text:'**结论** <img src=x onerror=alert(1)>',basis:'fact',citations:[]};
  const document={title:'**回答**',sections:[{title:'## 进展',blocks:[{type:'paragraph',content:statement},{type:'bullets',items:[statement]},{type:'numbered',items:[statement]},{type:'table',columns:['**项目**'],rows:[[statement]]}]}]};
  const {body}=render(AiDocument,{props:{document}});
  assert.doesNotMatch(body,/\*\*|##|<img|<script/);
  assert.match(body,/&lt;img/);
  assert.match(body,/结论/);
});
test('suggestion rationale uses the same reading projection',()=>{
  const {body}=render(ProposalPreview,{props:{item:{kind:'task',work_id:null,payload_json:'{}',reason:'**依据**：先核对 *资料*。'},works:[]}});
  assert.doesNotMatch(body,/\*\*/);
  assert.match(body,/依据：先核对 资料。/);
});
