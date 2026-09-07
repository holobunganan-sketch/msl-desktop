import test from 'node:test';
import assert from 'node:assert/strict';
let api={};try{api=await import('../src/lib/services/captureDrafts.ts');}catch{}
test('Unsaved notes follow their source context without leaking across projects',()=>{
  const a=api.draftKey({workId:1,entityKind:'work',entityId:1});
  const b=api.draftKey({workId:2,entityKind:'work',entityId:2});
  api.writeDraft(a,'synthetic project note');
  assert.equal(api.readDraft(b),'');
  assert.equal(api.readDraft(a),'synthetic project note');
  api.clearDraft(a);
  assert.equal(api.readDraft(a),'');
});
