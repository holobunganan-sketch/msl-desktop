import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const read = path => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8').replaceAll('\r\n', '\n');
const config = JSON.parse(read('src-tauri/tauri.conf.json'));

test('Windows display name is exactly MSL Desktop', () => {
  assert.equal(config.productName, 'MSL Desktop');
  assert.equal(config.identifier, 'com.zhounan.msl-desktop');
  assert.equal(JSON.parse(read('package.json')).name, 'msl-desktop');
});

test('release versions agree', () => {
  const version = JSON.parse(read('package.json')).version;
  assert.equal(config.version, version);
  assert.match(read('src-tauri/Cargo.toml'), new RegExp(`version = "${version.replaceAll('.', '\\.')}"`));
  assert.ok(read('src-tauri/Cargo.lock').includes(`name = "msl-desktop"\nversion = "${version}"`));
});

test('installer retains legacy upgrade identity and application data paths', () => {
  assert.equal(config.bundle.windows.nsis.template, 'windows/installer.nsi');
  const template = read('src-tauri/windows/installer.nsi');
  assert.ok(template.includes('!define PRODUCTNAME "{{product_name}}"'));
  assert.ok(template.includes('!define UNINSTKEY "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\msl-desktop"'));
  assert.ok(template.includes('!define MANUPRODUCTKEY "${MANUKEY}\\msl-desktop"'));
  assert.ok(read('src-tauri/src/db/mod.rs').includes('APP_DATA_DIR_NAME: &str = "MSLDesktop"'));
  assert.ok(template.includes('IsShortcutTarget "${oldPath}" "$INSTDIR\\${MAINBINARYNAME}.exe"'));
  assert.ok(template.includes('MigrateLegacyProductShortcut "$SMPROGRAMS\\msl-desktop.lnk"'));
  assert.equal((template.match(/!insertmacro PreserveUnrelatedShortcut /g) || []).length, 3);
  assert.ok(template.includes('IsShortcutTarget "${shortcut}" "$INSTDIR\\${MAINBINARYNAME}.exe"'));
});

test('release workflow derives the installer filename from productName', () => {
  const workflow = read('.github/workflows/release-windows.yml');
  assert.ok(workflow.includes('${productName}_${version}_x64-setup.exe'));
  assert.ok(workflow.includes('MSL-Desktop-Windows-x64.exe'));
});
