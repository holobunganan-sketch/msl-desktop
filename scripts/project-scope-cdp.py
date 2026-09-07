"""Project relationship regression. Isolated profiles and synthetic data only."""
import importlib.util
import json
import os
from pathlib import Path
import sqlite3
import time
import urllib.request

ROOT=Path(__file__).resolve().parent.parent
PROFILE=ROOT/".test-runtime"/os.environ.get("MSL_SCOPE_PROFILE","project-scope")
assert PROFILE.resolve().parent==(ROOT/".test-runtime").resolve()
for env,folder in [("APPDATA","appdata"),("LOCALAPPDATA","localappdata"),("TEMP","temp"),("TMP","temp")]:
    assert Path(os.environ[env]).resolve()==(PROFILE/folder).resolve(), "Isolation required"
spec=importlib.util.spec_from_file_location("cdp",ROOT/"scripts"/"release-functional-cdp.py")
cdp=importlib.util.module_from_spec(spec)
spec.loader.exec_module(cdp)
p=cdp.Page()
DB=PROFILE/"appdata"/"MSLDesktop"/"msl-desktop.db"
assert DB.exists(),"Isolated app database required"

def call(name,**args):
    return p.eval(f"window.__TAURI_INTERNALS__.invoke({json.dumps(name)},{json.dumps(args)})")

def check(name,result):
    print(f"{'PASS' if result else 'FAIL'} {name}",flush=True)
    assert result,name

def change(selector,value,event="change"):
    check("Control available",p.eval(f"(()=>{{const e=document.querySelector({json.dumps(selector)});if(!e)return false;e.value={json.dumps(str(value))};e.dispatchEvent(new Event({json.dumps(event)},{{bubbles:true}}));return true}})()"))
    p.wait(.15)

def click_text(selector,text):
    check("Action available",p.eval(f"(()=>{{const e=[...document.querySelectorAll({json.dumps(selector)})].find(e=>e.textContent.trim()==={json.dumps(text)});if(!e)return false;e.click();return true}})()"))
    p.wait(.6)

def select_proposal(number):
    p.click("nav-review");p.wait(.4)
    click_text(".proposal-item strong",f"合成归属事项 {number}")

def confirm():
    check("Confirm available",p.eval("(()=>{const buttons=[...document.querySelectorAll('.review-actions button')];const button=buttons.at(-1);if(!button)return false;button.click();return true})()"))
    p.wait(.9)
    check("No review error",not p.eval("Boolean(document.querySelector('.review-page .error'))"))

def rows():return call("list_ai_proposals",status=None,limit=100)

def geometry():
    return p.eval("""(()=>{const content=document.querySelector('.content-scroll');if(!content)return false;content.scrollTop=content.scrollHeight;const buttons=[...content.querySelectorAll('button')].filter(e=>e.getBoundingClientRect().height>0);return content.scrollWidth<=content.clientWidth+2&&buttons.every(e=>{const r=e.getBoundingClientRect();return r.left>=content.getBoundingClientRect().left-2&&r.right<=innerWidth+2})})()""")

def check_large_layout():
    for width,height in [(1440,1000),(1100,720)]:
        p.command('Emulation.setDeviceMetricsOverride',{'width':width,'height':height,'deviceScaleFactor':1,'mobile':False})
        p.eval("document.documentElement.dataset.fontSize='xlarge';document.querySelector('.content-scroll').scrollTop=0")
        p.wait(.3);p.screenshot(f'scope-populated-large-{width}.png')
        p.eval("(()=>{const sc=document.querySelector('.content-scroll');sc.scrollTop=sc.scrollHeight})()")
        p.wait(.3)
        check(f'Confirm actions reachable at large font {width}',p.eval("(()=>{const area=document.querySelector('.content-scroll').getBoundingClientRect();const buttons=[...document.querySelectorAll('.review-actions button')];return buttons.length>0&&buttons.every(e=>{const r=e.getBoundingClientRect();return r.bottom<=area.bottom+2&&r.top>=area.top&&r.right<=innerWidth&&r.left>=area.left})})()"))
        p.screenshot(f'scope-actions-large-{width}.png')

