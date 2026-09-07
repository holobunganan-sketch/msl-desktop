# MSL Desktop Workbench Rebuild Report

日期：2026-08-14  
范围：本地 Dashboard、结构化 CRUD、Workspace metadata reconcile、Brief v2、双语界面与 release 交付。

## 本次实现

- 重建统一中文优先的应用壳、侧栏、顶栏、design tokens、共享 Button/Card/Modal/Form/Toast/ConfirmDialog。
- 将 Today 重建为 Dashboard：四个指标卡、继续推进 Work、今日时间线、Waiting、Inbox、最近文件变化、Brief 范围选择与来源 chips。
- Work、Task、Waiting、Calendar、Inbox conversion、Quick Capture 均使用真实 IPC 调用、校验、loading、错误保留输入、成功 Toast 和 revision 刷新。
- Calendar 支持 Day/Week、编辑、删除、全天/非全天、类型、地点和 Work 关联。
- Workspace 只扫描文件元数据；首次绑定生成 quiet baseline，离线新增/修改/删除在启动 reconcile 后进入 Activity 和 Brief。
- Brief 使用 typed snapshot，覆盖 Work、Resume、Task、Waiting、Calendar、Inbox、Activity 和 `file.*` 变化；无 AI 时确定性本地摘要，AI 失败安全回退。
- Settings、Workspace、Search 与主要导航统一中英文；Provider 只显示 credential 状态，不显示或记录 Key 内容。
- 添加 CDP UI smoke、隔离运行脚本和最终视觉截图脚本。

## 验证结果

| 检查 | 结果 |
| --- | --- |
| `pnpm check` | 0 errors / 0 warnings |
| `cargo fmt --check` | PASS |
| `cargo test` | 34 passed / 0 failed |
| `pnpm build` | PASS |
| 隔离 debug UI smoke | 17 passed / 0 failed |
| 隔离 release UI smoke | 17 passed / 0 failed |
| 三次隔离重启与离线 reconcile | PASS；created/modified/deleted 各 1 |
| 正式 DB 元数据 | 与 STEP 01 完全一致；正式应用未启动 |
| release 构建 | `msl-desktop.exe` 与 NSIS setup.exe 均生成 |

## 交付物

- Release executable：`src-tauri/target/release/msl-desktop.exe`
- NSIS installer：`src-tauri/target/release/bundle/nsis/msl-desktop_0.1.0_x64-setup.exe`
- 视觉证据：`output/playwright/final-dashboard-*.png`、`final-work-modal-*.png`、`final-calendar-week-*.png`、`final-brief-sources-*.png`
- 完整逐步证据：`docs/luna-workbench-rebuild/EXECUTION_REPORT.md`

## 当前限制与视觉门

- 应用未签名；SmartScreen 的未知发布者提示属于本地构建预期。
- 文件监听和 Brief 只使用路径、大小、修改时间及结构化记录，不读取工作文件正文。
- 真实 AI Provider 需要用户自行配置凭据；验收使用 local fallback 和 mock/错误路径，未调用真实 Provider。
- 机械验收完成后，最终状态保持 `PARTIALLY_COMPLETED`，等待用户或更高能力模型审阅截图中的视觉层级、中文自然度、信息密度与 Dashboard 成熟度。
