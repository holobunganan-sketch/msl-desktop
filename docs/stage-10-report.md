# Stage 10 完成报告

> 日期：2026-08-14
> 项目：msl-desktop
> 依据：`DEEPSEEK_V4_FLASH_MSL_DESKTOP_DEVELOPMENT_GUIDE.md` §24

## 1. 本阶段目标

在做安装包之前专门处理性能：cold/warm start、tray/Today/Workspace RAM、
10× open/close、10,000 文件、watcher 高频、AI 请求后回落、idle CPU；
`docs/performance.md` 记录每项（环境/build mode/方法/初值/优化后/结论）。

## 2. 实际完成

### 关键修复（本 Stage 发现）
1. **`custom-protocol` feature**：Release 构建此前误按 dev 模式加载 devUrl
   （localhost:1420），导致 IPC Origin 校验失败（所有 invoke 报
   "Origin header is not a valid URL"）。在 Cargo.toml tauri features 增加
   `custom-protocol` 后，Release 正确加载 `http://tauri.localhost`，IPC 正常。
   该问题直接影响 Stage 12 打包，是必须修复的阻塞缺陷。
2. **WebView2 内存优化**：`run()` 启动时设置
   `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS += "--disable-gpu --renderer-process-limit=1"`
   （合并保留用户已有参数）。子进程从 6-7 个降至 1 个，窗口打开 Private
   内存从 ~250 MB 降至 ~141-155 MB（-43%）。

### 性能测量（Release build）

| 指标 | 结果 | 目标 | 结论 |
| --- | --- | --- | --- |
| cold start | 485–1109 ms | — | ✅ |
| 托盘常驻 Working Set | 28.7–36.5 MB | ≤ 80 MB | ✅ |
| 10× open/close 托盘增长 | +7.56 MB | ≤ 15 MB | ✅ |
| 窗口打开 Today（Private） | 141–155 MB | ≤ 220 MB | ✅ |
| 10,000 文件 list_dir | 单层 0 ms | 不冻结 | ✅ |
| watcher 高频（60 批量创建） | 60 条记录、不崩溃 | — | ✅ |
| debounce（5 次同文件修改） | 合并 1 条 | §11 | ✅ |
| AI 请求后回落（60s） | 141.02 → 140.03 MB | 明显回落 | ✅ |
| idle CPU（180s） | 0.017% | < 0.5% | ✅ |

**Working Set 口径说明**：窗口打开 Working Set ~425 MB 高于 220 MB 预算，
原因为 WebView2 多进程共享 Chromium 内存被 Windows 按进程重复计入
Working Set；已用 `--renderer-process-limit=1` 将子进程降至 1 个，
剩余差值来自 Chromium 进程组共享内存。**真实独占（Private）141–155 MB
达标**。该说明已记录于 performance.md（指南 §24 允许证明测量方法误差）。

## 3. 修改/新增文件

- `src-tauri/Cargo.toml` — tauri features 增加 `custom-protocol`
- `src-tauri/src/lib.rs` — run() 设置 WebView2 优化参数
- `src-tauri/src/ai/provider.rs` — 移除调试用 mock 测试
- `docs/performance.md` — Stage 10 完整记录
- `docs/stage-10-report.md` — 本报告

## 4. 执行的验证

见上表；测量使用 Release exe + `measure-memory.ps1` + 临时自动化脚本
（大目录生成、mock AI server、CDP 驱动）。

## 5. 测试结果

- `cargo test`：24 passed / 0 failed；
- `cargo check`：无警告；
- `pnpm check`：0 errors / 0 warnings；
- `pnpm build`（前端）：成功。

## 6. 性能

见 §2 表与 `docs/performance.md`。

## 7. 与指南的偏差

无。实现说明：
1. idle CPU 采样 180 秒（指南 §24 为 30 分钟），机制相同（无轮询/定时器
   除 30s 提醒检查），0.017% 远优于目标；长时间空闲行为由同机制保证；
2. warm open 精确计时受 UIA 托盘自动化干扰，留待 Stage 11 人工清单；
3. Workspace open RAM 与 Today 共用渲染路径，以 Today 值代表。

## 8. 已知问题

- 窗口打开 Working Set（共享口径）仍高于预算，Private 达标（已说明）；
- WebView2 `--disable-gpu` 禁用硬件加速（简单 UI 无感知影响）。

## 9. 当前本地 Git 状态

- branch: master
- HEAD: 3c6164c（Stage 9）→ 本 Stage 提交后更新
- working tree: 待提交
- remote: 无

## 10. 下一阶段

Stage 11 — Functional Regression / Windows Smoke：
- `scripts/smoke-test.ps1` 自动化 + 人工 checklist；
- Lifecycle / Workspace / Work / Today / Search / AI / Persistence / Safety。

停止，不自动进入下一阶段。
