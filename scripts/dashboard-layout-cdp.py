"""隔离 Dashboard 布局机械检查：只读取 DOM metrics，不读取工作文件正文。"""
import json, os, time, urllib.request, websocket

port = int(os.environ.get("MSL_CDP_PORT", "9341"))
page = next(item for item in json.load(urllib.request.urlopen(f"http://127.0.0.1:{port}/json")) if item.get("type") == "page")
ws = websocket.create_connection(page["webSocketDebuggerUrl"], timeout=20, suppress_origin=True)
seq = 0
def call(method, params=None):
    global seq
    seq += 1
    ident = seq
    ws.send(json.dumps({"id": ident, "method": method, "params": params or {}}))
    while True:
        result = json.loads(ws.recv())
        if result.get("id") == ident:
            return result.get("result", {})
def evaluate(expr):
    return call("Runtime.evaluate", {"expression": expr, "returnByValue": True}).get("result", {}).get("value")

failures = []
for language in ("zh-CN", "en-US"):
    if evaluate("document.documentElement.lang") != language:
        evaluate("document.querySelector('.locale-button')?.click()")
        time.sleep(0.4)
    for width, height in ((1024, 640), (1240, 721), (1440, 900), (2170, 1262), (2561, 1477)):
        call("Emulation.setDeviceMetricsOverride", {"width": width, "height": height, "deviceScaleFactor": 1, "mobile": False})
        evaluate("document.querySelector('[data-testid=nav-today]')?.click()")
        time.sleep(0.8)
        metrics = evaluate("""(()=>{
          const content=document.querySelector('.content-scroll');
          const focus=document.querySelector('.focus-grid');
          const support=document.querySelector('.support-grid');
          const brief=document.querySelector('.brief-hero');
          const toolbar=document.querySelector('.brief-toolbar');
          const strip=document.querySelector('.metric-strip');
          const headings=[...document.querySelectorAll('.focus-card .card-head h2')];
          const supportText=[...document.querySelectorAll('.support-grid strong,.support-grid small,.support-grid select,.support-grid input')];
          return {
            lang:document.documentElement.lang,
            bodyW:document.body.scrollWidth,bodyCW:document.body.clientWidth,
            contentH:content?.scrollHeight||0,contentCH:content?.clientHeight||0,
            dash:Boolean(document.querySelector('.dashboard-c1')),
            brief:Boolean(brief),
            metrics:Boolean(strip),
            focus:Boolean(focus),support:Boolean(support),
            translation:Boolean(document.querySelector('.translation-column')),
            review:Boolean(document.querySelector('[data-testid=nav-review]')),
            focusHeadingMin:headings.length?Math.min(...headings.map(e=>parseFloat(getComputedStyle(e).fontSize))):0,
            supportTextMin:supportText.length?Math.min(...supportText.map(e=>parseFloat(getComputedStyle(e).fontSize))):0,
            focusLarger:Boolean(focus&&support&&focus.getBoundingClientRect().height>support.getBoundingClientRect().height),
            briefContainsToolbar:Boolean(brief&&toolbar&&brief.getBoundingClientRect().bottom>=toolbar.getBoundingClientRect().bottom),
            toolbarClearsMetrics:Boolean(toolbar&&strip&&toolbar.getBoundingClientRect().bottom+8<=strip.getBoundingClientRect().top)
          };
        })()""")
        viewport_fits = width < 1180 or metrics["contentH"] <= metrics["contentCH"] + 2
        ok = metrics and viewport_fits and metrics["bodyW"] <= metrics["bodyCW"] + 1 and metrics["dash"] and metrics["brief"] and metrics["metrics"] and metrics["focus"] and metrics["support"] and not metrics["translation"] and metrics["review"] and metrics["focusHeadingMin"] >= 18 and metrics["supportTextMin"] >= 12 and metrics["focusLarger"] and metrics["briefContainsToolbar"] and metrics["toolbarClearsMetrics"]
        print(f"[{ 'PASS' if ok else 'FAIL' }] {language} {width}x{height} metrics={metrics}")
        if not ok: failures.append((language, width, height, metrics))

    # A short window with the largest text setting is allowed to scroll, but the
    # Brief action rail must remain inside its card and clear the metric strip.
    call("Emulation.setDeviceMetricsOverride", {"width": 2048, "height": 350, "deviceScaleFactor": 1, "mobile": False})
    evaluate("document.documentElement.dataset.fontSize='xlarge';document.querySelector('[data-testid=nav-today]')?.click()")
    time.sleep(0.5)
    compact = evaluate("""(()=>{
      const content=document.querySelector('.content-scroll');
      const brief=document.querySelector('.brief-hero');
      const toolbar=document.querySelector('[data-testid=dashboard-brief-actions]');
      const strip=document.querySelector('.metric-strip');
      if(!content||!brief||!toolbar||!strip) return null;
      const briefBox=brief.getBoundingClientRect();
      const toolbarBox=toolbar.getBoundingClientRect();
      const stripBox=strip.getBoundingClientRect();
      return {
        scrollEnabled:content.scrollHeight>content.clientHeight+2,
        toolbarInside:toolbarBox.top>=briefBox.top-1&&toolbarBox.bottom<=briefBox.bottom+1,
        stripClearsToolbar:stripBox.top>=toolbarBox.bottom+8,
        horizontalOverflow:content.scrollWidth>content.clientWidth+2
      };
    })()""")
    compact_ok = compact and compact["scrollEnabled"] and compact["toolbarInside"] and compact["stripClearsToolbar"] and not compact["horizontalOverflow"]
    print(f"[{ 'PASS' if compact_ok else 'FAIL' }] {language} extra-large-text 2048x350 metrics={compact}")
    if not compact_ok: failures.append((language, 2048, 350, compact))
call("Emulation.clearDeviceMetricsOverride")
ws.close()
raise SystemExit(1 if failures else 0)
