import {mount} from 'svelte';
import Fixture from './CatalogFixture.svelte';
const models=Array.from({length:12},(_,i)=>({id:i+1,provider_id:1,model_id:`model-${i}`,display_name:'Synthetic model '+i,protocol:'chat_completions',endpoint_path:'/chat/completions',capabilities_json:'{}',source:'manual',enabled:true,available:true}));
const providers=[{id:1,display_name:'Synthetic provider',provider_type:'custom',template_kind:'custom',base_url:'http://127.0.0.1:9999',enabled:true}];
const sessions=[1,2].map(id=>({id,title:'测试会话 '+id,scope_json:'[]',updated_at:1700000000}));
Object.assign(window,{__TAURI_INTERNALS__:{invoke:async(name:string,args:Record<string,number>={})=>{
 if(name==='list_provider_connections')return providers;
 if(name==='list_provider_models')return models;
 if(name==='provider_has_key')return true;
 if(name==='list_ai_task_routes')return [{task_kind:'workbench_qa',provider_model_id:1}];
 if(name==='list_works')return [];
 if(name==='list_qa_sessions')return sessions;
 if(name==='list_qa_turns'){await new Promise(r=>setTimeout(r,350));return [{id:args.sessionId,session_id:args.sessionId,question:'问题 '+args.sessionId,scope_json:'[]',status:'completed',answer_json:JSON.stringify({claims:Array.from({length:10},(_,i)=>({text:'合成回答 '+args.sessionId+' · '+i+'。这是用于验证历史阅读位置的合成段落。'.repeat(5),basis:'fact',citations:[]})),gaps:[]}),evidence_json:null,error:null,created_at:1700000000}];}
 if(name==='set_provider_model_enabled'){await new Promise(r=>setTimeout(r,250));const m=models.find(m=>m.id===args.id);if(m)m.enabled=Boolean(args.enabled);return null;}
 throw new Error('Unexpected fixture boundary '+name);
}}});
mount(Fixture,{target:document.getElementById('test')!});
