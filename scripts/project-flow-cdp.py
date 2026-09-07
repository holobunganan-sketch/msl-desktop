"""Synthetic-only end-to-end regression for project flow and durable AI tasks."""
import importlib.util
import json
import os
from pathlib import Path
import time
import urllib.request

spec = importlib.util.spec_from_file_location("cdp", Path(__file__).with_name("release-functional-cdp.py"))
cdp = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cdp)
root = Path(os.environ["MSL_WORKSPACE"]).resolve()
assert ".test-runtime" in root.parts
assert ".test-runtime" in Path(os.environ["APPDATA"]).parts
mock_url = "http://127.0.0.1:9462"
page = cdp.Page()
for _ in range(40):
    if page.eval("Boolean(window.__TAURI_INTERNALS__ && document.querySelector('[data-testid=nav-works]'))"):
        break
    time.sleep(.25)
if page.eval("document.documentElement.lang") != "zh-CN":
    page.eval("document.querySelector('.locale-button')?.click()")
    page.wait(.4)
page.eval("document.querySelector('[data-testid=background-jobs-panel] header button')?.click()")
checks = []

def check(label, result):
    checks.append(bool(result))
    print(f"[{'PASS' if result else 'FAIL'}] {label}", flush=True)

def invoke(command, **args):
    return page.eval(f"window.__TAURI_INTERNALS__.invoke({json.dumps(command)},{json.dumps(args)})")

def start(command, **args):
    return invoke("start_ai_job", request={"command": command, "args": args})

def finish(job):
    deadline = time.time()+45
    while time.time()<deadline:
        current=invoke("get_ai_job",id=job["id"])
        if current["status"]!="running":
            return current
        time.sleep(.4)
    raise RuntimeError("Synthetic job timed out")

def screenshot(name):
    page.command("Emulation.setDeviceMetricsOverride", {"width":1440,"height":1000,"deviceScaleFactor":1,"mobile":False})
    page.wait(.5)
    page.screenshot(name)