if os.environ.get("MSL_SCOPE_LAYOUT_ONLY")=="1":
    check_large_layout()
    check('No UI runtime exceptions',not p.errors)
    p.ws.close();raise SystemExit(0)

if os.environ.get("MSL_SCOPE_INSTALLED")=="1":
    p.click('nav-review');p.wait(.5)
    check('Installed build has explicit project routing',p.has('[data-testid="proposal-destination"]'))
    check('Installed build separates existing and new projects',p.eval("(()=>{const s=document.querySelector('[data-testid=proposal-destination]');return [...s.options].some(o=>o.value==='project')&&[...s.options].some(o=>o.value==='work')})()"))
    p.click('nav-plan');p.wait(.5)
    check('Installed build has project filter',p.has('[data-testid="project-filter"]'))
    p.click('task-create');p.wait(.3)
    check('Installed build offers independent or selected project',p.eval("document.querySelector('#task-work option').textContent.includes('独立事项')"))
    check('Installed database integrity',sqlite3.connect(f'file:{DB.as_posix()}?mode=ro',uri=True).execute('PRAGMA foreign_key_check').fetchall()==[])
    check('No installed runtime exceptions',not p.errors)
    p.ws.close();raise SystemExit(0)

if os.environ.get("MSL_SCOPE_REOPEN")=="1":
    works=call("list_works",status=None)
    check("Persisted project count",len(works)==4)
    tasks=call("list_tasks",status=None,workId=None)
    check("Persisted project-linked and independent tasks",any(t["work_id"] is None for t in tasks) and any(t["work_id"] for t in tasks))
    check("Foreign keys intact",sqlite3.connect(f"file:{DB.as_posix()}?mode=ro",uri=True).execute("PRAGMA foreign_key_check").fetchall()==[])
    p.ws.close();raise SystemExit(0)

if os.environ.get("MSL_SCOPE_GROUP")=="1":
    projects=call("list_works",status=None)
    project=next(w for w in projects if w["title"].startswith("合成项目 A"))
    other=next(w for w in projects if w["title"].startswith("合成项目 B"))
    run=call("list_analysis_runs",limit=1)[0]
    now=int(time.time())
    with sqlite3.connect(DB) as conn:
        for number in (7,8,9,10):
            conn.execute("INSERT INTO ai_proposals(analysis_run_id,kind,suggested_kind,operation,dedupe_key,title,payload_json,reason,source_refs_json,status,created_at,updated_at) VALUES(?,'task','task','create',?,?,'{}','合成批量归属测试','[]','pending',?,?)",[run['id'],f'scope-group-{number}',f'合成归属事项 {number}',now,now])
    select_proposal(7)
    change('[data-testid="proposal-destination"]','project')
    change('.proposal-routing .project-scope select',project['id'])
    change('[data-testid="proposal-project-kind"]','waiting')
    select_proposal(8)
    change('.proposal-routing .project-scope select',other['id'])
    p.eval("document.querySelector('[data-testid=group-review]').open=true;for(const label of document.querySelectorAll('[data-testid=group-review] label')){if(/合成归属事项 [78]$/.test(label.querySelector('strong').textContent))label.querySelector('input').click()}")
    p.click('confirm-proposal-group');p.wait(1.5)
    check('Batch confirms unsaved routing from both cards',any(w['title']=='合成归属事项 7' and w['work_id']==project['id'] for w in call('list_waiting',status=None,workId=None)) and any(t['title']=='合成归属事项 8' and t['work_id']==other['id'] for t in call('list_tasks',status=None,workId=None)))
    p.click('nav-today');p.wait(.5)
    # Keep only the first pending card selector scoped to its own controls.
    p.eval("document.querySelector('[data-testid=latest-decision-card] .decision-fields').closest('article')?.setAttribute('data-scope-home','1')")
    change('[data-testid="latest-decision-card"] .proposal-routing [data-testid="proposal-destination"]','project')
    change('[data-testid="latest-decision-card"] .project-scope select',other['id'])
    p.eval("document.querySelector('[data-testid=latest-decision-card] .decision-confirm').click()")
    p.wait(1)
    check('Home routes into selected project without duplication',len(call('list_works',status=None))==4 and len(call('get_work_detail',id=other['id'])['resume_history'])==2)
    p.click('nav-review');p.wait(.4)
    change('[data-testid="proposal-destination"]','project')
    change('.proposal-routing .project-scope select',project['id'])
    change('[data-testid="proposal-project-kind"]','waiting')
    check_large_layout()
    check('No group/home runtime exceptions',not p.errors)
    p.ws.close();raise SystemExit(0)

