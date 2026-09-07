"""Isolated native directory lifecycle acceptance. Synthetic files only."""
import importlib.util
import json
import os
from pathlib import Path
import sys
import sqlite3

ROOT = Path(__file__).resolve().parent.parent
PROFILE = ROOT / '.test-runtime' / 'directory-ui'
for key, folder in [('APPDATA','appdata'),('LOCALAPPDATA','localappdata'),('TEMP','temp'),('TMP','temp')]:
    assert Path(os.environ[key]).resolve() == (PROFILE/folder).resolve()
spec = importlib.util.spec_from_file_location('cdp', ROOT/'scripts/release-functional-cdp.py')
cdp = importlib.util.module_from_spec(spec); spec.loader.exec_module(cdp)
p = cdp.Page()
def check(label, value):
    print(('PASS ' if value else 'FAIL ')+label, flush=True)
    assert value, label
def call(command_name, **args):
    return p.eval(f'window.__TAURI_INTERNALS__.invoke({json.dumps(command_name)},{json.dumps(args)})')
def route(view):
    p.eval(f"window.dispatchEvent(new CustomEvent('dashboard:navigate',{{detail:{{view:{json.dumps(view)}}}}}))"); p.wait(.6)
def refresh():
    route('today'); route('workspace')
def click(testid):
    check('Action exists: '+testid,p.click(testid)); p.wait(.4)
DB=PROFILE/'appdata/MSLDesktop/msl-desktop.db'
if '--reopen' in sys.argv:
    check('Removed directory list stays empty after restart',call('get_project_directories')==[])
    with sqlite3.connect(DB) as conn:
        check('Directory history retained and retired',conn.execute('SELECT COUNT(*) FROM workspaces WHERE enabled=0').fetchone()[0]>=2)
        check('Database integrity',conn.execute('PRAGMA integrity_check').fetchall()==[('ok',)])
        check('Foreign keys intact',conn.execute('PRAGMA foreign_key_check').fetchall()==[])
    check('Retired primary watcher not restored',call('watcher_status') is None)
    sys.exit(0)

route('workspace')
check('Project-bound directory view replaces independent root pills',p.has('[data-testid=project-directory-list]'))
if '--baseline' in sys.argv: sys.exit(0)
if '--layout' in sys.argv:
    sample=call('create_work',title='合成：长期项目与资料整理')
    x=call('attach_work_folder',workId=sample['id'],path=str(PROFILE/'workspace/A'))
    y=call('attach_work_folder',workId=sample['id'],path=str(PROFILE/'workspace/B'))
    refresh()
    for width,height in [(1440,1000),(1100,720)]:
        p.command('Emulation.setDeviceMetricsOverride',{'width':width,'height':height,'deviceScaleFactor':1,'mobile':False})
        p.eval("document.documentElement.dataset.fontSize='xlarge';document.querySelector('.content-scroll').scrollTop=0")
        check('Populated directory view has no clipping '+str(width),p.eval("document.documentElement.scrollWidth<=innerWidth+2 && document.querySelector('.content-scroll').scrollWidth<=document.querySelector('.content-scroll').clientWidth+2"))
        p.screenshot(f'directory-final-{width}.png')
        check('Final directory operation remains reachable '+str(width),p.eval("(()=>{const s=document.querySelector('.content-scroll');const b=[...s.querySelectorAll('button')].filter(e=>e.checkVisibility()&&!e.closest('details:not([open])')).at(-1);b.scrollIntoView({block:'end'});return b.getBoundingClientRect().bottom<=s.getBoundingClientRect().bottom+2})()"))
        p.screenshot(f'directory-final-bottom-{width}.png')
    call('delete_work',id=sample['id']); refresh()
    check('Layout fixture removal restores empty directory list',call('get_project_directories')==[])
    check('No layout runtime exceptions',not p.errors)
    sys.exit(0)
if '--edges' in sys.argv:
    a=call('create_work',title='合成：监控保护项目')
    b=call('create_work',title='合成：第二个目录项目')
    x=call('bind_workspace',name='合成主目录',path=str(PROFILE/'workspace/C'))
    call('link_work_workspace',workId=a['id'],workspaceId=x['id'],isPrimary=True)
    y=call('attach_work_folder',workId=b['id'],path=str(PROFILE/'workspace/D'))
    result=call('workspace_rescan',workspaceId=y['id'])
    check('Explicit scan does not use another directory watched by the primary watcher',call('workspace_sync_status',workspaceId=y['id'])['baseline_count']==2)
    check('Primary watcher remains on its own root',call('watcher_status')['root']==x['root_path'])
    call('archive_work',id=b['id']); refresh(); click(f"directory-remove-{y['id']}")
    call('link_work_workspace',workId=a['id'],workspaceId=y['id'],isPrimary=False)
    click('directory-remove-confirm')
    check('A project bound while confirmation is open prevents removal',p.has('[role=dialog]') and y['id'] in call('list_work_workspaces',workId=a['id']))
    click('directory-remove-cancel')
    with sqlite3.connect(DB) as conn:
        refused=False
        try: conn.execute('UPDATE workspaces SET enabled=0 WHERE id=?',[y['id']])
        except sqlite3.IntegrityError: refused=True
        check('Database trigger also blocks direct disable of protected directory',refused)
    call('unlink_work_workspace',workId=a['id'],workspaceId=x['id'])
    check('Last unlink stops the live primary watcher immediately',call('watcher_status') is None)
    with sqlite3.connect(DB) as conn:
        check('Retired primary is not configured for startup',conn.execute("SELECT COUNT(*) FROM app_settings WHERE key='main_workspace'").fetchone()[0]==0)
    call('unlink_work_workspace',workId=a['id'],workspaceId=y['id']); call('unlink_work_workspace',workId=b['id'],workspaceId=y['id'])
    for command in ['workspace_rescan','workspace_sync_status','reindex_workspace_documents']:
        check(command+' rejects a retired directory',p.eval(f"window.__TAURI_INTERNALS__.invoke({json.dumps(command)},{{workspaceId:{x['id']}}}).then(()=>false,()=>true)"))
    check('Directory roots and files remain present',len(list((PROFILE/'workspace').rglob('*.txt')))==5)
    check('No edge-case UI exception',not p.errors)
    sys.exit(0)
