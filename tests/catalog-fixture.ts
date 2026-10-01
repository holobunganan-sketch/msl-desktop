import {mount} from 'svelte';
import Fixture from './CatalogFixture.svelte';
import {locale} from '../src/lib/i18n';
const models=Array.from({length:12},(_,i)=>({id:i+1,provider_id:1,model_id:`model-${i}`,display_name:'Synthetic model '+i,protocol:'chat_completions',endpoint_path:'/chat/completions',capabilities_json:'{}',source:'manual',enabled:true,available:true,created_at:1,updated_at:1}));
const providers=[{id:1,display_name:'Synthetic provider',provider_type:'custom',template_kind:'custom',base_url:'http://127.0.0.1:9999',legacy_model:'',auth_mode:'bearer',models_endpoint:null as string|null,last_models_refresh_at:null as number|null,enabled:true,created_at:1,updated_at:1}];
const templates=[
 {kind:'deepseek',vendor:'DeepSeek',display_name:'DeepSeek API',service_tier:'api',base_url:'https://synthetic.invalid/v1',models_endpoint:'https://synthetic.invalid/v1/models',auth_mode:'bearer',protocol:'chat_completions',endpoint_path:'/chat/completions',discovery:'remote',docs_url:'https://example.test/docs',note:'Synthetic API fixture. No network requests are made.',configurable:true,models:[]},
 {kind:'opencode_go',vendor:'OpenCode',display_name:'OpenCode Go',service_tier:'plan',base_url:'https://synthetic.invalid/go/v1',models_endpoint:'https://synthetic.invalid/go/v1/models',auth_mode:'bearer',protocol:null,endpoint_path:null,discovery:'remote',docs_url:'https://example.test/docs',note:'Synthetic plan. Check provider terms before background use.',configurable:true,models:[]},
 {kind:'restricted_plan',vendor:'Restricted provider',display_name:'Restricted Plan',service_tier:'plan',base_url:'https://synthetic.invalid/plan/v1',models_endpoint:null,auth_mode:'bearer',protocol:'chat_completions',endpoint_path:'/chat/completions',discovery:'curated',docs_url:'https://example.test/docs',note:'This synthetic plan only supports approved coding clients.',configurable:false,models:[]}
];
const sessions=[1,2].map(id=>({id,title:'测试会话 '+id,scope_json:'[]',updated_at:1700000000}));
let failNextModelList=false;
Object.assign(window,{__catalogFailNextModelList:()=>{failNextModelList=true;},__catalogSetLocale:(language:'zh-CN'|'en-US')=>locale.set(language),__TAURI_INTERNALS__:{invoke:async(name:string,args:Record<string,unknown>={})=>{
 if(name==='list_provider_connections')return providers;
 if(name==='list_provider_templates')return templates;
 if(name==='list_provider_models'){if(failNextModelList){failNextModelList=false;throw new Error('Synthetic catalog read failure');}return args.providerId ? models.filter(model=>model.provider_id===args.providerId) : models;}
 if(name==='create_provider_template'){
  const template=templates.find(item=>item.kind===args.templateKind);
  if(!template?.configurable)throw new Error('Unsupported synthetic preset');
  const id=Math.max(...providers.map(provider=>provider.id))+1;
  const connection={...providers[0],id,display_name:String(args.displayName||template.display_name),provider_type:template.kind,template_kind:template.kind,base_url:template.base_url,models_endpoint:template.models_endpoint};
  providers.push(connection);
  for(const [index,protocol] of ['responses','chat_completions','anthropic_messages','unknown'].entries())models.push({id:100*id+index,provider_id:id,model_id:`synthetic-${protocol}`,display_name:`Synthetic ${protocol}`,protocol,endpoint_path:protocol==='responses'?'/responses':protocol==='anthropic_messages'?'/messages':'/chat/completions',capabilities_json:protocol==='unknown'?'{}':'{}',source:'remote',enabled:protocol!=='unknown',available:true,created_at:1,updated_at:1});
  return connection;
 }
 if(name==='save_provider_connection'){
  const existing=providers.find(provider=>provider.id===args.id);
  const id=existing?.id??Math.max(...providers.map(provider=>provider.id))+1;
  const connection={...providers[0],...existing,id,display_name:String(args.displayName),provider_type:String(args.providerType),template_kind:String(args.templateKind),base_url:String(args.baseUrl),legacy_model:String(args.legacyModel??''),auth_mode:String(args.authMode),models_endpoint:args.modelsEndpoint ? String(args.modelsEndpoint) : null,enabled:Boolean(args.enabled)};
  const index=providers.findIndex(provider=>provider.id===id);if(index<0)providers.push(connection);else providers[index]=connection;
  return connection;
 }
 if(name==='save_provider_model'){
  const existing=models.find(model=>model.provider_id===args.providerId&&model.model_id===args.modelId);
  const model={id:existing?.id??Math.max(...models.map(item=>item.id))+1,provider_id:Number(args.providerId),model_id:String(args.modelId),display_name:String(args.displayName),protocol:String(args.protocol),endpoint_path:String(args.endpointPath),capabilities_json:String(args.capabilitiesJson),source:String(args.source),enabled:Boolean(args.enabled),available:Boolean(args.available),created_at:1,updated_at:1};
  const index=models.findIndex(item=>item.id===model.id);if(index<0)models.push(model);else models[index]=model;
  return model;
 }
 if(name==='refresh_provider_models'){await new Promise(resolve=>setTimeout(resolve,200));const current=models.filter(model=>model.provider_id===args.providerId);return {received:current.length,available:current.filter(model=>model.protocol!=='unknown').length,unknown:current.filter(model=>model.protocol==='unknown').length,source:'remote'};}
 if(name==='test_provider_model')return 'Synthetic probe succeeded';
 if(name==='delete_provider'){const index=providers.findIndex(provider=>provider.id===args.id);if(index>=0)providers.splice(index,1);for(let i=models.length-1;i>=0;i--)if(models[i].provider_id===args.id)models.splice(i,1);return null;}
 if(name==='provider_has_key')return true;
 if(name==='list_ai_task_routes')return [{task_kind:'workbench_qa',provider_model_id:1}];
 if(name==='list_works')return [];
 if(name==='list_qa_sessions')return sessions;
 if(name==='list_qa_turns'){await new Promise(r=>setTimeout(r,350));return [{id:args.sessionId,session_id:args.sessionId,question:'问题 '+args.sessionId,scope_json:'[]',status:'completed',answer_json:JSON.stringify({claims:Array.from({length:10},(_,i)=>({text:'合成回答 '+args.sessionId+' · '+i+'。这是用于验证历史阅读位置的合成段落。'.repeat(5),basis:'fact',citations:[]})),gaps:[]}),evidence_json:null,error:null,created_at:1700000000}];}
 if(name==='set_provider_model_enabled'){await new Promise(r=>setTimeout(r,250));const m=models.find(m=>m.id===args.id);if(m)m.enabled=Boolean(args.enabled);return null;}
 throw new Error('Unexpected fixture boundary '+name);
}}});
mount(Fixture,{target:document.getElementById('test')!});
