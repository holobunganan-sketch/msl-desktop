import json
import os
import re
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


PORT = int(os.environ.get("MSL_MOCK_AI_PORT", "9451"))
METRICS = {"repair_requests": 0, "focused_categories": [], "focused_documents": 0}


def request_text(payload):
    messages = payload.get("messages") or payload.get("input") or []
    for message in reversed(messages):
        if message.get("role") == "user":
            return message.get("content", "")
    return ""


def complete(payload):
    model = payload.get("model", "")
    counts=METRICS.setdefault("model_requests", {})
    counts[model]=counts.get(model,0)+1
    if model in ("synthetic-report","collab-report","collab-report-repair","collab-report-bad"):
        time.sleep(.5)
        repair="repair_report_contract" in str(request_text(payload))
        if model=="collab-report-bad" or (model=="collab-report-repair" and not repair):
            return "未经结构化的合成报告"
        if repair:
            METRICS["report_repairs"]=METRICS.get("report_repairs",0)+1
        snapshots=[]
        for message in payload.get("messages",[]):
            if message.get("role")=="user":
                try: snapshots.append(json.loads(message.get("content","")))
                except (ValueError,TypeError): pass
        snapshot=next(s for s in snapshots if "analysis" in s and "kind" in s)
        refs=snapshot["analysis"].get("source_refs",[])
        source=next((r for r in refs if r["source_type"] in ("task_completed","task_open","work","activity","inbox")),None)
        items=[]
        if source:
            for category,headline,change,impact,action in [
                ("progress","已形成下一步工作安排","合成记录中的沟通事项已整理，当前安排可供继续推进。","减少重复整理，有助于准备下一次沟通。","按已确认安排推进，变化时补充记录。"),
                ("blocker","反馈仍需后续核实","本次合成资料未提供完整反馈结果。","后续计划需随反馈调整，暂不判断事项已结束。","核实反馈内容后再确认完成状态。"),
                ("next","优先收束未完成事项","建议先处理已有安排，再检查新增事项。","让已启动的工作持续推进。","在下一周期先确认未完成事项的时间节点。")]:
                items.append(dict(category=category,project_id=None,headline=headline,change=change,impact=impact,next_action=action,certainty="inferred",horizon="next" if category=="next" else "current",evidence_refs=[source]))
        else:
            items=[dict(category="coverage",project_id=None,headline="本期可用记录不足",change="未获得足以判断业务成果的合成记录。",impact="",next_action="有工作进展时记录即可。",certainty="unknown",horizon="period",evidence_refs=[])]
        METRICS["report_has_period_changes"]=bool(snapshot.get("period_changes"))
        METRICS["report_has_weeklies"]=bool(snapshot.get("weekly_reports"))
        return json.dumps({"items":items})
    if model == "network-slow":
        time.sleep(65)
    if model.startswith("flow-"):
        time.sleep(2.5)
    if model == "flow-invalid":
        return '{"proposals":['
    if model == "flow-natural":
        snapshot=json.loads(request_text(payload))
        inbox=snapshot.get("focused_inbox")
        if not inbox:
            return json.dumps({"summary":"• 合成工作已检查","proposals":[]})
        context=inbox.get("capture_context") or {}
        METRICS["natural_context_received"]=bool(context)
        work_id=context.get("work_id")
        source=[{"source_type":"inbox","entity_id":inbox["id"]}]
        common={"work_id":work_id,"reason":"依据这次明确记录安排，待您确认","source_refs":source,"confidence":0.9}
        if context.get("entity_kind")=="task":
            proposals=[
                {**common,"kind":"task","operation":"update","target_id":context["entity_id"],"title":"合成拜访已完成","payload":{"status":"done"}},
                {**common,"kind":"waiting","operation":"create","title":"等待合成材料反馈","payload":{"waiting_for":"合成联系人","follow_up_at":int(time.time())+7200,"time_basis":"inferred","time_reason":"合成测试：预留收集资料的时间"}},
                {**common,"kind":"resume_point","operation":"create","title":"合成项目推进记录","payload":{"current_state":"沟通完成，等待补充材料","next_step":"收到材料后继续准备"}}
            ]
        else:
            proposals=[{**common,"kind":"task","operation":"create","title":"合成拜访沟通","payload":{"priority":"normal","scheduled_start":int(time.time())+3600,"due_at":int(time.time())+10800,"time_basis":"inferred","time_reason":"合成测试：根据准备工作预留一小时","notes":"合成记录整理后的下一步"}}]
        return json.dumps({"summary":"• 已整理记录中的下一步安排","proposals":proposals})
    if model == "flow-scope":
        if "repair_output_format" not in str(request_text(payload)):
            return json.dumps({"proposals":[{"kind":"work","operation":"create","title":"Synthetic invalid state","payload":{"status":"next"}}]})
        METRICS["repair_requests"] += 1
        return json.dumps({"summary":"• 合成项目归属分析完成","proposals":[{"kind":"task","operation":"create","title":f"合成归属事项 {i}","payload":{"status":"next","notes":"已核对合成资料，准备下一步沟通"},"reason":"合成测试建议，归属由用户确认","source_refs":[],"confidence":0.8} for i in range(1,7)]})
    if model == "flow-repair":
        if "repair_output_format" not in str(request_text(payload)):
            return '{"summary":"unfinished'
        METRICS["repair_requests"] += 1
        # Separate repair fixtures from previously accepted baseline proposals;
        # exact duplicate suppression is intentional product behavior.
        return json.dumps({"summary":"• Repaired synthetic analysis","proposals":[{"kind":"task","operation":"create","title":f"Repair fixture {time.time_ns()}","payload":{"priority":"normal"},"reason":"Synthetic format correction","source_refs":[],"confidence":0.9}]})
    system = payload.get("instructions", "")
    if not system:
        system = " ".join(
            str(message.get("content", ""))
            for message in payload.get("messages", [])
            if message.get("role") == "system"
        )
    if "translation engine" in system:
        try:
            source = json.loads(request_text(payload)).get("text", "")
        except (TypeError, json.JSONDecodeError):
            source = request_text(payload)
        if source.strip() == "你好":
            return "Hello"
        if source.strip().lower() == "hello":
            return "你好"
        return "Mock translation"
    if "weekly report as a numbered list" in system:
        return "完成长期项目阶段复盘\n跟进临时事项\n安排下一周期优先工作"
    if "detailed monthly report" in system:
        return "月度综合分析\n\n1. 跨项目进展与关联\n2. 历史变化、当前阻塞与下月优先级"
    if "management brief" in system:
        return "Mock secretary brief"
    if model == "flow-cognition":
        snapshot = json.loads(request_text(payload))
        METRICS["cognition_entries"] = len(snapshot.get("project_cognition", []))
        METRICS["selected_chars"] = sum(d.get("selected_chars", 0) for d in snapshot.get("documents", []))
        inbox = snapshot.get("focused_inbox")
        if inbox:
            facts = snapshot.get("brief", {})
            task = next((t for t in sorted(facts.get("tasks_open", []),key=lambda t:t.get("entity_id",0),reverse=True) if "认知合成" in (t.get("title") or "")), None)
            waiting = next((w for w in sorted(facts.get("waiting", []),key=lambda t:t.get("entity_id",0),reverse=True) if "认知合成" in (w.get("title") or "")), None)
            work_id = task.get("work_id") if task else None
            source = [{"source_type": "inbox", "entity_id": inbox["id"]}]
            proposals = [{"kind": "resume_point", "operation": "create", "work_id": work_id, "title": "认知合成：已收到资料", "payload": {"current_state": "合成资料已收到", "next_step": "确认后续会议"}, "reason": "来自用户明确记录", "source_refs": source, "confidence": 0.9}]
            for kind, item, status in [("task", task, "done"), ("waiting", waiting, "resolved")]:
                if item:
                    proposals.append({"kind": kind, "operation": "update", "target_id": item["entity_id"], "work_id": work_id, "title": item["title"], "payload": {"status": status}, "reason": "合成记录明确说明已完成", "source_refs": source, "confidence": 0.9})
            return json.dumps({"summary": "• 收到资料，等待可结束\n• 后续事项请确认", "proposals": proposals})
        return json.dumps({"summary": "• 已查看认知入口\n• 暂无新的决策", "proposals": []})
    if "Analyze the imported workspace evidence" in system:
        work_match = re.search(r"focus_work_id=(\d+)", system)
        workspace_match = re.search(r"focus_workspace_id=(\d+)", system)
        target_id = int(work_match.group(1)) if work_match else None
        workspace_id = int(workspace_match.group(1)) if workspace_match else None
        if model == "flow-project":
            snapshot = json.loads(request_text(payload))
            focus = snapshot.get("focused_work") or {}
            METRICS["focused_categories"] = [key for key in ("tasks", "waiting", "calendar", "resume_history") if focus.get(key)]
            METRICS["focused_documents"] = len(snapshot.get("documents", []))
            proposals = []
            for kind, title, fields in [
                ("task", "准备项目下一阶段沟通", {"notes": "来自合成项目证据", "priority": "normal"}),
                ("waiting", "等待项目方案反馈", {"waiting_for": "项目团队"}),
                ("calendar", "项目阶段复盘", {"start_at": int(time.time()) + 86400, "kind": "meeting"}),
                ("resume_point", "整理项目推进节点", {"current_state": "阶段资料已齐备", "next_step": "沟通下一阶段", "remember": "先确认建议"}),
            ]:
                proposals.append({"kind": kind, "operation": "create", "work_id": target_id, "title": title, "payload": fields, "reason": "合成项目跨类型分析建议", "source_refs": [], "confidence": 0.8})
            return json.dumps({"summary": "• 项目近期事项已整理\n• 请审阅下一阶段行动", "proposals": proposals})
        return json.dumps({
            "summary": "• Mock workspace organization completed",
            "proposals": [{
                "kind": "work",
                "operation": "update" if target_id else "create",
                "target_id": target_id,
                "work_id": target_id,
                "workspace_id": workspace_id,
                "title": "Mock AI organized work",
                "payload": {
                    "title": "Mock AI organized work",
                    "summary": "Synthetic workspace-derived project structure",
                },
                "reason": "Synthetic workspace organization decision",
                "source_refs": [],
                "confidence": 0.9,
            }],
        })
    if "Synthesize changes" in system or "Synthesize every supplied source" in system:
        return json.dumps({
            "summary": "Mock analysis completed (source_type=activity, entity_id=1)",
            "proposals": [
                {
                    "kind": "task",
                    "operation": "create",
                    "title": "Mock temporary task",
                    "payload": {"priority": "normal", "notes": "Synthetic task without duplicated title"},
                    "reason": "Synthetic latest-run decision",
                    "source_refs": [],
                    "confidence": 0.9,
                },
                {
                    "kind": "waiting",
                    "operation": "create",
                    "title": "Mock waiting item",
                    "payload": {"waiting_for": "Mock reviewer"},
                    "reason": "Synthetic classification decision",
                    "source_refs": [],
                    "confidence": 0.8,
                },
                {
                    "kind": "work",
                    "operation": "create",
                    "title": "Mock rejected project",
                    "payload": {"summary": "Synthetic negative classification example"},
                    "reason": "Synthetic rejection memory",
                    "source_refs": [],
                    "confidence": 0.7,
                },
            ],
        })
    return "OK"


