"""Verify installed UI version and a real refused-connection background failure."""
import json
import os
from pathlib import Path
import runpy
import socket
import time

assert '.test-runtime' in Path(os.environ['APPDATA']).parts
Page = runpy.run_path(str(Path(__file__).with_name('release-functional-cdp.py')))['Page']
page = Page()
def invoke(command, **args):
    return page.eval(f'window.__TAURI_INTERNALS__.invoke({json.dumps(command)},{json.dumps(args)})')

with socket.socket() as listener:
    listener.bind(('127.0.0.1', 0))
    port = listener.getsockname()[1]
provider = None
try:
    provider = invoke('save_provider_connection', displayName='Installed network Mock',
        baseUrl=f'http://127.0.0.1:{port}', enabled=True, apiKey='synthetic-install-key')
    model = invoke('save_provider_model', providerId=provider['id'], modelId='offline-mock',
        displayName='Offline Mock', protocol='chat_completions', endpointPath='/chat/completions', enabled=True, available=True)
    invoke('save_ai_task_route', taskKind='work_draft', providerModelId=model['id'])
    title = f'安装验证 · 合成项目 {int(time.time())}'
    work = invoke('create_work', title=title, status='active')
    page.click('nav-works'); page.wait(.4)
    page.eval(f"[...document.querySelectorAll('.work-item')].find(e=>e.textContent.includes({json.dumps(title)})).click()")
    page.wait(.4); page.click('organize-work-with-ai'); page.wait(.3); page.click('nav-inbox')
    deadline = time.monotonic()+20
    while time.monotonic()<deadline:
        job = next(j for j in invoke('list_ai_jobs') if j['command']=='start_workspace_work_draft' and j['args'].get('workId')==work['id'])
        if job['status']!='running': break
        time.sleep(.3)
    assert job['status']=='failed' and 'NET_CONNECT' in job['error'] and '代理' in job['error']
    assert 'synthetic-install-key' not in job['error']
    print('INSTALLED_NETWORK_DIAGNOSTIC_PASS=1', flush=True)
finally:
    if provider: invoke('delete_provider', id=provider['id'])

page.click('nav-settings'); page.wait(.5)
version = page.eval("document.querySelector('[data-testid=app-version]')?.textContent")
assert version and '0.1.2' in version
page.eval("document.querySelectorAll('.toast button').forEach(e=>e.click()); document.querySelector('[data-testid=app-version]').scrollIntoView({block:'center'})")
page.screenshot('installed-0.1.2.png')
print('INSTALLED_UI_VERSION_PASS=0.1.2', flush=True)
page.ws.close()
