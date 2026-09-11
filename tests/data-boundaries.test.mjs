import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
test('private runtime and backup artifacts cannot enter ordinary git add', () => {
  for (const name of ['msl-desktop-data/msl-desktop.db','backups/msl-backup.zip','export/personal.sqlite3','expert.mslbackup','runtime.db-wal','.test-runtime/example/appdata/msl-desktop.db','attachments/blobs/example.pdf','backup-state.json']) {
    const result=spawnSync('git',['check-ignore','--quiet',name]);
    assert.equal(result.status,0,`private path is not ignored: ${name}`);
  }
});
