#!/usr/bin/env python3
# smoke-cdp.py — Stage 11 CDP 数据流验证（被 smoke-test.ps1 调用）
# 依赖：websocket-client（pip install websocket-client）
#
# 默认模式：创建数据流（workspace/work/resume/task/waiting/calendar/inbox/
#   today/search/AI no-provider 错误路径），输出每步 PASS/FAIL。
# --check 模式：重启后验证数据仍在。
import json
import sys
import time
import urllib.request

import websocket

CDP_PORT = 9222


def connect():
    pages = json.load(urllib.request.urlopen(f"http://localhost:{CDP_PORT}/json"))
    page = next(p for p in pages if p.get("type") == "page")
    ws = websocket.create_connection(page["webSocketDebuggerUrl"], timeout=20)
    mid = [0]

    def invoke(cmd, args):
        mid[0] += 1
        ws.send(json.dumps({
            "id": mid[0],
            "method": "Runtime.evaluate",
            "params": {
                "expression": f"window.__TAURI_INTERNALS__.invoke('{cmd}', {json.dumps(args)})",
                "awaitPromise": True,
                "returnByValue": True,
            },
        }))
        while True:
            r = json.loads(ws.recv())
            if r.get("id") == mid[0]:
                return r.get("result", {}).get("result", {}).get("value")

    return ws, invoke


def run_flow():
    ws, invoke = connect()
    results = []

    def check(name, cond, detail=""):
        results.append((name, bool(cond), detail))
        tag = "PASS" if cond else "FAIL"
        print(f"[{tag}] {name} {detail}")

    now = int(time.time())
    day_start = now - (now % 86400)
    ws_path = fr"C:\Users\ZhouNan\AppData\Local\Temp\msl-smoke"

    # Workspace: bind（命令直接保存 + 启动 watcher）
    try:
        w = invoke("bind_workspace", {"name": "smoke-ws", "path": ws_path})
        check("bind_workspace", isinstance(w, dict) and w.get("root_path") == ws_path, f"id={w.get('id') if isinstance(w, dict) else w}")
    except Exception as e:
        check("bind_workspace", False, str(e)[:100])

    # Work + resume + tasks + waiting + calendar + inbox
    try:
        work = invoke("create_work", {"title": "冒烟测试 Work", "status": "active"})
        wid = work["id"] if isinstance(work, dict) else None
        check("create_work", wid is not None)
        rp = invoke("create_resume_point", {"workId": wid, "currentState": "冒烟进行中", "nextStep": "验证完成", "remember": "冒烟"})
        check("create_resume_point", isinstance(rp, dict) and rp.get("work_id") == wid)
        t1 = invoke("create_task", {"workId": wid, "title": "冒烟任务一", "priority": "high", "dueAt": now + 3600, "notes": None})
        t2 = invoke("create_task", {"workId": wid, "title": "冒烟任务二", "priority": "normal", "dueAt": None, "notes": None})
        check("create_task x2", isinstance(t1, dict) and isinstance(t2, dict))
        wt = invoke("create_waiting", {"workId": wid, "title": "冒烟等待", "waitingFor": "测试者", "followUpAt": now + 7200, "notes": None})
        check("create_waiting", isinstance(wt, dict) and wt.get("status") == "open")
        ev = invoke("create_calendar_event", {"workId": wid, "title": "冒烟日程", "startAt": now + 1800, "endAt": None, "allDay": False, "kind": "meeting", "location": None, "notes": None})
        check("create_calendar_event", isinstance(ev, dict) and ev.get("kind") == "meeting")
        inbox = invoke("create_inbox_item", {"content": "冒烟 Inbox 项"})
        check("create_inbox_item", isinstance(inbox, dict) and inbox.get("processed_at") is None)
    except Exception as e:
        check("work context creation", False, str(e)[:120])

    # Today
    try:
        today = invoke("get_today", {"dayStart": day_start, "dayEnd": day_start + 86400})
        check("get_today", isinstance(today, dict), f"works={len(today.get('continue_works', []))} tasks={len(today.get('today_tasks', []))} inbox={len(today.get('inbox_pending', []))}")
    except Exception as e:
        check("get_today", False, str(e)[:100])

    # Search（关键词同时出现在 Work 与 Task）
    try:
        s = invoke("search", {"query": "冒烟"})
        check("search", isinstance(s, dict) and len(s.get("works", [])) > 0 and len(s.get("tasks", [])) > 0,
              f"works={len(s.get('works', []))} tasks={len(s.get('tasks', []))}")
    except Exception as e:
        check("search", False, str(e)[:100])

    # AI：无 provider 应报明确错误（不崩溃）
    try:
        r = invoke("generate_morning_brief", {"date": time.strftime("%Y-%m-%d"),
            "yesterdayStart": day_start - 86400, "yesterdayEnd": day_start,
            "dayStart": day_start, "dayEnd": day_start + 86400, "force": False})
        check("ai no-provider error", isinstance(r, str) and "AI Provider" in r, str(r)[:80])
    except Exception as e:
        check("ai no-provider error", False, str(e)[:100])

    # Work detail 聚合
    try:
        d = invoke("get_work_detail", {"id": wid})
        check("get_work_detail", isinstance(d, dict) and d.get("latest_resume") is not None and len(d.get("tasks", [])) == 2,
              f"resume={d.get('latest_resume') is not None} tasks={len(d.get('tasks', []))} waiting={len(d.get('waiting', []))}")
    except Exception as e:
        check("get_work_detail", False, str(e)[:100])

    ws.close()
    all_pass = all(ok for _, ok, _ in results)
    print("ALL_PASS" if all_pass else "HAS_FAIL")
    return 0 if all_pass else 1


