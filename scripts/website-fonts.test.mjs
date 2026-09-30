import test from 'node:test';
import assert from 'node:assert/strict';
import { copyFile, cp, mkdir, mkdtemp, readFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { create as createFont } from 'fontkit';

const repository = fileURLToPath(new URL('../', import.meta.url));
const buildScript = fileURLToPath(new URL('./build-website.mjs', import.meta.url));

// Read the deployed font's Unicode-to-glyph mapping, not its advertised CSS
// family name: a browser keeps that name even when individual letters fall back.
function unicodeGlyphs(bytes) {
  // Fontkit's main entry registers both SFNT and compressed WOFF/WOFF2 readers.
  // Keep only actual non-.notdef glyphs, including after WOFF2 decompression.
  const font = createFont(bytes);
  assert.ok(Array.isArray(font.characterSet), 'Expected one font with a Unicode character set.');
  const glyphs = new Set(font.characterSet.filter(code => font.hasGlyphForCodePoint(code)));
  assert.ok(glyphs.size > 0, 'Font has no usable Unicode glyph mapping.');
  return glyphs;
}

function visibleText(markup) {
  const entities = { amp: '&', lt: '<', gt: '>', quot: '"', apos: "'", nbsp: ' ', copy: '©', ndash: '–', mdash: '—', hellip: '…' };
  return markup.replace(/<!--[\s\S]*?-->/g, '')
    .replace(/<(script|style)\b[^>]*>[\s\S]*?<\/\1>/gi, '')
    .replace(/<[^>]*>/g, ' ')
    .replace(/&(#x[0-9a-f]+|#\d+|[a-z]+);/gi, (entity, value) => {
      if (value.startsWith('#')) return String.fromCodePoint(Number.parseInt(value.slice(value[1].toLowerCase() === 'x' ? 2 : 1), value[1].toLowerCase() === 'x' ? 16 : 10));
      assert.ok(Object.hasOwn(entities, value), `Add decoding for HTML entity ${entity}.`);
      return entities[value];
    });
}

function assertCovered(glyphs, text, label) {
  const missing = [...new Set([...text].filter(char => !/\s/u.test(char) && !glyphs.has(char.codePointAt(0))))];
  assert.equal(missing.length, 0, `${label} would mix fallback fonts; missing glyphs: ${missing.map(char => `${char} (U+${char.codePointAt(0).toString(16).toUpperCase()})`).join(', ')}`);
}

let fixturePromise;
function fixture() {
  return fixturePromise ??= (async () => {
    // Only public source files enter this temporary build. The application's
    // installer, work data and normal website-dist directory remain untouched.
    const root = await mkdtemp(path.join(tmpdir(), 'msl-website-font-test-'));
    await cp(path.join(repository, 'website'), path.join(root, 'website'), { recursive: true });
    await mkdir(path.join(root, 'release'));
    for (const name of ['package.json', 'release/latest.json', 'release/history.json']) {
      await copyFile(path.join(repository, name), path.join(root, name));
    }
    const result = spawnSync(process.execPath, [buildScript, '--source-only'], { cwd: root, encoding: 'utf8' });
    assert.equal(result.status, 0, result.stderr || result.stdout);
    const output = path.join(root, 'website-dist');
    const html = await readFile(path.join(output, 'index.html'), 'utf8');
    const css = await readFile(path.join(output, 'style.css'), 'utf8');
    const family = css.match(/--serif:\s*["']([^"']+)["']/)?.[1];
    assert.ok(family, 'The editorial font must have an explicit primary family.');
    const face = [...css.matchAll(/@font-face\s*\{([^}]+)\}/g)].find(match => match[1].match(/font-family:\s*["']([^"']+)["']/)?.[1] === family)?.[1];
    assert.ok(face, 'The editorial font must be hosted with the website.');
    const relative = face.match(/url\(["']?([^\s"')]+)["']?\)/)?.[1];
    assert.ok(relative && !/^(https?:|data:|\/\/)/i.test(relative), 'The font must not depend on a third-party request.');
    const fontPath = path.resolve(output, relative.replace(/[?#].*$/, '').replace(/^\//, ''));
    assert.ok(fontPath.startsWith(`${output}${path.sep}`), 'Font URL must resolve inside the public build.');
    const glyphs = unicodeGlyphs(await readFile(fontPath));
    return { html, glyphs };
  })();
}

test('editorial font covers the reported mixed-baseline headings and version label', async () => {
  const { glyphs } = await fixture();
  for (const text of ['医学联络官的日常工作台。', '项目进展，一处查看。', '更新记录', '当前版本 v0.3.15']) {
    assertCovered(glyphs, text, text);
  }
});

test('every rendered serif heading, quotation and release aside has real glyphs', async () => {
  const { html, glyphs } = await fixture();
  const headings = [...html.matchAll(/<(h1|h2|blockquote)\b[^>]*>([\s\S]*?)<\/\1>/g)];
  assert.ok(headings.length >= 8, 'The test must inspect the rendered public headings and all example quotations.');
  for (const [markup, tag] of headings) assertCovered(glyphs, visibleText(markup), tag);
  const aside = html.match(/<div class="updates-aside">([\s\S]*?)<\/div>/)?.[1];
  assert.ok(aside, 'The actual dynamically rendered release aside must be checked.');
  assertCovered(glyphs, visibleText(aside), 'release aside');
});

test('the font also covers generated release titles and the rest of the visible public copy', async () => {
  const { html, glyphs } = await fixture();
  const body = html.match(/<body\b[^>]*>([\s\S]*?)<\/body>/)?.[1];
  assert.ok(body, 'The real website build must include the document body.');
  assertCovered(glyphs, visibleText(body), 'rendered website copy');
});

test('editorial font has reusable Chinese, Latin and punctuation coverage beyond current wording', async () => {
  const { glyphs } = await fixture();
  let cjkCount = 0;
  for (let code = 0x4e00; code <= 0x9fff; code++) if (glyphs.has(code)) cjkCount++;
  assert.ok(cjkCount >= 10000, `Font must cover at least 10,000 common-block Chinese characters; found ${cjkCount}.`);
  const ascii = Array.from({ length: 0x7e - 0x21 + 1 }, (_, index) => String.fromCodePoint(0x21 + index)).join('');
  assertCovered(glyphs, `${ascii}，。；：！？“”‘’（）《》、—…·`, 'future copy and version metadata');
});
