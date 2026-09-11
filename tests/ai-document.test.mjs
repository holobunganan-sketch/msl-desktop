import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { aiDocumentText } from '../src/lib/types/aiDocument.ts';

const component = fs.readFileSync(new URL('../src/lib/components/AiDocument.svelte', import.meta.url), 'utf8');
const types = fs.readFileSync(new URL('../src/lib/types/aiDocument.ts', import.meta.url), 'utf8');

test('readable AI document renders every supported block without raw HTML', () => {
  for (const token of ["block.type === 'paragraph'", "block.type === 'bullets'", "block.type === 'numbered'", '<table>']) assert.match(component, new RegExp(token.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')));
  assert.doesNotMatch(component, /\{@html/);
});

test('document text projection preserves open sections and deterministic numbering', () => {
  const item=(text,basis='suggestion')=>({text,basis,citations:[]});
  const document={schema_version:'msl.readable.v1',title:'完整输出',sections:[{title:'自由主题',blocks:[
    {type:'numbered',items:Array.from({length:40},(_,i)=>item(`事项 ${i+1}`))},
    {type:'table',columns:['事实','待核实'],rows:[[item('保留原文','fact'),item('信息不全','unknown')]]},
    {type:'paragraph',content:item('条件解释','inference')}
  ]}]};
  const text=aiDocumentText(document);
  assert.match(text,/40\. \[建议\] 事项 40/);
  assert.match(text,/保留原文 \| \[待核实\] 信息不全/);
  assert.match(text,/\[推断\] 条件解释/);
});
