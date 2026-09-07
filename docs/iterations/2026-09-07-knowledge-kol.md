# 全局连续问答与专家洞察实施记录

状态：PARTIALLY_COMPLETED（机械验收通过；最终视觉成熟度待用户审阅）。0.3.0 代码实施、发布构建、旧版卸载和新版正式安装均已完成。部署状态详见同目录 2026-09-07-reinstall-0.3.0.md。

## 准备
- Action：核对现有 0.2.2 源码、数据表、模型路由、后台任务和界面导航，保存已确认 spec 与分阶段计划。
- Expected：复用现有流程，不损失工作台实体关系，不污染正式配置。
- Observed：当前有 32 个业务表、schema 13，尚无问答/KOL 持久化；后台 JobRequest 和日历原实体投影可复用。现有大量脏修改已保留。
- Evidence：docs/superpowers/specs/2026-09-07-knowledge-kol.md；同名 plans；本轮只读源码检查。
- Verdict：PASS，开始 Task 1。

## Task 1 / 核心存储与契约
- Action：新增 schema 14、限定范围证据检索、连续问答持久化、专家交流和可编辑草稿、事务化确认、后台任务入口及两个模型路由。
- Expected：跨项目数据不串入；引文可核验；确认前不创建事项；确认重试不重复；跟进读取原实体状态。
- Observed：12 项新增 Rust 回归测试通过。新建 UI 入口测试在旧版上按预期失败。跟进列表排序错误先由测试重现，修正后通过。
- Evidence：cargo test knowledge_ --lib（12 passed）；tests/knowledge.test.mjs；scripts/knowledge-kol-cdp.py。
- Verdict：核心单元阶段 PASS；页面、Mock、迁移副本、完整回归和安装仍待完成，整体 IN_PROGRESS。

## Task 2–3 / 页面和工作流
- Action：新增双语问答及专家页面、@ 范围选择、会话删除确认、交流原话、可编辑洞察和勾选动作、会前准备、跨专家分析、秘书引用已确认交流来源；复用后台任务及原任务日历投影。
- Expected：页面切换不终止 AI；正文与引用分层显示；编辑不被自动刷新覆盖；原话与业务记录分离持久化。
- Observed：pnpm check 0 errors / 0 warnings；27 项 Node 测试通过。原生 @ 测试发现项目名称含空格无法完整匹配，已先补失败测试，再修正匹配规则，单测恢复通过。
- Evidence：tests/knowledge.test.mjs；.test-runtime/knowledge-ui2/artifacts/qa-at-debug.png；QaView、KolView、KolDraftEditor。
- Verdict：页面静态及单元检查 PASS；原生完整流程仍在验证。

## Task 4 / 全量 Rust 与迁移副本
- Action：更新旧测试中的 schema 预期为 14；对已存在的正式备份的独立副本测试升级；执行全部含 ignored 的 Rust 测试。
- Expected：全部既有实体保留，历史迁移链可重复运行，长请求不被旧 60 秒限制截断。
- Observed：172 passed / 0 failed / 0 ignored。早一轮长请求测试出现单次传输失败，完整重跑通过；未通过更改超时或删测试掩盖。原生测试与当前正式数据完全分离。
- Evidence：cargo test -- --include-ignored；.test-runtime/knowledge-kol/formal-migration-copy/msl-desktop.db。
- Verdict：PASS。原生 UI、重启、缓存及正式安装验证尚未完成，不宣称已交付。

## Task 4 / 最终代码回归与原生完整流程
- Action：修复 @ 选择过程中响应式变量提前失效；补充压缩历史和项目认知入口检索；对最终源码重新检查和构建，并使用新隔离目录从零测试。
- Expected：无浏览器异常；全局/项目问答、草稿确认、真实任务复用和缓存保护全部有效。
- Observed：pnpm check 0 errors / 0 warnings；Node 27 passed；cargo fmt --check 通过；最终 cargo test -- --include-ignored --test-threads=1 为 173 passed / 0 failed / 0 ignored。并行构建/测试曾使旧网络夹具触发 WouldBlock，停止并行构建后顺序执行全部测试通过，未删除任何测试或放宽断言。
- Observed：原生新建项目→目录索引→@ 问答→切页完成→多轮追问→错误 JSON 校正→伪造来源拒绝→失败重试→专家记录→编辑草稿→仅确认勾选动作→幂等写入→原任务日历投影→会前准备→跨专家草稿→缓存清理和源文件哈希保护通过；无 UI 异常。
- Evidence：scripts/knowledge-kol-cdp.py（无参数，knowledge-ui3）；.test-runtime/knowledge-ui3/artifacts/ 下 20 张中英文、1440/1100 大字号截图；最终 release 构建退出码 0。
- Verdict：主流程 PASS。范围补充、重启、旧页面回归及正式安装继续执行中。

