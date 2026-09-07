"""Synthetic-only cognition, natural capture, review/undo, and read-only regression."""
import hashlib
import json
import os
from pathlib import Path
import runpy
import time

Page = runpy.run_path(str(Path(__file__).with_name("release-functional-cdp.py")))["Page"]
root = Path(os.environ["MSL_WORKSPACE"]).resolve() / "cognition-fixture"
local = Path(os.environ["LOCALAPPDATA"]).resolve()
assert ".test-runtime" in root.parts and ".test-runtime" in local.parts
assert all(".test-runtime" in Path(os.environ[k]).parts for k in ("APPDATA", "LOCALAPPDATA", "TEMP", "TMP"))
root.mkdir(parents=True, exist_ok=True)
for n in range(3):
    (root / f"synthetic-{n}.txt").write_text(f"Synthetic cohort preparation {n}.\nFollow up the study and retain original files.", encoding="utf-8")
page = Page()
checks = []

def check(label, result):
    checks.append(bool(result))
    print(f"[{'PASS' if result else 'FAIL'}] {label}", flush=True)

def invoke(command, **args):
    return page.eval(f"window.__TAURI_INTERNALS__.invoke({json.dumps(command)},{json.dumps(args)})")

def finish(job):
    end = time.time() + 70
    while time.time() < end:
        value = invoke("get_ai_job", id=job["id"])
        if value["status"] != "running":
            return value
        time.sleep(.3)
    raise RuntimeError("isolated job did not finish")

def hashes():
    return {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest() for p in root.rglob("*") if p.is_file()}

def local_path(value):
    # Rust returns canonical Windows extended paths; normalize for Python comparisons.
    return Path(value.removeprefix("\\\\?\\")).resolve()

