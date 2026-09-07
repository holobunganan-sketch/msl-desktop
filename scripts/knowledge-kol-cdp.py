"""Native installed/release feature regression, isolated synthetic records only."""
import importlib.util
import json
import os
from pathlib import Path
import sys
import sqlite3,time,hashlib,urllib.request

ROOT=Path(__file__).resolve().parent.parent
PROFILE=ROOT/'.test-runtime'/os.environ.get('MSL_FLOW_PROFILE','knowledge-ui')
assert PROFILE.resolve().parent==(ROOT/'.test-runtime').resolve()
for key,folder in [('APPDATA','appdata'),('LOCALAPPDATA','localappdata'),('TEMP','temp'),('TMP','temp')]:
    assert Path(os.environ[key]).resolve()==(PROFILE/folder).resolve(),'Isolated profile required'
spec=importlib.util.spec_from_file_location('cdp',ROOT/'scripts'/'release-functional-cdp.py')
cdp=importlib.util.module_from_spec(spec);spec.loader.exec_module(cdp)
class AuditedPage(cdp.Page):
    def command(self,method,params=None):
        self.seq+=1;ident=self.seq
        self.ws.send(json.dumps({'id':ident,'method':method,'params':params or {}}))
        while True:
            message=json.loads(self.ws.recv())
            if message.get('method')=='Runtime.exceptionThrown':
                detail=message.get('params',{}).get('exceptionDetails',{})
                description=detail.get('exception',{}).get('description',detail.get('text','exception'))
                self.errors.append(description);print('UI_EXCEPTION '+description[:1800],flush=True)
            if message.get('method')=='Runtime.consoleAPICalled' and message.get('params',{}).get('type')=='error':
                description=str(message['params'].get('args',[]))[:1800]
                self.errors.append(description);print('UI_CONSOLE_ERROR '+description,flush=True)
            if message.get('id')==ident:return message
p=AuditedPage()
for _ in range(30):
    if p.has('[data-testid=nav-qa]'):break
    p.wait(.2)
def check(name,value):
    print(('PASS ' if value else 'FAIL ')+name,flush=True)
    assert value,name
check('Global workbench Q&A has its own navigation',p.has('[data-testid=nav-qa]'))
check('Experts and insights have independent navigation',p.has('[data-testid=nav-kol]'))
DB=PROFILE/'appdata'/'MSLDesktop'/'msl-desktop.db'
def call(command,**args):return p.eval(f'window.__TAURI_INTERNALS__.invoke({json.dumps(command)},{json.dumps(args)})')

if os.environ.get('MSL_EXPECTED_VERSION'):
    check('Actual application runtime version matches deployed release',call('plugin:app|version')==os.environ['MSL_EXPECTED_VERSION'])
def sql(query,args=()):
    with sqlite3.connect(DB) as conn:return conn.execute(query,args).fetchall()
def change(selector,value):
    p.eval(f'''(()=>{{const e=document.querySelector({json.dumps(selector)});if(!e)throw Error('Missing control');e.value={json.dumps(value)};e.dispatchEvent(new Event('input',{{bubbles:true}}));e.dispatchEvent(new Event('change',{{bubbles:true}}));}})()''');p.wait(.2)
def route(view,ident=None,work=None):
    p.eval(f"window.dispatchEvent(new CustomEvent('dashboard:navigate',{{detail:{json.dumps(dict(view=view,id=ident,workId=work))}}}))");p.wait(.5)
def wait_for(fn,label):
    for _ in range(100):
        if fn():check(label,True);return
        p.wait(.2)
    check(label,False)
def wait_job(job):
    wait_for(lambda:call('get_ai_job',id=job['id'])['status']!='running','Background job finished')
    return call('get_ai_job',id=job['id'])
