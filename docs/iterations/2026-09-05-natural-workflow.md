# 0.2.0 自然工作流重构

状态：PARTIALLY_COMPLETED（机械验收已通过，最终视觉待用户审阅）。当前安装的旧版未修改；验证仅使用 `.test-runtime/natural-workflow`，四个用户目录及 WebView2 隔离。本轮无真实 Provider 调用。

## Task 1 导航与建议呈现 — PASS
- Action：加入目标对象导航、易读的建议预览与精确跳转。
- Expected：项目/事项 ID 不丢失；更新和新建区分；缺失必需时间提示调整。
- Observed：先观察 4 个测试因函数不存在失败，实现后 4 个通过。
- Evidence：`tests/naturalWorkflow.test.mjs`，本轮终端测试结果。
- Verdict：PASS。

## Task 2 工作流入口与带背景记录 — PASS
- Action：五个主入口、事项阶段、带项目/来源的自然语言记录、后台整理；会话内按来源保留未保存草稿。
- Expected：原话先落地，AI 确认前不改正式实体；切换页面后记录背景仍准确。
- Observed：旧版隔离 UI 缺少五入口（RED）；新版隔离 UI 五入口及事项阶段通过。Rust 记录上下文测试通过。草稿隔离测试先失败后通过。
- Evidence：`scripts/natural-workflow-cdp.py --baseline`；`db::flow::tests`；`tests/captureDrafts.test.mjs`。
- Verdict：IN_PROGRESS，待完整 Mock UI 工作流。

## Task 3 首页与决定 — PASS
- Action：简报收拢；推进与决定双主卡；建议默认预览、调整后展开表单、采纳回执和撤销。
- Expected：减少分类负担，历史待办可找回，操作结果可追踪。
- Observed：前端检查 0 errors / 0 warnings；待原生截图和确认闭环。
- Evidence：本轮 `pnpm check`；目标组件源代码。
- Verdict：IN_PROGRESS。

## Task 4 项目推进与同一事项日程 — PASS
- Action：项目重点展示目标、进展、下一步、阻塞；文件/历史折叠；给原任务安排时间，不复制日历条目。
- Expected：ID/项目归属/备注保持；确认与撤销原子生效。
- Observed：两个新增 Rust 测试及 AI 确认/撤销排期测试通过；计划 UI 闭环待验证。
- Evidence：`db::flow::tests`、`confirmed_task_schedule_is_the_same_entity_and_undo_restores_time`。
- Verdict：IN_PROGRESS。

## Task 5 回归与交付 — 机械 PASS / 视觉待审
- Action：升级数据库 schema 11 和应用版本 0.2.0，构建隔离原生测试实例。
- Expected：全部机械检查通过，截图人工审阅，不接触正式安装与源工作文件。
- Observed：首个 release/NSIS 构建完成。Rust 第二轮 139 passed / 2 failed / 2 ignored；失败为新增表后两个测试的旧表数量/迁移计数，已更新正确期望值，待重跑。
- Evidence：`src-tauri/target/release/bundle/nsis/msl-desktop_0.2.0_x64-setup.exe`；本轮终端输出。
- Verdict：IN_PROGRESS，禁止据此宣布完成。
## 机械验收补充（最终包复核前）

Task 2 / 3 / 4 的 IN_PROGRESS 记录由以下证据更新为 PASS：

- Action：使用 release 原生 WebView、Mock Provider 和合成数据完整走通记录、暂缓、确认、撤销、安排时间、完成与后续等待；在分析期间切换页面。
- Expected：相同任务身份持续保留；无确认不写正式实体；原话和归属持久化；未提交草稿不串项目；历史待处理不失联；大字体内容和按钮可滚动到达。
- Observed：完整工作流、项目上下文、草稿隔离、状态/备注保留、三条关联建议的确认、旧待处理项、日期缺失阻止写入全部 PASS。双尺寸 11 个页面的横向溢出与末尾操作可达检查 PASS；中文/英文首页与审阅检查 PASS；没有捕获到 UI 运行异常。
- Evidence：`.test-runtime/natural-workflow-3/artifacts/workflow-ui.log`；截图同目录；`scripts/natural-workflow-cdp.py`。
- Verdict：Task 2 / 3 / 4 PASS。

### 持久化、数据库、只读与缓存