## Task 4 / 范围、旧页面与重启
- Action：补测全局、单项目、多项目、无效项目范围；同名不同机构专家与双专家综合分析；检查既有页面并重启隔离发布版。
- Expected：范围不串入，身份不误合并；旧功能可达；会话、跟进与中断状态持久化。
- Observed：补充测试全部通过；旧页面脚本 63 PASS，覆盖 11 个既有视图和 26 张截图；重启检查 6 PASS。新页面及待确认动作的大字号中英文布局无横向裁切、末尾操作可达，无运行时异常。已目视查看问答、交流、洞察及确认区截图。
- Evidence：knowledge-kol-cdp.py --supplement / --reopen；natural-workflow-cdp.py --layout；.test-runtime/knowledge-ui3/artifacts/。
- Verdict：机械验收 PASS；视觉成熟度待用户审阅。

## Task 5 / 正式替换前检查
- Action：停止隔离实例；确认正式应用未运行，冷备份正式 EXE、卸载器、快捷方式和 DB/WAL/SHM 三件套；以备份的第二份副本执行升级测试。
- Expected：全部原件保留，最新未检查点数据包含在副本中，升级不丢实体。
- Observed：文件哈希逐项一致；最新正式数据库副本迁移测试 1 passed。0.3.0 安装包 SHA256 为 1B68754E33AB9549654AF14E59A4396B46BD1FC7B4AEA25C51ACDB1BF21278BB。
- Evidence：.test-runtime/reinstall-0.3.0-20260907/backup/；formal-migration-copy/；本轮替换安装报告。
- Verdict：替换前检查 PASS，进入保留数据的卸载/安装。

## Task 5 / 部署与最终验证
- Action：保留数据卸载 0.2.2、安装 0.3.0；核对安装文件、快捷方式、注册项、实际运行版本；使用隔离数据运行正式安装 EXE 的新旧页面及后台问答；停止测试并复核数据。
- Expected：用户从原入口打开已验证新版，所有正式资料保留。
- Observed：卸载和安装均退出码 0；文件/注册项/运行时均为 0.3.0；安装 EXE 除 Tauri 包类型标记外与验证构建逐字节一致。安装版重启 7 PASS、新页面 45 PASS、旧页面 63 PASS、交互 8 PASS；正式数据库三件套哈希不变；所有测试进程停止。
- Evidence：2026-09-07-reinstall-0.3.0.md；.test-runtime/reinstall-0.3.0-20260907/artifacts/；0.3.0-使用说明.md。
- Verdict：功能机械验收与正式部署 PASS。视觉审阅材料已生成并抽查；等待用户审阅，整体状态遵守 PARTIALLY_COMPLETED 上限。

## 交付范围与已知限制
- 全局工作台问答覆盖业务记录、项目关联目录中的相关文档片段与派生摘要；支持 @ 单项目/多项目、连续追问、逐项出处与资料缺口，后台运行及重启后重试。
- 专家档案、原始交流、三类交叉洞察、编辑确认、会前准备、跨专家汇总和原任务跟进均已接入 SQLite。正式事项仅在用户确认勾选动作后写入；源文件只读。
- 检索有来源和字符预算，并披露覆盖与遗漏；单项目范围暂不混入缺少项目分段标签的旧跨项目报告，查询此类全文请使用全局范围。连续历史使用最近 10 条同范围记录。
- 引用校验核对来源身份及连续原文片段，不能保证模型推断的语义正确性。医学判断仍需核对原始证据。
- 验证均使用本地 Mock，不调用真实 AI Provider，不读取真实 API Key。缓存清理保护对话、交流、草稿、已确认洞察和正式事项。

## 视觉审阅入口（均为隔离合成数据）
- `.test-runtime/reinstall-0.3.0-20260907/artifacts/installed-qa-evidence-zh-1440.png`
- `.test-runtime/reinstall-0.3.0-20260907/artifacts/installed-kol-insights-zh-1440.png`
- `.test-runtime/reinstall-0.3.0-20260907/artifacts/installed-kol-confirm-zh-1100.png`
- 同目录还保留新旧页面中英文及大字号截图。未将机械几何检查等同于最终视觉成熟度通过。
