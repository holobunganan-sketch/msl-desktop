import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

// Tauri replaces UNK with NSS in the packaged binary, then restores the build.
// Permit exactly that one marker change; all other bytes must remain identical.
export function matchesInstalledBinary(built, installed) {
  if (built.equals(installed)) return true;
  if (built.length !== installed.length) return false;
  const prefix = Buffer.from('__TAURI_BUNDLE_TYPE_VAR_');
  const offset = built.indexOf(prefix);
  if (offset < 0 || installed.indexOf(prefix) !== offset
    || built.indexOf(prefix, offset + 1) >= 0
    || installed.indexOf(prefix, offset + 1) >= 0) return false;
  const typeOffset = offset + prefix.length;
  if (built.subarray(typeOffset, typeOffset + 3).toString() !== 'UNK'
    || installed.subarray(typeOffset, typeOffset + 3).toString() !== 'NSS') return false;
  const normalized = Buffer.from(installed);
  normalized.write('UNK', typeOffset, 'ascii');
  return built.equals(normalized);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [builtPath, installedPath] = process.argv.slice(2);
  if (!builtPath || !installedPath) throw new Error('Provide build and installed binary paths.');
  if (!matchesInstalledBinary(readFileSync(builtPath), readFileSync(installedPath))) {
    throw new Error('Installed binary differs from the verified local build.');
  }
  console.log('PASS: installed binary matches the build (allowing only the NSIS marker).');
}
