// Native WebView regression with an isolated profile; no real WeChat connection.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
assert.equal(process.env.MSL_ISOLATED_TEST, '1');
assert.ok(process.env.MSL_TEST_PROFILE);
const profile = path.resolve(process.env.MSL_TEST_PROFILE);
assert.ok(profile.split(path.sep).includes('.test-runtime'));
for (const [name, part] of Object.entries({APPDATA:'appdata',LOCALAPPDATA:'localappdata',TEMP:'temp',TMP:'temp'})) {
  assert.equal(path.resolve(process.env[name] ?? ''), path.join(profile, part));
}
const {chromium} = createRequire(path.join(process.env.MSL_NODE_MODULES, 'runtime.cjs'))('playwright');
let browser;
for (const host of ['127.0.0.1','[::1]']) {
  try { browser = await chromium.connectOverCDP(`http://${host}:${process.env.MSL_CDP_PORT}`); break; } catch {}
}
assert.ok(browser);
try {
  const page = browser.contexts()[0].pages()[0];
  const invoke = (command,args={}) => page.evaluate(({command,args}) => window.__TAURI_INTERNALS__.invoke(command,args), {command,args});
  await page.locator('.content-scroll').waitFor();
  assert.equal(path.resolve((await invoke('backup_status')).data_directory), path.join(profile,'appdata','MSLDesktop'));
  await page.evaluate(() => window.dispatchEvent(new CustomEvent('dashboard:navigate',{detail:{view:'settings'}})));
  const card = page.getByTestId('weixin-settings');
  await card.waitFor();
  // The menu is for actions; ordinary capture must not require a menu selection.
  await card.getByText('召唤秘书', {exact:true}).waitFor({timeout:5000});
  const menu = card.getByRole('list',{name:'微信操作菜单'});
  assert.equal(await menu.getByRole('listitem').count(), 5);
  assert.equal(await menu.getByText('记一句话',{exact:true}).count(),0);
  for (const choice of ['全局整理','生成每日简报','查看整理进度','查看待确认数量','退出菜单']) {
    await menu.getByText(choice,{exact:true}).waitFor();
  }
  const artifacts = path.join(profile,'artifacts');
  fs.mkdirSync(artifacts,{recursive:true});
  for (const [width,height,font] of [[1440,900,'standard'],[1280,800,'standard'],[1024,768,'xlarge']]) {
    await page.setViewportSize({width,height});
    await page.evaluate(font=>document.documentElement.dataset.fontSize=font,font);
    await card.scrollIntoViewIfNeeded();
    assert.ok(await card.evaluate(el=>el.scrollWidth-el.clientWidth<=1));
    await page.screenshot({path:path.join(artifacts,`weixin-menu-${width}.png`),fullPage:true});
  }
  await card.getByRole('button',{name:'扫码绑定并启用',exact:true}).click();
  await card.getByRole('alert').filter({hasText:'隔离测试环境'}).waitFor();
  assert.equal((await invoke('get_weixin_status')).bound,false);
  console.log('PASS: five-action menu excludes ordinary capture, three viewport sizes, real login blocked');
} finally { await browser.close(); }