p.eval("localStorage.setItem('msl-locale','zh-CN')")
p.command("Emulation.setDeviceMetricsOverride",{"width":1440,"height":1000,"deviceScaleFactor":1,"mobile":False})
schedule=call("get_analysis_schedule");schedule.update(enabled=False,daily_enabled=False)
call("save_analysis_schedule",schedule=schedule)
check("Fresh test profile",not call("list_works",status=None))
project=call("create_work",title="合成项目 A · 长期学术沟通")
other=call("create_work",title="合成项目 B · 证据整理")
provider=call("save_provider_connection",id=None,displayName="Synthetic scope",providerType="custom",baseUrl="http://127.0.0.1:9464/v1",legacyModel="",templateKind="custom",authMode="bearer",modelsEndpoint=None,enabled=True,apiKey="synthetic-scope-key")
model=call("save_provider_model",providerId=provider["id"],modelId="flow-scope",displayName="Synthetic scope",protocol="chat_completions",endpointPath="/chat/completions",capabilitiesJson="{}",source="manual",enabled=True,available=True)
call("save_ai_task_route",taskKind="global_analysis",providerModelId=model["id"])
p.click("nav-review");p.wait(.5);p.click("review-run-analysis");p.wait(.3);p.click("nav-plan");p.wait(.3)
check("Page switch during analysis",p.has(".plan"))
for _ in range(40):
    runs=call("list_analysis_runs",limit=5)
    if runs and runs[0]["status"] in ("completed","failed"):break
    p.wait(.5)
check("Mock invalid status repaired before queue",runs[0]["status"]=="completed" and len(rows())==6)
metrics=json.load(urllib.request.urlopen("http://127.0.0.1:9464/metrics"))
check("Repair request exercised",metrics["repair_requests"]>=1)
check("AI has not written entities",len(call("list_works",status=None))==2 and not call("list_tasks",status=None,workId=None))

# Reproduce an old saved draft without touching any real database.
legacy=next(r for r in rows() if r["title"]=="合成归属事项 1")
with sqlite3.connect(DB) as conn:
    conn.execute("UPDATE ai_proposals SET kind='work',user_edited=1 WHERE id=?",[legacy["id"]])
select_proposal(1)
check("Legacy mixed status is explained",p.has(".repair-note"))
check("Legacy status can be changed",p.has('[data-testid="proposal-work-status"]'))
p.screenshot("scope-legacy-repaired.png")
confirm()
check("Legacy draft confirms without invalid status",next(w for w in call("list_works",status=None) if w["title"]==legacy["title"])["status"]=="active")

select_proposal(2)
change('[data-testid="proposal-destination"]','work')
check("Changing type clears task-only state",p.eval("document.querySelector('[data-testid=proposal-work-status]').value")=="active")
confirm()
check("New project is distinct",len(call("list_works",status=None))==4)

select_proposal(3)
change('[data-testid="proposal-destination"]','project')
confirm_error=p.eval("document.querySelector('.review-actions button:last-child').click()")
p.wait(.5)
check("Existing project must be explicitly selected",p.eval("document.querySelector('.review-page .error')?.innerText.includes('选择')"))
change('.proposal-routing .project-scope select',project['id'])
p.screenshot("scope-existing-project.png")
confirm()
detail=call("get_work_detail",id=project["id"])
check("Progress saved to selected existing project",len(detail["resume_history"])==1 and len(call("list_works",status=None))==4)

