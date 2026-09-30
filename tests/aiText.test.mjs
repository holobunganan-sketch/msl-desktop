import test from 'node:test';
import assert from 'node:assert/strict';
const {aiText, briefLines} = await import('../src/lib/services/aiText.ts').catch(()=>({}));

test('briefs display complete emphasis and legacy orphan closing markers without punctuation debris',()=>{
  assert.equal(typeof briefLines,'function');
  assert.deepEqual(briefLines('• 项目动态摘要**\n• **高优先级行动项**\n## 待办事项\n- 核对 **演示项目** 的证据。'),
    ['项目动态摘要','高优先级行动项','待办事项','核对 演示项目 的证据。']);
});
test('AI reading surfaces normalize headings, emphasis, links and lists',()=>{
  assert.equal(typeof aiText,'function');
  assert.equal(aiText('## **进展**\n\n- 已完成 *资料核对*\n- 查看 [来源](https://example.org/evidence)'),
    '进展\n\n• 已完成 资料核对\n• 查看 来源（https://example.org/evidence）');
  assert.equal(aiText('**重点**、__结论__、~~旧安排~~、`工作编号`'),'重点、结论、旧安排、工作编号');
});
test('plain text, identifiers, negative numbers and literal code retain their meaning',()=>{
  assert.equal(typeof aiText,'function');
  assert.equal(aiText('IL-6，p < 0.05，-2 mg，2 * 3，file_name_v2，C:\\Work\\A'), 'IL-6，p < 0.05，-2 mg，2 * 3，file_name_v2，C:\\Work\\A');
  assert.equal(aiText('```text\na ** b\nfile_name\n```'),'a ** b\nfile_name');
  assert.equal(aiText('\\*保留字面星号\\*'),'*保留字面星号*');
});
test('tables and task lists retain readable content, including completion state',()=>{
  assert.equal(typeof aiText,'function');
  const text=aiText('| 项目 | 进展 |\n| --- | --- |\n| A | **完成** |\n\n- [x] 核对\n- [ ] 跟进');
  assert.match(text,/项目 · 进展\nA · 完成/);
  assert.match(text,/☑ 核对\n☐ 跟进/);
});
test('legacy cleanup is brief-only and never removes source identifiers or signed quantities',()=>{
  assert.equal(typeof briefLines,'function');
  assert.deepEqual(briefLines('• -2 mg\n• 文件名 file_name\n• 2 ** 3\n• 结论（source_type: task, entity_id: 1）'),['-2 mg','文件名 file_name','2 ** 3','结论']);
});
