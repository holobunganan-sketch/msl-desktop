import { test } from 'node:test';
import assert from 'node:assert/strict';
import { matchesInstalledBinary } from './verify-installed-binary.mjs';

const built = Buffer.from('MZ\0header__TAURI_BUNDLE_TYPE_VAR_UNK\0program');
const packaged = Buffer.from('MZ\0header__TAURI_BUNDLE_TYPE_VAR_NSS\0program');

test('accepts the single NSIS bundle marker patch without changing input', () => {
  assert.equal(matchesInstalledBinary(built, packaged), true);
  assert.equal(built.toString(), 'MZ\0header__TAURI_BUNDLE_TYPE_VAR_UNK\0program');
});

test('accepts byte-identical binaries', () => {
  assert.equal(matchesInstalledBinary(built, Buffer.from(built)), true);
});

test('rejects any additional changed program byte', () => {
  const changed = Buffer.from(packaged);
  changed[changed.length - 1] ^= 1;
  assert.equal(matchesInstalledBinary(built, changed), false);
});

test('rejects a different installer type, length, or marker position', () => {
  assert.equal(matchesInstalledBinary(built, Buffer.from(packaged.toString().replace('NSS', 'MSI'))), false);
  assert.equal(matchesInstalledBinary(built, Buffer.concat([packaged, Buffer.from('x')])), false);
  assert.equal(matchesInstalledBinary(built, Buffer.from(packaged.toString().replace('header__', 'heade__').replace('\0program', 'r\0program'))), false);
});

test('rejects ambiguous duplicate markers', () => {
  assert.equal(matchesInstalledBinary(Buffer.concat([built, built]), Buffer.concat([packaged, built])), false);
});
