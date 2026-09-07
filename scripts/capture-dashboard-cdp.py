"""隔离 Dashboard 视觉证据截图，不读取工作文件正文。"""
import base64, json, os, time, urllib.request, websocket

port=int(os.environ.get("MSL_CDP_PORT","9341")); out=os.path.abspath(".test-runtime/luna-ai-secretary/artifacts")
os.makedirs(out, exist_ok=True)
page=next(i for i in json.load(urllib.request.urlopen(f"http://127.0.0.1:{port}/json")) if i.get("type")=="page")
ws=websocket.create_connection(page["webSocketDebuggerUrl"],timeout=20,suppress_origin=True); seq=0
def call(method,params=None):
 global seq;seq+=1; ident=seq;ws.send(json.dumps({"id":ident,"method":method,"params":params or {}}))
 while 1:
  m=json.loads(ws.recv())
  if m.get("id")==ident:return m.get("result",{})
def ev(expr):
 return call("Runtime.evaluate",{"expression":expr,"returnByValue":True}).get("result",{}).get("value")
ev('document.querySelector("[data-testid=nav-today]")?.click()');time.sleep(.4)
for language in ("zh-CN","en-US"):
 if language=="en-US":ev('document.documentElement.lang==="zh-CN"&&document.querySelector(".locale-button")?.click()')
 else:ev('document.documentElement.lang==="en-US"&&document.querySelector(".locale-button")?.click()')
 time.sleep(.3)
 for width,height in ((1024,640),(1440,900)):
  call("Emulation.setDeviceMetricsOverride",{"width":width,"height":height,"deviceScaleFactor":1,"mobile":False});time.sleep(.2)
  image=call("Page.captureScreenshot",{"format":"png","captureBeyondViewport":False}).get("data")
  path=os.path.join(out,f"dashboard-{language}-{width}x{height}.png")
  with open(path,"wb") as handle:handle.write(base64.b64decode(image))
  print(path)
call("Emulation.clearDeviceMetricsOverride");ws.close()
