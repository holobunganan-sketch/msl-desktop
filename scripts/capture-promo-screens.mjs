/**
 * Capture three public-facing product screenshots from an explicitly isolated,
 * synthetic MSL Desktop profile. It never opens the user's normal database.
 */
import { mkdir, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

if (process.env.MSL_PROMO_CAPTURE !== "1") {
  throw new Error("Set MSL_PROMO_CAPTURE=1 to confirm an isolated promotional capture.");
}

const port = Number(process.env.MSL_CDP_PORT ?? "9462");
const output = resolve(process.env.MSL_ARTIFACTS ?? ".test-runtime/promo-demo/artifacts");
const targets = await (await fetch(`http://[::1]:${port}/json`)).json();
const target = targets.find((item) => item.type === "page");
if (!target?.webSocketDebuggerUrl) throw new Error("No MSL Desktop debug page is available.");

const socket = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((resolveSocket, rejectSocket) => {
  socket.addEventListener("open", resolveSocket, { once: true });
  socket.addEventListener("error", rejectSocket, { once: true });
});
let sequence = 0;
const pending = new Map();
socket.addEventListener("message", ({ data }) => {
  const message = JSON.parse(String(data));
  const resolvePending = pending.get(message.id);
  if (resolvePending) {
    pending.delete(message.id);
    resolvePending(message);
  }
});

function command(method, params = {}) {
  const id = ++sequence;
  socket.send(JSON.stringify({ id, method, params }));
  return new Promise((resolveCommand, rejectCommand) => {
    pending.set(id, (message) => {
      if (message.error) rejectCommand(new Error(message.error.message));
      else resolveCommand(message.result ?? {});
    });
  });
}

async function evaluate(expression) {
  const result = await command("Runtime.evaluate", {
    expression,
    awaitPromise: true,
    returnByValue: true,
  });
  if (result.exceptionDetails) throw new Error(result.exceptionDetails.text ?? "Page evaluation failed");
  return result.result?.value;
}

async function invoke(name, args = {}) {
  return evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(name)}, ${JSON.stringify(args)})`);
}

function sleep(ms) {
  return new Promise((done) => setTimeout(done, ms));
}

async function capture(name) {
  const { data } = await command("Page.captureScreenshot", { format: "png", captureBeyondViewport: false });
  await writeFile(resolve(output, name), Buffer.from(data, "base64"));
}

await command("Runtime.enable");
await command("Page.enable");
await command("Emulation.setDeviceMetricsOverride", {
  width: 1440,
  height: 1000,
  deviceScaleFactor: 1,
  mobile: false,
});
await mkdir(output, { recursive: true });

const existing = await invoke("list_works", { status: null });
if (existing.some((work) => !work.title.startsWith("演示 · "))) {
  throw new Error("Refusing to capture: the connected profile contains non-demo project data.");
}

let projects = existing;
if (!projects.length) {
  const primary = await invoke("create_work", { title: "演示 · 慢病医学教育项目", status: "active" });
  const secondary = await invoke("create_work", { title: "演示 · 专家沟通计划", status: "active" });
  const now = Math.floor(Date.now() / 1000);
  await invoke("create_resume_point", {
    workId: primary.id,
    currentState: "本周资料框架已梳理完成，正在汇总沟通重点。",
    nextStep: "确认本周专家沟通安排，并补充会议资料。",
    remember: "优先保留需要确认的证据问题。",
  });
  await invoke("create_task", {
    workId: primary.id,
    title: "确认本周资料梳理重点",
    priority: "high",
    dueAt: now + 86400,
    notes: "演示数据：用于展示事项与项目的关联。",
  });
  await invoke("create_task", {
    workId: secondary.id,
    title: "准备专家沟通提纲",
    priority: "normal",
    dueAt: now + 2 * 86400,
    notes: "演示数据：梳理证据需求与后续问题。",
  });
  await invoke("create_waiting", {
    workId: primary.id,
    title: "等待活动时间确认",
    waitingFor: "协作方",
    followUpAt: now + 2 * 86400,
    notes: "演示数据：确认后再安排日历。",
  });
  await invoke("create_calendar_event", {
    workId: primary.id,
    title: "项目沟通会",
    startAt: now + 3 * 86400,
    endAt: null,
    allDay: false,
    kind: "meeting",
    location: null,
    notes: "演示数据：项目阶段沟通。",
  });
  await invoke("create_inbox_item", {
    content: "演示记录：整理近期反馈，判断是否需要纳入项目计划。",
  });
  projects = [primary, secondary];
}

await evaluate("document.documentElement.dataset.fontSize='standard'");
await evaluate("document.querySelector('[data-testid=nav-today]')?.click()");
await sleep(550);
await capture("promo-today-zh.png");

await evaluate(`window.dispatchEvent(new CustomEvent('dashboard:navigate',{detail:{view:'works',id:${projects[0].id}}}))`);
await sleep(550);
await capture("promo-project-zh.png");

await evaluate("document.querySelector('[data-testid=nav-matters]')?.click()");
await sleep(550);
await capture("promo-matters-zh.png");

await command("Emulation.clearDeviceMetricsOverride");
socket.close();
console.log(`PROMO_SCREENSHOTS=${output}`);