provider = None
try:
    page.command("Emulation.setDeviceMetricsOverride", {"width":1440,"height":1000,"deviceScaleFactor":1,"mobile":False})
    if page.eval("document.documentElement.lang") != "zh-CN":
        page.eval("document.querySelector('.locale-button').click()")
    schedule = invoke("get_analysis_schedule")
    schedule["enabled"] = False
    schedule["daily_enabled"] = False
    invoke("save_analysis_schedule", schedule=schedule)
    work = invoke("create_work", title=f"认知合成项目 {int(time.time())}", status="active")
    wid = work["id"]
    task = invoke("create_task", workId=wid, title="认知合成：收取资料", priority="normal")
    waiting = invoke("create_waiting", workId=wid, title="认知合成：等待资料", waitingFor="合成协作方")
    invoke("create_resume_point", workId=wid, currentState="合成准备阶段", nextStep="接收资料", remember="仅为测试")
    before = hashes()
    ws = invoke("attach_work_folder", workId=wid, path=str(root))
    for _ in range(40):
        if invoke("workspace_document_status", workspaceId=ws["id"])["ready"] >= 3:
            break
        time.sleep(.2)
    # A reused folder already has ready documents. Wait for the new scan receipt,
    # not just the old ready-count, before asserting unchanged versions.
    scan=invoke("start_ai_job",request={"command":"refresh_project_cognition","args":{"scope":"work","scopeId":wid}})
    assert finish(scan)["status"]=="completed"
    entries = [invoke("get_project_cognition", scope=scope, scopeId=ident) for scope,ident in [("global",None),("workspace",ws["id"]),("work",wid)]]
    check("Global, folder and project Markdown entries are materialized", all(Path(e["path"]).is_file() for e in entries))
    check("Cognition writes stay outside source folders", all(local_path(e["path"]).is_relative_to(local) and not local_path(e["path"]).is_relative_to(root) for e in entries))
    entry = entries[2]
    check("Project cognition includes workflow and document entry", "认知合成" in entry["markdown"] and "synthetic-0.txt" in entry["markdown"])
    check("Sources unchanged by association and cognition", hashes()==before)
    same=invoke("get_project_cognition", scope="work", scopeId=wid)
    check("Unchanged cognition reuses its version", same["version"]==entry["version"])
    generated=local_path(entry["path"])
    assert generated.is_relative_to(local) and generated.name=="PROJECT.md"
    generated.unlink()
    rebuilt=invoke("get_project_cognition", scope="work", scopeId=wid)
    check("Evicted cognition Markdown rebuilds without altering source", generated.exists() and rebuilt["version"]==entry["version"] and hashes()==before)

    # Fixture setup uses IPC directly, so remount the view to load those records.
    page.click("nav-inbox");page.wait(.2);page.click("nav-works");page.wait(.5)
    page.eval(f"[...document.querySelectorAll('.work-item')].find(e=>e.textContent.includes({json.dumps(work['title'])})).click()")
    page.wait(.4)
    page.eval("document.querySelector('[data-testid=cognition-work] summary').click()")
    page.wait(.6)
    check("Project cognition expands into readable Markdown", page.has('[data-testid="cognition-markdown"]'))
    page.eval("document.querySelector('[data-testid=cognition-work]').scrollIntoView({block:'start'})")
    page.screenshot("cognition-project-entry.png")
    page.click("refresh-cognition");page.wait(.2);page.click("nav-inbox")
    job=next(j for j in invoke("list_ai_jobs") if j["command"]=="refresh_project_cognition")
    check("Cognition refresh finishes after navigating away",finish(job)["status"]=="completed")

    provider=invoke("save_provider_connection",id=None,displayName="Cognition Mock",providerType="custom",baseUrl="http://127.0.0.1:9462",legacyModel="",templateKind="custom",authMode="bearer",modelsEndpoint=None,enabled=True,apiKey="synthetic-cognition-key")
    model=invoke("save_provider_model",providerId=provider["id"],modelId="flow-cognition",displayName="Cognition mock",protocol="chat_completions",endpointPath="/chat/completions",capabilitiesJson="{}",source="manual",enabled=True,available=True)
    invoke("save_ai_task_route",taskKind="global_analysis",providerModelId=model["id"])
    page.click("nav-inbox");page.wait(.5)
    text=f"{work['title']}：资料已收到，收取任务已完成，结束等待，并更新项目进展。"
    page.eval(f"(()=>{{const e=document.querySelector('#inbox-natural-text');e.value={json.dumps(text)};e.dispatchEvent(new Event('input',{{bubbles:true}}));}})()")
    page.click("capture-and-organize");page.wait(.5);page.click("nav-today")
    jobs=invoke("list_ai_jobs");job=next(j for j in jobs if j["command"]=="organize_inbox_item")
    check("Natural capture starts a durable background job",job["status"]=="running")
    result=finish(job)
    check("Natural capture completes after a page switch", result["status"]=="completed")
    if result["status"]!="completed": raise RuntimeError(result.get("error"))
    proposals=[p for p in invoke("list_ai_proposals",status="pending",limit=100) if p["analysis_run_id"]==result["result"]]
    check("One observation yields related progress/task/waiting drafts",{p["kind"] for p in proposals}=={"resume_point","task","waiting"})
    detail=invoke("get_work_detail",id=wid)
    check("No completion before explicit confirmation",next(t for t in detail["tasks"] if t["id"]==task["id"])["status"]=="next" and next(w for w in detail["waiting"] if w["id"]==waiting["id"])["status"]=="open")
    # Exercise homepage shortcuts as well as the deeper review editor.
    page.click("nav-inbox");page.wait(.2);page.click("nav-today");page.wait(.6)
    for proposal in [p for p in proposals if p['kind'] in ('task','resume_point')]:
        page.eval(f"document.querySelector('[data-testid=latest-decision-{proposal['id']}] .decision-confirm').click()")
        page.wait(.5)
        updated=invoke('get_work_detail',id=wid)
        if proposal['kind']=='task':
            check('Homepage confirmation preserves completion status',next(t for t in updated['tasks'] if t['id']==task['id'])['status']=='done')
        else:
            check('Homepage confirmation preserves project progress',updated['latest_resume']['current_state']=='合成资料已收到')
        page.eval("[...document.querySelectorAll('.decision-card button')].find(b=>b.textContent.includes('撤销刚才')).click()")
        page.wait(.5)
    page.click("nav-review");page.wait(.6)
    check("Review offers grouped confirmation and decision tools",page.has('[data-testid="group-review"]') and page.has('[data-testid="review-tools"]'))
    page.screenshot("cognition-review-decisions.png")
    page.eval("document.querySelector('[data-testid=group-review] summary').click()")
    for proposal in proposals:
        page.eval(f"document.querySelector('[data-testid=group-review] input[value=\"{proposal['id']}\"]').click()")
    page.click("confirm-proposal-group");page.wait(.8)
    receipts=invoke("list_confirmation_receipts")
    group=next(r for r in receipts if set(r["proposal_ids"])=={p["id"] for p in proposals})
    detail=invoke("get_work_detail",id=wid)
    check("Confirmed group updates task and waiting state",next(t for t in detail["tasks"] if t["id"]==task["id"])["status"]=="done" and next(w for w in detail["waiting"] if w["id"]==waiting["id"])["status"]=="resolved")
    check("Confirmed group updates project progress",detail["latest_resume"]["current_state"]=="合成资料已收到")
    memory_before=invoke("get_classification_memory_stats")["feedback_count"]
    page.click("review-tools");page.wait(.4)
    page.eval("document.querySelectorAll('.tabs button')[1].click()")
    page.click(f"undo-{group['id']}");page.wait(.6)
    page.eval("document.querySelector('.modal-header .close')?.click()")
    detail=invoke("get_work_detail",id=wid)
    check("Undo restores both entity states",next(t for t in detail["tasks"] if t["id"]==task["id"])["status"]=="next" and next(w for w in detail["waiting"] if w["id"]==waiting["id"])["status"]=="open")
    check("Undo restores classification feedback",invoke("get_classification_memory_stats")["feedback_count"]==memory_before-len(proposals))

    proposals=[p for p in invoke("list_ai_proposals",status="pending",limit=100) if p["analysis_run_id"]==result["result"]]
    one=next(p for p in proposals if p["kind"]=="waiting")
    count=invoke("get_classification_memory_stats")["feedback_count"]
    invoke("reject_ai_proposal",id=one["id"],expectedUpdatedAt=one["updated_at"],reason="duplicate")
    check("Duplicate rejection does not teach a category error",invoke("get_classification_memory_stats")["feedback_count"]==count)
    one=next(p for p in proposals if p["kind"]=="task")
    invoke("reject_ai_proposal",id=one["id"],expectedUpdatedAt=one["updated_at"],reason="wrong_category")
    cards=invoke("list_classification_memories")
    card=next(c for c in cards if "认知合成" in c["cue"])
    check("Explicit category error creates explainable memory",card["rejected"]==1)
    invoke("edit_classification_memory",id=card["id"],expectedUpdatedAt=card["updated_at"],preferredKind="waiting",workId=wid)
    corrected=next(c for c in invoke("list_classification_memories") if c["id"]==card["id"])
    check("User can correct category and project preference",corrected["preferred_kind"]=="waiting" and corrected["preferred_work_id"]==wid)
    page.click("nav-review");page.wait(.5);page.click("review-tools");page.wait(.5)
    page.screenshot("cognition-memory-controls.png")
    invoke("edit_classification_memory",id=corrected["id"],expectedUpdatedAt=corrected["updated_at"],preferredKind=None,workId=None)
    check("User can forget a specific memory",not any(c["id"]==card["id"] for c in invoke("list_classification_memories")))
    page.eval("document.querySelector('.modal-header .close')?.click()")

    protected_before=hashes()
    plan=invoke("preview_storage_cleanup",categories=["cognition","extracted"])
    cleanup=invoke("execute_storage_cleanup",planId=plan["plan_id"])
    check("Cleanup removes only rebuildable app files",cleanup["deleted_count"]>0 and hashes()==protected_before)
    invoke("reindex_workspace_documents",workspaceId=ws["id"])
    check("Unchanged sources re-extract after cache cleanup",invoke("workspace_document_status",workspaceId=ws["id"])["ready"]==3)
    invoke("get_project_cognition",scope="work",scopeId=wid)
    check("Cognition is rebuilt following cache cleanup",generated.exists() and hashes()==protected_before)
    job=invoke("start_ai_job",request={"command":"run_analysis_now","args":{"trigger":"manual"}})
    check("Cognition-first global analysis completes with Mock",finish(job)["status"]=="completed")
    import urllib.request
    metrics=json.load(urllib.request.urlopen("http://127.0.0.1:9462/metrics"))
    check("Mock received bounded cognition entries",metrics.get("cognition_entries",0)>0 and metrics.get("selected_chars",999999)<=12000)
    check("Workspace source files remain byte-identical",hashes()==before)
    state={"work_id":wid,"workspace_id":ws["id"],"source_hashes":before,"root":str(root),"cognition_path":str(generated)}
    (Path(os.environ["MSL_ARTIFACTS"])/"cognition-state.json").write_text(json.dumps(state),encoding="utf-8")
finally:
    if provider:
        invoke("delete_provider",id=provider["id"])
    print(f"COGNITION_FLOW_PASS={sum(checks)} COGNITION_FLOW_FAIL={len(checks)-sum(checks)}")
if not all(checks): raise SystemExit(1)
