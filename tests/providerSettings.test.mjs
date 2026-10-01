import test, {after} from 'node:test';
import assert from 'node:assert/strict';
import {createServer} from 'vite';

// Only render browser components and exercise catalog transforms: no native core,
// keyring, database, provider requests, or personal settings are accessed.
const server=await createServer({server:{middlewareMode:true,hmr:false},logLevel:'silent'});
after(()=>server.close());
const {render}=await server.ssrLoadModule('svelte/server');
const {default:Settings}=await server.ssrLoadModule('/src/lib/components/ProviderSettings.svelte');
const catalog=await server.ssrLoadModule('/src/lib/services/providerCatalog.ts').catch(()=>({}));
const card=await server.ssrLoadModule('/src/lib/components/ProviderConnectionCard.svelte').catch(()=>({}));
const template={kind:'opencode_go',vendor:'OpenCode',display_name:'OpenCode Go',service_tier:'plan',base_url:'https://example.test/go/v1',models_endpoint:'https://example.test/go/v1/models',auth_mode:'bearer',protocol:null,endpoint_path:null,discovery:'remote',docs_url:'https://example.test/docs',note:'Restricted to approved coding clients.',models:[]};
const connection={id:2,display_name:'Research account',provider_type:'opencode_go',base_url:template.base_url,legacy_model:'',enabled:true,template_kind:'opencode_go',auth_mode:'bearer',models_endpoint:template.models_endpoint,last_models_refresh_at:null,created_at:1,updated_at:1};
const model=(changes={})=>({id:21,provider_id:2,model_id:'example-model',display_name:'Example model',protocol:'responses',endpoint_path:'/responses',capabilities_json:'{}',source:'remote',enabled:false,available:true,created_at:1,updated_at:1,...changes});

test('settings exposes one add-connection entry without opening credential forms',()=>{
  const body=render(Settings).body;
  assert.match(body,/data-testid="add-provider-connection"/);
  assert.doesNotMatch(body,/<input[^>]*type="password"/);
});

test('vendor grouping keeps separate accounts under the same vendor and retains custom connections',()=>{
  assert.equal(typeof catalog.groupConnections,'function');
  const groups=catalog.groupConnections([connection,{...connection,id:3,display_name:'Second account'},{...connection,id:4,template_kind:'custom'}],[template]);
  assert.deepEqual(groups.map(g=>[g.vendor,g.connections.map(c=>c.id)]),[['OpenCode',[2,3]],['custom',[4]]]);
});

test('unknown protocols and explicitly unsupported metadata cannot enter supported protocol groups',()=>{
  assert.equal(typeof catalog.groupModels,'function');
  const groups=catalog.groupModels([model(),model({id:22,protocol:'chat_completions'}),model({id:23,protocol:'anthropic_messages'}),model({id:24,protocol:'responses',capabilities_json:'{"needs_protocol":true}'}),model({id:25,protocol:'responses',capabilities_json:'{"unsupported":true}'}),model({id:26,protocol:'mystery'}),model({id:27,capabilities_json:'{"unsupported_protocol":"google_generate_content"}'})]);
  assert.deepEqual(groups.map(g=>[g.protocol,g.models.map(m=>m.id)]),[['responses',[21]],['chat_completions',[22]],['anthropic_messages',[23]],['unknown',[24,25,26,27]]]);
});

test('account access labels distinguish remote availability from a bundled catalog',()=>{
  assert.equal(typeof catalog.modelAccess,'function');
  assert.equal(catalog.modelAccess(model({source:'template'}),'en-US'),'Catalog only · access unverified');
  assert.equal(catalog.modelAccess(model(),'en-US'),'Listed in remote catalog');
  assert.equal(catalog.modelAccess(model({available:false}),'zh-CN'),'当前目录未列出');
  assert.equal(catalog.modelAccess(model({capabilities_json:'{"needs_protocol":true}'}),'en-US'),'Protocol required');
  assert.equal(catalog.modelAccess(model({capabilities_json:'{"unsupported_protocol":"google_generate_content"}'}),'en-US'),'Unsupported protocol');
});

test('each add request creates an independent template connection with its own nickname and key',async()=>{
  assert.equal(typeof catalog.createConnection,'function');
  const requests=[];
  const invoke=async(name,payload)=>{requests.push({name,payload});return {...connection,id:requests.length,display_name:payload.displayName};};
  const first=await catalog.createConnection(invoke,{template,displayName:'First',apiKey:'synthetic-first'});
  const second=await catalog.createConnection(invoke,{template,displayName:'Second',apiKey:'synthetic-second'});
  assert.notEqual(first.id,second.id);
  assert.deepEqual(requests,[{name:'create_provider_template',payload:{templateKind:'opencode_go',displayName:'First',apiKey:'synthetic-first',enabled:true}},{name:'create_provider_template',payload:{templateKind:'opencode_go',displayName:'Second',apiKey:'synthetic-second',enabled:true}}]);
});

