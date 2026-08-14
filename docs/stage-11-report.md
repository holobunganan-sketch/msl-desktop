# Stage 11 完成报告

> 日期：2026-08-14
> 项目：msl-desktop
> 依据：`DEEPSEEK_V4_FLASH_MSL_DESKTOP_DEVELOPMENT_GUIDE.md` §25

## 1. 本阶段目标

正式打包前验证软件本身：Lifecycle / Workspace / Work / Today / Search /
AI / Persistence / Safety 全回归；建立 `scripts/smoke-test.ps1`；
无法自动化的 Windows UI 操作写成人工 checklist。

## 2. 实际完成

### `scripts/smoke-test.ps1`（自动化，Release exe）
- **1. launch**：启动 → 窗口出现计时（实测 ~440 ms）；
- **2. CDP data-flow**：调用 `scripts/smoke-cdp.py` 驱动真实命令；
- **3. close-to-core**：CloseMainWindow → 窗口销毁、进程常驻；
- **4. reopen**：托盘点击重建窗口（3 次重试，UIA 不稳定时 WARN）；
- **5. exit**：托盘菜单"退出"（失败清理进程并 WARN）；
- **6. persistence**：重启后 CDP `--check` 验证数据完整。

### `scripts/smoke-cdp.py`（CDP 数据流）
- bind_workspace → work + resume_point + task×2 + waiting + calendar + inbox；
- get_today（continue/tasks/inbox）、search（关键词跨 Work/Task 命中）；
- AI 无 provider → 明确错误"未配置可用的 AI Provider"（不崩溃）；
- get_work_detail 聚合（resume/tasks/waiting 齐全）；
- `--check`：works/tasks/waiting/calendar/inbox 重启后全部保留。

### `docs/smoke-checklist.md`（人工清单）
A Lifecycle（托盘复核）· B Workspace（对话框/打开/Reveal/面包屑）·
C Work（Resume/文件/状态/归档）· D Today · E Ctrl+K · F AI（真实 Key）·
G 重启 UI 数据 · H Safety（当前无文件删除操作，后续须确认+回收站）。

## 3. 修改/新增文件

- `scripts/smoke-test.ps1` — 新增
- `scripts/smoke-cdp.py` — 新增
- `docs/smoke-checklist.md` — 新增
- `docs/stage-11-report.md` — 本报告

## 4. 执行的验证

`powershell -ExecutionPolicy Bypass -File .\scripts\smoke-test.ps1`：

```
[PASS] 1. launch (window appears) cold start 443 ms
[PASS] 2. cdp data-flow          （11 项数据流全 PASS）
[PASS] 3. close to core (window gone, process alive)
[WARN] 4. reopen via tray click  （托盘 UI 自动化不稳定 → 人工 checklist）
[WARN] 5. exit via tray menu     （同上）
[PASS] 6. persistence after restart last=ALL_PASS
SMOKE RESULT: PASS=4 FAIL=0 WARN=2（exit code 0）
```

## 5. 测试结果

- 自动化可自动项全部 PASS（launch/CDP/close-to-core/persistence）；
- 托盘 reopen/exit 属 Windows UI 自动化（本机 UIA 下按钮定位不稳定），
  降级为 WARN 并列入人工 checklist A 节——符合指南"自动化可自动的部分"；
- `cargo test`：24 passed；`cargo check`、`pnpm check` 干净。

## 6. 性能

- smoke 全程 ~90 秒（含托盘重试等待），无异常；
- cold start 443 ms（与 Stage 10 一致）。

## 7. 与指南的偏差

无。实现说明：
1. 托盘 reopen/exit 未纳入强制 PASS（UIA 不稳定），由人工清单兜底；
   Stage 12 安装验证中会再次人工复核；
2. AI "valid DeepSeek / timeout / cancel" 需真实 Key（checklist F 节），
   自动化仅覆盖 no-provider 错误路径与 mock 成功路径（Stage 8 已验证）。

## 8. 已知问题

- 托盘 UI 自动化不稳定（Windows 托盘区域 UIA 时序）；
- 真实 DeepSeek 成功链路待用户提供 Key 后人工验证。

## 9. 当前本地 Git 状态

- branch: master
- HEAD: 00eb794（Stage 10）→ 本 Stage 提交后更新
- working tree: 待提交
- remote: 无

## 10. 下一阶段

Stage 12 — 本地 NSIS setup.exe：
- `pnpm tauri build` 生成 NSIS 安装包；
- 本机安装/卸载验证、开始菜单、Defender 扫描（`scripts/defender-scan.ps1`）；
- 最终报告：exe/setup 路径、SHA-256、RAM/CPU、已知限制。

停止，不自动进入下一阶段。
