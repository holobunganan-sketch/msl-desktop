"""Rendered visibility regression. Synthetic fixtures; refuses non-isolated APPDATA.

Checks actual viewport geometry and user-scrollable ancestors, not DOM presence.
Hidden-overflow ancestors are deliberately NOT scrolled by the test.
"""
import json
import os
import pathlib
import runpy
import sqlite3
import sys
import time

Page = runpy.run_path(str(pathlib.Path(__file__).with_name('release-functional-cdp.py')))['Page']
root = pathlib.Path(os.environ['APPDATA']).resolve()
assert '.test-runtime' in root.parts, 'Requires isolated APPDATA'
db_path = root / 'MSLDesktop' / 'msl-desktop.db'
assert db_path.exists(), 'Start isolated application first'


def seed():
    now = int(time.time())
    with sqlite3.connect(db_path) as db:
        if db.execute("SELECT COUNT(*) FROM works WHERE title LIKE '可见性测试%'").fetchone()[0]:
            return
        for n in range(8):
            title = f'可见性测试 {n + 1} · 跨部门学术交流项目与阶段性材料整理'
            work = db.execute('INSERT INTO works(title,status,summary,created_at,updated_at) VALUES(?,?,?,?,?)',
                              (title, 'active', '测试内容：确认阶段进度与下一步安排。' * 10, now, now)).lastrowid
            db.execute('INSERT INTO resume_points(work_id,current_state,next_step,remember,created_at) VALUES(?,?,?,?,?)',
                       (work, '已完成材料核对与时间协调。' * 8, '确认交流安排，准备下一阶段的会议材料。' * 4, '测试提醒', now))
            db.execute('INSERT INTO tasks(work_id,title,priority,due_at,notes,created_at,updated_at) VALUES(?,?,?,?,?,?,?)',
                       (work, f'测试任务 {n + 1}：整理医学资料并核对下一阶段的交流计划与行动清单', ['normal','high','low'][n % 3], now - 60, '测试备注', now, now))
            db.execute('INSERT INTO waiting_items(work_id,title,waiting_for,started_at,follow_up_at,created_at,updated_at) VALUES(?,?,?,?,?,?,?)',
                       (work, '测试等待：确认会议时间与详细日程安排', '测试协作方', now, now, now, now))
            db.execute('INSERT INTO inbox_items(content,created_at) VALUES(?,?)', ('测试收件：需要整理并归入项目的会议记录。' * 5, now))
        brief = db.execute('INSERT INTO daily_briefs(brief_date,generated_at,content) VALUES(?,?,?)',
                           (time.strftime('%Y-%m-%d'), now, '\n'.join('• 测试进展：项目材料已更新，待确认新的工作计划与后续交流安排。' * 2 for _ in range(6)))).lastrowid
        run = db.execute("INSERT INTO analysis_runs(trigger,status,started_at,finished_at,summary,brief_id,created_at) VALUES('manual','completed',?,?,?,?,?)",
                         (now, now, '测试分析完成，建议等待用户确认。', brief, now)).lastrowid
        for n in range(6):
            title = f'测试建议 {n + 1}：核对详细日程并安排下一次项目交流'
            db.execute("INSERT INTO ai_proposals(analysis_run_id,kind,operation,dedupe_key,title,payload_json,reason,created_at,updated_at) VALUES(?,'task','create',?,?,?,?,?,?)",
                       (run, f'visibility-{n}', title, json.dumps({'title':title,'priority':'normal','notes':'测试备注。' * 20}, ensure_ascii=False), '测试理由：依据项目安排提出建议。' * 12, now, now))
        for kind in ('weekly', 'monthly'):
            db.execute("INSERT INTO reports(kind,period_start,period_end,status,content,generated_at,created_at,updated_at) VALUES(?,?,?,'completed',?,?,?,?)",
                       (kind, now - 7*86400, now, '\n'.join(f'{n+1}. 测试报告段落：总结项目进展、等待事项和后续建议。' * 8 for n in range(20)), now, now, now))
        db.execute('UPDATE analysis_schedule_state SET enabled=0,daily_enabled=0')
        db.execute('UPDATE report_schedule_state SET weekly_enabled=0,monthly_enabled=0')