provider=None
try:
    provider=invoke("save_provider_connection", id=None, displayName="Flow Mock", providerType="custom", baseUrl=mock_url, legacyModel="", templateKind="custom", authMode="bearer", modelsEndpoint=None, enabled=True, apiKey="synthetic-flow-key")
    def route(task, model):
        m=invoke("save_provider_model",providerId=provider["id"],modelId=model,displayName=model,protocol="chat_completions",endpointPath="/chat/completions",capabilitiesJson="{}",source="manual",enabled=True,available=True)
        invoke("save_ai_task_route",taskKind=task,providerModelId=m["id"])
    schedule=invoke("get_analysis_schedule");schedule["enabled"]=False
    invoke("save_analysis_schedule",schedule=schedule)
    work=invoke("create_work",title=f"医学项目 · 阶段推进（测试 {int(time.time())}）",status="active")
    wid=work["id"]
    invoke("update_work",id=wid,title=work["title"],status="active",summary="通过自然流转衔接资料准备、项目沟通与阶段复盘。")
    invoke("create_task",workId=wid,title="确认下一阶段沟通计划",priority="normal",dueAt=int(time.time())+30*86400)
    invoke("create_waiting",workId=wid,title="等待方案意见",waitingFor="项目团队")
    invoke("create_calendar_event",workId=wid,title="项目复盘会",startAt=int(time.time())+40*86400,allDay=False,kind="meeting")
    invoke("create_resume_point",workId=wid,currentState="阶段准备完成",nextStep="安排下一轮沟通",remember="保留历史记录")
    folder_ids=[]
    for name in ("资料准备", "沟通记录"):
        folder=root/name;folder.mkdir(parents=True,exist_ok=True)
        (folder/"synthetic.txt").write_text("Synthetic project evidence for isolated regression only.",encoding="utf-8")
        folder_ids.append(invoke("attach_work_folder",workId=wid,path=str(folder))["id"])
    invoke("attach_work_folder",workId=wid,path=str(root/"资料准备"))
    check("Folder association is idempotent and supports multiple folders",set(invoke("list_work_workspaces",workId=wid))==set(folder_ids))
    page.click("nav-works");page.wait(.7)
    page.eval(f"[...document.querySelectorAll('.work-item')].find(e=>e.textContent.includes({json.dumps(work['title'])})).click()")
    page.wait(.6)
    check("Project displays associated folder cards",page.eval("document.querySelectorAll('.linked-folder').length===2"))
    check("Project-wide AI action is available",page.has('[data-testid="organize-work-with-ai"]'))
    screenshot("flow-project-folders.png")

    inbox=invoke("create_inbox_item",content="已整理阶段反馈，可以继续下一轮沟通。")
    page.click("nav-inbox");page.wait(.6);page.click(f"inbox-project-{inbox['id']}");page.wait(.4)
    page.eval(f"(()=>{{let w=document.querySelector('#inbox-work');w.value={json.dumps(str(wid))};w.dispatchEvent(new Event('change',{{bubbles:true}}));let k=document.querySelector('#project-item-kind');k.value='resume_point';k.dispatchEvent(new Event('change',{{bubbles:true}}));}})()")
    screenshot("flow-inbox-project.png")
    page.eval("document.querySelector('.modal-form button[type=submit]').click()");page.wait(.7)
    processed=next(x for x in invoke("list_inbox") if x["id"]==inbox["id"])
    detail=invoke("get_work_detail",id=wid)
    check("Inbox UI converts directly into project progress",processed["converted_to_type"]=="resume_point" and detail["latest_resume"]["current_state"]==inbox["content"])
    check("Progress retains next step and reminders",detail["latest_resume"]["next_step"]=="安排下一轮沟通" and detail["latest_resume"]["remember"]=="保留历史记录")
    duplicate=page.eval(f"window.__TAURI_INTERNALS__.invoke('convert_inbox_to_task',{{inboxId:{inbox['id']},title:'duplicate'}}).then(()=>false,()=>true)")
    check("Processed inbox cannot create duplicate entities",duplicate)

    route("work_draft","flow-project")
    page.click("nav-works");page.wait(.5)
    page.eval(f"[...document.querySelectorAll('.work-item')].find(e=>e.textContent.includes({json.dumps(work['title'])})).click()")
    page.wait(.5);page.click("organize-work-with-ai");page.wait(.35)
    page.click("nav-inbox")
    jobs=invoke("list_ai_jobs");job=next(j for j in jobs if j["command"]=="start_workspace_work_draft" and j["args"].get("workId")==wid)
    check("Project analysis remains running after page switch",job["status"]=="running")
    result=finish(job)
    check("Project analysis finishes in background",result["status"]=="completed")
    metrics=json.load(urllib.request.urlopen(mock_url+"/metrics"))
    check("AI receives all project categories and both folder documents",len(metrics["focused_categories"])==4 and metrics["focused_documents"]>=2)
    proposals=[p for p in invoke("list_ai_proposals",status="pending",limit=200) if p["analysis_run_id"]==result["result"]]
    before=invoke("get_work_detail",id=wid)
    check("AI drafts are queued without writing project entities",len(proposals)==4 and len(before["tasks"])==len(detail["tasks"]))
    for p in proposals:
        invoke("confirm_ai_proposal",id=p["id"],expectedUpdatedAt=p["updated_at"],editedPayload=json.loads(p["payload_json"]))
    after=invoke("get_work_detail",id=wid)
    check("Confirmed suggestions enter each project category",all(len(after[k])==len(before[k])+1 for k in ("tasks","waiting","calendar","resume_history")))

    route("global_analysis","flow-repair")
    repairs_before=json.load(urllib.request.urlopen(mock_url+"/metrics"))["repair_requests"]
    job=start("run_analysis_now",trigger="manual")
    same=start("run_analysis_now",trigger="manual")
    check("Repeated analysis click reuses the active job",job["id"]==same["id"])
    page.eval("location.reload()");page.wait(.8)
    result=finish(job)
    check("Malformed JSON recovers with one model correction after page reload",result["status"]=="completed" and json.load(urllib.request.urlopen(mock_url+"/metrics"))["repair_requests"]==repairs_before+1)
    page.click("nav-review");page.wait(.6)
    check("Review renders recovered proposals",page.eval("document.querySelectorAll('.proposal-item').length>0"))
    screenshot("flow-review.png")
    route("global_analysis","flow-invalid")
    before=len(invoke("list_ai_proposals",status=None,limit=500))
    result=finish(start("run_analysis_now",trigger="manual"))
    check("Persistent invalid output stops safely with actionable error",result["status"]=="failed" and "校正" in result["error"])
    check("Failed analysis adds no partial proposals",len(invoke("list_ai_proposals",status=None,limit=500))==before)

    for kind in ("weekly", "monthly"):
        route(f"{kind}_report", "flow-delayed")
        job=start("generate_report",kind=kind,periodStart=int(time.time())-7*86400,periodEnd=int(time.time()))
        page.click("nav-inbox")
        check(f"{kind} report completes away from its page",finish(job)["status"]=="completed")
    route("daily_brief","flow-delayed")
    now=int(time.time())
    job=start("generate_brief",date=time.strftime("%Y-%m-%d"),periodStart=now-86400,periodEnd=now,todayStart=now-100,todayEnd=now+86400,locale="zh-CN",force=True)
    page.click("nav-reports")
    check("Daily brief completes in the background",finish(job)["status"]=="completed")

    route("translation","flow-delayed")
    page.click("nav-translation");page.wait(.5)
    page.eval("(()=>{let e=document.querySelector('[data-testid=translation-source]');e.value='你好';e.dispatchEvent(new Event('input',{bubbles:true}));})()")
    page.wait(.2);page.eval("document.querySelector('.source-panel .app-button.primary').click()");page.wait(.3);page.click("nav-works")
    job=next(j for j in invoke("list_ai_jobs") if j["command"]=="translate_text")
    finish(job)
    page.click("nav-translation");page.wait(.5)
    check("Translation survives navigation with input and result intact",page.eval("document.querySelector('[data-testid=translation-source]').value==='你好' && document.querySelector('[data-testid=translation-result]').textContent==='Hello'"))
    page.eval("location.reload()");page.wait(.8);page.click("nav-translation");page.wait(.6)
    check("Translation result restores from SQLite after reload",page.eval("document.querySelector('[data-testid=translation-result]').textContent==='Hello'"))
    page.click("background-jobs-toggle");page.wait(.3)
    screenshot("flow-background-jobs.png")
    check("Global background status panel is available",page.has('[data-testid="background-jobs-panel"]'))
    page.eval("document.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true}))");page.wait(.1)
    check("Background panel closes with Escape",not page.has('[data-testid="background-jobs-panel"]'))
    page.click("background-jobs-toggle");page.click("nav-works");page.wait(.2)
    check("Background panel dismisses when navigating",not page.has('[data-testid="background-jobs-panel"]'))
    before=len(invoke("list_ai_proposals",status=None,limit=500))
    reports_before=len(invoke("list_reports",limit=100))
    plan=invoke("preview_storage_cleanup",categories=[])
    invoke("execute_storage_cleanup",planId=plan["plan_id"])
    check("Cache cleanup preserves project entities, proposals and reports",len(invoke("list_ai_proposals",status=None,limit=500))==before and len(invoke("list_reports",limit=100))==reports_before and len(invoke("get_work_detail",id=wid)["tasks"])==len(after["tasks"]))
    check("Cache cleanup leaves original folder files intact",all((root/name/"synthetic.txt").is_file() for name in ("资料准备","沟通记录")))
    (cdp.ARTIFACTS/"project-flow-state.json").write_text(json.dumps({"work_id":wid,"folder_ids":folder_ids,"task_count":len(after["tasks"]),"translation_job_id":job["id"]}),encoding="utf-8")
    check("No unexpected UI exceptions",not page.errors)
finally:
    if provider:
        invoke("delete_provider",id=provider["id"])
    page.ws.close()
print(f"PROJECT_FLOW_PASS={sum(checks)} PROJECT_FLOW_FAIL={len(checks)-sum(checks)}",flush=True)
raise SystemExit(0 if all(checks) else 1)
