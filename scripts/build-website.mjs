import { readFile, writeFile, mkdir, readdir, copyFile, lstat, rm } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';

// Run from the repository root. This build reads public website sources only.
const root = path.resolve(process.cwd());
const output = path.join(root, 'website-dist');
const assetName = 'MSL-Desktop-Windows-x64.exe';
const semver = /^\d+\.\d+\.\d+$/;
const escapeHtml = value => String(value).replace(/[&<>"']/g, char => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[char]);
const json = async name => JSON.parse(await readFile(path.join(root, name), 'utf8'));

function options(args) {
  let sourceOnly = false;
  let installer;
  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--source-only') sourceOnly = true;
    else if (args[i] === '--installer' && args[i + 1] && !args[i + 1].startsWith('--')) installer = path.resolve(root, args[++i]);
    else throw new Error(`Unknown or incomplete option: ${args[i]}`);
  }
  if (sourceOnly && installer) throw new Error('Use --source-only or --installer, not both.');
  return { sourceOnly, installer };
}

function validateHistory(history, version) {
  if (history.schemaVersion !== 1 || !Array.isArray(history.releases) || history.releases[0]?.version !== version) {
    throw new Error('History latest version must match package.json.');
  }
  const seen = new Set();
  for (const entry of history.releases) {
    if (!semver.test(entry.version) || seen.has(entry.version)) throw new Error('History contains an invalid or duplicate version.');
    seen.add(entry.version);
    if (!/^\d{4}-\d{2}-\d{2}$/.test(entry.date) || !Number.isFinite(Date.parse(entry.date))) throw new Error(`Invalid date: ${entry.version}`);
    if (!['current', 'released', 'local'].includes(entry.status) || !['published', 'local'].includes(entry.dateKind)) throw new Error(`Invalid history status: ${entry.version}`);
    if ((entry.status === 'released') !== (entry.dateKind === 'published')) throw new Error(`Published and local history dates must remain distinct: ${entry.version}`);
    if (!entry.title || !entry.summary || !Array.isArray(entry.changes) || !entry.changes.length || entry.changes.some(change => typeof change !== 'string' || !change.trim())) throw new Error(`Missing history content: ${entry.version}`);
    if (entry.source && (!entry.source.label || !/^https:\/\//.test(entry.source.url))) throw new Error(`Invalid history source: ${entry.version}`);
  }
}

function renderEntry(entry, latest = false) {
  const label = entry.status === 'released' ? '正式发布' : entry.status === 'local' ? '本地更新记录' : '本次更新';
  const date = entry.date.replaceAll('-', '.');
  const source = entry.source ? `<a class="release-source" href="${escapeHtml(entry.source.url)}" target="_blank" rel="noopener">${escapeHtml(entry.source.label)} <span aria-hidden="true">↗</span><span class="sr-only">（在新标签页打开）</span></a>` : '';
  return `<details class="update-entry${latest ? ' update-latest' : ''}" data-version="${entry.version}"${latest ? ' open' : ''}>
          <summary>
            <span class="update-meta"><span class="update-version">v${entry.version}</span><span class="update-status">${label}</span><time datetime="${entry.date}">${date}</time></span>
            <span class="update-heading">${escapeHtml(entry.title)}</span>
            <span class="update-toggle" aria-hidden="true"></span>
          </summary>
          <div class="update-body"><p class="update-intro">${escapeHtml(entry.summary)}</p><ul>${entry.changes.map(change => `<li>${escapeHtml(change)}</li>`).join('')}</ul>${source}</div>
        </details>`;
}

function renderHistory(history) {
  const current = history.releases.filter(entry => entry.status !== 'local');
  const local = history.releases.filter(entry => entry.status === 'local');
  return `<section id="updates" class="updates wrap" aria-labelledby="updates-title">
      <div class="section-heading"><div><p class="eyebrow">版本与日期</p><h2 id="updates-title">更新记录</h2></div><p>按版本查看新增功能和修复。<br>早期本地版本单独列在下方。</p></div>
      <div class="updates-layout"><div class="updates-aside"><span class="index">RELEASE NOTES</span><p>当前版本<br>v${history.releases[0].version}</p><a class="text-link" href="#download">下载当前版本 <span aria-hidden="true">↓</span></a></div><div class="updates-list">
        ${current.map((entry, index) => renderEntry(entry, index === 0)).join('\n        ')}
        ${local.length ? `<details class="history-archive"><summary>早期本地版本 <span>${local.length} 个版本</span></summary><p class="history-note">日期来自本地构建或提交记录，不代表 GitHub 发布日期。</p>${local.map(entry => renderEntry(entry)).join('\n')}</details>` : ''}
      </div></div>
    </section>`;
}

async function publicAssets(directory, prefix = '') {
  let entries;
  try { entries = await readdir(directory, { withFileTypes: true }); }
  catch (error) { if (error.code === 'ENOENT') return []; throw error; }
  const files = [];
  for (const entry of entries.sort((a, b) => a.name.localeCompare(b.name, 'en'))) {
    if (entry.isSymbolicLink()) throw new Error(`Website assets cannot contain symlinks: ${entry.name}`);
    const relative = path.join(prefix, entry.name);
    if (entry.isDirectory()) files.push(...await publicAssets(path.join(directory, entry.name), relative));
    else if (/\.(png|jpe?g|webp|svg|gif|ico|ttf|woff2?)$/i.test(entry.name) || /^OFL[-\w]*\.txt$/.test(entry.name)) files.push(relative);
  }
  return files;
}

async function main() {
  const { sourceOnly, installer: explicitInstaller } = options(process.argv.slice(2));
  const [pkg, latest, history] = await Promise.all([json('package.json'), json('release/latest.json'), json('release/history.json')]);
  if (!semver.test(pkg.version) || latest.version !== pkg.version || latest.asset !== assetName) throw new Error('Package and latest release version or asset do not match.');
  validateHistory(history, pkg.version);
  const source = path.join(root, 'website');
  const template = await readFile(path.join(source, 'index.html'), 'utf8');
  if (!template.includes('<!-- RELEASE_HISTORY -->') || !template.includes('{{RELEASE_VERSION}}')) throw new Error('Website template is missing a release placeholder.');
  const stylesheet = await readFile(path.join(source, 'style.css'));
  const styleHash = createHash('sha256').update(stylesheet).digest('hex').slice(0, 16);
  const html = template.replaceAll('{{RELEASE_VERSION}}', pkg.version)
    .replaceAll('{{STYLE_HASH}}', styleHash)
    .replace('<!-- RELEASE_HISTORY -->', renderHistory(history));
  const assets = await publicAssets(path.join(source, 'assets'));
  // Resolve and read the exact current installer before touching the previous build.
  let installerBytes;
  let installerPath;
  if (!sourceOnly) {
    if (explicitInstaller) installerPath = explicitInstaller;
    else {
      const config = await json('src-tauri/tauri.conf.json');
      if (config.version !== pkg.version) throw new Error('Tauri and package versions do not match.');
      installerPath = path.join(root, 'src-tauri/target/release/bundle/nsis', `${config.productName}_${pkg.version}_x64-setup.exe`);
    }
    installerBytes = await readFile(installerPath);
    if (!installerBytes.length) throw new Error(`Installer is empty: ${installerPath}`);
  }
  // The sole removable directory is this repository's generated website-dist.
  if (path.dirname(output) !== root || path.basename(output) !== 'website-dist') throw new Error('Unsafe output path.');
  try { if ((await lstat(output)).isSymbolicLink()) throw new Error('Output directory cannot be a symlink.'); }
  catch (error) { if (error.code !== 'ENOENT') throw error; }
  await rm(output, { recursive: true, force: true });
  await mkdir(path.join(output, 'release'), { recursive: true });
  await writeFile(path.join(output, 'index.html'), html);
  const publicFiles = ['style.css', 'site.js', '_headers', 'robots.txt', '7291789f4cb338b8130a0e8537feb0bd.txt'];
  for (const file of publicFiles) {
    try { await copyFile(path.join(source, file), path.join(output, file)); }
    catch (error) { if (error.code !== 'ENOENT') throw error; }
  }
  try {
    const sitemap = await readFile(path.join(source, 'sitemap.xml'), 'utf8');
    await writeFile(path.join(output, 'sitemap.xml'), sitemap.replaceAll('{{RELEASE_DATE}}', history.releases[0].date));
  } catch (error) { if (error.code !== 'ENOENT') throw error; }
  for (const asset of assets) {
    const target = path.join(output, 'assets', asset);
    await mkdir(path.dirname(target), { recursive: true });
    await copyFile(path.join(source, 'assets', asset), target);
  }
  await writeFile(path.join(output, 'release/latest.json'), `${JSON.stringify(latest, null, 2)}\n`);
  if (installerBytes) {
    await mkdir(path.join(output, 'downloads'));
    await writeFile(path.join(output, 'downloads', assetName), installerBytes);
    const sha256 = createHash('sha256').update(installerBytes).digest('hex');
    await writeFile(path.join(output, 'downloads/SHA256SUMS.txt'), `${sha256}  ${assetName}\n`);
    console.log(`Built website v${pkg.version}; installer SHA-256 ${sha256}`);
  } else console.log(`Built website v${pkg.version} (source preview only; installer omitted).`);
  console.log(output);
}

main().catch(error => { console.error(error.message); process.exitCode = 1; });
