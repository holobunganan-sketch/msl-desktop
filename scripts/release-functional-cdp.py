#!/usr/bin/env python3
"""隔离 release 功能 smoke：仅检查 DOM 状态与截图，不输出正文、Key 或 Provider 响应。"""
import base64
import json
import os
import pathlib
import time
import urllib.request

import websocket

PORT = int(os.environ.get("MSL_CDP_PORT", "9342"))
ARTIFACTS = pathlib.Path(os.environ.get("MSL_ARTIFACTS", ".test-runtime/luna-ai-secretary/artifacts"))
MOCK_AI_URL = os.environ.get("MSL_MOCK_AI_URL")
WORKSPACE = os.environ.get("MSL_WORKSPACE")


class Page:
    def __init__(self):
        pages = json.load(urllib.request.urlopen(f"http://127.0.0.1:{PORT}/json", timeout=10))
        target = next(item for item in pages if item.get("type") == "page")
        self.ws = websocket.create_connection(target["webSocketDebuggerUrl"], timeout=20, suppress_origin=True)
        self.seq = 0
        self.errors = []
        self.command("Runtime.enable")
        self.command("Page.enable")

    def command(self, method, params=None):
        self.seq += 1
        ident = self.seq
        self.ws.send(json.dumps({"id": ident, "method": method, "params": params or {}}))
        while True:
            message = json.loads(self.ws.recv())
            if message.get("method") == "Runtime.exceptionThrown":
                self.errors.append("exception")
            if message.get("method") == "Runtime.consoleAPICalled" and message.get("params", {}).get("type") == "error":
                self.errors.append("console.error")
            if message.get("id") == ident:
                return message

    def eval(self, expression):
        result = self.command("Runtime.evaluate", {"expression": expression, "awaitPromise": True, "returnByValue": True})
        value = result.get("result", {}).get("result", {})
        if "exceptionDetails" in result.get("result", {}):
            raise RuntimeError(str(result["result"]["exceptionDetails"]))
        return value.get("value")

    def click(self, testid):
        return self.eval(f"(()=>{{const e=document.querySelector('[data-testid=\\\"{testid}\\\"]'); if(!e) return false; e.click(); return true;}})()")

    def body_has(self, text):
        return text in (self.eval("document.body.innerText") or "")

    def main_has(self, text):
        encoded = json.dumps(text)
        return self.eval(
            f"(document.querySelector('.content-scroll')?.innerText || '').includes({encoded})"
        )

    def has(self, selector):
        encoded = json.dumps(selector)
        return self.eval(f"Boolean(document.querySelector({encoded}))")

    def wait(self, seconds=0.9):
        time.sleep(seconds)

    def screenshot(self, name):
        ARTIFACTS.mkdir(parents=True, exist_ok=True)
        response = self.command("Page.captureScreenshot", {"format": "png", "captureBeyondViewport": False})
        data = response.get("result", {}).get("data", "")
        path = ARTIFACTS / name
        path.write_bytes(base64.b64decode(data))
        print(f"SCREENSHOT={path.resolve()}")


