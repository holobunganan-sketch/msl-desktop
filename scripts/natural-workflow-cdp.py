"""Native WebView workflow checks. Only isolated synthetic data; no real providers."""
import importlib.util
import json
import os
from pathlib import Path
import sys
import sqlite3
import time
import urllib.request

ROOT=Path(__file__).resolve().parent.parent
PROFILE=ROOT/'.test-runtime'/os.environ.get('MSL_FLOW_PROFILE','natural-workflow')
assert PROFILE.resolve().parent==(ROOT/'.test-runtime').resolve()
for key,folder in [('APPDATA','appdata'),('LOCALAPPDATA','localappdata'),('TEMP','temp'),('TMP','temp')]:
    assert Path(os.environ[key]).resolve()==(PROFILE/folder).resolve(), 'Isolation required'
spec=importlib.util.spec_from_file_location('cdp',ROOT/'scripts'/'release-functional-cdp.py')
cdp=importlib.util.module_from_spec(spec);spec.loader.exec_module(cdp)
p=cdp.Page()

def check(label,result):
    print(('PASS ' if result else 'FAIL ')+label,flush=True)
    assert result,label

def call(name,**args):
    return p.eval(f'window.__TAURI_INTERNALS__.invoke({json.dumps(name)},{json.dumps(args)})')

def change(selector,value,event='input'):
    p.eval(f'''(()=>{{const e=document.querySelector({json.dumps(selector)});if(!e)throw Error('Missing control');e.value={json.dumps(str(value))};e.dispatchEvent(new Event({json.dumps(event)},{{bubbles:true}}));}})()''');p.wait(.1)

check('Workbench destinations include experts and preserve existing pages',p.eval("['nav-today','nav-works','nav-kol','nav-matters','nav-calendar','nav-reports'].every(id=>document.querySelector(`[data-testid=primary-navigation] [data-testid=${id}]`))"))
check('Matters opens the original capture queue',p.click('nav-matters'))
p.wait()
check('Matter stages are explicit',p.has('[data-testid=matters-tabs]'))
if '--baseline' in sys.argv:
    sys.exit(0)

DB=PROFILE/'appdata'/'MSLDesktop'/'msl-desktop.db'
def click(selector):
    check('Action exists',p.eval(f"(()=>{{const e=document.querySelector({json.dumps(selector)});if(!e)return false;e.click();return true;}})()"));p.wait(.35)
def route(kind,ident=None):
    p.eval(f"window.dispatchEvent(new CustomEvent('dashboard:navigate',{{detail:{{view:{json.dumps(kind)},id:{json.dumps(ident)}}}}}))");p.wait(.45)
def legacy(kind,ident=None):
    mapping={'task':('matters','plan'),'review':('matters','review'),'inbox':('matters','inbox'),'waiting':('matters','waiting')}
    view,section=mapping[kind]
    detail={'view':view,'section':section,'id':ident}
    p.eval(f"window.dispatchEvent(new CustomEvent('dashboard:navigate',{{detail:{json.dumps(detail)}}}))");p.wait(.45)
def tasks():return call('list_tasks',status=None,workId=None)
def proposals():return call('list_ai_proposals',status=None,limit=500)
def wait_run(previous):
    for _ in range(60):
        runs=call('list_analysis_runs',limit=5)
        run=next((r for r in runs if r['id']>previous),None)
        if run and run['status'] in ('completed','failed'):
            check('Background Mock analysis completed',run['status']=='completed');return run
        p.wait(.25)
    raise AssertionError('Background analysis did not finish')
def sql(query,args=()):
    with sqlite3.connect(DB) as conn:return conn.execute(query,args).fetchall()
def adopt(ident):
    legacy('review',ident)
    check('Readable decision shown before fields',p.has('.proposal-preview') and not p.has('[data-testid=proposal-destination]'))
    p.click('review-confirm');p.wait(.65)
    check('Adoption produces an actionable receipt',p.has('[data-testid=review-receipt-view]'))
def screenshot(name):
    p.eval("document.querySelector('.content-scroll').scrollTop=0");p.wait(.15);p.screenshot(name)
