"""0.2.1 native regression; only an explicitly isolated synthetic profile."""
import importlib.util
import json
import os
from pathlib import Path
import sqlite3
import sys
import time
import urllib.request

ROOT=Path(__file__).resolve().parent.parent
PROFILE=ROOT/'.test-runtime'/os.environ.get('MSL_FLOW_PROFILE','collaborative-ui')
assert PROFILE.resolve().parent==(ROOT/'.test-runtime').resolve()
for key,folder in [('APPDATA','appdata'),('LOCALAPPDATA','localappdata'),('TEMP','temp'),('TMP','temp')]:
    assert Path(os.environ[key]).resolve()==(PROFILE/folder).resolve(), 'Isolation required'
spec=importlib.util.spec_from_file_location('cdp',ROOT/'scripts/release-functional-cdp.py')
cdp=importlib.util.module_from_spec(spec);spec.loader.exec_module(cdp)
p=cdp.Page()
DB=PROFILE/'appdata/MSLDesktop/msl-desktop.db'
def check(name,value):
    print(('PASS ' if value else 'FAIL ')+name,flush=True)
    assert value,name
def call(name,**args):return p.eval(f'window.__TAURI_INTERNALS__.invoke({json.dumps(name)},{json.dumps(args)})')
def sql(query,args=()):
    with sqlite3.connect(DB) as conn:return conn.execute(query,args).fetchall()
def route(view,section=None,ident=None):
    p.eval(f"window.dispatchEvent(new CustomEvent('dashboard:navigate',{{detail:{json.dumps(dict(view=view,section=section,id=ident))}}}))");p.wait(.5)
def metrics():return json.load(urllib.request.urlopen('http://127.0.0.1:9466/metrics'))
def count(model):return metrics().get('model_requests',{}).get(model,0)
def model(name,kind='weekly_report'):
    provider=next(row for row in call('list_provider_connections') if row['base_url']=='http://127.0.0.1:9466/v1')
    result=call('save_provider_model',providerId=provider['id'],modelId=name,displayName='Synthetic '+name,protocol='chat_completions',endpointPath='/chat/completions',capabilitiesJson='{}',source='manual',enabled=True,available=True)
    call('save_ai_task_route',taskKind=kind,providerModelId=result['id'])
    return result
def job(command,**args):
    row=call('start_ai_job',request={'command':command,'args':args})
    route('calendar')
    for _ in range(120):
        current=next((r for r in call('list_ai_jobs',limit=100) if r['id']==row['id']),None)
        if current and current['status'] in ('completed','failed'):return current
        p.wait(.15)
    raise AssertionError('Background job did not finish')

check('Release version is 0.2.1',p.eval("window.__TAURI_INTERNALS__.invoke('plugin:app|version')")=='0.2.1')
if '--screenshots' in sys.argv:
    for width,height,font in [(1440,1000,'large'),(1100,720,'xlarge')]:
        p.command('Emulation.setDeviceMetricsOverride',{'width':width,'height':height,'deviceScaleFactor':1,'mobile':False})
        p.eval(f"document.documentElement.dataset.fontSize={json.dumps(font)}")
        route('reports')
        rows=call('list_reports',limit=100)
        index=next(i for i,r in enumerate(rows) if r['kind']=='weekly' and r['status']=='completed')
        p.eval(f"document.querySelectorAll('.history-list button')[{index}].click()")
        p.wait(.2)
        p.eval("document.querySelectorAll('.toast button').forEach(e=>e.click());document.querySelector('[data-testid=readable-report]').scrollIntoView({block:'center'})")
        p.wait(.2)
        check('Readable report remains reachable at '+str(width),p.eval("document.querySelector('.content-scroll').scrollWidth<=document.querySelector('.content-scroll').clientWidth+2"))
        p.screenshot(f'collaborative-report-final-{width}.png')
    p.ws.close()
    sys.exit(0)
if '--reopen' in sys.argv:
    stats=call('get_ai_efficiency_stats')
    check('Reuse counters survive reopening',stats['reused_checks']>=2 and stats['input_chars_saved']>0)
    check('Report contract and readable content survive reopening',bool(sql("SELECT id FROM reports WHERE content LIKE '%下一步%' AND source_counts_json LIKE '%report-spec-v2%'")))
    check('Calendar dates and original identities persist',bool(sql('SELECT id FROM tasks WHERE due_at IS NOT NULL')) and bool(sql('SELECT id FROM waiting_items WHERE follow_up_at IS NOT NULL')))
    check('SQLite integrity preserved',sql('PRAGMA integrity_check')==[('ok',)] and sql('PRAGMA foreign_key_check')==[])
    sys.exit(0)

