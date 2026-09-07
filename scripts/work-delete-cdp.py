"""Isolated UI regression for deleting a Work without deleting workspace files."""
import importlib.util
import json
import os
from pathlib import Path
import time


spec = importlib.util.spec_from_file_location(
    "cdp", Path(__file__).with_name("release-functional-cdp.py")
)
cdp = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cdp)

workspace = Path(os.environ["MSL_WORKSPACE"]).resolve()
assert ".test-runtime" in workspace.parts
page = cdp.Page()
checks = []


def check(label, value):
    ok = bool(value)
    checks.append(ok)
    print(f"[{'PASS' if ok else 'FAIL'}] {label}", flush=True)


def invoke(command, **args):
    return page.eval(
        f"window.__TAURI_INTERNALS__.invoke({json.dumps(command)},{json.dumps(args)})"
    )


try:
    for _ in range(40):
        if page.eval(
            "Boolean(window.__TAURI_INTERNALS__ && document.querySelector('[data-testid=nav-works]'))"
        ):
            break
        time.sleep(0.25)

    source_folder = workspace / "delete-regression"
    source_folder.mkdir(parents=True, exist_ok=True)
    source_file = source_folder / "keep-me.txt"
    source_file.write_text("Synthetic isolated deletion regression.", encoding="utf-8")

    work = invoke("create_work", title="删除按钮隔离测试项目", status="active")
    work_id = work["id"]
    task = invoke(
        "create_task",
        workId=work_id,
        title="删除项目后仍需处理",
        priority="normal",
        dueAt=None,
    )
    invoke("create_resume_point", workId=work_id, currentState="测试", nextStep="测试", remember="")
    invoke("attach_work_folder", workId=work_id, path=str(source_folder))

    page.click("nav-works")
    page.wait(0.6)
    page.eval(
        f"[...document.querySelectorAll('.work-item')].find(e=>e.textContent.includes({json.dumps(work['title'])}))?.click()"
    )
    page.wait(0.5)
    check("Work detail exposes delete button", page.has('[data-testid="work-delete"]'))
    page.click("work-delete")
    page.wait(0.2)
    check("Delete requires a confirmation dialog", page.has('[data-testid="work-delete-confirm"]'))
    check("Confirmation explains source files are kept", page.body_has("工作区中的文件不会被删除"))

    page.eval("document.querySelector('.modal-backdrop .secondary')?.click()")
    page.wait(0.2)
    check("Cancel keeps the Work", any(row["id"] == work_id for row in invoke("list_works", status=None)))

    page.click("work-delete")
    page.wait(0.2)
    page.click("work-delete-confirm")
    page.wait(0.6)
    check("Confirmed deletion removes the Work", all(row["id"] != work_id for row in invoke("list_works", status=None)))
    saved_task = next(row for row in invoke("list_tasks", status=None, workId=None) if row["id"] == task["id"])
    check("Related task remains as a temporary item", saved_task["work_id"] is None)
    check("Workspace source file remains intact", source_file.is_file())
    check("No unexpected UI exceptions", not page.errors)
finally:
    page.ws.close()

print(f"WORK_DELETE_PASS={sum(checks)} WORK_DELETE_FAIL={len(checks)-sum(checks)}", flush=True)
raise SystemExit(0 if checks and all(checks) else 1)
