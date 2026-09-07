"""Native Q&A regressions. Only synthetic data in explicitly isolated profiles."""
import importlib.util,json,os,sys,sqlite3,time,urllib.request
from pathlib import Path
ROOT=Path(__file__).resolve().parent.parent
PROFILE=ROOT/'.test-runtime'/os.environ.get('MSL_FLOW_PROFILE','qa-chat')
assert PROFILE.resolve().parent==(ROOT/'.test-runtime').resolve()
for key,folder in [('APPDATA','appdata'),('LOCALAPPDATA','localappdata'),('TEMP','temp'),('TMP','temp')]:
    assert Path(os.environ[key]).resolve()==(PROFILE/folder).resolve(),'Isolated profile required'
spec=importlib.util.spec_from_file_location('cdp',ROOT/'scripts'/'release-functional-cdp.py')
cdp=importlib.util.module_from_spec(spec);spec.loader.exec_module(cdp)
p=cdp.Page()
def check(label,ok):
    print(('PASS ' if ok else 'FAIL ')+label,flush=True);assert ok,label
def call(name,**args):return p.eval(f'window.__TAURI_INTERNALS__.invoke({json.dumps(name)},{json.dumps(args)})')
def change(selector,value):
    p.eval(f"(()=>{{const e=document.querySelector({json.dumps(selector)});e.value={json.dumps(value)};e.dispatchEvent(new Event('input',{{bubbles:true}}));e.dispatchEvent(new Event('change',{{bubbles:true}}));}})()");p.wait(.25)
def until(fn,label):
    for _ in range(100):
        if fn():check(label,True);return
        p.wait(.2)
    check(label,False)
def sql(query):
    with sqlite3.connect(PROFILE/'appdata'/'MSLDesktop'/'msl-desktop.db') as c:return c.execute(query).fetchall()
for _ in range(30):
    if p.has('[data-testid=nav-qa]'):break
    p.wait(.2)
p.click('nav-qa');p.wait(.6)
p.command('Emulation.setDeviceMetricsOverride',{'width':1440,'height':1000,'deviceScaleFactor':1,'mobile':False})
check('Chat model is selectable on the Q&A page',p.has('[data-testid=qa-model]'))
if '--baseline' in sys.argv:p.ws.close();sys.exit(0)
if os.environ.get('MSL_EXPECTED_VERSION'):check('Expected installed runtime',call('plugin:app|version')==os.environ['MSL_EXPECTED_VERSION'])
if '--reopen' in sys.argv or '--installed-smoke' in sys.argv:
    until(lambda:p.has('[data-testid=qa-answer]'),'Conversation reloads after restart')
    check('Q&A model independently persisted',p.eval("document.querySelector('[data-testid=qa-model]').selectedOptions[0].textContent").find('Chat B')>=0)
    if '--installed-smoke' in sys.argv:
        completed=sql("SELECT COUNT(*) FROM qa_turns WHERE status='completed'")[0][0]
        change('[data-testid=qa-question]','安装版检查：继续梳理当前工作')
        p.click('qa-send');p.click('nav-kol');p.wait(.2)
        until(lambda:sql("SELECT COUNT(*) FROM qa_turns WHERE status='completed'")[0][0]>completed,'Actual installed app answers in background using the saved model')
        p.click('nav-qa');p.wait(.6)
else:
    check('Synthetic profile starts empty',not call('list_qa_sessions'))
    # Credentials and network are synthetic; app uses real route/database/jobs code.
    provider=call('save_provider_connection',id=None,displayName='Local chat fixture',providerType='custom',baseUrl='http://127.0.0.1:9481/v1',legacyModel='',templateKind='custom',authMode='none',modelsEndpoint=None,enabled=True,apiKey=None)
    models=[]
    for name,label in [('qa-good','Chat A'),('qa-good-b','Chat B')]:
        models.append(call('save_provider_model',providerId=provider['id'],modelId=name,displayName=label,protocol='chat_completions',endpointPath='/chat/completions',capabilitiesJson='{}',source='manual',enabled=True,available=True))
    call('save_ai_task_route',taskKind='general',providerModelId=models[0]['id'])
    call('save_ai_task_route',taskKind='kol_analysis',providerModelId=models[0]['id'])
    call('save_ai_task_route',taskKind='workbench_qa',providerModelId=models[0]['id'])
    p.click('nav-kol');p.wait(.2);p.click('nav-qa');p.wait(.6)
    p.screenshot('qa-chat-welcome-zh-1440.png')
    change('[data-testid=qa-model]',str(models[1]['id']))
    until(lambda:next(r for r in call('list_ai_task_routes') if r['task_kind']=='workbench_qa')['provider_model_id']==models[1]['id'],'Model selection writes the independent Q&A route')
    check('Other task routes remain unchanged',all(r['provider_model_id']==models[0]['id'] for r in call('list_ai_task_routes') if r['task_kind'] in ('general','kol_analysis')))
    change('[data-testid=qa-question]','请总结当前项目的推进情况')
    before=len(call('list_qa_sessions'))
    p.eval("document.querySelector('[data-testid=qa-question]').dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',isComposing:true,bubbles:true}))")
    p.wait(.2);check('IME Enter never submits',len(call('list_qa_sessions'))==before)
    p.eval("document.querySelector('[data-testid=qa-question]').dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true,cancelable:true}))")
    until(lambda:len(call('list_qa_sessions'))==before+1,'Enter submits a question')
    p.click('nav-kol');p.wait(.2)
    until(lambda:sql("SELECT COUNT(*) FROM qa_turns WHERE status='completed'")[0][0]==1,'Answer completes while another page is open')
    p.click('nav-qa');p.wait(.6)
    change('[data-testid=qa-question]','这些信息还缺少什么？');p.click('qa-send')
    until(lambda:sql("SELECT COUNT(*) FROM qa_turns WHERE status='completed'")[0][0]==2,'Continuous follow-up completes')
    p.wait(2.4)
    check('Transcript presents questions in chronological order',p.eval("[...document.querySelectorAll('[data-testid^=qa-turn-]')].map(e=>e.dataset.testid)")==['qa-turn-1','qa-turn-2'])
    with urllib.request.urlopen('http://127.0.0.1:9481/metrics') as r:metrics=json.load(r)
    check('Chosen model reaches provider and receives conversation history',metrics['requests'].get('qa-good-b',0)>=2 and metrics['history_received'])
    change('[data-testid=qa-question]','保留这段未发出的追问')
    p.click('qa-new');p.wait(.2)
    check('New chat has an independent empty draft',p.eval("document.querySelector('[data-testid=qa-question]').value")=='')
    p.click('qa-session-1');p.wait(.4)
    check('Switching conversations restores the unsent draft',p.eval("document.querySelector('[data-testid=qa-question]').value")=='保留这段未发出的追问')
    p.click('nav-kol');p.wait(.2);p.click('nav-qa');p.wait(.4)
    check('Page navigation preserves unsent draft',p.eval("document.querySelector('[data-testid=qa-question]').value")=='保留这段未发出的追问')
    change('[data-testid=qa-question]','')