def main():
    page = Page()
    page.wait(1.2)
    checks = []

    def check(label, value):
        ok = bool(value)
        checks.append(ok)
        print(f"[{ 'PASS' if ok else 'FAIL' }] {label}")

    page.click("nav-today")
    page.wait()
    check("Dashboard excludes translation module", not (page.main_has("AI 翻译") or page.main_has("AI translation")))
    check("Dashboard has latest-analysis decision card", page.has('[data-testid="latest-decision-card"]'))
    check("Dashboard has direct analysis interval control", page.has('[data-testid="dashboard-analysis-interval"]'))
    check("Dashboard omits recent-analysis history card", not page.has('.analysis-card'))
    dashboard_geometry = page.eval("(()=>{const e=document.querySelector('.content-scroll');return {innerWidth,innerHeight,bottom:e?.getBoundingClientRect().bottom,overflowY:e?getComputedStyle(e).overflowY:null,scrollHeight:e?.scrollHeight||0,clientHeight:e?.clientHeight||0}})()")
    check("Dashboard viewport is bounded and long content has a scroll path", dashboard_geometry and dashboard_geometry["bottom"] <= dashboard_geometry["innerHeight"] + 2 and (dashboard_geometry["scrollHeight"] <= dashboard_geometry["clientHeight"] + 2 or dashboard_geometry["overflowY"] in ("auto", "scroll")))
    print(f"DASHBOARD_GEOMETRY={dashboard_geometry}")
    brief_fallback = page.eval("""(async()=>{
      const now=Math.floor(Date.now()/1000);
      const start=now-86400;
      await window.__TAURI_INTERNALS__.invoke('save_ai_task_route',{taskKind:'daily_brief',providerModelId:null});
      await window.__TAURI_INTERNALS__.invoke('save_ai_task_route',{taskKind:'general',providerModelId:null});
      const result=await window.__TAURI_INTERNALS__.invoke('generate_brief',{
        date:new Date().toISOString().slice(0,10),periodStart:start,periodEnd:now,
        todayStart:start,todayEnd:now,locale:'zh-CN',force:true
      });
      return {
        fallback:Boolean(result && result.content && result.content.trim() && result.ai_used===false),
        friendlyWarning:Boolean(result && result.warning && !String(result.warning).includes('daily_brief') && !String(result.warning).includes('配置错误'))
      };
    })()""")
    check("Brief falls back locally when AI route is unavailable", brief_fallback.get("fallback"))
    check("Local Brief warning is user-facing", brief_fallback.get("friendlyWarning"))
    page.screenshot("release-dashboard.png")
    check("Translation navigation", page.click("nav-translation"))
    page.wait()
    check("Translation page", page.body_has("AI 翻译") or page.body_has("AI translation"))
    check("Large translation input", page.has('[data-testid="translation-source"]'))
    translation_failure_safe = page.eval("""(async()=>{
      await window.__TAURI_INTERNALS__.invoke('save_ai_task_route',{taskKind:'translation',providerModelId:null});
      await window.__TAURI_INTERNALS__.invoke('save_ai_task_route',{taskKind:'general',providerModelId:null});
      try {
        await Promise.race([
          window.__TAURI_INTERNALS__.invoke('translate_text',{input:'synthetic translation check',style:'written'}),
          new Promise((_,reject)=>setTimeout(()=>reject(new Error('translation-timeout')),3000))
        ]);
      } catch(error) {
        if(String(error).includes('translation-timeout')) return false;
      }
      return Boolean(document.body && document.querySelector('[data-testid="translation-source"]'));
    })()""")
    check("Translation failure stays responsive", translation_failure_safe)
    page.screenshot("release-translation.png")
    check("Review Center navigation", page.click("nav-review"))
    page.wait()
    check("Review Center content", page.body_has("AI 审阅") or page.body_has("AI Review"))
    check("Review Center has seven-day status filter", page.has('.review-filters'))
    check("Review Center shows latest analysis workflow status", page.has('[data-testid="analysis-workflow-status"]'))
    check("Review Center provides analysis action", page.has('[data-testid="review-run-analysis"]'))
    page.screenshot("release-review.png")
    check("Reports navigation", page.click("nav-reports"))
    page.wait()
    check("Reports page", page.body_has("周报与月报") or page.body_has("Weekly and monthly reports"))
    check("Weekly report control", page.has('[data-testid="generate-weekly-report"]'))
    check("Monthly report control", page.has('[data-testid="generate-monthly-report"]'))
    check("Report schedule card", page.has('[data-testid="report-schedule-card"]'))
    check("Weekly history clear control", page.has('[data-testid="clear-weekly-history"]'))
    page.screenshot("release-reports.png")
    check("Workspace navigation", page.click("nav-workspace"))
    page.wait()
    check("Workspace document status", page.body_has("工作目录") or page.body_has("Workspace"))
    page.screenshot("release-workspace.png")
    check("Settings navigation", page.click("nav-settings"))
    page.wait(1.4)
    check("DeepSeek template", page.body_has("DeepSeek"))
    check("OpenCode Go template", page.body_has("OpenCode Go"))
    check("Muse Spark shared model", page.body_has("Muse Spark 1.2 Contributor"))
    check("DeepSeek API Key input", page.has('[data-testid="template-deepseek-api-key"]'))
    check("OpenCode Go API Key input", page.has('[data-testid="template-opencode-go-api-key"]'))
    check("Dedicated Muse Responses setup", page.has('[data-testid="template-muse-api-key"]'))
    # Seed an empty-key template in the isolated database only, then verify legacy/template
    # connections expose an update field. This does not write to Windows Credential Manager.
    page.eval("""(async()=>{
      const rows=await window.__TAURI_INTERNALS__.invoke('list_provider_connections');
      if(!rows.some((row)=>row.template_kind==='deepseek')){
        await window.__TAURI_INTERNALS__.invoke('create_provider_template',{templateKind:'deepseek',apiKey:null,enabled:true});
      }
      return true;
    })()""")
    page.eval("location.reload()")
    page.wait(1.2)
    page.click("nav-settings")
    page.wait(1.2)
    check("Existing provider API Key update input", page.has('[data-testid^="connection-"][data-testid$="-api-key"]'))
    check("Analysis schedule", page.body_has("AI 秘书分析调度") or page.body_has("AI secretary schedule"))
    manual_analysis_finishes = page.eval("""(async()=>{
      await window.__TAURI_INTERNALS__.invoke('save_ai_task_route',{taskKind:'global_analysis',providerModelId:null});
      await window.__TAURI_INTERNALS__.invoke('save_ai_task_route',{taskKind:'general',providerModelId:null});
      try {
        await Promise.race([
          window.__TAURI_INTERNALS__.invoke('run_analysis_now',{trigger:'manual'}),
          new Promise((_,reject)=>setTimeout(()=>reject(new Error('analysis-timeout')),3000))
        ]);
      } catch(error) {
        if(String(error).includes('analysis-timeout')) return false;
      }
      const runs=await window.__TAURI_INTERNALS__.invoke('list_analysis_runs',{limit:1});
      return Boolean(runs[0] && runs[0].status==='failed' && runs[0].finished_at && runs[0].error_message);
    })()""")
    check("Manual analysis never remains running after configuration failure", manual_analysis_finishes)
    check("Review Center opens after a failed analysis", page.click("nav-review"))
    page.wait()
    check("Review Center explains why no new proposals were created", page.has('.workflow-status.status-failed'))
    page.click("nav-settings")
    page.wait()
    check("Storage governance", page.body_has("存储与清理") or page.body_has("Storage & cleanup"))
    check("Font size settings", page.has('[data-testid="font-size-standard"]'))
    check("Theme settings", page.has('[data-testid="theme-mist"]'))
    check("Model enable controls", page.has('[data-testid^="model-toggle-"]'))
    toggle_started = page.eval("""(async()=>{
      const button=document.querySelector('[data-testid^="model-toggle-"]');
      if(!button) return false;
      const id=Number(button.getAttribute('data-testid').replace('model-toggle-',''));
      const models=await window.__TAURI_INTERNALS__.invoke('list_provider_models',{providerId:null});
      const model=models.find((item)=>item.id===id);
      if(!model) return false;
      window.__MSL_MODEL_TOGGLE_TEST__={id,before:model.enabled};
      button.click();
      return true;
    })()""")
    check("Model toggle starts", toggle_started)
    page.wait(0.8)
    toggle_changed = page.eval("""(async()=>{
      const state=window.__MSL_MODEL_TOGGLE_TEST__;
      if(!state) return false;
      const models=await window.__TAURI_INTERNALS__.invoke('list_provider_models',{providerId:null});
      const model=models.find((item)=>item.id===state.id);
      return Boolean(model && model.enabled!==state.before);
    })()""")
    check("Model toggle persists", toggle_changed)
    page.eval("""(async()=>{
      const state=window.__MSL_MODEL_TOGGLE_TEST__;
      if(state) await window.__TAURI_INTERNALS__.invoke('set_provider_model_enabled',{id:state.id,enabled:state.before});
      return true;
    })()""")
    check("Select large font size", page.click("font-size-large"))
    check("Select sage theme", page.click("theme-sage"))
    page.wait(0.5)
    check("Appearance applies immediately", page.eval("document.documentElement.dataset.fontSize==='large' && document.documentElement.dataset.theme==='sage'"))
    page.eval("location.reload()")
    page.wait(1.2)
    check("Appearance persists after restart", page.eval("document.documentElement.dataset.fontSize==='large' && document.documentElement.dataset.theme==='sage'"))
    page.click("nav-settings")
    page.wait(0.8)
    muse_protocol = page.eval("""(async()=>{
      let rows=await window.__TAURI_INTERNALS__.invoke('list_provider_connections');
      let opencode=rows.find((row)=>row.template_kind==='opencode_go');
      if(!opencode){
        opencode=await window.__TAURI_INTERNALS__.invoke('create_provider_template',{templateKind:'opencode_go',apiKey:null,enabled:true});
      }
      const models=await window.__TAURI_INTERNALS__.invoke('list_provider_models',{providerId:opencode.id});
      const muse=models.find((model)=>model.model_id==='muse-spark-1.2-contributor');
      return Boolean(muse && muse.protocol==='responses' && muse.endpoint_path==='/responses');
    })()""")
    check("Muse uses shared OpenCode Go Responses connection", muse_protocol)
    grok_protocol = page.eval("""(async()=>{
      const rows=await window.__TAURI_INTERNALS__.invoke('list_provider_connections');
      const opencode=rows.find((row)=>row.template_kind==='opencode_go');
      if(!opencode) return false;
      const models=await window.__TAURI_INTERNALS__.invoke('list_provider_models',{providerId:opencode.id});
      const grok=models.find((model)=>model.model_id==='grok-4.6');
      return Boolean(grok && grok.protocol==='responses' && grok.endpoint_path==='/responses');
    })()""")
    check("Grok 4.6 uses documented Responses connection", grok_protocol)
    if MOCK_AI_URL:
        secretary_flow = page.eval("""(async()=>{
          let provider=null;
          try {
            provider=await window.__TAURI_INTERNALS__.invoke('save_provider_connection',{
              id:null,displayName:'Synthetic secretary',providerType:'custom',baseUrl:%s,
              legacyModel:'',templateKind:'custom',authMode:'bearer',modelsEndpoint:null,enabled:true,
              apiKey:'synthetic-secretary-key'
            });
            const model=await window.__TAURI_INTERNALS__.invoke('save_provider_model',{
              providerId:provider.id,modelId:'mock-secretary',displayName:'Mock secretary',
              protocol:'chat_completions',endpointPath:'/chat/completions',capabilitiesJson:'{}',
              source:'manual',enabled:true,available:true
            });
            for(const taskKind of ['daily_brief','global_analysis','work_draft','weekly_report','monthly_report','translation']) {
              await window.__TAURI_INTERNALS__.invoke('save_ai_task_route',{taskKind,providerModelId:model.id});
            }
            const now=Math.floor(Date.now()/1000);
            const brief=await window.__TAURI_INTERNALS__.invoke('generate_brief',{
              date:new Date().toISOString().slice(0,10),periodStart:now-86400,periodEnd:now,
              todayStart:now-86400,todayEnd:now,locale:'zh-CN',force:true
            });
            const runId=await window.__TAURI_INTERNALS__.invoke('run_analysis_now',{trigger:'manual'});
            const runs=await window.__TAURI_INTERNALS__.invoke('list_analysis_runs',{limit:10});
            const run=runs.find((item)=>item.id===runId);
            const globalProposalCount=(await window.__TAURI_INTERNALS__.invoke('list_latest_analysis_proposals',{limit:20})).length;
            const weeklyId=await window.__TAURI_INTERNALS__.invoke('generate_report',{kind:'weekly',periodStart:now-7*86400,periodEnd:now});
            const monthlyId=await window.__TAURI_INTERNALS__.invoke('generate_report',{kind:'monthly',periodStart:now-8*86400,periodEnd:now+60});
            const reports=await window.__TAURI_INTERNALS__.invoke('list_reports',{kind:null,limit:20});
            const weekly=reports.find((item)=>item.id===weeklyId);
            const monthly=reports.find((item)=>item.id===monthlyId);
            const zh=await window.__TAURI_INTERNALS__.invoke('translate_text',{input:'你好',style:'written'});
            const en=await window.__TAURI_INTERNALS__.invoke('translate_text',{input:'hello',style:'written'});
            let workDraft=false;
            if(%s){
              let workspaces=await window.__TAURI_INTERNALS__.invoke('get_workspaces');
              let workspace=workspaces.find((item)=>item.root_path===%s);
              if(!workspace){
                workspace=await window.__TAURI_INTERNALS__.invoke('bind_workspace',{name:'Synthetic workspace',path:%s});
              }
              const work=await window.__TAURI_INTERNALS__.invoke('create_work',{title:'Synthetic AI filing project',status:'active'});
              const draftRunId=await window.__TAURI_INTERNALS__.invoke('start_workspace_work_draft',{workspaceId:workspace.id,workId:work.id});
              const proposals=await window.__TAURI_INTERNALS__.invoke('list_latest_analysis_proposals',{limit:20});
              const proposal=proposals.find((item)=>item.kind==='work'&&item.target_id===work.id);
              if(proposal){
                await window.__TAURI_INTERNALS__.invoke('confirm_ai_proposal',{
                  id:proposal.id,expectedUpdatedAt:proposal.updated_at,editedPayload:JSON.parse(proposal.payload_json)
                });
                const linked=await window.__TAURI_INTERNALS__.invoke('list_work_workspaces',{workId:work.id});
                workDraft=linked.includes(workspace.id);
              }
            }
            const cleared=await window.__TAURI_INTERNALS__.invoke('clear_report_history',{kind:'weekly'});
            const afterClear=await window.__TAURI_INTERNALS__.invoke('list_reports',{kind:null,limit:20});
            return {
              brief:Boolean(brief.ai_used && brief.content==='• Mock secretary brief'),
              analysis:Boolean(run && run.status==='completed' && run.summary==='• Mock analysis completed'),
              proposals:globalProposalCount===3,
              weekly:Boolean(weekly && weekly.status==='completed' && /^1[.] /m.test(weekly.content) && /^3[.] /m.test(weekly.content)),
              monthly:Boolean(monthly && monthly.status==='completed' && JSON.parse(monthly.source_report_ids_json).includes(weeklyId)),
              translation:zh==='Hello' && en==='你好',
              workDraft,
              reportClear:cleared>=1 && !afterClear.some((item)=>item.id===weeklyId) && afterClear.some((item)=>item.id===monthlyId)
            };
          } finally {
            if(provider) await window.__TAURI_INTERNALS__.invoke('delete_provider',{id:provider.id});
          }
        })()""" % (json.dumps(MOCK_AI_URL), "true" if WORKSPACE else "false", json.dumps(WORKSPACE), json.dumps(WORKSPACE)))
        check("Mock-routed brief uses the selected provider protocol", secretary_flow.get("brief"))
        check("Manual analysis completes and records its summary", secretary_flow.get("analysis"))
        check("Latest completed analysis exposes its proposals", secretary_flow.get("proposals"))
        check("Weekly report persists numbered Mock AI output", secretary_flow.get("weekly"))
        check("Monthly report references overlapping weekly evidence", secretary_flow.get("monthly"))
        check("Translation is constrained to Chinese-English output", secretary_flow.get("translation"))
        check("Work organization enters review and links the selected workspace after confirmation", secretary_flow.get("workDraft"))
        check("Weekly history clears without removing the monthly report", secretary_flow.get("reportClear"))
        check("Review Center reopens after completed analysis", page.click("nav-review"))
        page.wait()
        check("Review Center visibly lists latest pending proposals", page.eval("document.querySelectorAll('.proposal-item').length >= 3"))
        check("Review Center uses a friendly structured form", page.has('.detail-card'))
        check("Advanced JSON stays collapsed by default", page.has('.advanced-panel:not([open])'))
        page.screenshot("release-review-with-proposals.png")
        memory_flow = page.eval("""(async()=>{
          const before=await window.__TAURI_INTERNALS__.invoke('get_classification_memory_stats');
          const pending=await window.__TAURI_INTERNALS__.invoke('list_ai_proposals',{status:'pending',limit:100});
          const rows=['Mock temporary task','Mock waiting item','Mock rejected project'].map(title=>pending.find(item=>item.title===title));
          if(rows.some(item=>!item)) return false;
          const accepted=rows[0];
          await window.__TAURI_INTERNALS__.invoke('confirm_ai_proposal',{
            id:accepted.id,expectedUpdatedAt:accepted.updated_at,
            editedPayload:JSON.parse(accepted.payload_json)
          });
          const correctedSource=rows[1];
          const corrected=await window.__TAURI_INTERNALS__.invoke('update_ai_proposal_classification',{
            id:correctedSource.id,expectedUpdatedAt:correctedSource.updated_at,kind:'task',workId:null,
            title:correctedSource.title,payload:JSON.parse(correctedSource.payload_json)
          });
          await window.__TAURI_INTERNALS__.invoke('confirm_ai_proposal',{
            id:corrected.id,expectedUpdatedAt:corrected.updated_at,
            editedPayload:JSON.parse(corrected.payload_json)
          });
          const rejected=rows[2];
          await window.__TAURI_INTERNALS__.invoke('reject_ai_proposal',{
            id:rejected.id,expectedUpdatedAt:rejected.updated_at,reason:'wrong_category'
          });
          const stats=await window.__TAURI_INTERNALS__.invoke('get_classification_memory_stats');
          return Boolean(
            stats.feedback_count===before.feedback_count+3 &&
            stats.accepted_count===before.accepted_count+1 &&
            stats.corrected_count===before.corrected_count+1 &&
            stats.rejected_count===before.rejected_count+1
          );
        })()""")
        check("Accepted, corrected, and explicit category-error reviews become classification memory", memory_flow)
        check("Work navigation", page.click("nav-works"))
        page.wait()
        check("Work page offers AI organization without manual file entry", page.has('[data-testid="organize-work-with-ai"]') and not page.has('.file-form'))
        quick_progress = page.eval("""(async()=>{
          const input=document.querySelector('[data-testid="work-quick-progress"]');
          const form=input?.closest('form');
          if(!input||!form) return false;
          const setter=Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set;
          setter.call(input,'Synthetic one-line progress');
          input.dispatchEvent(new Event('input',{bubbles:true}));
          form.requestSubmit();
          await new Promise(resolve=>setTimeout(resolve,500));
          const works=await window.__TAURI_INTERNALS__.invoke('list_works',{status:null});
          for(const work of works){
            const detail=await window.__TAURI_INTERNALS__.invoke('get_work_detail',{id:work.id});
            if(detail.latest_resume?.current_state==='Synthetic one-line progress') return true;
          }
          return false;
        })()""")
        check("One-line progress capture persists without structured form work", quick_progress)
        check("Structured progress details stay collapsed by default", page.has('[data-testid="work-progress-details"]:not([open])'))
        page.screenshot("release-work-ai.png")
        page.click("nav-settings")
        page.wait()
    page.screenshot("release-settings.png")
    check("No unexpected runtime errors", not page.errors)
    page.ws.close()
    print(f"RELEASE_FUNCTIONAL_PASS={sum(checks)} RELEASE_FUNCTIONAL_FAIL={len(checks)-sum(checks)}")
    return 0 if all(checks) else 1


if __name__ == "__main__":
    raise SystemExit(main())
