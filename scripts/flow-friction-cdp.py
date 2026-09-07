#!/usr/bin/env python3
"""Checks low-friction recording and non-duplicated empty review UI."""
import json
import os
import time
import urllib.request

import websocket


PORT = int(os.environ.get("MSL_CDP_PORT", "9358"))
page = next(
    item
    for item in json.load(urllib.request.urlopen(f"http://127.0.0.1:{PORT}/json"))
    if item.get("type") == "page"
)
ws = websocket.create_connection(page["webSocketDebuggerUrl"], timeout=20, suppress_origin=True)
seq = 0


def evaluate(expression):
    global seq
    seq += 1
    ident = seq
    ws.send(json.dumps({
        "id": ident,
        "method": "Runtime.evaluate",
        "params": {"expression": expression, "awaitPromise": True, "returnByValue": True},
    }))
    while True:
        response = json.loads(ws.recv())
        if response.get("id") == ident:
            return response.get("result", {}).get("result", {}).get("value")


checks = []


def check(label, value):
    ok = bool(value)
    checks.append(ok)
    print(f"[{ 'PASS' if ok else 'FAIL' }] {label}")


time.sleep(1)
evaluate("document.querySelector('[data-testid=nav-review]')?.click()")
time.sleep(0.5)
review = evaluate("""(()=>({
  emptyStates:document.querySelectorAll('[data-testid=review-empty-state]').length,
  duplicateSidePanel:Boolean(document.querySelector('.proposal-list')),
  emptyVisible:(()=>{const e=document.querySelector('[data-testid=review-empty-state]');if(!e)return false;const r=e.getBoundingClientRect();return r.width>300&&r.bottom<=innerHeight+1;})()
}))()""")
check("Review shows one empty state", review and review["emptyStates"] == 1)
check("Review removes redundant empty side panel", review and not review["duplicateSidePanel"])
check("Review empty state is not clipped", review and review["emptyVisible"])

evaluate("""(async()=>{
  const works=await window.__TAURI_INTERNALS__.invoke('list_works',{status:null});
  if(!works.length) await window.__TAURI_INTERNALS__.invoke('create_work',{title:'Synthetic flow check',status:'active'});
  document.querySelector('[data-testid=nav-works]')?.click();
  return true;
})()""")
time.sleep(0.5)
work = evaluate("""(()=>({
  quick:Boolean(document.querySelector('[data-testid=work-quick-progress]')),
  advanced:Boolean(document.querySelector('[data-testid=work-progress-details]:not([open])')),
  manualFileForm:Boolean(document.querySelector('.file-form'))
}))()""")
check("Work offers a one-line progress capture", work and work["quick"])
check("Detailed progress fields are optional and collapsed", work and work["advanced"])
check("Work does not require a manual file form", work and not work["manualFileForm"])

ws.close()
print(f"FLOW_FRICTION_PASS={sum(checks)} FLOW_FRICTION_FAIL={len(checks)-sum(checks)}")
raise SystemExit(0 if all(checks) else 1)
