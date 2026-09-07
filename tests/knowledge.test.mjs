import test from 'node:test';
import assert from 'node:assert/strict';
let api={};try{api=await import('../src/lib/services/knowledge.ts');}catch{}
test('@ completion uses the caret, supports Chinese and preserves later text',()=>{
 assert.deepEqual(api.mentionAt('请看 @免疫 后续',6),{start:3,end:6,query:'免疫'});
 assert.equal(api.mentionAt('mail@example.com',16),null);
 assert.equal(api.mentionAt('普通问题',4),null);
 assert.deepEqual(api.mentionAt('@Project Alpha',14),{start:0,end:14,query:'Project Alpha'});
});
test('scope uses real stable IDs and duplicates cannot widen it',()=>{
 assert.deepEqual(api.addScope([2,1],2),[1,2]);
 assert.throws(()=>api.addScope([1],0));
});
test('clearing a date yields null, never epoch or an invented appointment',()=>{
 assert.equal(api.parseLocalTime(''),null);
 const value=api.parseLocalTime('2026-09-07T14:30');
 assert.equal(api.localTime(value),'2026-09-07T14:30');
});
