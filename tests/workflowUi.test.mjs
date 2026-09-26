import test,{after} from 'node:test';
import assert from 'node:assert/strict';
import {createServer} from 'vite';
import {fileURLToPath} from 'node:url';
const server=await createServer({resolve:{alias:{$lib:fileURLToPath(new URL('../src/lib',import.meta.url))}},server:{middlewareMode:true,hmr:false},logLevel:'error'});
after(()=>server.close());
const {render}=await server.ssrLoadModule('svelte/server');
const {default:Review}=await server.ssrLoadModule('/src/lib/components/AiReviewCenter.svelte');
const {default:Feedback}=await server.ssrLoadModule('/src/lib/components/ManualCompletionFeedback.svelte');
const {default:Recent}=await server.ssrLoadModule('/src/lib/components/RecentCompletions.svelte');
const {default:Plan}=await server.ssrLoadModule('/src/lib/components/PlanView.svelte');
const {default:DeleteDialog}=await server.ssrLoadModule('/src/lib/components/ItemDeleteDialog.svelte');
const {latestCompletion}=await server.ssrLoadModule('/src/lib/stores/manualCompletions.ts');
test('scoped review announces the retained project boundary',()=>{
 const {body}=render(Review,{props:{workId:3}});
 assert.match(body,/data-testid="review-project-scope"/);
 assert.match(body,/当前项目的建议/);
});
test('manual feedback retains a separate recovery action with long titles',()=>{
 latestCompletion.set({id:'synthetic',entity_kind:'task',entity_id:3,title:'长标题'.repeat(50),created_at:1,undone_at:null});
 const {body}=render(Feedback);
 assert.match(body,/data-testid="manual-completion-undo"/);
 assert.match(body,/恢复为未完成/);
 assert.match(body,/最近完成/);
 latestCompletion.set(null);
});
test('durable completion history stays collapsed until requested',()=>{
 const {body}=render(Recent);
 assert.match(body,/<details[^>]*data-testid="recent-completions"/);
 assert.doesNotMatch(body,/<details[^>]*\sopen(?:\s|>)/);
});
test('completed view has no conflicting status selector and active status names remain translated',()=>{
 const done=render(Plan,{props:{mode:'done'}}).body;
 assert.doesNotMatch(done,/<option value="doing"/);
 const active=render(Plan).body;
 assert.match(active,/<option value="doing"[^>]*>进行中<\/option>/);
});
test('delete confirmation exposes the full business snapshot being confirmed',()=>{
 const record={id:3,title:'Synthetic task',updated_at:10,work_id:1,status:'doing',priority:'high',notes:'Fresh notes after concurrent update',due_at:2000000000};
 const {body}=render(DeleteDialog,{props:{record,kind:'task',works:[{id:1,title:'Synthetic project'}],onclose(){},oncomplete(){}}});
 assert.match(body,/Fresh notes after concurrent update/);
 assert.match(body,/Synthetic project/);
 assert.match(body,/进行中/);
 assert.match(body,/截止时间/);
});