class Handler(BaseHTTPRequestHandler):
    def send_json(self, status, payload):
        body = json.dumps(payload).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        if self.path == "/metrics":
            self.send_json(200, METRICS)
        elif self.path.endswith("/models"):
            self.send_json(200, {"data": [{"id": "mock-secretary"}]})
        else:
            self.send_json(404, {"error": {"message": "not found"}})

    def do_POST(self):
        size = int(self.headers.get("Content-Length", "0"))
        payload = json.loads(self.rfile.read(size) or b"{}")
        if self.path == "/network-unauthorized":
            self.send_json(401, {"error": "synthetic-work-document"})
            return
        text = complete(payload)
        if self.path.endswith("/responses"):
            self.send_json(
                200,
                {
                    "id": "mock-response",
                    "model": payload.get("model"),
                    "output_text": text,
                    "output": [{"content": [{"type": "output_text", "text": text}]}],
                },
            )
        elif self.path.endswith("/messages"):
            self.send_json(
                200,
                {
                    "id": "mock-message",
                    "model": payload.get("model"),
                    "content": [{"type": "text", "text": text}],
                },
            )
        else:
            self.send_json(
                200,
                {
                    "id": "mock-chat",
                    "model": payload.get("model"),
                    "choices": [{"message": {"role": "assistant", "content": text}}],
                },
            )

    def log_message(self, *_):
        return


ThreadingHTTPServer(("127.0.0.1", PORT), Handler).serve_forever()