def start(command,**args):return call('start_ai_job',request=dict(command=command,args=args))
def layout(label):
    check(label+' no horizontal clipping',p.eval("(()=>{const s=document.querySelector('.content-scroll');return s.scrollWidth<=s.clientWidth+2&&document.documentElement.scrollWidth<=innerWidth+2})()"))
    check(label+' actions remain reachable',p.eval("(()=>{const s=document.querySelector('.content-scroll');s.scrollTop=s.scrollHeight;const r=s.getBoundingClientRect();const e=[...s.querySelectorAll('button,input,textarea,select')].filter(e=>e.checkVisibility({visibilityProperty:true})&&!e.closest('details:not([open])')).at(-1);if(!e)return true;e.scrollIntoView({block:'nearest'});const t=e.getBoundingClientRect();return t.bottom<=r.bottom+2&&t.right<=r.right+2&&t.left>=r.left-2})()"))
def screenshot(name):p.eval("document.querySelector('.content-scroll').scrollTop=0");p.wait(.2);p.screenshot(name)

if '--reopen' in sys.argv:
    check('Completed conversation and scoped followups survived restart',sql("SELECT COUNT(*) FROM qa_turns WHERE status='completed'")[0][0]>=3 and sql('SELECT COUNT(*) FROM kol_actions')[0][0]==1)
    check('Interrupted questions recovered visibly',sql("SELECT COUNT(*) FROM qa_turns WHERE status='interrupted'")[0][0]>=1)
    route('qa');check('Restored conversation rendered',p.has('[data-testid=qa-page]'))
    check('SQLite integrity and references preserved',sql('PRAGMA integrity_check')==[('ok',)] and not sql('PRAGMA foreign_key_check'))
    p.ws.close();sys.exit(0)

if '--installed-smoke' in sys.argv:
    session=call('list_qa_sessions')[0]
    route('qa',session['id'])
    before=sql("SELECT COUNT(*) FROM qa_turns WHERE status='completed'")[0][0]
    change('[data-testid=qa-question]','安装验证：请核对当前范围中的进展与依据')
    p.click('qa-send');p.wait(.2);route('kol')
    wait_for(lambda:sql("SELECT COUNT(*) FROM qa_turns WHERE status='completed'")[0][0]>before,'Installed application completes Mock Q&A after page change')
    if p.eval("document.querySelector('.locale-button').textContent.trim()")!='中':p.eval("document.querySelector('.locale-button').click()");p.wait(.3)
    p.command('Emulation.setDeviceMetricsOverride',{'width':1440,'height':1000,'deviceScaleFactor':1,'mobile':False})
    route('qa',session['id'])
    check('Installed answer exposes cited evidence',p.has('[data-testid=qa-answer]') and p.has('.sources'))
    p.eval("document.querySelector('[data-testid=qa-answer]').closest('article').scrollIntoView({block:'start'})")
    p.wait(.3);p.screenshot('installed-qa-evidence-zh-1440.png')
    experts=call('list_kol_experts');original=next(e for e in experts if '第二家' not in e['institution'])
    route('kol',original['id']);p.click('kol-tab-insights');p.wait(.3)
    screenshot('installed-kol-insights-zh-1440.png')
    p.command('Emulation.setDeviceMetricsOverride',{'width':1100,'height':720,'deviceScaleFactor':1,'mobile':False})
    p.click('kol-tab-drafts');p.wait(.3);p.click('kol-confirm');p.wait(.3)
    layout('Installed editable confirmation 1100')
    p.screenshot('installed-kol-confirm-zh-1100.png')
    check('Installed interaction has no runtime errors',not p.errors)
    p.ws.close();sys.exit(0)

