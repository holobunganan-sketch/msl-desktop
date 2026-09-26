import test, { after } from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'vite';
import { readFileSync } from 'node:fs';

// Render the actual components; never start the native core or access user data.
const server = await createServer({ server: { middlewareMode: true, hmr:false }, logLevel: 'error' });
after(() => server.close());
const { default: Page } = await server.ssrLoadModule('/src/routes/+page.svelte');
const { render } = await server.ssrLoadModule('svelte/server');
const { default: ProposalPreview } = await server.ssrLoadModule('/src/lib/components/ProposalPreview.svelte');
const { default: Overview } = await server.ssrLoadModule('/src/lib/components/DashboardOverview.svelte');
const { default: Matters } = await server.ssrLoadModule('/src/lib/components/MattersView.svelte');

test('daily brief can be started from the visible header without opening a disclosure', () => {
  const { body } = render(Page);
  const hero = body.slice(body.indexOf('data-testid="dashboard-brief-hero"'), body.indexOf('aria-label="工作概览"'));
  assert.match(hero, /<button[^>]*data-testid="generate-daily-brief"[^>]*>/);
  assert.match(hero, /整理每日简报/);
  assert.doesNotMatch(hero.slice(0,hero.indexOf('data-testid="generate-daily-brief"')),/<details/);
});

test('matter stages keep navigation and actions without an extra instruction banner', () => {
  for (const section of ['inbox','review','plan','waiting','done']) {
    const {body} = render(Matters, {props:{section}});
    assert.match(body, /data-testid="matters-tabs"/);
    assert.doesNotMatch(body, /data-testid="stage-guide"/);
  }
});

test('sidebar retains navigation without the footer motto', () => {
  const {body} = render(Page);
  assert.match(body, /data-testid="nav-settings"/);
  assert.doesNotMatch(body, /专注医学交流|让每一次跟进都有来路/);
});

test('greeting selection changes between openings and stays fixed within a session', async () => {
  const greetings = await server.ssrLoadModule('/src/lib/services/dashboardPresentation.ts').catch(()=>({}));
  assert.equal(typeof greetings.createSessionGreeting, 'function');
  const memory=new Map();
  const storage={getItem:key=>memory.get(key)??null,setItem:(key,value)=>memory.set(key,value)};
  const first=greetings.createSessionGreeting(storage,()=>0);
  assert.equal(first('zh-CN'),first('zh-CN'));
  const next=greetings.createSessionGreeting(storage,()=>0);
  assert.notEqual(first('zh-CN'),next('zh-CN'));
  assert.notEqual(first('zh-CN').title,first('en-US').title);
  assert.ok(first('zh-CN').note.length>0);
  const variants=new Set();
  for(let i=0;i<16;i++)variants.add(greetings.createSessionGreeting(undefined,()=>i/16)('zh-CN').title);
  assert.ok(variants.size>=6);
  const denied={getItem(){throw new Error('blocked');},setItem(){throw new Error('blocked');}};
  assert.doesNotThrow(()=>greetings.createSessionGreeting(denied,()=>.5)('zh-CN'));
});

test('file changes are translated instead of exposing event codes', async () => {
  const presentation=await server.ssrLoadModule('/src/lib/services/dashboardPresentation.ts').catch(()=>({}));
  assert.equal(typeof presentation.fileChangeLabel,'function');
  assert.equal(presentation.fileChangeLabel('file.modified','zh-CN'),'已更新');
  assert.equal(presentation.fileChangeLabel('file.created','zh-CN'),'新资料');
  assert.equal(presentation.fileChangeLabel('file.deleted','en-US'),'Removed');
  assert.equal(presentation.fileChangeLabel('internal.unknown','zh-CN'),'资料变化');
});

test('dashboard exposes the overview as a named navigation region', () => {
  const { body } = render(Page);
  assert.match(body, /aria-label="工作概览"/);
  assert.match(body, /data-testid="overview-projects"/);
  assert.match(body, /data-testid="overview-decisions"/);
});

test('dashboard keeps both work and decision actions available', () => {
  const { body } = render(Page);
  assert.match(body, /继续推进/);
  assert.match(body, /需要您决定/);
  assert.match(body, /让秘书整理一次/);
  assert.match(body, /data-testid="dashboard-secretary-card"/);
});

test('shell renders the shared loop logo with the exact product name', () => {
  const { body } = render(Page);
  assert.match(body, /src="\/brand\/msl-loop.svg"/);
  assert.match(body, />MSL Desktop<\/strong>/);
});

test('compact suggestion retains its scope and puts the full rationale in a disclosure', () => {
  const item = {kind:'task', work_id:1, payload_json:JSON.stringify({title:'准备交流材料',description:'保留完整的工作背景'}), reason:'依据最近一次交流记录'};
  const { body } = render(ProposalPreview, {props:{item,works:[{id:1,title:'演示项目'}],compact:true}});
  assert.match(body, /演示项目/);
  assert.match(body, /<details[^>]*data-testid="proposal-details"/);
  assert.match(body, /查看安排详情/);
  assert.match(body, /依据最近一次交流记录/);
  assert.doesNotMatch(body, /<details[^>]*\sopen(?:\s|>)/);
});

test('compact calendar suggestion keeps the missing-time warning visible', () => {
  const item = {kind:'calendar',work_id:null,payload_json:'{}',reason:'演示依据'};
  const {body} = render(ProposalPreview,{props:{item,works:[],compact:true}});
  assert.match(body, /请确认安排时间/);
  assert.match(body, /请先点“调整”/);
  assert.ok(body.indexOf('请先点“调整”') < body.indexOf('data-testid="proposal-details"'));
});

test('overview distinguishes zero from loading and supports English', () => {
  const {body} = render(Overview,{props:{en:true,counts:{projects:0,tasks:12,waiting:0,calendar:1,inbox:null,decisions:2}}});
  assert.match(body, /aria-label="Work overview"/);
  assert.match(body, />0<\/strong>/);
  assert.match(body, />12<\/strong>/);
  assert.match(body, /aria-label="Loading"/);
});

test('default supporting text has readable contrast on its muted surface', () => {
  const css=readFileSync(new URL('../src/lib/styles/app.css',import.meta.url),'utf8');
  const color=name=>css.match(new RegExp(`--color-${name}:\\s*#([0-9a-f]{6})`))[1];
  const luminance=hex=>hex.match(/../g).map(v=>parseInt(v,16)/255).map(v=>v<=.04045?v/12.92:((v+.055)/1.055)**2.4).reduce((sum,v,i)=>sum+v*[.2126,.7152,.0722][i],0);
  assert.ok((luminance(color('surface-muted'))+.05)/(luminance(color('muted'))+.05)>=4.5);
});
