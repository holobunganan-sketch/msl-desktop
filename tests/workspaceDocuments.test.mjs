import test, { after } from 'node:test';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { createServer } from 'vite';

const server = await createServer({
  resolve: { alias: { $lib: fileURLToPath(new URL('../src/lib', import.meta.url)) } },
  server: { middlewareMode: true, hmr: false },
  logLevel: 'silent',
});
after(() => server.close());
const component = await server.ssrLoadModule('/src/lib/components/WorkspaceDocuments.svelte').catch(() => ({}));
const { render } = await server.ssrLoadModule('svelte/server');
const { locale } = await server.ssrLoadModule('/src/lib/i18n/index.ts');
function renderDocuments(documents, language = 'zh-CN') {
  assert.equal(typeof component.default, 'function', 'Workspace documents must be rendered by the reusable list component');
  locale.set(language);
  return render(component.default, { props: { documents } }).body;
}
const document = (id, status = 'ready', error = null) => ({
  id, relative_path: `synthetic-document-${id}.pdf`, extract_status: status, error_message: error,
});

test('document statuses explain reading outcomes without exposing internal enum values', () => {
  const cases = [
    ['ready', '可用于分析', 'Available for analysis'],
    ['pending', '等待读取', 'Waiting to be read'],
    ['unsupported', '暂不支持此格式', 'Format not supported'],
    ['needs_ocr', '需要文字识别', 'Text recognition needed'],
    ['too_large', '文件超出读取上限', 'File exceeds the reading limit'],
    ['failed_parse', '内容读取失败', 'Could not read the contents'],
    ['failed_encoding', '文字编码无法识别', 'Text encoding not recognized'],
    ['future_internal_state', '读取状态待确认', 'Reading status needs checking'],
  ];
  for (const [status, chinese, english] of cases) {
    assert.ok(renderDocuments([document(1, status)]).includes(chinese));
    assert.ok(renderDocuments([document(1, status)], 'en-US').includes(english));
    assert.ok(!renderDocuments([document(1, status)]).includes(`>${status}<`));
  }
});

test('documents beyond the first eight remain reachable in a closed, labelled disclosure', () => {
  const documents = Array.from({ length: 12 }, (_, i) => document(i + 1));
  documents[11] = document(12, 'failed_parse', 'Synthetic document could not be read.');
  const body = renderDocuments(documents);
  const disclosureStart = body.indexOf('<details');
  assert.ok(disclosureStart > 0, 'A visible disclosure must lead to the remaining documents');
  const initialRows = body.slice(0, disclosureStart);
  const disclosure = body.slice(disclosureStart);
  assert.ok(initialRows.includes('synthetic-document-8.pdf'));
  assert.ok(!initialRows.includes('synthetic-document-9.pdf'));
  assert.match(disclosure, /<summary[^>]*>.*查看其余 4 份资料/s);
  assert.doesNotMatch(disclosure.match(/^<details[^>]*>/)?.[0] ?? '', /\sopen(?:\s|=|>)/);
  for (let id = 9; id <= 12; id++) assert.ok(disclosure.includes(`synthetic-document-${id}.pdf`));
  assert.ok(disclosure.includes('Synthetic document could not be read.'));
  assert.match(renderDocuments(documents, 'en-US'), /Show 4 more documents/);
});

test('empty and short document lists do not offer an empty disclosure', () => {
  const empty = renderDocuments([]);
  assert.doesNotMatch(empty, /<details/);
  assert.match(empty, /暂无/);
  const short = renderDocuments([document(1), document(2)]);
  assert.doesNotMatch(short, /<details/);
  assert.match(short, /synthetic-document-2\.pdf/);
});
