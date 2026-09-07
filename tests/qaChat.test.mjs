import test from 'node:test';
import assert from 'node:assert/strict';
let chat={};try {chat=await import('../src/lib/services/qaChat.ts');} catch {}
test('model choices exclude disabled providers, disabled models and unavailable models',()=>{
 assert.equal(typeof chat.chatModels,'function');
 const providers=[{id:1,enabled:true},{id:2,enabled:false}];
 const models=[{id:11,provider_id:1,enabled:true,available:true},{id:12,provider_id:1,enabled:false,available:true},{id:13,provider_id:2,enabled:true,available:true},{id:14,provider_id:1,enabled:true,available:false}];
 assert.deepEqual(chat.chatModels(providers,models).map(m=>m.id),[11]);
});
test('Q&A follows its own route and explicit unconfigured route never silently falls back',()=>{
 assert.equal(typeof chat.chatModelId,'function');
 assert.equal(chat.chatModelId([{task_kind:'general',provider_model_id:1},{task_kind:'workbench_qa',provider_model_id:2}]),2);
 assert.equal(chat.chatModelId([{task_kind:'general',provider_model_id:1},{task_kind:'workbench_qa',provider_model_id:null}]),null);
 assert.equal(chat.chatModelId([{task_kind:'general',provider_model_id:1}]),1);
});
test('Enter sends, Shift Enter adds a line, IME confirmation never sends a question',()=>{
 assert.equal(typeof chat.chatEnter,'function');
 for(const [event,want] of [[{key:'Enter'},true],[{key:'Enter',shiftKey:true},false],[{key:'Enter',isComposing:true},false],[{key:'Enter',keyCode:229},false],[{key:'a'},false]])assert.equal(chat.chatEnter(event),want);
});
