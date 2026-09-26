import test, { after } from 'node:test';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { createServer } from 'vite';
import {parse} from 'svelte/compiler';
import {readFileSync} from 'node:fs';

// Exercise the real component without starting Tauri or touching saved work.
const server = await createServer({
  resolve: { alias: { $lib: fileURLToPath(new URL('../src/lib', import.meta.url)) } },
  server: { middlewareMode: true, hmr:false },
  logLevel: 'error',
});
after(() => server.close());
const { default: NaturalCapture } = await server.ssrLoadModule('/src/lib/components/NaturalCapture.svelte');
const { render } = await server.ssrLoadModule('svelte/server');
const { locale } = await server.ssrLoadModule('/src/lib/i18n/index.ts');

test('capture reserves its live feedback area before the first save', () => {
  for (const language of ['zh-CN', 'en-US']) {
    locale.set(language);
    const { body } = render(NaturalCapture);
    // Removing this always-mounted region reintroduces the post-save layout shift.
    const feedback = body.slice(body.indexOf('data-testid="natural-capture-feedback"'));
    assert.match(feedback, /aria-live="polite"/);
    assert.ok(body.indexOf('data-testid="natural-capture-feedback"') > body.indexOf('data-testid="natural-capture-submit"'));
  }
});

test('capture reserves receipt actions without exposing unavailable links before save', () => {
  locale.set('zh-CN');
  const { body } = render(NaturalCapture);
  // The controls keep their space but cannot receive focus or navigate before saving.
  assert.match(body, /<div[^>]*data-testid="capture-receipt-actions"[^>]*aria-hidden="true"/);
  assert.match(body, /<button[^>]*data-testid="capture-view-note"[^>]*disabled/);
  assert.match(body, /<button[^>]*data-testid="capture-view-suggestions"[^>]*disabled/);
});

test('the actual capture receipt handler keeps the project scope when opening suggestions', async()=>{
 const source=readFileSync(new URL('../src/lib/components/NaturalCapture.svelte',import.meta.url),'utf8');
 const root=parse(source,{modern:true});
 const walk=node=>{if(!node||typeof node!=='object')return null;if(node.type==='RegularElement'&&node.attributes?.some(a=>a.name==='data-testid'&&a.value?.[0]?.data==='capture-view-suggestions'))return node;for(const value of Object.values(node)){if(Array.isArray(value)){for(const child of value){const found=walk(child);if(found)return found;}}else{const found=walk(value);if(found)return found;}}return null;};
 const button=walk(root),attribute=button.attributes.find(a=>a.name==='onclick');
 const expression=(Array.isArray(attribute.value)?attribute.value[0]:attribute.value).expression;
 const route=await server.ssrLoadModule('/src/lib/services/navigation.ts');
 let received;
 const navigate=(kind,id,workId)=>received=typeof kind==='string'?route.resolveDestination(kind,id,workId):kind;
 // Execute the component's actual click closure; deleting its context argument must fail this test.
 const click=Function('navigateTo','context',`return (${source.slice(expression.start,expression.end)})`)(navigate,{workId:3,entityKind:'work',entityId:3});
 click();
 assert.deepEqual(received,{view:'matters',section:'review',workId:3});
});