def layout(label):
    check(label+' no horizontal clipping',p.eval("(()=>{const s=document.querySelector('.content-scroll');return s.scrollWidth<=s.clientWidth+2&&document.documentElement.scrollWidth<=innerWidth+2})()"))
    check(label+' final content remains reachable',p.eval("(()=>{const s=document.querySelector('.content-scroll');s.scrollTop=s.scrollHeight;const r=s.getBoundingClientRect();const visible=[...s.querySelectorAll('button,input,textarea,select')].filter(e=>e.checkVisibility({contentVisibilityAuto:true,visibilityProperty:true})&&!e.closest('details:not([open])'));const e=visible.at(-1);if(!e)return true;e.scrollIntoView({block:'nearest'});const t=e.getBoundingClientRect();return t.bottom<=r.bottom+2&&t.right<=r.right+2&&t.left>=r.left-2})()"))

if '--continuity' in sys.argv:
    check('Status-only confirmation preserves existing task notes',tasks()[0]['notes']=='合成记录整理后的下一步')
    sys.exit(0)

if '--reports' in sys.argv:
    provider=next(row for row in call('list_provider_connections') if row['base_url']=='http://127.0.0.1:9466/v1')
    model=call('save_provider_model',providerId=provider['id'],modelId='synthetic-report',displayName='Synthetic reports',protocol='chat_completions',endpointPath='/chat/completions',capabilitiesJson='{}',source='manual',enabled=True,available=True)
    call('save_ai_task_route',taskKind='weekly_report',providerModelId=model['id'])
    route('reports');p.click('generate-weekly-report');p.wait(.2);route('works')
    for _ in range(40):
        reports=call('list_reports',limit=10)
        if reports and reports[0]['status'] in ('completed','failed'):break
        p.wait(.25)
    check('Mock report generation survives page change',bool(reports) and reports[0]['status']=='completed')
    route('reports')
    check('Populated report viewer and numbered content remain accessible',p.has('.report-viewer') and p.eval("document.querySelectorAll('.weekly-content li').length>=3"))
    check('Populated report removes empty message',not p.has('.reports-page .empty'))
    layout('Populated report')
    screenshot('workflow-report-populated.png')
    check('Report UI has no runtime exceptions',not p.errors)
    sys.exit(0)

if '--reopen' in sys.argv:
    check('Same scheduled task and completed state persist',any(t['title']=='合成拜访已完成' and t['status']=='done' and t['scheduled_start'] for t in tasks()))
    check('Project-linked waiting and progress persist',bool(sql('SELECT id FROM waiting_items WHERE work_id IS NOT NULL')) and bool(sql('SELECT id FROM resume_points')))
    check('Capture context survives reopening',len(sql('SELECT inbox_id FROM capture_context'))>=2)
    check('SQLite integrity and references intact',sql('PRAGMA integrity_check')==[('ok',)] and sql('PRAGMA foreign_key_check')==[])
    check('No runtime exceptions after restart',not p.errors)
    sys.exit(0)