schedule=call('get_analysis_schedule'); schedule.update(enabled=False,daily_enabled=False); call('save_analysis_schedule',schedule=schedule)
check('Synthetic profile starts empty',call('list_works',status=None)==[])
a=call('create_work',title='合成项目 A · 长期跟进')
b=call('create_work',title='合成项目 B · 已归档资料')
x=call('attach_work_folder',workId=a['id'],path=str(PROFILE/'workspace/A'))
y=call('attach_work_folder',workId=b['id'],path=str(PROFILE/'workspace/B'))
call('link_work_workspace',workId=b['id'],workspaceId=x['id'],isPrimary=False)
for _ in range(50):
    if call('workspace_document_status',workspaceId=y['id'])['ready']==1: break
    p.wait(.2)
refresh()
check('Shared directory appears once',p.eval("document.querySelectorAll('[data-testid^=directory-select-]').length")==2)
check('Both project names shown for shared directory',p.eval(f"document.querySelector('[data-testid=directory-card-{x['id']}]').innerText.includes('合成项目 A')&&document.querySelector('[data-testid=directory-card-{x['id']}]').innerText.includes('合成项目 B')"))
check('Active project disables removal',p.eval(f"document.querySelector('[data-testid=directory-remove-{x['id']}]').disabled"))
rejected=p.eval(f"window.__TAURI_INTERNALS__.invoke('remove_workspace',{{workspaceId:{x['id']}}}).then(()=>false,()=>true)")
check('Backend independently blocks active binding removal',rejected)
click(f"directory-select-{y['id']}")
check('Selected directory drives document panel',p.eval("document.querySelector('[data-testid=workspace-documents]').innerText.includes('only-b.txt')&&!document.querySelector('[data-testid=workspace-documents]').innerText.includes('only-a.txt')"))
click('workspace-scan-selected')
check('Scan uses selected ID and selected directory',call('workspace_sync_status',workspaceId=y['id'])['baseline_count']==1)
call('archive_work',id=b['id']); refresh()
check('Archived-only binding enables removal',not p.eval(f"document.querySelector('[data-testid=directory-remove-{y['id']}]').disabled"))
click(f"directory-remove-{y['id']}"); click('directory-remove-cancel')
check('Cancel preserves archived binding',call('list_work_workspaces',workId=b['id'])==[x['id'],y['id']])
click(f"directory-remove-{y['id']}")
check('Confirmation says source files remain',p.eval("document.querySelector('[role=dialog]').innerText.includes('文件')"))
click('directory-remove-confirm')
check('Confirmed archived folder disappears',not p.has(f"[data-testid=directory-card-{y['id']}]") and y['id'] not in call('list_work_workspaces',workId=b['id']))
call('unlink_work_workspace',workId=a['id'],workspaceId=x['id']); refresh()
check('Shared folder remains for other project',p.has(f"[data-testid=directory-card-{x['id']}]") and not p.eval(f"document.querySelector('[data-testid=directory-remove-{x['id']}]').disabled"))
for width,height in [(1440,1000),(1100,720)]:
    p.command('Emulation.setDeviceMetricsOverride',{'width':width,'height':height,'deviceScaleFactor':1,'mobile':False})
    p.eval("document.documentElement.dataset.fontSize='xlarge'")
    check('No horizontal clipping '+str(width),p.eval("document.documentElement.scrollWidth<=innerWidth+2 && document.querySelector('.content-scroll').scrollWidth<=document.querySelector('.content-scroll').clientWidth+2"))
    p.screenshot(f'directory-bound-{width}.png')
call('unlink_work_workspace',workId=b['id'],workspaceId=x['id']); refresh()
check('Last unlink empties list and selected panels',call('get_project_directories')==[] and not p.has('[data-testid=workspace-documents]') and not p.has('[data-testid=workspace-browser]'))
call('link_work_workspace',workId=a['id'],workspaceId=x['id'],isPrimary=True); refresh()
check('Rebinding restores same directory identity',len(call('get_project_directories'))==1 and call('get_project_directories')[0]['id']==x['id'])
call('delete_work',id=a['id']); refresh()
check('Deleting last project retires its directory',call('get_project_directories')==[])
check('All source files unchanged', (PROFILE/'workspace/A/only-a.txt').read_text(encoding='utf-8')=='Synthetic A.\n' and (PROFILE/'workspace/B/only-b.txt').read_text(encoding='utf-8')=='Synthetic B.\n')
check('No UI runtime exceptions',not p.errors)
p.screenshot('directory-empty.png')
p.ws.close()
