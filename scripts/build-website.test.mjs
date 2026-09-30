import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, readFile, writeFile, access } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const script = fileURLToPath(new URL('./build-website.mjs', import.meta.url));

async function fixture() {
  const root = await mkdtemp(path.join(tmpdir(), 'msl-website-test-'));
  await mkdir(path.join(root, 'website/assets'), { recursive: true });
  await mkdir(path.join(root, 'release'));
  await mkdir(path.join(root, 'src-tauri'));
  await writeFile(path.join(root, 'package.json'), JSON.stringify({ version: '0.3.15' }));
  await writeFile(path.join(root, 'release/latest.json'), JSON.stringify({ version: '0.3.15', asset: 'MSL-Desktop-Windows-x64.exe' }));
  await writeFile(path.join(root, 'src-tauri/tauri.conf.json'), JSON.stringify({ productName: 'MSL Desktop', version: '0.3.15' }));
  await writeFile(path.join(root, 'release/history.json'), JSON.stringify({ schemaVersion: 1, releases: [
    { version: '0.3.15', date: '2026-09-30', status: 'current', dateKind: 'local', title: '整理与确认', summary: '留下原话 <保留出处>', changes: ['甲 & 乙', '用户确认'], source: null },
    { version: '0.3.14', date: '2026-09-27', status: 'released', dateKind: 'published', title: '连续推进', summary: '正式发布', changes: ['恢复已完成事项'], source: { url: 'https://github.com/example/repo/releases/tag/v0.3.14', label: '发布记录', publishedAt: '2026-09-26T16:41:07Z' } },
    { version: '0.3.8', date: '2026-09-11', status: 'local', dateKind: 'local', title: '本地资料', summary: '本地里程碑', changes: ['目录备份'], source: { url: 'https://github.com/example/repo/commit/abc', label: '提交记录' } },
  ] }));
  await writeFile(path.join(root, 'website/index.html'), '<!doctype html><html lang="zh-CN"><head><meta name="msl-release" content="{{RELEASE_VERSION}}"></head><body><span data-release-version>v{{RELEASE_VERSION}}</span><a href="/downloads/MSL-Desktop-Windows-x64.exe">下载</a><!-- RELEASE_HISTORY --></body></html>');
  await writeFile(path.join(root, 'website/style.css'), 'body { color: navy; }');
  await writeFile(path.join(root, 'website/site.js'), '/* public script */');
  await writeFile(path.join(root, 'website/sitemap.xml'), '<lastmod>{{RELEASE_DATE}}</lastmod>');
  await writeFile(path.join(root, 'website/assets/icon.png'), Buffer.from([137, 80, 78, 71]));
  return root;
}

function build(root, args = ['--source-only']) {
  return spawnSync(process.execPath, [script, ...args], { cwd: root, encoding: 'utf8' });
}

test('build renders readable static history, expands latest, and labels local dates separately', async () => {
  const root = await fixture();
  const result = build(root);
  assert.equal(result.status, 0, result.stderr);
  const html = await readFile(path.join(root, 'website-dist/index.html'), 'utf8');
  assert.match(html, /id="updates"/);
  assert.match(html, /data-release-version>v0\.3\.15</);
  assert.match(html, /<details[^>]*data-version="0\.3\.15"[^>]* open>/);
  assert.match(html, /<details[^>]*data-version="0\.3\.14"[^>]*>/);
  assert.doesNotMatch(html, /<details[^>]*data-version="0\.3\.14"[^>]* open/);
  assert.match(html, /本地更新记录/);
  assert.match(html, /正式发布/);
  assert.match(html, /留下原话 &lt;保留出处&gt;/);
  assert.match(html, /甲 &amp; 乙/);
  assert.match(html, /datetime="2026-09-30"/);
  assert.doesNotMatch(html, /\{\{|RELEASE_HISTORY/);
  assert.equal(await readFile(path.join(root, 'website-dist/sitemap.xml'), 'utf8'), '<lastmod>2026-09-30</lastmod>');
});

test('build rejects inconsistent latest versions before replacing previous output', async () => {
  const root = await fixture();
  await mkdir(path.join(root, 'website-dist'));
  await writeFile(path.join(root, 'website-dist/index.html'), 'previous valid build');
  await writeFile(path.join(root, 'release/latest.json'), JSON.stringify({ version: '0.3.14', asset: 'MSL-Desktop-Windows-x64.exe' }));
  const result = build(root);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /version/i);
  assert.equal(await readFile(path.join(root, 'website-dist/index.html'), 'utf8'), 'previous valid build');
});

test('build rejects a latest history entry that disagrees with the package', async () => {
  const root = await fixture();
  const historyFile = path.join(root, 'release/history.json');
  const history = JSON.parse(await readFile(historyFile, 'utf8'));
  history.releases[0].version = '0.3.16';
  await writeFile(historyFile, JSON.stringify(history));
  const result = build(root);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /history.*version/i);
  await assert.rejects(access(path.join(root, 'website-dist')));
});