if '--supplement' in sys.argv:
    projects=call('list_works',status=None);a=next(x for x in projects if ' A ' in x['title']);b=next(x for x in projects if ' B ' in x['title'])
    session=call('create_qa_session',title='合成：范围切换',scope=[])
    for scope in [[],[b['id']],[a['id'],b['id']]]:
        turn=call('queue_qa_question',sessionId=session['id'],question='比较当前范围内的工作',scope=scope)
        job=wait_job(start('ask_workbench',turnId=turn['id'],locale='zh-CN'));check('Scope variant answered',job['status']=='completed')
        pack=json.loads(job['result']['evidence_json']);actual={json.loads(s['text'])['work_id'] for s in pack['sources'] if s['kind']=='task'}
        check('Global or selected projects are enforced before generation',actual==set(scope or [a['id'],b['id']]))
    prior=len(call('list_qa_turns',sessionId=session['id']))
    result=p.eval(f"window.__TAURI_INTERNALS__.invoke('queue_qa_question',{{sessionId:{session['id']},question:'无效范围',scope:[999999]}}).then(()=>false).catch(()=>true)")
    check('Deleted or unknown scope never falls back to global',result and len(call('list_qa_turns',sessionId=session['id']))==prior)
    original=call('list_kol_experts')[0]
    another=call('save_kol_expert',id=None,revision=None,name=original['name'],institution='第二家合成医院',specialty='',projects=[b['id']],archived=False)
    check('Same-name experts remain separate identities',original['id']!=another['id'])
    call('capture_kol_note',expertId=another['id'],workId=b['id'],inboxId=None,content='希望了解随访证据的适用人群，与其他专家的意见需分别核对。',occurredAt=int(time.time()))
    job=wait_job(start('analyze_kol',expertId=None,purpose='synthesize',locale='zh-CN'));check('Two-expert synthesis succeeds',job['status']=='completed')
    draft=next(d for d in call('list_kol_drafts',expertId=None) if d['id']==job['result']['draft_id'])
    pack=json.loads(draft['evidence_json']);check('Synthesis counts independent experts and original notes',pack['counts']['expert']==2 and pack['counts']['kol_note']==2)
    check('Pending followup editor can be restored',wait_job(start('analyze_kol',expertId=original['id'],purpose='organize',locale='zh-CN'))['status']=='completed')
    for width,height in [(1440,1000),(1100,720)]:
        p.command('Emulation.setDeviceMetricsOverride',{'width':width,'height':height,'deviceScaleFactor':1,'mobile':False})
        p.eval("document.documentElement.dataset.fontSize='xlarge'")
        for language in ('zh','en'):
            want='中' if language=='zh' else 'EN'
            if p.eval("document.querySelector('.locale-button').textContent.trim()")!=want:p.eval("document.querySelector('.locale-button').click()");p.wait(.3)
            route('kol',original['id']);p.click('kol-tab-drafts');p.wait(.3)
            check('Pending actions visible',p.has('[data-testid=kol-action-title-0]'))
            p.click('kol-confirm');p.wait(.2);layout(f'Editable KOL actions {language} {width}')
            p.screenshot(f'knowledge-kol-confirm-{language}-{width}.png')
            screenshot(f'knowledge-kol-edit-{language}-{width}.png')
    check('No runtime exceptions in supplementary flows',not p.errors)
    p.ws.close();sys.exit(0)

