#!/usr/bin/env python3
"""隔离 WebView2 UI smoke：只通过 DOM/键盘操作，不读取工作文件正文或真实密钥。"""
import json
import os
import sys
import time
import urllib.request

import websocket

PORT = int(os.environ.get("MSL_CDP_PORT", "9333"))


class CdpPage:
    def __init__(self):
        pages = json.load(urllib.request.urlopen(f"http://127.0.0.1:{PORT}/json", timeout=10))
        page = next(item for item in pages if item.get("type") == "page")
        # suppress_origin avoids sending an Origin header; keep the debug gate narrow.
        self.ws = websocket.create_connection(page["webSocketDebuggerUrl"], timeout=20, suppress_origin=True)
        self.seq = 0
        self.errors = []
        self.ws.send(json.dumps({"id": 1, "method": "Runtime.enable"}))
        self.seq = 1

    def recv_until(self, wanted):
        while True:
            message = json.loads(self.ws.recv())
            if message.get("method") == "Runtime.exceptionThrown":
                self.errors.append(message.get("method"))
            if message.get("method") == "Runtime.consoleAPICalled" and message.get("params", {}).get("type") == "error":
                self.errors.append(message.get("method"))
            if message.get("id") == wanted:
                return message

    def eval(self, expression):
        self.seq += 1
        ident = self.seq
        self.ws.send(json.dumps({"id": ident, "method": "Runtime.evaluate", "params": {"expression": expression, "awaitPromise": True, "returnByValue": True}}))
        response = self.recv_until(ident)
        result = response.get("result", {}).get("result", {})
        if "exceptionDetails" in response.get("result", {}):
            raise RuntimeError(str(response["result"]["exceptionDetails"]))
        return result.get("value")

    def click_testid(self, testid):
        return self.eval(f"(()=>{{const e=document.querySelector('[data-testid=\\\"{testid}\\\"]'); if(!e) return false; e.click(); return true;}})()")

    def click_text(self, text):
        encoded = json.dumps(text)
        return self.eval(f"(()=>{{const es=[...document.querySelectorAll('button')]; const e=es.find(x=>x.textContent.trim().includes({encoded})); if(!e) return false; e.click(); return true;}})()")

    def fill(self, selector, value):
        encoded = json.dumps(value)
        return self.eval(f"(()=>{{const e=document.querySelector({json.dumps(selector)}); if(!e) return false; const s=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value')?.set || Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype,'value')?.set; if(s) s.call(e,{encoded}); else e.value={encoded}; e.dispatchEvent(new Event('input',{{bubbles:true}})); e.dispatchEvent(new Event('change',{{bubbles:true}})); return true;}})()")

    def key(self, key):
        encoded = json.dumps(key)
        return self.eval(f"document.activeElement?.dispatchEvent(new KeyboardEvent('keydown',{{key:{encoded},bubbles:true}}))")

    def text(self, selector):
        return self.eval(f"document.querySelector({json.dumps(selector)})?.textContent || ''")

    def wait(self, seconds=0.35):
        time.sleep(seconds)


def main():
    page = CdpPage()
    page.wait(1.0)
    if page.eval("document.documentElement.lang") != "zh-CN":
        page.eval("document.querySelector('.locale-button')?.click()")
        page.wait()
    results = []

    def check(name, condition, detail=""):
        ok = bool(condition)
        results.append(ok)
        print(f"[{ 'PASS' if ok else 'FAIL' }] {name}{(' — ' + detail) if detail else ''}")

    check("Chinese default", page.eval("document.documentElement.lang") == "zh-CN")
    check("empty Work validation", page.click_testid("nav-works"))
    page.wait(); check("open Work form", page.click_testid("work-create")); page.wait()
    page.click_testid("work-save"); page.wait()
    check("empty Work error visible", bool(page.eval("!!document.querySelector('[role=alert],.status.error')")))
    page.key("Escape"); page.wait()

    page.click_testid("work-create"); page.wait(); page.fill("#new-work-title", "UI 冒烟工作"); page.click_testid("work-save"); page.wait()
    check("valid Work created", "UI 冒烟工作" in (page.text("body") or ""))

    page.click_testid("nav-plan"); page.wait(); page.click_testid("task-create"); page.wait(); page.fill("#task-title", "UI 冒烟任务"); page.click_testid("task-save"); page.wait(); check("Task form submits", "UI 冒烟任务" in (page.text("body") or ""))
    page.click_testid("nav-waiting"); page.wait(); page.click_testid("waiting-create"); page.wait(); page.fill("#waiting-title", "UI 冒烟等待"); page.fill("#waiting-for", "测试联系人"); page.click_testid("waiting-save"); page.wait(); check("Waiting form submits", "UI 冒烟等待" in (page.text("body") or ""))
    page.click_testid("nav-calendar"); page.wait(); page.click_testid("calendar-create"); page.wait(); page.fill("#calendar-title", "UI 冒烟日程"); page.click_testid("calendar-save"); page.wait(); check("Calendar form submits", "UI 冒烟日程" in (page.text("body") or ""))

    page.eval("(()=>{const e=document.querySelector('[data-testid=\\\"quick-capture\\\"]'); e.focus(); const s=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set; s.call(e,'UI 冒烟 Inbox'); e.dispatchEvent(new Event('input',{bubbles:true})); e.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true})); return true;})()"); page.wait(1.0); check("Quick Capture same-page refresh", page.click_testid("nav-inbox")); page.wait(1.0)
    page.click_testid("nav-inbox"); page.wait(1.0); check("Inbox visible", bool(page.eval("!!document.querySelector('.inbox')"))); check("Inbox conversion entry", bool(page.eval("(()=>{const e=document.querySelector('[data-testid^=\\\"inbox-task-\\\"]'); if(!e) return false; e.click(); return true;})()"))); page.wait()

    page.eval("document.querySelector('.locale-button')?.click()"); page.wait(); check("English toggle", page.eval("document.documentElement.lang") == "en-US"); page.eval("document.querySelector('.locale-button')?.click()"); page.wait(); check("Chinese toggle back", page.eval("document.documentElement.lang") == "zh-CN")
    page.click_testid("nav-today"); page.wait(1.0); check("Dashboard counters", bool(page.eval("!!document.querySelector('.metric-strip button')"))); check("local Brief generates", page.click_text("生成今日简报") or page.click_text("重新生成") or page.click_text("Generate today’s brief") or page.click_text("Regenerate")); page.wait(1.0); check("Brief content non-empty", len(page.text(".brief-summary") or "") > 0)

    check("no unexpected window errors", not page.errors, ",".join(page.errors[:3]))
    page.ws.close()
    print(f"UI_SMOKE_PASS={sum(results)} UI_SMOKE_FAIL={len(results)-sum(results)}")
    return 0 if all(results) else 1


if __name__ == "__main__":
    sys.exit(main())
