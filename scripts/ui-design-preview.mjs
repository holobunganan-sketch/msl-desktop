// Isolated, browser-only visual fixture. This server never starts the Rust core.
import { createServer } from 'vite';
if (process.env.MSL_ISOLATED_TEST !== '1') throw new Error('An isolated test environment is required.');
for (const key of ['APPDATA','LOCALAPPDATA','TEMP','TMP']) {
  if (!process.env[key]?.includes('ui-design-')) throw new Error(`Missing isolated ${key}`);
}
const server = await createServer({
  server: { host:'127.0.0.1', port:1432, strictPort:true, open:false },
  plugins: [{
    name:'isolated-design-fixture',
    configureServer(vite) {
      vite.middlewares.use(async (req,res,next) => {
        if (!req.url?.startsWith('/__design')) return next();
        const html = await vite.transformIndexHtml('/__design', `<!doctype html><html lang="zh-CN"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>MSL Desktop · 合成界面预览</title></head><body><div id="fixture-root"></div><script type="module">import '/scripts/ui-design-fixture.mjs';import {mount} from 'svelte';import App from '/src/routes/+page.svelte';mount(App,{target:document.getElementById('fixture-root')});</script></body></html>`);
        res.setHeader('Content-Type','text/html; charset=utf-8');res.end(html);
      });
    }
  }]
});
await server.listen();
console.log('Synthetic UI preview: http://127.0.0.1:1432/__design');
