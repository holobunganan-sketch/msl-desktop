#!/usr/bin/env python3
"""Isolated WebView layout audit. Reports geometry only; never prints app content."""
import json
import os
import time
import urllib.request

import websocket


PORT = int(os.environ.get("MSL_CDP_PORT", "9346"))
VIEWPORTS = [(1024, 640), (1280, 720), (1440, 900)]
VIEWS = [
    "today",
    "works",
    "plan",
    "waiting",
    "calendar",
    "inbox",
    "review",
    "reports",
    "translation",
    "workspace",
    "settings",
]


class Page:
    def __init__(self):
        targets = json.load(
            urllib.request.urlopen(f"http://127.0.0.1:{PORT}/json", timeout=10)
        )
        target = next(item for item in targets if item.get("type") == "page")
        self.ws = websocket.create_connection(
            target["webSocketDebuggerUrl"], timeout=20, suppress_origin=True
        )
        self.seq = 0

    def command(self, method, params=None):
        self.seq += 1
        ident = self.seq
        self.ws.send(json.dumps({"id": ident, "method": method, "params": params or {}}))
        while True:
            message = json.loads(self.ws.recv())
            if message.get("id") == ident:
                return message

    def eval(self, expression):
        response = self.command(
            "Runtime.evaluate",
            {"expression": expression, "awaitPromise": True, "returnByValue": True},
        )
        return response.get("result", {}).get("result", {}).get("value")


def main():
    page = Page()
    failures = []
    page.eval("document.documentElement.dataset.fontSize='xlarge'")
    for width, height in VIEWPORTS:
        page.command(
            "Emulation.setDeviceMetricsOverride",
            {
                "width": width,
                "height": height,
                "deviceScaleFactor": 1,
                "mobile": False,
            },
        )
        for view in VIEWS:
            page.eval(
                f"document.querySelector('[data-testid=\"nav-{view}\"]')?.click()"
            )
            time.sleep(0.18)
            if view == "today":
                page.eval(
                    """(()=>{const summary=document.querySelector('.brief-summary');
                    if(summary) summary.textContent=['• 过去进展：完成跨来源复盘','• 当前推进：两项工作正在推进','• 今日日程：上午沟通，下午整理材料','• 待办与等待：三项需要跟进','• 工作目录变化：两份资料已更新','• 建议推进：先处理逾期事项'].join('\\n');})()"""
                )
            result = page.eval(
                """(()=>{
                  const root=document.querySelector('.content-scroll');
                  const visible=(element)=>{
                    const style=getComputedStyle(element);
                    const rect=element.getBoundingClientRect();
                    return style.display!=='none' && style.visibility!=='hidden' && rect.width>1 && rect.height>1;
                  };
                  const clipped=[...document.querySelectorAll('button,input,select,textarea,a,[role=button]')]
                    .filter(visible)
                    .filter((element)=>{
                      const rect=element.getBoundingClientRect();
                      return rect.left < -1 || rect.right > innerWidth + 1;
                    }).length;
                  return {
                    horizontalOverflow:Boolean(root && root.scrollWidth > root.clientWidth + 2),
                    clippedControls:clipped,
                    briefClipped:Boolean(document.querySelector('.brief-summary') &&
                      document.querySelector('.brief-summary').scrollHeight >
                      document.querySelector('.brief-summary').clientHeight + 1),
                    heroControlsClipped:[...document.querySelectorAll('.brief-hero button,.brief-hero summary')]
                      .filter(visible)
                      .some((element)=>{
                        const rect=element.getBoundingClientRect();
                        const hero=document.querySelector('.brief-hero').getBoundingClientRect();
                        return rect.left < hero.left - 1 || rect.right > hero.right + 1 ||
                          rect.top < hero.top - 1 || rect.bottom > hero.bottom + 1;
                      })
                  };
                })()"""
            )
            if result["horizontalOverflow"] or result["clippedControls"]:
                failures.append(
                    f"{width}x{height} {view}: horizontal={result['horizontalOverflow']} "
                    f"clipped_controls={result['clippedControls']}"
                )
            if view == "today" and (result["briefClipped"] or result["heroControlsClipped"]):
                failures.append(
                    f"{width}x{height} today: brief_clipped={result['briefClipped']} "
                    f"hero_controls_clipped={result['heroControlsClipped']}"
                )
    page.ws.close()
    for failure in failures:
        print(f"[FAIL] {failure}")
    print(f"LAYOUT_AUDIT_PASS={0 if failures else 1} FAILURES={len(failures)}")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