def run_check():
    ws, invoke = connect()
    results = []

    def check(name, cond, detail=""):
        results.append((name, bool(cond), detail))
        tag = "PASS" if cond else "FAIL"
        print(f"[{tag}] {name} {detail}")

    try:
        works = invoke("list_works", {"status": None})
        check("works persist", isinstance(works, list) and any("冒烟" in w.get("title", "") for w in works), f"count={len(works)}")
    except Exception as e:
        check("works persist", False, str(e)[:100])
    try:
        tasks = invoke("list_tasks", {"status": None, "workId": None})
        check("tasks persist", isinstance(tasks, list) and len(tasks) >= 2, f"count={len(tasks) if isinstance(tasks, list) else 'err'}")
    except Exception as e:
        check("tasks persist", False, str(e)[:100])
    try:
        wt = invoke("list_waiting", {"status": None, "workId": None})
        check("waiting persist", isinstance(wt, list) and len(wt) >= 1, f"count={len(wt) if isinstance(wt, list) else 'err'}")
    except Exception as e:
        check("waiting persist", False, str(e)[:100])
    try:
        ev = invoke("list_calendar_events", {"start": 0, "end": 9999999999, "workId": None})
        check("calendar persist", isinstance(ev, list) and len(ev) >= 1, f"count={len(ev) if isinstance(ev, list) else 'err'}")
    except Exception as e:
        check("calendar persist", False, str(e)[:100])
    try:
        inbox = invoke("list_inbox", {})
        check("inbox persist", isinstance(inbox, list) and len(inbox) >= 1, f"count={len(inbox) if isinstance(inbox, list) else 'err'}")
    except Exception as e:
        check("inbox persist", False, str(e)[:100])

    ws.close()
    all_pass = all(ok for _, ok, _ in results)
    print("ALL_PASS" if all_pass else "HAS_FAIL")
    return 0 if all_pass else 1


if __name__ == "__main__":
    mode = sys.argv[1] if len(sys.argv) > 1 else ""
    if mode == "--check":
        sys.exit(run_check())
    else:
        sys.exit(run_flow())
