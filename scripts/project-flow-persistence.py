"""Read only synthetic application state after an actual process restart."""
import importlib.util
import json
import os
from pathlib import Path

assert ".test-runtime" in Path(os.environ["APPDATA"]).parts
spec=importlib.util.spec_from_file_location("cdp",Path(__file__).with_name("release-functional-cdp.py"))
cdp=importlib.util.module_from_spec(spec);spec.loader.exec_module(cdp)
page=cdp.Page();page.wait(1)
expected=json.loads((cdp.ARTIFACTS/"project-flow-state.json").read_text(encoding="utf-8"))
def invoke(command,**args):
    return page.eval(f"window.__TAURI_INTERNALS__.invoke({json.dumps(command)},{json.dumps(args)})")
detail=invoke("get_work_detail",id=expected["work_id"])
jobs=invoke("list_ai_jobs")
checks=[len(detail["tasks"])==expected["task_count"],set(invoke("list_work_workspaces",workId=expected["work_id"]))==set(expected["folder_ids"]),any(j["command"]=="translate_text" and j["status"]=="completed" and j["result"]=="Hello" for j in jobs)]
page.click("nav-translation");page.wait(.7)
checks.append(page.eval("document.querySelector('[data-testid=translation-result]').textContent==='Hello'"))
print(f"PROJECT_RESTART_PERSISTENCE_PASS={sum(checks)} FAIL={len(checks)-sum(checks)}")
page.ws.close()
raise SystemExit(0 if all(checks) else 1)
