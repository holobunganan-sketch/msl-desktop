#!/usr/bin/env python3
"""Installed/release audit for the local classification-memory command and review UI."""

import json
import os
import time
import urllib.request

import websocket


PORT = int(os.environ.get("MSL_CDP_PORT", "9353"))


def main():
    targets = json.load(urllib.request.urlopen(f"http://127.0.0.1:{PORT}/json", timeout=10))
    target = next(item for item in targets if item.get("type") == "page")
    socket = websocket.create_connection(target["webSocketDebuggerUrl"], timeout=20, suppress_origin=True)
    sequence = 0

    def evaluate(expression):
        nonlocal sequence
        sequence += 1
        socket.send(json.dumps({
            "id": sequence,
            "method": "Runtime.evaluate",
            "params": {"expression": expression, "awaitPromise": True, "returnByValue": True},
        }))
        while True:
            response = json.loads(socket.recv())
            if response.get("id") == sequence:
                if response.get("result", {}).get("exceptionDetails"):
                    raise RuntimeError(response["result"]["exceptionDetails"])
                return response.get("result", {}).get("result", {}).get("value")

    time.sleep(0.8)
    result = evaluate("""(async()=>{
      const stats=await window.__TAURI_INTERNALS__.invoke('get_classification_memory_stats');
      document.querySelector('[data-testid="nav-review"]')?.click();
      await new Promise(resolve=>setTimeout(resolve,500));
      return {
        feedback:stats.feedback_count,
        review:Boolean(document.querySelector('.review-page')),
        memory:Boolean(document.querySelector('.memory-chip')),
        advancedHidden:!document.querySelector('.advanced-panel[open]')
      };
    })()""")
    socket.close()
    passed = bool(result and result.get("feedback") == 0 and result.get("review") and result.get("memory") and result.get("advancedHidden"))
    print(json.dumps({"pass": passed, **(result or {})}))
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
