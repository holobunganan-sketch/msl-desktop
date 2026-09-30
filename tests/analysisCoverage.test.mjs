import { test } from 'node:test';
import assert from 'node:assert/strict';
const { analysisCoverage } = await import('../src/lib/services/analysisCoverage.ts').catch(() => ({}));

test('analysis coverage distinguishes excerpts, unread and unavailable sources', () => {
  assert.equal(typeof analysisCoverage, 'function');
  const view = analysisCoverage(JSON.stringify({ documents: 8, documents_read: 3,
    documents_excerpted: 2, documents_unread: 5, documents_unavailable: 1,
    documents_metadata_omitted: 4, document_coverage: [
      { path: 'synthetic.txt', status: 'excerpt', workspace_id: 1 },
      { path: 'scan.pdf', status: 'unavailable', workspace_id: 1 }
    ] }), 'zh-CN');
  assert.equal(view.partial, true);
  assert.equal(view.read, 3);
  assert.equal(view.unread, 5);
  assert.equal(view.unavailable, 1);
  assert.equal(view.omitted, 4);
  assert.equal(view.files[0].label, '节选');
  assert.equal(view.files[1].label, '尚不可读取');
});

test('old analysis counts never imply all materials were read', () => {
  assert.equal(analysisCoverage('{"documents":3}', 'zh-CN'), null);
  assert.equal(analysisCoverage('broken', 'zh-CN'), null);
});

test('complete coverage remains distinct from a run without file evidence', () => {
  const view = analysisCoverage('{"documents":1,"documents_read":1,"documents_excerpted":0,"documents_unread":0,"documents_unavailable":0}', 'en-US');
  assert.equal(view.partial, false);
  assert.equal(view.read, 1);
});
