import test,{before,after,beforeEach,afterEach} from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {createServer} from 'vite';
import path from 'node:path';
if(process.env.MSL_ISOLATED_TEST!=='1')throw Error('Isolated execution required');
for(const name of ['APPDATA','LOCALAPPDATA','TEMP','TMP'])if(!process.env[name]?.includes('iteration-20260930'))throw Error(`Isolated ${name} required`);
const {chromium}=createRequire(path.join(process.env.MSL_NODE_MODULES,'package.json'))('playwright');
let server,browser,page,base;
before(async()=>{
  server=await createServer({server:{host:'127.0.0.1',port:0,strictPort:false,open:false,hmr:false,fs:{allow:[process.cwd()]}},logLevel:'error',plugins:[{name:'project-category-test',configureServer(vite){vite.middlewares.use(async(req,res,next)=>{
    if(!req.url?.startsWith('/__projects-test'))return next();
    const html=await vite.transformIndexHtml('/__projects-test','<!doctype html><html lang="zh-CN"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1"><link rel="icon" href="data:,"></head><body><div id="fixture-root"></div><script type="module" src="/tests/fixtures/homepageCapture.mjs"></script></body></html>');res.setHeader('Content-Type','text/html; charset=utf-8');res.end(html);
  });}}]});
  await server.listen();base=`http://127.0.0.1:${server.httpServer.address().port}`;
  browser=await chromium.launch({channel:'msedge',headless:true});
});
after(async()=>{await browser?.close();server?.httpServer?.closeAllConnections();await server?.close();});
beforeEach(async()=>{page=await browser.newPage({viewport:{width:1440,height:900}});page.setDefaultTimeout(15000);const errors=[];page.on('pageerror',e=>errors.push(e.message));await page.goto(`${base}/__projects-test`,{waitUntil:'domcontentloaded',timeout:60000});try{await page.getByTestId('nav-works').waitFor({timeout:60000});}catch(e){throw Error(`${e.message}; browser errors: ${errors.join('; ')}`);}await page.getByTestId('nav-works').click();await page.locator('.project-category-tabs').waitFor();});
afterEach(async()=>page?.close());

test('category views keep legacy projects visible without assigning inferred categories',async()=>{
  assert.equal(await page.locator('.work-list .work-item').count(),4);
  assert.equal(await page.locator('.category-hint').isVisible(),true);
  await page.locator('.project-category-tabs button').nth(1).click();
  assert.equal(await page.locator('.work-list .work-item').count(),1);
  assert.match(await page.locator('.work-list').innerText(),/真实世界研究讨论/);
  await page.locator('.project-category-tabs button').nth(2).click();
  assert.match(await page.locator('.work-list').innerText(),/区域学术沟通计划/);
  await page.locator('.project-category-tabs button').first().click();
  assert.equal(await page.locator('.work-list .work-item').count(),4);
  const calls=await page.evaluate(()=>window.__CAPTURE_TEST__.calls);
  assert.equal(calls.filter(c=>c.name==='set_project_category').length,0);
});

test('linking a nonclinical task shows the same item under the clinical project without changing its owner',async()=>{
  await page.locator('.project-category-tabs button').nth(2).click();
  let row=page.locator('.row-item').filter({has:page.getByRole('button',{name:'准备下次专家交流的证据资料',exact:true})});
  await row.locator('.clinical-link summary').click();
  await row.getByRole('combobox',{name:'关联临床研究'}).selectOption('3');
  await row.getByRole('button',{name:'保存关联',exact:true}).click();
  await page.waitForFunction(()=>window.__CAPTURE_TEST__.calls.some(c=>c.name==='set_clinical_link'));
  await page.locator('.project-category-tabs button').nth(1).click();
  row=page.locator('.row-item').filter({has:page.getByRole('button',{name:'准备下次专家交流的证据资料',exact:true})});
  await row.waitFor();assert.equal(await row.count(),1);assert.match(await row.innerText(),/关联事项 · 演示 · 区域学术沟通计划/);
  await row.scrollIntoViewIfNeeded();await page.screenshot({path:path.join(process.env.TEMP,'projects-clinical-association.png'),fullPage:true});
  await page.locator('.project-category-tabs button').nth(2).click();
  assert.equal(await page.getByRole('button',{name:'准备下次专家交流的证据资料',exact:true}).count(),1);
  const calls=await page.evaluate(()=>window.__CAPTURE_TEST__.calls);
  assert.equal(calls.filter(c=>c.name==='create_task'||c.name==='update_task').length,0);
});

test('a linked unprocessed note is visible in its clinical project and opens the original inbox record',async()=>{
  await page.getByTestId('nav-matters').click();
  const inboxRow=page.locator('[data-inbox-id="1"]');
  await inboxRow.locator('.clinical-link summary').click();
  await inboxRow.getByRole('combobox',{name:'关联临床研究'}).selectOption('3');
  await inboxRow.getByRole('button',{name:'保存关联',exact:true}).click();
  await page.waitForFunction(()=>window.__CAPTURE_TEST__.calls.some(c=>c.name==='set_clinical_link'));
  await page.getByTestId('nav-works').click();await page.locator('.project-category-tabs button').nth(1).click();
  await page.getByRole('heading',{name:'演示 · 真实世界研究讨论',exact:true}).waitFor();
  const note=page.getByTestId('project-linked-inbox-1');
  assert.equal(await note.count(),1,'A linked original note must appear before it is converted into a task');
  assert.match(await note.innerText(),/下次交流前，先整理专家提出的长期随访问题/);
  await note.getByRole('button',{name:'查看原话 ↗',exact:true}).click();
  await page.locator('[data-inbox-id="1"].focused').waitFor();
  assert.equal(await page.locator('[data-inbox-id="1"]').count(),1);
  const calls=await page.evaluate(()=>window.__CAPTURE_TEST__.calls);
  assert.equal(calls.filter(c=>c.name.startsWith('convert_inbox')||c.name==='create_inbox_item').length,0);
});
