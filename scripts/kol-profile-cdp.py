"""Synthetic native regression: workbench navigation and independent expert department."""
import importlib.util
import json
import os
from pathlib import Path
import sys

ROOT=Path(__file__).resolve().parent.parent
PROFILE=ROOT/'.test-runtime'/os.environ.get('MSL_FLOW_PROFILE','kol-profile')
assert PROFILE.resolve().parent==(ROOT/'.test-runtime').resolve()
for key,folder in [('APPDATA','appdata'),('LOCALAPPDATA','localappdata'),('TEMP','temp'),('TMP','temp')]:
    assert Path(os.environ[key]).resolve()==(PROFILE/folder).resolve(), 'Isolated profile required'
spec=importlib.util.spec_from_file_location('native_cdp',ROOT/'scripts'/'release-functional-cdp.py')
cdp=importlib.util.module_from_spec(spec);spec.loader.exec_module(cdp)
p=cdp.Page()
def check(label,ok):
    print(('PASS ' if ok else 'FAIL ')+label,flush=True)
    assert ok,label
def call(name,**args):return p.eval(f'window.__TAURI_INTERNALS__.invoke({json.dumps(name)},{json.dumps(args)})')
def change(selector,value):
    p.eval(f"(()=>{{const e=document.querySelector({json.dumps(selector)});e.value={json.dumps(value)};e.dispatchEvent(new Event('input',{{bubbles:true}}));}})()");p.wait(.15)
for _ in range(30):
    if p.has('[data-testid=nav-kol]'):break
    p.wait(.2)
check('Experts are in Workbench above Tools',p.has('[data-testid=primary-navigation] [data-testid=nav-kol]'))
check('Q&A remains in Tools',p.has('[data-testid=tools-navigation] [data-testid=nav-qa]'))
if '--baseline' in sys.argv:p.ws.close();sys.exit(0)
check('Release runtime version',call('plugin:app|version')=='0.3.1')
p.click('nav-kol');p.wait(.4)
if '--reopen' in sys.argv:
    saved=next(e for e in call('list_kol_experts') if e['name']=='合成字段专家')
    check('Institution and department persist independently after restart',saved['institution']=='合成教学医院' and saved['department']=='肾内科')
    p.click(f"kol-expert-{saved['id']}");p.wait(.4);p.click('kol-edit-profile');p.wait(.2)
    check('Reopened form restores department',p.eval("document.querySelector('[data-testid=kol-department]').value")=='肾内科')
else:
    check('Fresh synthetic profile',not call('list_kol_experts'))
    p.click('kol-new');p.wait(.2)
    change('[data-testid=kol-name]','合成字段专家')
    change('[data-testid=kol-institution]','合成教学医院')
    change('[data-testid=kol-department]','肾内科')
    p.click('kol-save-expert');p.wait(.5)
    saved=call('list_kol_experts')[0]
    check('Save keeps independent institution and department',saved['institution']=='合成教学医院' and saved['department']=='肾内科')
    change('[data-testid=kol-search]','肾内科')
    check('Searching department finds expert',p.has(f'[data-testid=kol-expert-{saved["id"]}]'))
    change('[data-testid=kol-search]','不存在的科室')
    check('Department search excludes unrelated expert',not p.has(f'[data-testid=kol-expert-{saved["id"]}]'))
    change('[data-testid=kol-search]','')
    p.click('kol-edit-profile');p.wait(.2)
    change('[data-testid=kol-department]','');p.click('kol-save-expert');p.wait(.4)
    check('Department is optional and can be explicitly cleared',call('list_kol_experts')[0]['department']=='')
    p.click('kol-edit-profile');p.wait(.2);change('[data-testid=kol-department]','肾内科');p.click('kol-save-expert');p.wait(.4)
    p.click('kol-edit-profile');p.wait(.2)
for width,height in [(1440,1000),(1100,720)]:
    p.command('Emulation.setDeviceMetricsOverride',{'width':width,'height':height,'deviceScaleFactor':1,'mobile':False})
    p.eval("document.documentElement.dataset.fontSize='xlarge'")
    for language in ('zh','en'):
        if p.eval("document.querySelector('.locale-button').textContent.trim()")!=('中' if language=='zh' else 'EN'):p.eval("document.querySelector('.locale-button').click()");p.wait(.2)
        check(f'{language} {width} independent field labels',p.eval("document.querySelector('[data-testid=kol-institution]').closest('label').innerText") == ('机构' if language=='zh' else 'Institution') and p.eval("document.querySelector('[data-testid=kol-department]').closest('label').innerText")==('科室（可稍后补充）' if language=='zh' else 'Department (optional)'))
        check(f'{language} {width} no horizontal clipping',p.eval("(()=>{const e=document.querySelector('.content-scroll');return e.scrollWidth<=e.clientWidth+2&&document.documentElement.scrollWidth<=innerWidth+2})()"))
        p.eval("document.querySelector('[data-testid=kol-save-expert]').scrollIntoView({block:'center',behavior:'instant'})")
        p.wait(.2)
        check(f'{language} {width} save button fully reachable and clickable',p.eval("(()=>{const e=document.querySelector('[data-testid=kol-save-expert]'),r=e.getBoundingClientRect(),s=document.querySelector('.content-scroll').getBoundingClientRect(),hit=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return r.top>=s.top&&r.bottom<=s.bottom&&e.contains(hit)})()"))
        p.eval("document.querySelector('.content-scroll').scrollTop=0");p.wait(.2);p.screenshot(f'kol-profile-{language}-{width}.png')
check('No UI runtime exceptions',not p.errors)
p.ws.close()
