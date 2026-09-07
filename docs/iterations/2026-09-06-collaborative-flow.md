# 协作流程、请求节约与可读报告

状态：PARTIALLY_COMPLETED（实施及机械验收通过；最终用户视觉审阅待定）。基线为已安装 0.2.0，当前脏工作树保留；本轮不启动正式数据实例、不调用真实 AI、不修改源工作文件。

## 设计与执行准备

- Action：核对源码、上一轮报告和用户本轮五项需求，形成设计和顺序实施计划。
- Expected：功能颗粒度保持，明确请求复用边界、周报内容契约和确认后日历关系。
- Observed：现有周期分析每轮请求；周报无结构验证；日历只投影预约任务。设计保留现有模型和扫描频率，新增严格命中复用及更多时间节点投影。
- Evidence：本轮 spec/plan；`commands/ai_secretary.rs`、`ai/reports.rs`、`CalendarView.svelte`。
- Verdict：PASS，开始 Task 1。

## Task 1 核心机制 — PASS（集成待核）
- Action：增加无损紧凑输入、周期事实指纹、固定单行复用元数据和设置说明。
- Expected：不丢失非空信息，跨日期/截止节点/状态/模型改变时重新分析。
- Observed：3 个新测试先按预期失败（null 未压缩、迁移表不存在、时钟漂移误判），实现后 3/3 PASS；原建议批次保留，手动分析不走复用。
- Evidence：ai::efficiency::tests，本轮 RED/GREEN 终端输出。
- Verdict：核心 PASS，新增完整基线状态测试及 Mock 请求计数在集成阶段继续核验。

## Task 2 报告契约 — 核心 PASS / 集成待核
- Action：新增版本化报告 spec、结构化结论、周期证据验证、真实项目查找、确定性编号渲染和单次校正；补入周期业务变动与进展历史。
- Expected：错误/伪造来源/不明项目/技术字段/错误周期归因不能以正常报告保存；读者看到成果、影响、下一步。
- Observed：4 个契约测试先失败，实现后报告模块 7/7 PASS。阅读分组 2 个测试先失败后补入实现；旧报告兼容。周期有缺口时展示范围说明。
- Evidence：ai::report_contract::tests、ai::reports::tests、tests/reportReading.test.mjs。
- Verdict：后端核心 PASS，背景修复与双尺寸截图待集成验证。

## Task 3 协作与时间流转 — 核心 PASS / 集成待核
- Action：增加预约/截止/等待跟进的原事项投影，推算依据标记、日期校验、按阶段指引和分别对应“只保存/交秘书整理”的回执。示例只填草稿。
- Expected：未采用建议不写入日历；已确认节点跟随原事项更新；不重复建立日历实体。
- Observed：日历投影 4 个测试及时间预览测试先 RED 后 GREEN，Node 合计 11 PASS；推算日期校验测试 RED 后 GREEN。前端检查无错误或警告。
- Evidence：tests/calendarProjection.test.mjs、tests/naturalWorkflow.test.mjs、ai::schema::tests。
- Verdict：核心 PASS，开始全量回归与原生 Mock 测试。

## 集成首轮与补充修正
- Action：运行全量 Rust/Node、隔离原生自然工作流与请求计数，并逐张检查关键截图。
- Expected：复用不丢建议；坏报告只有一次校正；记录/采用/撤销/日历跳转保持同一实体。
- Observed：首轮 Rust 发现两处旧迁移数量断言，更新为新 schema 后全量通过。153 项中 151 PASS、2 个需显式执行的测试另跑 2/2 PASS（数据库既有备份副本迁移、65 秒 Mock 响应）。Node 23/23；原生自然工作流和协作专项全部 PASS。
- Evidence：.test-runtime/collaborative-ui/artifacts/natural-workflow.log、collaborative-flow.log；原生双尺寸截图。
- Verdict：首轮 PASS。截图审查后补充“同条建议的预约与截止均应展示”；新测试 RED 后 GREEN。另增加输入截断禁止复用的保守护栏；新 Rust 测试 RED 后 GREEN。归档项目成果回顾测试 RED 后 GREEN，加入项目名称目录。现有全量结果更新为 Rust 152 PASS + 2 专项，Node 24 PASS，最终重构建和持久化待核。

