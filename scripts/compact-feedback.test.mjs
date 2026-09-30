import test, {after} from 'node:test';
import assert from 'node:assert/strict';
import {createServer} from 'vite';
const server=await createServer({server:{middlewareMode:true,hmr:false},logLevel:'error'});
after(()=>server.close());
const {render}=await server.ssrLoadModule('svelte/server');
const {default:Feedback}=await server.ssrLoadModule('/src/lib/components/ManualCompletionFeedback.svelte');
test('compact completion feedback occupies no empty row but still exposes errors',()=>{
  assert.doesNotMatch(render(Feedback,{props:{collapseWhenEmpty:true}}).body,/data-testid="manual-completion-feedback"/);
  assert.match(render(Feedback,{props:{collapseWhenEmpty:true,error:'Synthetic retry needed'}}).body,/Synthetic retry needed/);
  assert.match(render(Feedback).body,/data-testid="manual-completion-feedback"/);
});
