import test, {after} from 'node:test';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
import {createServer} from 'vite';

const server = await createServer({
  resolve:{alias:{$lib:fileURLToPath(new URL('../src/lib',import.meta.url))}},
  server:{middlewareMode:true,hmr:false}, logLevel:'error',
});
after(()=>server.close());
const {default:Sources}=await server.ssrLoadModule('/src/lib/components/KnowledgeSources.svelte');
const {render}=await server.ssrLoadModule('svelte/server');
const {locale}=await server.ssrLoadModule('/src/lib/i18n/index.ts');
locale.set('zh-CN');
const source=(changes={})=>({id:'task:6',kind:'task',entity_id:6,title:'合成来源',text:'{"expert_id":999,"material_id":999}',timestamp:1,trust:'record',hash:'synthetic',...changes});
function body(sources){return render(Sources,{props:{citations:sources.map(s=>({source_id:s.id,quote:'合成摘录'})),pack:{sources,as_of:1,scope_ids:[],counts:{},omitted:0,notes:[]}}}).body;}

test('source buttons require trusted available location, never raw entity IDs or source prose',()=>{
  assert.doesNotMatch(body([source()]), /打开记录/);
  assert.match(body([source({location:{entity_kind:'task',entity_id:6,work_id:3,available:true}})]), /打开记录/);
});
test('deleted and remote sources cannot navigate even when they retain an available target',()=>{
  for(const changes of [
    {trust:'deleted',location:{entity_kind:'task',entity_id:6,available:true}},
    {location:{entity_kind:'task',entity_id:6,available:true,remote_only:true}},
  ]) assert.doesNotMatch(body([source(changes)]), /打开记录/);
  assert.match(body([source({trust:'deleted'})]), /来源已删除/);
  assert.match(body([source({location:{entity_kind:'document',entity_id:6,available:false,remote_only:true}})]), /其他设备/);
});
test('expert counts use canonical locations when source text is incomplete or misleading',()=>{
  const notes=[1,2].map(id=>source({id:`kol_note:${id}`,kind:'kol_note',entity_id:id,location:{entity_kind:'kol_note',entity_id:id,expert_id:id,available:true}}));
  assert.match(body(notes), /2 位专家/);
});
test('material preview requires a trusted material identity and version',()=>{
  const material=source({id:'kol_material:7',kind:'kol_material',entity_id:7,trust:'extracted_text',text:'incomplete prose'});
  assert.doesNotMatch(body([material]), /查看资料/);
  assert.match(body([{...material,text:'incomplete prose',location:{entity_kind:'kol_material',entity_id:7,material_id:4,blob_hash:'synthetic-version',locator:'第 3 页',revision:2,expert_id:2,available:true}}]), /查看资料/);
  assert.match(body([{...material,text:'incomplete prose',location:{entity_kind:'kol_material',entity_id:7,material_id:4,blob_hash:'synthetic-version',locator:'第 3 页',revision:2,expert_id:2,available:true}}]), /第 3 页/);
});
