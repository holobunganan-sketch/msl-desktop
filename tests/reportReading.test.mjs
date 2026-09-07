import {test} from 'node:test';
import assert from 'node:assert/strict';
const reader=await import('../src/lib/services/reportReading.ts').catch(()=>({}));
test('report reading keeps a finding and its supporting explanation together',()=>{
  assert.equal(typeof reader.reportBlocks,'function');
  assert.deepEqual(reader.reportBlocks('1. 项目 A｜完成摘要\n   本期进展：已核对资料。\n   下一步：确认反馈。\n\n2. 独立事项｜准备会议\n   工作影响：减少重复准备。'),[
    {headline:'项目 A｜完成摘要',body:'本期进展：已核对资料。\n下一步：确认反馈。'},
    {headline:'独立事项｜准备会议',body:'工作影响：减少重复准备。'}]);
});
test('legacy reports remain readable and empty reports stay empty',()=>{
  assert.equal(typeof reader.reportBlocks,'function');
  assert.deepEqual(reader.reportBlocks(null),[]);
  assert.deepEqual(reader.reportBlocks('旧版月报\n保留全部正文'),[{headline:'旧版月报',body:'保留全部正文'}]);
});