if '--layout' not in sys.argv:
    if '--resume' in sys.argv:
        existing=call('list_works',status=None);a=next(w for w in existing if w['title']=='合成项目 A · 证据交流');b=next(w for w in existing if w['title']=='合成项目 B · 独立研究')
        models={m['model_id']:m['id'] for m in call('list_provider_models',providerId=None)}
        source_hash=hashlib.sha256((PROFILE/'workspace'/'synthetic-evidence.txt').read_bytes()).hexdigest()
    else:
        check('Synthetic test begins with no projects',not call('list_works',status=None))
        schedule=call('get_analysis_schedule');schedule.update(enabled=False,daily_enabled=False);call('save_analysis_schedule',schedule=schedule)
        a=call('create_work',title='合成项目 A · 证据交流');b=call('create_work',title='合成项目 B · 独立研究')
        call('create_task',workId=a['id'],title='合成：准备随访证据',priority='normal',dueAt=None,notes='待核对')
        call('create_task',workId=b['id'],title='Beta only synthetic',priority='normal',dueAt=None,notes=None)
        folder=call('bind_workspace',name='Synthetic evidence',path=str(PROFILE/'workspace'))
        call('link_work_workspace',workId=a['id'],workspaceId=folder['id'],isPrimary=True)
        call('reindex_workspace_documents',workspaceId=folder['id'])
        source_hash=hashlib.sha256((PROFILE/'workspace'/'synthetic-evidence.txt').read_bytes()).hexdigest()
        provider=call('save_provider_connection',id=None,displayName='Synthetic knowledge provider',providerType='custom',baseUrl='http://127.0.0.1:9481/v1',legacyModel='',templateKind='custom',authMode='none',modelsEndpoint=None,enabled=True,apiKey=None)
        models={}
        for name in ('qa-good','qa-repair','qa-forged','kol-good'):
            models[name]=call('save_provider_model',providerId=provider['id'],modelId=name,displayName=name,protocol='chat_completions',endpointPath='/chat/completions',capabilitiesJson='{}',source='manual',enabled=True,available=True)['id']
        for kind,name in [('workbench_qa','qa-good'),('kol_analysis','kol-good')]:call('save_ai_task_route',taskKind=kind,providerModelId=models[name])
    p.command('Emulation.setDeviceMetricsOverride',{'width':1440,'height':1000,'deviceScaleFactor':1,'mobile':False})
    route('qa');change('[data-testid=qa-question]','@合成项目 A')
    p.eval("document.querySelector('[data-testid=qa-question]').setSelectionRange(8,8);document.querySelector('[data-testid=qa-question]').dispatchEvent(new KeyboardEvent('keyup',{bubbles:true}))")
    wait_for(lambda:p.has(f'[data-testid=qa-project-{a["id"]}]'),'@ project selector offers real IDs')
    p.click(f'qa-project-{a["id"]}');change('[data-testid=qa-question]','当前项目推进到哪里？');p.click('qa-send');p.wait(.3);route('works',a['id'])
    wait_for(lambda:bool(sql("SELECT id FROM qa_turns WHERE status='completed'")),'Question finishes after leaving its page')
    session=call('list_qa_sessions')[0];route('qa',session['id']);check('Answer has sources and uncertainty',p.has('[data-testid=qa-answer]') and p.has('.sources') and p.has('[data-testid=qa-gaps]'))
    change('[data-testid=qa-question]','这件事情下一步应该注意什么？');p.click('qa-send')
    wait_for(lambda:sql("SELECT COUNT(*) FROM qa_turns WHERE status='completed'")[0][0]==2,'Continuous followup persisted')
    check('Followup inherits project scope',all(json.loads(t['scope_json'])==[a['id']] for t in call('list_qa_turns',sessionId=session['id'])))
    call('save_ai_task_route',taskKind='workbench_qa',providerModelId=models['qa-repair'])
    turn=call('queue_qa_question',sessionId=session['id'],question='校验格式恢复',scope=[a['id']]);job=wait_job(start('ask_workbench',turnId=turn['id'],locale='zh-CN'));check('Invalid JSON repaired once',job['status']=='completed')
    call('save_ai_task_route',taskKind='workbench_qa',providerModelId=models['qa-forged'])
    turn=call('queue_qa_question',sessionId=session['id'],question='校验伪造引用',scope=[a['id']]);job=wait_job(start('ask_workbench',turnId=turn['id'],locale='zh-CN'));check('Forged citation rejected instead of shown as success',job['status']=='failed')
    call('save_ai_task_route',taskKind='workbench_qa',providerModelId=models['qa-good']);check('Failed question can be retried',wait_job(start('ask_workbench',turnId=turn['id'],locale='zh-CN'))['status']=='completed')
    route('kol');p.click('kol-new');change('[data-testid=kol-name]','合成专家');change('[data-testid=kol-institution]','合成机构 · 肾内科');p.click('kol-save-expert')
    wait_for(lambda:bool(call('list_kol_experts')),'Expert saved with minimal fields')
    expert=call('list_kol_experts')[0];change('[data-testid=kol-note]','希望了解长期随访证据，也想讨论临床实践障碍和研究合作机会。');p.click('kol-save-analyze');p.wait(.3);route('today')
    wait_for(lambda:bool(call('list_kol_drafts',expertId=expert['id'])),'Expert draft completes in background')
    check('AI draft creates no formal followup',sql('SELECT COUNT(*) FROM kol_actions')==[(0,)])
    route('kol',expert['id']);p.click('kol-tab-drafts');p.wait(.5)
    check('Editable plain-language draft shown',p.has('[data-testid=kol-draft-editor]') and p.has('[data-testid=kol-draft-summary]'))
    change('[data-testid=kol-draft-summary]','用户核对：明确长期随访证据需求，再确认适用的临床情境。');p.wait(3)
    check('Background polling cannot overwrite unsaved review edits',p.eval("document.querySelector('[data-testid=kol-draft-summary]').value.startsWith('用户核对')"))
    p.click('kol-action-enabled-1');p.click('kol-confirm');p.wait(.2);p.click('kol-commit')
    wait_for(lambda:sql('SELECT COUNT(*) FROM kol_actions')==[(1,)],'Only selected action is committed')
    draft=call('list_kol_drafts',expertId=expert['id'])[0];call('review_kol_draft',id=draft['id'],revision=1,decision='confirm',payload=json.loads(draft['payload_json']))
    check('Repeated confirmation creates no duplicates',sql('SELECT COUNT(*) FROM kol_actions')==[(1,)])
    check('Followup is original scheduled task, no copied calendar event',sql('SELECT COUNT(*) FROM tasks WHERE scheduled_start IS NOT NULL')==[(1,)] and sql('SELECT COUNT(*) FROM calendar_events')==[(0,)])
    check('Three insight categories preserved',len(json.loads(sql('SELECT categories_json FROM kol_insights')[0][0]))==3)
    p.click('kol-tab-followups');p.wait(.3);check('Expert followup reads live workbench task',p.body_has('合成：补充专家所需的随访证据'))
    job=wait_job(start('analyze_kol',expertId=expert['id'],purpose='prepare',locale='zh-CN'));check('Preparation supported without committing actions',job['status']=='completed' and not json.loads(call('list_kol_drafts',expertId=expert['id'])[0]['payload_json'])['actions'])
    job=wait_job(start('analyze_kol',expertId=None,purpose='synthesize',locale='zh-CN'));check('Cross-expert synthesis has a durable draft',job['status']=='completed')
    metrics=json.load(urllib.request.urlopen('http://127.0.0.1:9481/metrics'))
    check('Provider sees same-scope history and linked document evidence only',metrics['scope_isolation'] and metrics['history_received'] and metrics['document_received'])
    check('Repair is bounded at two attempts',metrics['requests']['qa-forged']==2 and metrics['requests']['qa-repair']==2)
    counts=sql('SELECT (SELECT COUNT(*) FROM qa_turns),(SELECT COUNT(*) FROM kol_notes),(SELECT COUNT(*) FROM kol_insights),(SELECT COUNT(*) FROM kol_drafts),(SELECT COUNT(*) FROM tasks)')
    cleanup=call('preview_storage_cleanup',categories=None);call('execute_storage_cleanup',planId=cleanup['plan_id'])
    check('Cache cleanup preserves all new formal records',counts==sql('SELECT (SELECT COUNT(*) FROM qa_turns),(SELECT COUNT(*) FROM kol_notes),(SELECT COUNT(*) FROM kol_insights),(SELECT COUNT(*) FROM kol_drafts),(SELECT COUNT(*) FROM tasks)'))
    check('Source document is unchanged after AI and cleanup',source_hash==hashlib.sha256((PROFILE/'workspace'/'synthetic-evidence.txt').read_bytes()).hexdigest())
    call('queue_qa_question',sessionId=session['id'],question='合成：重启恢复检查',scope=[a['id']])

for width,height in [(1440,1000),(1100,720)]:
    p.command('Emulation.setDeviceMetricsOverride',{'width':width,'height':height,'deviceScaleFactor':1,'mobile':False})
    p.eval("document.documentElement.dataset.fontSize='xlarge'")
    for language in ('zh','en'):
        want='中' if language=='zh' else 'EN'
        if p.eval("document.querySelector('.locale-button').textContent.trim()")!=want:p.eval("document.querySelector('.locale-button').click()");p.wait(.4)
        route('qa',call('list_qa_sessions')[0]['id']);layout(f'QA {language} {width}');screenshot(f'knowledge-qa-{language}-{width}.png')
        expert=call('list_kol_experts')[0];route('kol',expert['id'])
        for tab in ('record','drafts','insights','followups'):
            p.click('kol-tab-'+tab);p.wait(.3);layout(f'KOL {tab} {language} {width}');screenshot(f'knowledge-kol-{tab}-{language}-{width}.png')
check('SQLite integrity and references intact',sql('PRAGMA integrity_check')==[('ok',)] and not sql('PRAGMA foreign_key_check'))
check('No UI runtime exceptions',not p.errors)
p.ws.close()