p.command('Emulation.setDeviceMetricsOverride',{'width':1440,'height':1000,'deviceScaleFactor':1,'mobile':False})
if p.eval("document.querySelector('.locale-button').textContent.trim()")== 'EN':click('.locale-button')
schedule=call('get_analysis_schedule');schedule.update(enabled=False,daily_enabled=False);call('save_analysis_schedule',schedule=schedule)
if '--layout' not in sys.argv:
    check('Synthetic profile starts empty',not call('list_works',status=None))
    project=call('create_work',title='合成项目 A · 长期学术沟通')
    other=call('create_work',title='合成项目 B · 证据整理')
    provider=call('save_provider_connection',id=None,displayName='Synthetic natural workflow',providerType='custom',baseUrl='http://127.0.0.1:9466/v1',legacyModel='',templateKind='custom',authMode='bearer',modelsEndpoint=None,enabled=True,apiKey='synthetic-natural-key')
    model=call('save_provider_model',providerId=provider['id'],modelId='flow-natural',displayName='Synthetic natural workflow',protocol='chat_completions',endpointPath='/chat/completions',capabilitiesJson='{}',source='manual',enabled=True,available=True)
    call('save_ai_task_route',taskKind='global_analysis',providerModelId=model['id'])
    route('works',project['id']);p.click('project-record');p.wait(.2)
    change('[data-testid=natural-capture-text]','合成：准备拜访，请安排在这个项目中')
    change('[data-testid=quick-capture]','合成顶栏草稿')
    route('works',other['id']);p.click('project-record');p.wait(.2)
    check('Drafts do not leak into another project',p.eval("document.querySelector('[data-testid=natural-capture-text]').value===''&&document.querySelector('[data-testid=quick-capture]').value===''"))
    route('works',project['id']);p.click('project-record');p.wait(.2)
    check('Returning to the project restores both unsaved drafts',p.eval("document.querySelector('[data-testid=natural-capture-text]').value.includes('准备拜访')&&document.querySelector('[data-testid=quick-capture]').value==='合成顶栏草稿'"))
    p.eval("document.querySelector('[data-testid=quick-capture]').dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true}))")
    p.click('natural-capture-submit');p.wait(.25);p.click('nav-calendar');p.wait(.3)
    check('Switching page leaves analysis running',p.has('.calendar'))
    run=wait_run(0)
    check('Capture saved once with project context',len(sql('SELECT inbox_id FROM capture_context WHERE work_id=?',[project['id']]))==1)
    check('AI receives explicit capture context',json.load(urllib.request.urlopen('http://127.0.0.1:9466/metrics'))['natural_context_received'])
    check('No business entities written before confirmation',not tasks() and len(call('list_works',status=None))==2)
    item=next(r for r in proposals() if r['analysis_run_id']==run['id'])
    legacy('review',item['id'])
    check('Default review needs no field editing',p.has('.proposal-preview') and not p.has('[data-testid=proposal-destination]'))
    screenshot('workflow-review-preview.png')
    p.click('review-adjust');p.wait(.15)
    check('Adjust reveals type and project controls',p.has('[data-testid=proposal-destination]') and p.has('.proposal-routing .project-scope'))
    p.click('review-adjust');p.wait(.15)
    memory_before=sql('SELECT COUNT(*) FROM classification_memories')[0][0]
    p.click('review-defer');p.wait(.5)
    check('Later preserves note and teaches no wrong classification',sql('SELECT COUNT(*) FROM classification_memories')[0][0]==memory_before and sql('SELECT processed_at FROM inbox_items')[0][0] is None)
    check('Deferred decision is still accessible',any(r['id']==item['id'] and r['status']=='pending' for r in proposals()))
    adopt(item['id'])
    check('One task with the selected project and time',len(tasks())==1 and tasks()[0]['work_id']==project['id'] and tasks()[0]['scheduled_start'])
    p.click('review-receipt-undo');p.wait(.5)
    check('Undo restores queue and original capture',not tasks() and next(r for r in proposals() if r['id']==item['id'])['status']=='pending')
    adopt(item['id']);task=tasks()[0]
    p.click('review-receipt-view');p.wait(.4)
    check('Receipt opens exact task',p.eval("document.querySelector('#task-title')?.value")==task['title'])
    click('.modal-header .close')
    p.click(f"task-schedule-{task['id']}");p.wait(.15)
    day=time.strftime('%Y-%m-%d')
    change('[data-testid=task-schedule-start]',day+'T15:00')
    change('[data-testid=task-schedule-end]',day+'T16:00')
    p.click('task-schedule-save');p.wait(.5)
    check('Scheduling preserves task ID and creates no duplicate event',len(tasks())==1 and tasks()[0]['id']==task['id'] and sql('SELECT COUNT(*) FROM calendar_events')==[(0,)])
    route('calendar')
    check('Calendar displays original task',p.has(f"[data-task-id='{task['id']}']"))
    click(f"[data-task-id='{task['id']}']")
    check('Calendar opens original task editor',p.eval("document.querySelector('#task-title')?.value")==task['title'])
    click('.modal-header .close')
    p.eval(f"[...document.querySelectorAll('[data-testid=task-row-{task['id']}] button')].find(e=>e.textContent==='记进展').click()")
    p.wait(.2);change('[data-testid=natural-capture-text]','合成：拜访已完成，等对方补充材料')
    p.click('natural-capture-submit');p.wait(.3);p.click('nav-today');p.wait(.3)
    followup=wait_run(run['id'])
    check('Completion still needs confirmation',tasks()[0]['status']!='done' and not call('list_waiting',status=None,workId=None))
    followups=[r for r in proposals() if r['analysis_run_id']==followup['id']]
    check('Related progress arrives as three clear decisions',len(followups)==3)
    for suggestion in followups:adopt(suggestion['id'])
    check('Follow-up advances task, waiting and project together',tasks()[0]['status']=='done' and call('list_waiting',status=None,workId=None)[0]['work_id']==project['id'] and len(call('get_work_detail',id=project['id'])['resume_history'])==1)
    check('Status-only confirmation preserves existing task notes',tasks()[0]['notes']=='合成记录整理后的下一步')
    # Retain a synthetic older decision; no real data is read or changed.
    now=int(time.time())
    with sqlite3.connect(DB) as conn:
        conn.execute("INSERT INTO ai_proposals(analysis_run_id,kind,suggested_kind,operation,dedupe_key,title,payload_json,reason,source_refs_json,status,created_at,updated_at) VALUES(?,'task','task','create','natural-old-pending','合成：上周尚未处理的安排','{}','合成旧建议','[]','pending',?,?)",[followup['id'],now-9*86400,now-9*86400])
    legacy('review')
    check('Older unresolved decision remains accessible beyond seven days',p.body_has('合成：上周尚未处理的安排'))
    with sqlite3.connect(DB) as conn:
        conn.execute("INSERT INTO ai_proposals(analysis_run_id,kind,suggested_kind,operation,dedupe_key,title,payload_json,reason,source_refs_json,status,created_at,updated_at) VALUES(?,'calendar','calendar','create','natural-missing-date','合成：待定日期的讨论','{}','请确认日期','[]','pending',?,?)",[followup['id'],now,now])
    missing=next(item for item in proposals() if item['dedupe_key']=='natural-missing-date')
    legacy('review',missing['id']);p.click('review-confirm');p.wait(.3)
    check('Missing time opens adjustment without writing an event',p.has('[data-testid=proposal-destination]') and sql('SELECT COUNT(*) FROM calendar_events')==[(0,)])
    for width,height in [(1440,1000),(1100,720)]:
        p.command('Emulation.setDeviceMetricsOverride',{'width':width,'height':height,'deviceScaleFactor':1,'mobile':False})
        p.eval("document.documentElement.dataset.fontSize='xlarge'")
        layout(f'Expanded review {width}')
        p.screenshot(f'workflow-expanded-actions-{width}.png')
    route('works',project['id'])
    check('Project landing shows exact project and next step',p.eval("document.querySelector('.detail-head h2').textContent")==project['title'] and p.has('.rp-next'))
    check('Folders and verbose history are collapsed',p.eval("!document.querySelector('[data-testid=work-folder-card]').open"))
    p.click('nav-matters');p.wait(.2);p.click('nav-done');p.wait(.3)
    check('Completed task is discoverable in completed stage',p.has(f"[data-testid=task-row-{task['id']}]"))

# Screenshots and geometry cover all destinations; fixtures contain no real work.
for width,height in [(1440,1000),(1100,720)]:
    p.command('Emulation.setDeviceMetricsOverride',{'width':width,'height':height,'deviceScaleFactor':1,'mobile':False})
    p.eval("document.documentElement.dataset.fontSize='xlarge'")
    for name in ('today','works','inbox','review','task','waiting','calendar','reports','translation','workspace','settings'):
        if name in ('inbox','review','task','waiting'):legacy(name)
        else:route(name)
        if name=='reports':check('Single empty report message',p.eval("document.querySelectorAll('.reports-page .empty').length<=1"))
        layout(f'{name} {width}')
        screenshot(f'workflow-{name}-zh-{width}.png')
    route('today');click('.locale-button');layout(f'today English {width}');screenshot(f'workflow-today-en-{width}.png')
    legacy('review');layout(f'review English {width}');screenshot(f'workflow-review-en-{width}.png')
    click('.locale-button')
check('Database integrity and foreign keys intact',sql('PRAGMA integrity_check')==[('ok',)] and sql('PRAGMA foreign_key_check')==[])
check('No UI runtime exceptions',not p.errors)
p.ws.close()