# Stub only the OS clipboard boundary; exercise the real answer-formatting handler.
# The user's clipboard is never read or changed.
p.eval("window.__qaClipboardDescriptor=Object.getOwnPropertyDescriptor(navigator,'clipboard');Object.defineProperty(navigator,'clipboard',{configurable:true,value:{writeText:async text=>{window.__qaCopied=text}}});document.querySelector('.qa-answer-actions button').click()")
until(lambda:p.eval("typeof window.__qaCopied==='string'"),'Copy handler produces text without touching the OS clipboard')
check('Copy preserves answer, inference label and uncertainty',p.eval("window.__qaCopied.includes('当前记录中')&&(window.__qaCopied.includes('推断：')||window.__qaCopied.includes('Inference:'))&&window.__qaCopied.includes('现有资料不能确认事项已经完成')"))
p.eval("if(window.__qaClipboardDescriptor)Object.defineProperty(navigator,'clipboard',window.__qaClipboardDescriptor);else delete navigator.clipboard;delete window.__qaCopied;delete window.__qaClipboardDescriptor")
for width,height in [(1440,1000),(1100,720),(900,650)]:
    p.command('Emulation.setDeviceMetricsOverride',{'width':width,'height':height,'deviceScaleFactor':1,'mobile':False})
    p.eval("document.documentElement.dataset.fontSize='xlarge'")
    for language in ('zh','en'):
        if p.eval("document.querySelector('.locale-button').textContent.trim()")!=('中' if language=='zh' else 'EN'):p.eval("document.querySelector('.locale-button').click()");p.wait(.2)
        check(f'{language} {width} no horizontal clipping',p.eval("(()=>{const e=document.querySelector('.content-scroll');return e.scrollWidth<=e.clientWidth+2&&document.documentElement.scrollWidth<=innerWidth+2})()"))
        check(f'{language} {width} composer stays visible without page scrolling',p.eval("(()=>{const e=document.querySelector('[data-testid=qa-send]'),r=e.getBoundingClientRect(),s=document.querySelector('.content-scroll').getBoundingClientRect();return r.top>=s.top&&r.bottom<=s.bottom+1&&e.contains(document.elementFromPoint(r.x+r.width/2,r.y+r.height/2))})()"))
        check(f'{language} {width} transcript cannot overlap composer',p.eval("document.querySelector('[data-testid=qa-transcript]').getBoundingClientRect().bottom<=document.querySelector('[data-testid=qa-composer]').getBoundingClientRect().top+1"))
        check(f'{language} {width} reading area keeps useful height',p.eval("document.querySelector('[data-testid=qa-transcript]').getBoundingClientRect().height")>=190)
        p.eval("document.querySelector('[data-testid=qa-transcript]').scrollTop=document.querySelector('[data-testid=qa-transcript]').scrollHeight")
        check(f'{language} {width} end of latest answer is reachable',p.eval("(()=>{const t=document.querySelector('[data-testid=qa-transcript]'),e=[...t.querySelectorAll('.qa-answer-actions')].at(-1),r=e.getBoundingClientRect(),s=t.getBoundingClientRect();return r.bottom<=s.bottom+1&&r.top>=s.top})()"))
        if p.eval("getComputedStyle(document.querySelector('.qa-history-toggle')).display!=='none'"):
            p.eval("document.querySelector('.qa-history-toggle').click()")
            check(f'{language} {width} history drawer has a reachable close control',p.eval("(()=>{const e=document.querySelector('.qa-history-close'),r=e.getBoundingClientRect();return r.width>0&&e.contains(document.elementFromPoint(r.x+r.width/2,r.y+r.height/2))})()"))
            p.eval("document.querySelector('.qa-history-close').click()")
            check(f'{language} {width} history drawer closes',p.eval("getComputedStyle(document.querySelector('.qa-history')).display==='none'"))
        p.screenshot(f'qa-chat-{language}-{width}.png')
check('Sources and uncertainty remain accessible',p.has('.sources') and p.has('[data-testid=qa-gaps]'))
check('SQLite references and integrity remain valid',sql('PRAGMA integrity_check')==[('ok',)] and not sql('PRAGMA foreign_key_check'))
check('No UI runtime errors',not p.errors)
p.ws.close()
