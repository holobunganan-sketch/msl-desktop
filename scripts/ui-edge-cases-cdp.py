"""UI edge cases through the real isolated WebView; never calls an AI provider."""
import json
import os
import pathlib
import runpy
import sqlite3
import sys

helpers=runpy.run_path(str(pathlib.Path(__file__).with_name('ui-visibility-cdp.py')))
Page,REACH=helpers['Page'],helpers['REACH']
p=Page()
checks=[]
with sqlite3.connect(helpers['db_path']) as db:
    # Other regression suites may already have confirmed synthetic proposals.
    empty_status=next((status for status in ('rejected','superseded','confirmed') if db.execute('SELECT COUNT(*) FROM ai_proposals WHERE status=?',(status,)).fetchone()[0]==0),None)
assert empty_status, 'Fixture needs an empty review status'
def check(label,ok,evidence=None):
    checks.append({'name':label,'pass':bool(ok),'evidence':evidence})
    print(('PASS ' if ok else 'FAIL ')+label+((' '+json.dumps(evidence)) if not ok else ''),flush=True)
def reach(selector):return p.eval(f'({REACH})({json.dumps(selector)})')
def locale(value):
    if p.eval('document.documentElement.lang')!=value:
        p.eval("document.querySelector('.locale-button').click()");p.wait(.4)
def reset_scroll():p.eval("document.querySelectorAll('*').forEach(e=>{if(e.scrollHeight>e.clientHeight)e.scrollTop=0})")

for lang in ('en-US','zh-CN'):
    locale(lang)
    p.command('Emulation.setDeviceMetricsOverride',{'width':960,'height':600,'deviceScaleFactor':1.5,'mobile':False})
    p.eval("document.documentElement.dataset.fontSize='xlarge'")
    for nav in ('today','works','plan','waiting','calendar','inbox','review','reports','translation','workspace','settings'):
        p.click('nav-'+nav);p.wait(.25)
        result=reach('.view-body button,.view-body input,.view-body select,.view-body textarea')
        check(f'{lang}/{nav}/reachable',result['totalFailures']==0,result)
        width=p.eval("(()=>{let e=document.querySelector('.content-scroll');return e.scrollWidth<=e.clientWidth+2})()")
        check(f'{lang}/{nav}/no-horizontal-scroll',width)
    for nav,button in (('plan','task-create'),('waiting','waiting-create'),('works','work-create'),('calendar','calendar-create')):
        p.click('nav-'+nav);p.wait(.2);p.click(button);p.wait(.2)
        result=reach('.modal button,.modal input,.modal select,.modal textarea')
        check(f'{lang}/{nav}/modal',result['count']>0 and result['totalFailures']==0,result)
        reset_scroll();p.screenshot(f'edge-{lang}-{nav}-modal.png')
        p.eval("document.querySelector('.modal .close')?.click()")
    p.click('nav-review');p.wait(.3)
    p.eval("(()=>{let e=document.querySelector('.review-filters select');e.value="+json.dumps(empty_status)+";e.dispatchEvent(new Event('change',{bubbles:true}))})()");p.wait(.3)
    result=reach('[data-testid=review-empty-state]')
    check(f'{lang}/review-empty',result['count']==1 and not result['totalFailures'],result)
    p.click('background-jobs-toggle');p.wait(.2)
    result=reach('.job-popover header button,.job-popover .job')
    check(f'{lang}/jobs-popover',result['count']>0 and not result['totalFailures'],result)
    p.screenshot(f'edge-{lang}-background.png')
    p.eval("document.querySelector('.job-popover header button')?.click()")
    p.eval("document.querySelector('.search-button').click()");p.wait(.2)
    result=reach('.overlay input')
    check(f'{lang}/search',result['count']==1 and not result['totalFailures'],result)
    p.command('Input.dispatchKeyEvent',{'type':'keyDown','key':'Escape','code':'Escape','windowsVirtualKeyCode':27})
    p.command('Input.dispatchKeyEvent',{'type':'keyUp','key':'Escape','code':'Escape','windowsVirtualKeyCode':27})

# Simultaneous AI error notifications must not cover a single application control.
with sqlite3.connect(helpers['db_path']) as db:
    db.execute('UPDATE provider_settings SET enabled=0')
p.eval("(async()=>{for(const taskKind of ['work_draft','general'])await window.__TAURI_INTERNALS__.invoke('save_ai_task_route',{taskKind,providerModelId:null})})()")
p.click('nav-works');p.wait(.3);p.click('organize-work-with-ai');p.wait(2)
count=p.eval("document.querySelectorAll('.toast').length")
check('AI failure produces a single notification',count==1,count)
geometry=p.eval("""(()=>{let e=document.querySelector('.toast'),c=document.querySelector('.content-scroll');return e&&{inFlow:!['fixed','absolute'].includes(getComputedStyle(e.parentElement).position),bottom:e.getBoundingClientRect().bottom,contentTop:c.getBoundingClientRect().top}})()""")
check('Notification never overlays content',geometry and geometry['inFlow'] and geometry['bottom']<=geometry['contentTop']+1,geometry)
result=reach('.works button')
check('All work controls reachable during an AI error',not result['totalFailures'],result)
reset_scroll();p.screenshot('edge-ai-failure-inline.png')
p.eval("document.querySelector('.toast button')?.click()")
p.click('nav-today');p.wait(.3)
for width,height in ((1280,720),(1440,900)):
    p.command('Emulation.setDeviceMetricsOverride',{'width':width,'height':height,'deviceScaleFactor':1,'mobile':False})
    reset_scroll();p.screenshot(f'edge-dashboard-top-{width}.png')
    p.eval("(()=>{const c=document.querySelector('.content-scroll');c.scrollTop=c.scrollHeight})()")
    p.screenshot(f'edge-dashboard-bottom-{width}.png')
    result=reach('.support-grid button,.support-grid input,.support-grid select')
    check(f'{width}/dashboard-support-controls',result['count']>0 and not result['totalFailures'],result)
out=pathlib.Path(os.environ['MSL_ARTIFACTS']);out.mkdir(parents=True,exist_ok=True)
(out/'edge-results.json').write_text(json.dumps(checks,ensure_ascii=False,indent=2),encoding='utf-8')
print(f"RESULT={sum(c['pass'] for c in checks)}/{len(checks)}")
sys.exit(0 if all(c['pass'] for c in checks) else 1)