REACH = r"""(selector) => {
 const targets=[...document.querySelectorAll(selector)].filter(e=>e.checkVisibility({visibilityProperty:true,contentVisibilityAuto:true}) && !e.closest('details:not([open])'));
 const failures=[];
 for (const e of targets) {
   for(let a=e.parentElement;a&&a!==document.body;a=a.parentElement){
     const s=getComputedStyle(a);
     if(s.position==='fixed') break;
     if(/auto|scroll/.test(s.overflowY)&&a.scrollHeight>a.clientHeight){
       const r=e.getBoundingClientRect(),b=a.getBoundingClientRect(),scale=b.height/a.offsetHeight||1;
       a.scrollTop+=(r.top+r.height/2-b.top-b.height/2)/scale;
     }
   }
   const r=e.getBoundingClientRect();
   let top=0,left=0,right=innerWidth,bottom=innerHeight;
   for(let a=e.parentElement;a&&a!==document.body;a=a.parentElement){
     const s=getComputedStyle(a),b=a.getBoundingClientRect();
     if(/auto|scroll|hidden|clip/.test(s.overflowY)){top=Math.max(top,b.top);bottom=Math.min(bottom,b.bottom);}
     if(/auto|scroll|hidden|clip/.test(s.overflowX)){left=Math.max(left,b.left);right=Math.min(right,b.right);}
     if(s.position==='fixed') break;
   }
   const x=r.left+r.width/2,y=r.top+r.height/2,hit=document.elementFromPoint(x,y);
   const fits=r.top>=top-2&&r.bottom<=bottom+2&&r.left>=left-2&&r.right<=right+2;
   const hitOK=hit&&(hit===e||e.contains(hit)||hit.contains(e));
   if(!fits||!hitOK) failures.push({tag:e.tagName,id:e.dataset.testid||e.id||e.className,box:[r.left,r.top,r.right,r.bottom].map(Math.round),clip:[left,top,right,bottom].map(Math.round),hit:!!hitOK});
 }
 return {count:targets.length,failures:failures.slice(0,12),totalFailures:failures.length};
}"""


def main():
    seed()
    p=Page()
    p.command('Page.reload')
    p.wait(2)
    checks=[]
    def check(name, ok, evidence=None):
        checks.append({'name':name,'pass':bool(ok),'evidence':evidence})
        print(('PASS ' if ok else 'FAIL ')+name+((' '+json.dumps(evidence,ensure_ascii=False)) if not ok else ''), flush=True)
    quick='--quick' in sys.argv
    sizes=[(1280,720,'xlarge'),(1440,900,'standard')] if quick else [(960,600,'xlarge'),(1280,720,'xlarge'),(1440,900,'large'),(1920,1080,'standard'),(1280,720,'compact')]
    selectors={
      'today':'.brief-toolbar button,.metric-strip button,.agenda-list button,.decision-list button,.decision-list select,.support-grid button,.support-grid input,.support-grid select',
      'works':'.works button,.works select',
      'plan':'.plan button,.plan select',
      'waiting':'.waiting button,.waiting select',
      'inbox':'.inbox button,.inbox select',
      'calendar':'.calendar button,.calendar select',
      'review':'.review-page button,.review-page select,.review-page input',
      'reports':'.reports-page button,.reports-page select,.reports-page input',
      'translation':'.translation-page button,.translation-page select',
      'workspace':'.workspace-view button,.workspace-view select',
      'settings':'.settings button,.settings select,.settings input'
    }
    for w,h,font in sizes:
        p.command('Emulation.setDeviceMetricsOverride',{'width':w,'height':h,'deviceScaleFactor':1,'mobile':False})
        p.eval(f'document.documentElement.dataset.fontSize={json.dumps(font)}')
        for nav,selector in selectors.items():
            p.click('nav-'+nav);p.wait(.22)
            label=f'{w}x{h}/{font}/{nav}'
            geo=p.eval("""(()=>{const box=s=>{let e=document.querySelector(s),r=e.getBoundingClientRect();return [r.left,r.top,r.right,r.bottom]};return {viewport:[innerWidth,innerHeight],shell:box('.app-shell'),content:box('.content-scroll'),jobs:box('.job-center')}})()""")
            check(label+'/viewport', all(geo[k][3]<=h+2 and geo[k][2]<=w+2 for k in ('shell','content','jobs')),geo)
            reach=p.eval(f'({REACH})({json.dumps(selector)})')
            check(label+'/controls',reach['count']>0 and reach['totalFailures']==0,reach)
            if nav=='plan':
                badges=p.eval("""(()=>Array.from(document.querySelectorAll('.prio')).every(e=>{let r=document.createRange();r.selectNodeContents(e);let a=r.getBoundingClientRect(),b=e.getBoundingClientRect();return a.left>=b.left-1&&a.right<=b.right+1}))()""")
                check(label+'/badge-text',badges)
            if nav in ('today','review','works','plan') and font=='xlarge':
                p.eval("document.querySelector('.content-scroll').scrollTop=0")
                p.screenshot(f'visibility-{nav}-{w}-{font}.png')
        p.click('nav-plan');p.wait(.15);p.click('task-create');p.wait(.15)
        modal=p.eval(f'({REACH})(\'.modal button,.modal input,.modal select,.modal textarea\')')
        check(f'{w}x{h}/{font}/modal',modal['count']>0 and modal['totalFailures']==0,modal)
        p.eval("document.querySelector('.modal .close')?.click()")
    out=pathlib.Path(os.environ['MSL_ARTIFACTS']);out.mkdir(parents=True,exist_ok=True)
    (out/'visibility-results.json').write_text(json.dumps(checks,ensure_ascii=False,indent=2),encoding='utf-8')
    print(f"RESULT={sum(c['pass'] for c in checks)}/{len(checks)}",flush=True)
    sys.exit(0 if all(c['pass'] for c in checks) else 1)

if __name__=='__main__':main()
