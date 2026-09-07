#!/usr/bin/env python3
"""Capture final visual-gate evidence from the isolated release WebView."""
import base64
import json
import os
import time
import urllib.request
import websocket

PORT = int(os.environ.get("MSL_CDP_PORT", "9334"))
ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
OUT = os.path.join(ROOT, ".test-runtime", "luna-ai-secretary", "artifacts", "ui-c1")


class Page:
    def __init__(self):
        page = next(x for x in json.load(urllib.request.urlopen(f"http://127.0.0.1:{PORT}/json")) if x.get("type") == "page")
        self.ws = websocket.create_connection(page["webSocketDebuggerUrl"], timeout=30, suppress_origin=True)
        self.seq = 0

    def call(self, method, params=None):
        self.seq += 1
        ident = self.seq
        self.ws.send(json.dumps({"id": ident, "method": method, "params": params or {}}))
        while True:
            msg = json.loads(self.ws.recv())
            if msg.get("id") == ident:
                if "error" in msg:
                    raise RuntimeError(msg["error"])
                return msg.get("result", {})

    def eval(self, expression):
        result = self.call("Runtime.evaluate", {"expression": expression, "awaitPromise": True, "returnByValue": True})
        if "exceptionDetails" in result:
            raise RuntimeError(result["exceptionDetails"])
        return result.get("result", {}).get("value")

    def click_testid(self, value):
        return self.eval(f"(()=>{{const e=document.querySelector('[data-testid=\\\"{value}\\\"]');if(!e)return false;e.click();return true;}})()")

    def click_button_text(self, value):
        text = json.dumps(value)
        return self.eval(f"(()=>{{const e=[...document.querySelectorAll('button')].find(x=>x.textContent.includes({text}));if(!e)return false;e.click();return true;}})()")

    def wait(self, seconds=0.8):
        time.sleep(seconds)

    def viewport(self, width, height):
        self.call("Emulation.setDeviceMetricsOverride", {"width": width, "height": height, "deviceScaleFactor": 1, "mobile": False})
        self.wait(0.3)

    def capture(self, name):
        data = self.call("Page.captureScreenshot", {"format": "png", "fromSurface": True})["data"]
        path = os.path.join(OUT, name)
        with open(path, "wb") as handle:
            handle.write(base64.b64decode(data))
        print(f"SCREENSHOT={os.path.abspath(path)}")


def main():
    os.makedirs(OUT, exist_ok=True)
    page = Page()
    page.call("Page.enable")
    page.call("Runtime.enable")
    if page.eval("document.documentElement.lang") != "zh-CN":
        page.eval("document.querySelector('.locale-button')?.click()")
        page.wait(0.5)
    page.click_testid("nav-today")
    page.wait(1.0)
    # Generate once so the brief state is visible in the evidence.
    page.click_button_text("生成今日简报") or page.click_button_text("重新生成") or page.click_button_text("Generate today's brief") or page.click_button_text("Regenerate")
    page.wait(1.2)
    page.wait(0.4)
    for width, height in ((1024, 640), (1440, 900)):
        page.viewport(width, height)
        page.capture(f"final-dashboard-zh-{width}x{height}.png")
    page.eval("document.querySelector('.locale-button')?.click()")
    page.wait(0.6)
    for width, height in ((1024, 640), (1440, 900)):
        page.viewport(width, height)
        page.capture(f"final-dashboard-en-{width}x{height}.png")
    page.eval("document.querySelector('.locale-button')?.click()")
    page.wait(0.6)
    page.click_testid("nav-works")
    page.wait(0.7)
    for width, height in ((1024, 640), (1440, 900)):
        page.viewport(width, height)
        page.capture(f"final-works-{width}x{height}.png")
    page.click_testid("work-create")
    page.wait(0.3)
    for width, height in ((1024, 640), (1440, 900)):
        page.viewport(width, height)
        page.capture(f"final-work-modal-{width}x{height}.png")
    page.eval("document.querySelector('[role=dialog] button[aria-label]')?.click()")
    page.click_testid("nav-review")
    page.wait(0.8)
    for width, height in ((1024, 640), (1440, 900)):
        page.viewport(width, height)
        page.capture(f"final-ai-review-{width}x{height}.png")
    page.click_testid("nav-reports")
    page.wait(0.8)
    for width, height in ((1024, 640), (1440, 900)):
        page.viewport(width, height)
        page.capture(f"final-reports-{width}x{height}.png")
    page.click_testid("nav-settings")
    page.wait(1.0)
    for width, height in ((1024, 640), (1440, 900)):
        page.viewport(width, height)
        page.capture(f"final-settings-{width}x{height}.png")
    page.click_testid("nav-calendar")
    page.wait(1.0)
    for width, height in ((1024, 640), (1440, 900)):
        page.viewport(width, height)
        page.capture(f"final-calendar-week-{width}x{height}.png")
    page.ws.close()


if __name__ == "__main__":
    main()