## 最终验收 — 机械 PASS / 视觉待用户审阅
- Action：最终 0.2.1 release/NSIS 重构建后，在全新 collaborative-final 隔离配置中再次运行完整原生流程、专项 Mock、已填充页面布局及真实停止/重启测试；关闭本轮全部测试实例和 Mock。
- Expected：所测应用版本与安装包一致；所有功能边界保持；不接触正式运行数据和真实 Provider。
- Observed：
  - pnpm check：0 errors / 0 warnings；pnpm build、pnpm tauri build 成功。构建保留既有 SSR 未使用导入提示和 Windows 链接信息提示，不影响退出码。
  - cargo fmt --check PASS；cargo test：152 PASS、2 专项默认忽略；cargo test -- --ignored：2/2 PASS，合计 154。
  - node --test tests/*.test.mjs：24/24 PASS。
  - 原生自然工作流：108 PASS；协作专项：29 PASS；已填充页面布局：63 PASS；停止/重启后两个持久化脚本：5 + 8 PASS。
  - 无变化 interval/daily 核查均零新增请求，新增记录重新请求；手动必请求；换模型不能沿用旧模型结果；连续失败不能成为成功基线。
  - 周报正常、坏输出单次校正、再次失败停止、自定义月报融合周期变动及重叠周报均 PASS。复制正文、折叠生成设置、切页后台生成 PASS。
  - 原生 UI 验证日历中的任务截止和等待跟进均回到原事项；calendar_events 无重复复制记录；采用前无实体写入，撤销恢复原项与原话。
  - SQLite integrity_check=ok，foreign_key_check 为空；既有正式备份的独立副本迁移后 works/tasks/waiting/calendar/inbox 数量不变，原备份 SHA256 未变化。
  - 1440×1000、1100×720、大字号、中英文：首页/项目/事项各阶段/日历/回顾/翻译/工作目录/设置无横向裁切，末端操作可通过滚动到达；已查看关键实际截图。
- Evidence：
  - .test-runtime/collaborative-final/artifacts/natural-workflow.log
  - .test-runtime/collaborative-final/artifacts/collaborative-flow.log
  - .test-runtime/collaborative-final/artifacts/populated-layout.log
  - .test-runtime/collaborative-final/artifacts/reopen.log
  - .test-runtime/collaborative-final/artifacts/natural-reopen.log
  - .test-runtime/collaborative-final/artifacts/workflow-review-preview.png
  - .test-runtime/collaborative-final/artifacts/collaborative-report-final-1440.png
  - .test-runtime/collaborative-final/artifacts/collaborative-report-final-1100.png
  - .test-runtime/collaborative-flow/formal-migration-copy-final.db（源为上一轮备份，仅副本迁移）
- Verdict：机械 PASS，最终观感待用户确认，因此总体保留 PARTIALLY_COMPLETED。

## 发布产物与边界
- 安装包：src-tauri/target/release/bundle/nsis/msl-desktop_0.2.1_x64-setup.exe
- 大小：5,402,554 bytes
- SHA256：06DF6AF8BC7E1DD6235231614DDB833C2193B48D4256B0D85B22842EE3FD370A
- 保留的旧备份 SHA256：4D112506F60785CA270186C9403ED64AF26659D85CD318EEFC1781344BB75CC4。
- 本轮未卸载/替换正式 0.2.0，未启动正式数据实例；未读取真实 Key、调用真实 AI、修改源工作目录。测试证据保留在项目隔离目录。
- 字符压缩计数为输入负载的减少量，不能换算为实际 token 或账单；没有宣称固定节约比例。保留原有模型、扫描周期和信息选择颗粒度，新增历史变动证据；已知截断会禁用周期跳过。
- 来源检查验证引用存在、项目身份、时间范围和输出结构；语义真实性、实际模型文风及远端网络可用性仍需真实使用评估。截图内容全部为合成测试数据；历史“失败”记录来自刻意的错误响应测试。
- AI 排期由提示约束、时间字段校验、可见依据及用户采用共同把关；没有承诺全自动全局最优排期。采用后自动进入日历，源工作文件保持只读。