# The preceding natural-workflow regression populated only synthetic records.
check('Only synthetic fixtures are present',all('合成' in row['title'] for row in call('list_works',status=None)))
model('flow-natural','global_analysis')
start=count('flow-natural')
original=call('run_analysis_now',trigger='manual')
check('Manual analysis calls the model',count('flow-natural')==start+1)
proposals_before=call('list_latest_analysis_proposals',limit=100)
reused=call('run_analysis_now',trigger='interval')
check('Unchanged interval uses zero additional requests',reused==original and count('flow-natural')==start+1)
check('Reuse keeps the original decision batch',call('list_latest_analysis_proposals',limit=100)==proposals_before and call('list_analysis_runs',limit=1)[0]['status']=='reused')
check('Daily unchanged check also reuses after date validation',call('run_analysis_now',trigger='daily')==original and count('flow-natural')==start+1)
call('create_inbox_item',content='合成：新增反馈需要重新核查',source='manual')
check('New evidence invalidates reuse',call('run_analysis_now',trigger='interval')!=original and count('flow-natural')==start+2)
call('run_analysis_now',trigger='manual')
check('Manual rerun never skips the model',count('flow-natural')==start+3)
model('flow-invalid','global_analysis')
bad_before=count('flow-invalid')
first_bad=job('run_analysis_now',trigger='interval')
second_bad=job('run_analysis_now',trigger='interval')
check('Changed model never reuses another model result',first_bad['status']=='failed' and count('flow-invalid')>bad_before)
check('Failed analysis cannot become a reuse baseline',second_bad['status']=='failed' and count('flow-invalid')==bad_before+4)
model('flow-natural','global_analysis')
stats=call('get_ai_efficiency_stats')
check('Fixed-size efficiency metrics contain no prompts',stats['reused_checks']>=2 and stats['input_chars_saved']>0 and sql('SELECT COUNT(*) FROM ai_efficiency_state')==[(1,)])

route('matters','inbox')
check('Current stage explains the next action',p.has('[data-testid=stage-guide]') and p.body_has('先记下发生了什么'))
works=call('list_works',status=None)
route('works',ident=works[0]['id']);p.click('project-record');p.wait(.3)
before=sql('SELECT COUNT(*) FROM inbox_items');requests=count('flow-natural')
check('Empty record offers a draft starter',p.click('capture-example'))
p.wait(.15)
check('Draft starter does not save or request AI',sql('SELECT COUNT(*) FROM inbox_items')==before and count('flow-natural')==requests and bool(p.eval("document.querySelector('[data-testid=natural-capture-text]').value")))

route('calendar')
check('Confirmed task deadline appears in Calendar',p.has('[data-time-kind=deadline][data-task-id]'))
check('Confirmed waiting follow-up appears in Calendar',p.has('[data-time-kind=followup][data-waiting-id]'))
check('Projection creates no duplicate calendar records',sql('SELECT COUNT(*) FROM calendar_events')==[(0,)])
p.eval("document.querySelector('[data-time-kind=followup][data-waiting-id]').click()");p.wait(.4)
check('Follow-up opens its original waiting editor',p.has('#waiting-title'))
route('calendar')
p.eval("document.querySelector('[data-time-kind=deadline][data-task-id]').click()");p.wait(.4)
check('Deadline opens its original task editor',p.has('#task-title'))

model('collab-report')
result=job('generate_report',kind='weekly')
check('Weekly generation completes while another page is open',result['status']=='completed')
report=call('list_reports',limit=1)[0]
check('Weekly report has structured human-readable findings',report['status']=='completed' and '下一步：' in report['content'] and 'entity_id' not in report['content'] and report['content'].startswith('1. '))
check('Report receives business change history',metrics()['report_has_period_changes'])
route('reports')
p.command('Emulation.setDeviceMetricsOverride',{'width':1440,'height':1000,'deviceScaleFactor':1,'mobile':False})
p.eval("document.documentElement.dataset.fontSize='large'")
check('Report prose is prominent and settings are collapsed',p.has('[data-testid=readable-report]') and p.eval("!document.querySelector('[data-testid=report-options]').open"))
# Test clipboard behavior without altering the user's system clipboard.
p.eval("Object.defineProperty(navigator.clipboard,'writeText',{configurable:true,value:async text=>{window.__copiedReport=text;}})")
p.click('copy-report');p.wait(.2)
check('Copy exports readable prose',p.eval("window.__copiedReport?.startsWith('1. ') && !window.__copiedReport?.includes('entity_id')"))
p.eval("document.querySelector('[data-testid=readable-report]').scrollIntoView({block:'center'});")
p.wait(.2);p.screenshot('collaborative-report-reading.png')
model('collab-report-repair')
before=count('collab-report-repair');result=job('generate_report',kind='weekly')
check('Malformed report gets one validated repair',result['status']=='completed' and count('collab-report-repair')==before+2)
model('collab-report-bad')
before=count('collab-report-bad');result=job('generate_report',kind='weekly')
check('Repeatedly invalid report stops after one repair',result['status']=='failed' and count('collab-report-bad')==before+2 and call('list_reports',limit=1)[0]['content'] is None)
model('collab-report','monthly_report')
now=int(time.time())
result=job('generate_report',kind='monthly',periodStart=now-7*86400,periodEnd=now+1)
check('Custom monthly combines business history and overlapping weeklies',result['status']=='completed' and metrics()['report_has_weeklies'] and metrics()['report_has_period_changes'])
check('Input and output keep independent records intact',sql('PRAGMA integrity_check')==[('ok',)] and sql('PRAGMA foreign_key_check')==[])
check('Native UI has no runtime exceptions',not p.errors)
p.ws.close()