test('a plan that prohibits application use cannot be created through its preset',async()=>{
  assert.equal(typeof catalog.createConnection,'function');
  await assert.rejects(()=>catalog.createConnection(async()=>connection,{template:{...template,configurable:false},displayName:'Blocked plan',apiKey:'synthetic'}),/provider|plan/i);
});

test('explicit manual protocol repair restores missing models while preserving capacity and disabled choice',()=>{
  assert.equal(typeof catalog.modelSavePayload,'function');
  const original=model({capabilities_json:'{"needs_protocol":true,"unsupported":true,"unsupported_protocol":"google_generate_content","context_window":64000}',available:false});
  const payload=catalog.modelSavePayload(connection.id,{modelId:original.model_id,displayName:'Fixed',protocol:'chat_completions',endpointPath:'/chat/completions',enabled:false},original);
  assert.deepEqual(payload,{providerId:2,modelId:'example-model',displayName:'Fixed',protocol:'chat_completions',endpointPath:'/chat/completions',capabilitiesJson:'{"context_window":64000}',source:'manual',enabled:false,available:true,confirmAvailable:true});
  assert.throws(()=>catalog.modelSavePayload(2,{modelId:'x',displayName:'x',protocol:'mystery',endpointPath:'/x',enabled:true}),/protocol/i);
});

test('manual model endpoints reject traversal before saving',()=>{
  assert.equal(typeof catalog.modelSavePayload,'function');
  for(const path of ['responses','/../messages']) assert.throws(()=>catalog.modelSavePayload(2,{modelId:'x',displayName:'x',protocol:'responses',endpointPath:path,enabled:false}),/endpoint/i);
});

test('fold persistence isolates vendor, connection, and protocol keys and ignores malformed saved values',()=>{
  assert.equal(typeof catalog.readFolds,'function');
  const memory=new Map([['msl.provider-model-folds','{"2":true,"3":"yes"}']]);
  const storage={getItem:key=>memory.get(key)??null,setItem:(key,value)=>memory.set(key,value)};
  let folds=catalog.readFolds(storage);
  assert.deepEqual(folds,{'models:2':true});
  folds=catalog.saveFold(storage,folds,'vendor:OpenCode',true);
  folds=catalog.saveFold(storage,folds,'protocol:2:responses',true);
  folds=catalog.saveFold(storage,folds,'models:3',false);
  assert.deepEqual(catalog.readFolds(storage),{'models:2':true,'vendor:OpenCode':true,'protocol:2:responses':true,'models:3':false});
});

test('connection card defaults its model and protocol sections closed and retains recovery controls',()=>{
  assert.equal(typeof card.default,'function');
  const body=render(card.default,{props:{connection,models:[model(),model({id:22,protocol:'unknown'})],template,hasKey:true,folds:{},locale:'en-US',busy:null,onfold(){},onkey(){},onrefresh(){},onedit(){},onremove(){},onmodel(){},ontoggle(){},ontest(){}}}).body;
  assert.match(body,/data-testid="models-fold-2"/);
  assert.match(body,/data-testid="protocol-fold-2-responses"/);
  assert.match(body,/data-testid="connection-2-api-key"/);
  assert.match(body,/data-testid="model-edit-22"/);
  assert.match(body,/data-testid="model-toggle-22"[^>]*disabled/);
  assert.match(body,/Restricted to approved coding clients/);
  assert.doesNotMatch(body,/<details[^>]*\sopen(?:\s|>)/);
});

test('custom connections retain refresh, rename, and manual model entry controls',()=>{
  assert.equal(typeof card.default,'function');
  const body=render(card.default,{props:{connection:{...connection,template_kind:'custom'},models:[],hasKey:false,folds:{},locale:'zh-CN',busy:null,onfold(){},onkey(){},onrefresh(){},onedit(){},onremove(){},onmodel(){},ontoggle(){},ontest(){}}}).body;
  for(const id of ['connection-refresh-2','connection-edit-2','model-add-2']) assert.match(body,new RegExp(`data-testid="${id}"`));
});

test('new Contributor variants keep their training-data notice',()=>{
  assert.equal(typeof card.default,'function');
  const body=render(card.default,{props:{connection,models:[model({model_id:'muse-spark-future-contributor'})],hasKey:true,folds:{},locale:'en-US',busy:null,onfold(){},onkey(){},onrefresh(){},onedit(){},onremove(){},onmodel(){},ontoggle(){},ontest(){}}}).body;
  assert.match(body,/prompts and completions may be used to train future models/);
});