select_proposal(4)
change('.proposal-routing .project-scope select',project['id'])
confirm()
check("Task belongs to project A",any(t["work_id"]==project["id"] for t in call("list_tasks",status=None,workId=None)))

select_proposal(5)
change('[data-testid="proposal-destination"]','project')
change('.proposal-routing .project-scope select',other['id'])
change('[data-testid="proposal-project-kind"]','waiting')
confirm()
check("Waiting belongs to project B",any(w["work_id"]==other["id"] for w in call("list_waiting",status=None,workId=None)))

select_proposal(6)
change('[data-testid="proposal-destination"]','calendar')
change('.proposal-routing .project-scope select',project['id'])
date_selector='.detail-grid input[type="datetime-local"]'
change(date_selector,time.strftime('%Y-%m-%dT15:00'))
confirm()
detail=call("get_work_detail",id=project["id"])
check("Project overview aggregates task event and progress",len(detail["tasks"])==1 and len(detail["calendar"])==1 and len(detail["resume_history"])==1)

for kind in ('task','waiting','calendar','resume_point'):
    inbox=call('create_inbox_item',content=f'合成收件箱 {kind}')
    p.click('nav-inbox');p.wait(.4)
    if not p.has(f'[data-testid="inbox-project-{inbox["id"]}"]'):
        p.click('nav-plan');p.click('nav-inbox');p.wait(.4)
    p.click(f'inbox-project-{inbox["id"]}');p.wait(.3)
    change('#project-item-kind',kind)
    change('#inbox-work',other['id'])
    if kind=='task':p.screenshot('scope-inbox-routing.png')
    p.eval("document.querySelector('.modal-form').requestSubmit()")
    p.wait(.7)
    check(f"Inbox routing to existing project: {kind}",next(i for i in call('list_inbox') if i['id']==inbox['id'])["converted_to_type"]==kind)
check("Inbox adds items without creating projects",len(call('list_works',status=None))==4)

# Independent items remain supported and editable.
call('create_task',workId=None,title='合成独立事项',priority='normal',dueAt=None,notes=None)
call('create_waiting',workId=None,title='合成独立等待',waitingFor='Team',followUpAt=None,notes=None)
call('create_calendar_event',workId=None,title='合成独立日程',startAt=int(time.time()),endAt=None,allDay=False,kind='other',location=None,notes=None)
for nav,selector in [('plan','.task-list li'),('waiting','.w-list li'),('calendar','.event-card')]:
    p.click('nav-'+nav);p.wait(.4)
    change('[data-testid="project-filter"]','independent')
    check(f"Independent filter: {nav}",p.eval(f"[...document.querySelectorAll({json.dumps(selector)})].length>0 && [...document.querySelectorAll('[data-testid=project-badge]')].every(e=>e.classList.contains('independent'))"))
    change('[data-testid="project-filter"]',str(other['id']))
    check(f"Project filter: {nav}",p.eval("[...document.querySelectorAll('[data-testid=project-badge]')].every(e=>e.textContent.includes('合成项目 B'))"))
    change('[data-testid="project-filter"]','all')
    p.screenshot(f'scope-{nav}.png')

for width,height in [(1440,1000),(1100,720)]:
    p.command('Emulation.setDeviceMetricsOverride',{'width':width,'height':height,'deviceScaleFactor':1,'mobile':False})
    for nav in ['today','works','plan','waiting','calendar','inbox','review']:
        p.click('nav-'+nav);p.wait(.25)
        check(f"Reachable layout {nav} {width}",geometry())
    p.screenshot(f'scope-review-{width}.png')
check('No runtime exceptions',not p.errors)
check('Database integrity',sqlite3.connect(f'file:{DB.as_posix()}?mode=ro',uri=True).execute('PRAGMA integrity_check').fetchone()[0]=='ok')
p.ws.close()
print('PROJECT_SCOPE_CDP_PASS',flush=True)
