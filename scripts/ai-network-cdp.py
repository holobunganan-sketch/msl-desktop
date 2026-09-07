"""Exercise the real background job with a >60s loopback response and an offline endpoint."""
import importlib.util
import json
import os
from pathlib import Path
import time

assert '.test-runtime' in Path(os.environ['APPDATA']).parts
spec = importlib.util.spec_from_file_location('cdp', Path(__file__).with_name('release-functional-cdp.py'))
cdp = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cdp)
page = cdp.Page()
checks = []

def invoke(command, **args):
    return page.eval(f'window.__TAURI_INTERNALS__.invoke({json.dumps(command)},{json.dumps(args)})')

def check(name, condition):
    checks.append(bool(condition))
    print(f"[{'PASS' if condition else 'FAIL'}] {name}", flush=True)

def finish(job, timeout=90):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        current = invoke('get_ai_job', id=job['id'])
        if current['status'] != 'running':
            return current
        time.sleep(.5)
    raise RuntimeError('Isolated job exceeded test deadline')

provider = None
try:
    schedule = invoke('get_analysis_schedule')
    schedule['enabled'] = False
    invoke('save_analysis_schedule', schedule=schedule)
    provider = invoke('save_provider_connection', displayName='Network regression Mock',
        baseUrl='http://127.0.0.1:9462', enabled=True, apiKey='synthetic-network-key')
    def route(model_id, path='/chat/completions'):
        model = invoke('save_provider_model', providerId=provider['id'], modelId=model_id,
            displayName=model_id, protocol='chat_completions', endpointPath=path,
            capabilitiesJson='{}', source='manual', enabled=True, available=True)
        invoke('save_ai_task_route', taskKind='work_draft', providerModelId=model['id'])
    route('network-slow')
    work = invoke('create_work', title=f'网络修复回归 · 合成项目 {int(time.time())}', status='active')
    page.click('nav-works'); page.wait(.5)
    page.eval(f"[...document.querySelectorAll('.work-item')].find(e=>e.textContent.includes({json.dumps(work['title'])})).click()")
    page.wait(.5); page.click('organize-work-with-ai'); page.wait(.4)
    page.click('nav-inbox')
    job = next(j for j in invoke('list_ai_jobs') if j['command']=='start_workspace_work_draft' and j['args'].get('workId')==work['id'])
    check('Slow project organization continues after navigation', job['status']=='running')
    result = finish(job)
    check('Analysis completes after the previous 60-second deadline', result['status']=='completed' and result['finished_at']-result['created_at']>=65)
    proposals = [p for p in invoke('list_ai_proposals', status='pending', limit=200) if p['analysis_run_id']==result['result']]
    check('Slow response reaches editable confirmation queue', len(proposals)==1)
    detail = invoke('get_work_detail', id=work['id'])
    check('Unconfirmed analysis does not change project title', detail['work']['title']==work['title'])
    route('network-unauthorized', '/network-unauthorized')
    before_failed = len(invoke('list_ai_proposals', status=None, limit=500))
    failed = finish(invoke('start_ai_job', request={'command':'start_workspace_work_draft','args':{'workId':work['id']}}))
    check('Authentication failure becomes a finished failed job', failed['status']=='failed')
    check('Failure identifies HTTP 401 and API Key action', '401' in failed['error'] and 'API Key' in failed['error'])
    check('Provider body and synthetic credential are not exposed', 'synthetic-work-document' not in failed['error'] and 'synthetic-network-key' not in failed['error'])
    page.click('background-jobs-toggle')
    # A direct IPC test job is picked up by the app-wide 1.8-second refresh.
    # Wait for the matching terminal state instead of racing a fixed .5s sleep.
    deadline=time.monotonic()+6
    shown=False
    while time.monotonic()<deadline:
        shown=page.eval("document.querySelector('[data-testid=background-jobs-panel]').textContent.includes('401')")
        if shown: break
        page.wait(.2)
    check('Background panel shows specific failure', shown)
    page.screenshot('network-error-actionable.png')
    page.eval("document.querySelector('[data-testid=background-jobs-panel] header button').click()")
    check('Failed request creates no partial proposals', len(invoke('list_ai_proposals', status=None, limit=500))==before_failed)
    route('mock-secretary')
    recovered = finish(invoke('start_ai_job', request={'command':'start_workspace_work_draft','args':{'workId':work['id']}}))
    check('Retry after corrected connection completes', recovered['status']=='completed')
finally:
    if provider:
        invoke('delete_provider', id=provider['id'])
print(json.dumps({'passed':sum(checks),'total':len(checks)}), flush=True)
raise SystemExit(0 if all(checks) else 1)
