"""Capture the installed release using synthetic data, including real wheel scrolling."""
import json
import os
import pathlib
import runpy
helpers=runpy.run_path(str(pathlib.Path(__file__).with_name('ui-visibility-cdp.py')))
p=helpers['Page']()
p.command('Emulation.setDeviceMetricsOverride',{'width':1280,'height':720,'deviceScaleFactor':1,'mobile':False})
p.eval("document.documentElement.dataset.fontSize='xlarge';delete document.documentElement.dataset.theme")
p.click('nav-today');p.wait(.5)
p.eval("document.querySelectorAll('.toast button').forEach(e=>e.click());document.querySelector('.content-scroll').scrollTop=0")
p.screenshot('dashboard-top.png')
p.command('Input.dispatchMouseEvent',{'type':'mouseWheel','x':1250,'y':620,'deltaX':0,'deltaY':10000})
p.wait(.4)
scrolled=p.eval("document.querySelector('.content-scroll').scrollTop>0")
assert scrolled, 'Wheel must reveal the bottom of the dashboard'
result=p.eval(f"({helpers['REACH']})('.support-grid button,.support-grid input,.support-grid select')")
assert not result['totalFailures'], result
p.screenshot('dashboard-bottom.png')
p.click('nav-plan');p.wait(.3)
p.screenshot('tasks-readable.png')
p.click('task-create');p.wait(.2)
p.eval("document.querySelector('.modal-body').scrollTop=10000")
p.screenshot('modal-bottom.png')
p.eval("document.querySelector('.modal .close').click()")
p.click('nav-settings');p.wait(.5)
version=p.eval("document.querySelector('[data-testid=app-version]')?.textContent")
assert version and '0.1.1' in version, 'Installed UI must identify version 0.1.1'
p.eval(f"({helpers['REACH']})('[data-testid=app-version]')")
p.screenshot('installed-version.png')
out=pathlib.Path(os.environ['MSL_ARTIFACTS'])
(out/'installed-verification.json').write_text(json.dumps({'version':version,'realWheelScroll':scrolled,'supportControls':result['count'],'failures':result['totalFailures']},indent=2),encoding='utf-8')
print('INSTALLED_VERSION_AND_WHEEL_PASS=1')
