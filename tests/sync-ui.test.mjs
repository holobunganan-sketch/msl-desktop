import test from 'node:test';
import assert from 'node:assert/strict';
import { connectionActions, syncStatusLabel } from '../src/lib/services/sync.ts';

test('empty and unrelated folders offer safe initialization only', () => {
  assert.deepEqual(connectionActions({ kind: 'empty' }), ['initialize_from_local']);
  assert.deepEqual(connectionActions({ kind: 'unrelated' }), ['initialize_from_local']);
  assert.deepEqual(connectionActions({ kind: 'incomplete' }), []);
});

test('existing dataset requires an explicit first direction', () => {
  assert.deepEqual(connectionActions({ kind: 'existing' }), ['use_folder', 'replace_with_local']);
});

test('status describes the local folder boundary honestly', () => {
  assert.equal(syncStatusLabel({ running: true, state: { phase: 'running' } }, 'zh-CN'), '正在与同步文件夹交换数据');
  assert.equal(syncStatusLabel({ running: false, state: { phase: 'connected' } }, 'zh-CN'), '已连接；等待下一次同步');
  assert.doesNotMatch(syncStatusLabel({ running: false, state: { phase: 'connected' } }, 'zh-CN'), /云端.*完成/);
});
