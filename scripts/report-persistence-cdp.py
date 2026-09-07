#!/usr/bin/env python3
"""Isolated report persistence/cache audit; prints booleans and counts only."""

import argparse
import json
import os
import urllib.request

import websocket


PORT = int(os.environ.get("MSL_CDP_PORT", "9346"))


class Page:
    def __init__(self):
        pages = json.load(urllib.request.urlopen(f"http://127.0.0.1:{PORT}/json", timeout=10))
        target = next(item for item in pages if item.get("type") == "page")
        self.ws = websocket.create_connection(target["webSocketDebuggerUrl"], timeout=20, suppress_origin=True)
        self.seq = 0

    def eval(self, expression):
        self.seq += 1
        ident = self.seq
        self.ws.send(json.dumps({"id": ident, "method": "Runtime.evaluate", "params": {"expression": expression, "awaitPromise": True, "returnByValue": True}}))
        while True:
            message = json.loads(self.ws.recv())
            if message.get("id") == ident:
                return message.get("result", {}).get("result", {}).get("value")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("mode", choices=["prepare", "verify"])
    args = parser.parse_args()
    page = Page()
    if args.mode == "prepare":
        result = page.eval("""(async()=>{
          const before=await window.__TAURI_INTERNALS__.invoke('list_reports',{kind:null,limit:100});
          const schedule=await window.__TAURI_INTERNALS__.invoke('get_report_schedule');
          schedule.weekly_weekday=4;schedule.weekly_hour=16;schedule.weekly_minute=15;
          await window.__TAURI_INTERNALS__.invoke('save_report_schedule',{schedule});
          const plan=await window.__TAURI_INTERNALS__.invoke('preview_storage_cleanup',{categories:[]});
          await window.__TAURI_INTERNALS__.invoke('execute_storage_cleanup',{planId:plan.plan_id});
          const after=await window.__TAURI_INTERNALS__.invoke('list_reports',{kind:null,limit:100});
          return {reports:after.length,preserved:before.length===after.length,protected:Number(plan.protected_counts.kept_reports||0)>=before.length};
        })()""")
        ok = result["reports"] >= 2 and result["preserved"] and result["protected"]
        print(f"REPORT_CACHE_SAFETY_PASS={int(ok)} REPORT_COUNT={result['reports']}")
    else:
        result = page.eval("""(async()=>{
          const reports=await window.__TAURI_INTERNALS__.invoke('list_reports',{kind:null,limit:100});
          const schedule=await window.__TAURI_INTERNALS__.invoke('get_report_schedule');
          const proposals=await window.__TAURI_INTERNALS__.invoke('list_recent_ai_proposals',{cutoffCreatedAt:Math.floor(Date.now()/1000)-7*86400,status:null,limit:500});
          return {reports:reports.length,schedule:schedule.weekly_weekday===4&&schedule.weekly_hour===16&&schedule.weekly_minute===15,proposals:proposals.length};
        })()""")
        ok = result["reports"] >= 2 and result["schedule"] and result["proposals"] >= 2
        print(f"REPORT_RESTART_PERSISTENCE_PASS={int(ok)} REPORT_COUNT={result['reports']} PROPOSAL_COUNT={result['proposals']}")
    page.ws.close()
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
