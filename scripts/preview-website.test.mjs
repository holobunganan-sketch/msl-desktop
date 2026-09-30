import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { startWebsitePreview } from './preview-website.mjs';

test('local preview renders the same complete history as deployment before accepting navigation', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'msl-site-preview-'));
  await mkdir(path.join(root, 'website/assets'), { recursive: true });
  await mkdir(path.join(root, 'release'));
  await writeFile(path.join(root, 'package.json'), JSON.stringify({ version: '0.3.15' }));
  await writeFile(path.join(root, 'release/latest.json'), JSON.stringify({ version: '0.3.15', asset: 'MSL-Desktop-Windows-x64.exe' }));
  await writeFile(path.join(root, 'release/history.json'), JSON.stringify({ schemaVersion: 1, releases: [
    { version: '0.3.15', date: '2026-09-30', status: 'current', dateKind: 'local', title: '本地完整预览', summary: '应展示实际内容', changes: ['可展开的更新条目'] },
  ] }));
  await writeFile(path.join(root, 'website/index.html'), '<html><head><link rel="stylesheet" href="style.css?v={{STYLE_HASH}}"></head><body><a class="nav-updates" href="#updates">更新记录</a><span>{{RELEASE_VERSION}}</span><!-- RELEASE_HISTORY --></body></html>');
  await writeFile(path.join(root, 'website/style.css'), 'body { color: navy; }');
  await writeFile(path.join(root, 'website/.env'), 'must not be served');
  await writeFile(path.join(root, 'private.db'), 'must not be served');
  const preview = await startWebsitePreview({ root, port: 0 });
  try {
    const response = await fetch(preview.url);
    assert.equal(response.status, 200);
    const html = await response.text();
    assert.match(html, /href="#updates"/);
    assert.match(html, /id="updates"/);
    assert.match(html, /data-version="0\.3\.15"[^>]* open/);
    assert.match(html, /可展开的更新条目/);
    assert.doesNotMatch(html, /\{\{|<!-- RELEASE_HISTORY -->/);
    assert.equal(response.headers.get('cache-control'), 'no-store');
    assert.equal((await fetch(`${preview.url}style.css`)).status, 200);
    for (const name of ['.env', 'private.db', '%2e%2e%2fprivate.db', 'website/index.html']) {
      assert.ok([403, 404].includes((await fetch(`${preview.url}${name}`)).status), name);
    }
    const download = await fetch(`${preview.url}downloads/MSL-Desktop-Windows-x64.exe`, { redirect: 'manual' });
    assert.equal(download.status, 302);
    assert.equal(download.headers.get('location'), 'https://msl-desktop.pages.dev/downloads/MSL-Desktop-Windows-x64.exe');
  } finally {
    await preview.close();
  }
});
