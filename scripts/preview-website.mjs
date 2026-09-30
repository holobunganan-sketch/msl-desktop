import { createServer } from 'node:http';
import { cp, copyFile, mkdir, mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const buildScript = fileURLToPath(new URL('./build-website.mjs', import.meta.url));
const mimeTypes = { '.html': 'text/html; charset=utf-8', '.css': 'text/css; charset=utf-8', '.js': 'text/javascript; charset=utf-8', '.json': 'application/json', '.png': 'image/png', '.svg': 'image/svg+xml', '.woff2': 'font/woff2', '.ttf': 'font/ttf' };

export async function startWebsitePreview({ root = process.cwd(), port = 4174 } = {}) {
  // Build public sources in a private temporary directory, preserving release
  // packages already staged in the repository's website-dist directory.
  const temporaryParent = path.resolve(tmpdir());
  const temporary = await mkdtemp(path.join(temporaryParent, 'msl-website-preview-'));
  const dispose = async () => {
    if (path.dirname(temporary) !== temporaryParent || !path.basename(temporary).startsWith('msl-website-preview-')) throw new Error('Unsafe preview cleanup path.');
    await rm(temporary, { recursive: true, force: true });
  };
  let server;
  try {
    await cp(path.join(root, 'website'), path.join(temporary, 'website'), { recursive: true });
    await mkdir(path.join(temporary, 'release'));
    for (const name of ['package.json', 'release/history.json', 'release/latest.json']) {
      await copyFile(path.join(root, name), path.join(temporary, name));
    }
    const result = spawnSync(process.execPath, [buildScript, '--source-only'], { cwd: temporary, encoding: 'utf8' });
    if (result.status !== 0) throw new Error(result.stderr || result.stdout || 'Website preview build failed.');
    const output = path.join(temporary, 'website-dist');
    server = createServer(async (request, response) => {
      response.setHeader('Cache-Control', 'no-store');
      response.setHeader('X-Content-Type-Options', 'nosniff');
      if (!['GET', 'HEAD'].includes(request.method)) return response.writeHead(405).end();
      let pathname;
      try { pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname); }
      catch { return response.writeHead(400).end(); }
      if (['/downloads/MSL-Desktop-Windows-x64.exe', '/downloads/SHA256SUMS.txt'].includes(pathname)) {
        return response.writeHead(302, { Location: `https://msl-desktop.pages.dev${pathname}` }).end();
      }
      const target = path.resolve(output, `.${pathname === '/' ? '/index.html' : pathname}`);
      if (!target.startsWith(`${output}${path.sep}`)) return response.writeHead(403).end();
      try {
        const bytes = await readFile(target);
        response.writeHead(200, { 'Content-Type': mimeTypes[path.extname(target)] || 'text/plain; charset=utf-8' });
        response.end(request.method === 'HEAD' ? undefined : bytes);
      } catch { response.writeHead(404).end(); }
    });
    await new Promise((resolve, reject) => {
      server.once('error', reject);
      server.listen(port, '127.0.0.1', resolve);
    });
    return {
      url: `http://127.0.0.1:${server.address().port}/`,
      close: async () => {
        server.closeAllConnections();
        await new Promise(resolve => server.close(resolve));
        await dispose();
      },
    };
  } catch (error) {
    server?.close();
    await dispose();
    throw error;
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const preview = await startWebsitePreview();
    console.log(`MSL website preview: ${preview.url}`);
    console.log('Includes the complete release history. Downloads use the official website. Ctrl+C to close.');
    for (const signal of ['SIGINT', 'SIGTERM']) process.once(signal, () => preview.close().then(() => process.exit(0)));
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