- Action：结束隔离进程并重启同一 release；迁移旧正式数据库备份的专用副本；执行全部 Rust 回归。
- Expected：原任务ID、完成状态、排期、项目等待与推进记录保持；业务实体计数与外键完整；目录和缓存边界不变。
- Observed：重启检查 8 项 PASS。数据库完整性和外键检查 PASS。备份副本迁移至 schema 11 后实体计数不变。默认 Rust 套件 141 PASS / 2 ignored，两项忽略测试分别显式运行 PASS（正式数据库备份副本迁移、67秒 loopback 慢请求）；共 143 项通过。缓存路径穿越、junction、保护实体、清理后重新索引、认知外置及增量读取测试均包含在通过项中。
- Evidence：`.test-runtime/natural-workflow-3/artifacts/workflow-reopen.log`；`.test-runtime/natural-workflow/formal-migration-copy/msl-desktop.db`；本轮 cargo test 终端结果。
- Verdict：PASS。生产安装、生产数据库与工作目录均未作为运行测试目标。

### 实际检查后修正的细节

1. 审阅页多层重复标题被压缩；窄窗口先呈现当前建议，历史列表移后；辅助分析信息折叠。
2. 日历在窄窗口转为多行日期卡片，避免项目名称挤成纵向长条。
3. 后台任务入口折叠断点与导航保持一致，普通桌面宽度保留名称。
4. 原始上下文与顶栏草稿分别按项目保留，会话中切换页面不会错挂到其他项目；未保存草稿不持久写盘，退出应用后不保留。
5. 缺失字段保留“未修改”的语义，审阅准备函数不自动填入空备注。
6. 首页未来安排同时显示日期与时间，避免误看成今天。
7. 回顾页空列表原来重复呈现两张相同空卡，改为单个空状态。
8. 测试中发现的旧迁移计数、错误记忆表名、隐藏 details 子元素的可达性判断均已根据真实结构修正，未通过改业务数据绕过断言。

### 当前交付边界

运行测试只调用本地 Mock，不能据此宣称真实 AI 的内容质量或远端网络可用性已验证。大字号和内容多时允许页面自然滚动，不强行裁切进一屏。最终视觉成熟度由用户审阅截图；没有视觉确认时状态最多 PARTIALLY_COMPLETED。尚未执行安装替换。
## 最终交付结果

- Action：对最终源码再次执行前端检查、16项 Node 测试、Rust 格式检查；生成最终 release/NSIS；使用同一隔离数据复核全部页面和周报生成。
- Expected：完整保留功能与安全边界；安装包来自本次源码；单一空报告状态和实际报告均可用。
- Observed：pnpm check 为 0 errors / 0 warnings；Node 16/16 PASS；cargo fmt --check PASS；Rust 141项默认通过加2项显式忽略测试通过；pnpm build 与 pnpm tauri build PASS；最终 layout 全部 PASS；Mock 周报在切页后完成、序号内容可达、空状态消失。
- Evidence：`frontend-check.log`、`frontend-tests.log`、`workflow-ui.log`、`workflow-reopen.log`、`final-layout.log`、`report-regression.log`、`release-build.log`，均位于 `.test-runtime/natural-workflow-3/artifacts/`。
- Verdict：机械验收 PASS。视觉初检已逐张查看首页、项目、审阅（含展开操作区）、记录、等待、日历与回顾的截图；最终用户视觉批准未取得，状态保持 PARTIALLY_COMPLETED。

### 可交付安装包

路径：`src-tauri/target/release/bundle/nsis/msl-desktop_0.2.0_x64-setup.exe`  
版本：0.2.0；字节数：5,382,287；构建完成：2026-09-05 22:53（本地时间）  
SHA256：`E5178CEBC4A534DE331CA9762A4FB4DE36AF218DB61536A3387CB20CE623234C`

当前安装未替换。所有代码、既有未提交成果、测试证据保留在原项目中。未建立提交、分支或委派任务。

### 建议用户审阅的截图

- `.test-runtime/natural-workflow-3/artifacts/workflow-today-zh-1440.png`
- `.test-runtime/natural-workflow-3/artifacts/workflow-review-zh-1100.png`
- `.test-runtime/natural-workflow-3/artifacts/workflow-expanded-actions-1100.png`
- `.test-runtime/natural-workflow-3/artifacts/workflow-calendar-zh-1100.png`

本轮没有测试真实 AI 内容质量，也没有承诺任意窗口、任意文本长度都能完全无滚动显示；长内容保留自然滚动路径。