test('source-only build copies public assets without copying source downloads or private files', async () => {
  const root = await fixture();
  await mkdir(path.join(root, 'website/downloads'));
  await writeFile(path.join(root, 'website/downloads/old.exe'), 'stale installer');
  await writeFile(path.join(root, 'website/.env'), 'synthetic-secret');
  await writeFile(path.join(root, 'website/assets/fixture.db'), 'synthetic database');
  const result = build(root);
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(await readFile(path.join(root, 'website-dist/assets/icon.png')), Buffer.from([137, 80, 78, 71]));
  for (const file of ['downloads/old.exe', '.env', 'assets/fixture.db', 'downloads/MSL-Desktop-Windows-x64.exe']) {
    await assert.rejects(access(path.join(root, 'website-dist', file)));
  }
});

test('full build stages the exact versioned installer and hand-checked checksum at the stable URL', async () => {
  const root = await fixture();
  const bundle = path.join(root, 'src-tauri/target/release/bundle/nsis');
  await mkdir(bundle, { recursive: true });
  await writeFile(path.join(bundle, 'MSL Desktop_0.3.14_x64-setup.exe'), 'old');
  await writeFile(path.join(bundle, 'MSL Desktop_0.3.15_x64-setup.exe'), 'abc');
  const result = build(root, []);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(await readFile(path.join(root, 'website-dist/downloads/MSL-Desktop-Windows-x64.exe'), 'utf8'), 'abc');
  assert.equal(await readFile(path.join(root, 'website-dist/downloads/SHA256SUMS.txt'), 'utf8'), 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  MSL-Desktop-Windows-x64.exe\n');
  assert.deepEqual(JSON.parse(await readFile(path.join(root, 'website-dist/release/latest.json'), 'utf8')), { version: '0.3.15', asset: 'MSL-Desktop-Windows-x64.exe' });
});

test('explicit installer supports staging a verified published release asset', async () => {
  const root = await fixture();
  const installer = path.join(root, 'MSL-Desktop-Windows-x64.exe');
  await writeFile(installer, 'published-fixture');
  const result = build(root, ['--installer', installer]);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(await readFile(path.join(root, 'website-dist/downloads/MSL-Desktop-Windows-x64.exe'), 'utf8'), 'published-fixture');
});

test('full build refuses a missing current installer and never selects an older bundle', async () => {
  const root = await fixture();
  const bundle = path.join(root, 'src-tauri/target/release/bundle/nsis');
  await mkdir(bundle, { recursive: true });
  await writeFile(path.join(bundle, 'MSL Desktop_0.3.14_x64-setup.exe'), 'old');
  const result = build(root, []);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /0\.3\.15.*x64-setup\.exe/);
  await assert.rejects(access(path.join(root, 'website-dist')));
});

test('repeated builds are deterministic and clear obsolete output files', async () => {
  const root = await fixture();
  const first = build(root);
  assert.equal(first.status, 0, first.stderr);
  const before = await readFile(path.join(root, 'website-dist/index.html'), 'utf8');
  await writeFile(path.join(root, 'website-dist/obsolete.js'), 'old');
  const second = build(root);
  assert.equal(second.status, 0, second.stderr);
  assert.equal(await readFile(path.join(root, 'website-dist/index.html'), 'utf8'), before);
  await assert.rejects(access(path.join(root, 'website-dist/obsolete.js')));
});

test('stylesheet URL changes with CSS content even when the app version stays the same', async () => {
  const root = await fixture();
  const template = path.join(root, 'website/index.html');
  await writeFile(template, (await readFile(template, 'utf8')).replace('</head>', '<link rel="stylesheet" href="style.css?v={{STYLE_HASH}}"></head>'));
  assert.equal(build(root).status, 0);
  const before = await readFile(path.join(root, 'website-dist/index.html'), 'utf8');
  const oldUrl = before.match(/href="(style\.css\?v=[^"]+)"/)[1];
  await writeFile(path.join(root, 'website/style.css'), 'body { color: teal; }');
  assert.equal(build(root).status, 0);
  const after = await readFile(path.join(root, 'website-dist/index.html'), 'utf8');
  const newUrl = after.match(/href="(style\.css\?v=[^"]+)"/)[1];
  assert.notEqual(oldUrl, newUrl, 'New typography must not reuse a stylesheet URL cached with old fonts.');
  assert.doesNotMatch(after, /\{\{/);
  assert.match(after, /content="0\.3\.15"/);
});
