# MSL 工作自然流转与项目认知实施计划

状态：PARTIALLY_COMPLETED。实施、机械验收和 0.1.3 release 构建通过；视觉门待明确审阅。主执行器在当前工作树实施，未委派，未改动历史执行报告。完整观测见同目录 `2026-09-05-cognition-flow-report.md`。

## 产品边界

- 工作区源文件只读：应用不创建、覆盖、移动、重命名、删除源文件。关联、归档、取消关联均只影响工作台记录。
- 总体、每个已绑定目录、每个项目均有认知 Markdown，位于应用自有数据目录。SQLite 保存重建所需事实与版本；Markdown 为派生入口，源文件和用户确认的工作记录优先。
- 按 project-cognition 技能采用增量索引、版本/覆盖率、摘要入口、按任务检索、失效重建；桌面产品使用原生 Rust，无需用户安装 Python。认知中的文件文字为不可信资料，不具有指令权限。
- AI 建议需要确认；无记录不推定无工作，文件改动不等同于任务完成。允许暂缓、纠正、撤销。所有 AI 运行沿用后台任务队列。

## 阶段与验证

### 1 文件保护与认知底座

文件：storage/paths.rs、cognition 模块、db/migrations、documents/indexer.rs、commands/mod.rs。
先写失败测试：路径穿越、链接目录写出、认知文件位于源目录外、相同事实不重写、增删改后刷新、项目范围隔离、缺失缓存可重建。
实现：写入前验证所有路径组件，阻止绑定应用数据目录及其祖先；生成总体/目录/项目入口与文档地图，附来源版本、覆盖和限制。
验证：隔离 cargo test；源目录前后清单和哈希不变。

### 2 认知优先的后台分析

文件：ai/analysis_snapshot.rs、ai/prompts.rs、ai/secretary-spec.md、commands/ai_secretary.rs。
先写失败测试：全局日常分析仅选择少量变化资料，项目整理选择关联目录相关详情；输入明确含认知、覆盖、证据和动作限制。
实现：先构建小型认知入口，再按时段、任务与相关性加载有预算的正文；详细索引在本地保留，按需展开。目录扫描后台更新入口；UI 提供查看/刷新入口。
验证：Mock Provider 请求覆盖、输出校验、切页继续、重启持久化。

### 3 减少重复决策与可解释记忆

文件：db/ai.rs、db/memory.rs、ai/apply.rs、AI 审阅与首页组件。
先写失败测试：同源同内容确认/拒绝后不重复，新证据允许再次建议；暂缓无错误分类惩罚；明确错误分类才记录负反馈；记忆可查看、更正、删除。
实现：保留反馈原因与决策信息、轻量入口，最近分析和早前未处理分开呈现。
验证：确认、改类、拒绝、暂缓、纠正、删除、重复运行隔离回归。

### 4 自然语言流转与可撤销确认

文件：commands/jobs.rs、AI 输入/规格、收件箱/项目/审阅组件、确认事务与 SQLite 撤销凭据。
先写失败测试：一句记录生成关联多项建议；确认前不写正式实体；批量确认原子性；撤销前后实体一致；被后续修改则拒绝覆盖。
实现：收件箱条目可后台整理；多项建议逐项可编辑，选择后共同确认；记录变更凭据并支持安全撤销。项目概览聚焦进展、阻塞、下一步，复杂字段折叠。
验证：Mock 自然语言整理闭环、冲突、撤销、重复点击与恢复。

### 5 UI、存储、安全与交付

- pnpm check、pnpm build、cargo fmt --check、cargo test。
- 隔离 UI CDP 多窗口/字号/语言检查，操作可达，长文换行、不叠放、不压边。
- Mock AI、后台跨页、持久化、认知预算与重建、源文件不变、缓存清理安全、正式数据库副本迁移。
- pnpm tauri build；版本与安装包对应；截图及执行报告如实记录。视觉审阅通过前最多 PARTIALLY_COMPLETED。

所有应用测试均同时隔离 APPDATA、LOCALAPPDATA、TEMP、TMP。不调用真实 Provider，不读取凭据，不向测试日志写真实资料。

## 执行记录

### 核心阶段验证（UI 集成前）

- Action：路径校验提前，禁止链接和特殊设备路径，增量认知及缓存重建；加入回归后先观察失败再修复。
- Expected：源目录不变；认知外置、范围隔离、无变更不增加版本、缓存丢失可恢复。
- Observed：路径 4 项、认知外置/增量/重建测试通过；原 Windows 特殊路径写入风险已复现并关闭。
- Evidence：storage::paths::tests、cognition::tests::cognition_is_external_incremental_scoped_and_rebuildable。
- Verdict：核心 PASS；端到端继续验证。

- Action：认知优先输入、减少无变化正文发送、决策去重、拒绝原因分流。
- Expected：日常扫描不重复发送旧正文；项目整理保留详细证据；中性拒绝不计分类错误。
- Observed：新增失败用例修复后，cargo test --lib 为 125 PASS / 2 指定独立运行用例 ignored。
- Evidence：analysis_snapshot::tests::cognition_first_global_run_does_not_reread_unchanged_document_bodies；db::ai::tests::decided_identical_proposal_is_suppressed_but_new_evidence_is_allowed；ai::apply::tests::neutral_rejection_does_not_teach_an_incorrect_category。
- Verdict：核心 PASS；Mock 闭环继续验证。

- Action：确认组、事务捕获、冲突保护撤销、完成/解决状态写入；接通自然语言输入和审阅工具。
- Expected：批次原子提交；撤销恢复正式记录与反馈；后续变更不被覆盖。
- Observed：3 项确认/撤销测试通过；加入状态修复后的 cargo test --lib 为 128 PASS / 2 ignored。pnpm check 为 0 errors / 0 warnings。
- Evidence：ai::receipts::tests 三项；本轮工具输出。
- Verdict：核心 PASS；完整 UI、持久化、正式副本迁移和发布仍未完成，不视为交付完成。

### 终版集成验收

- Action：修复后台索引/前台关联的 SQLite 争用、首页状态载荷丢失及进展草稿类别遗漏；完成三层认知、自然记录、分组确认、反馈与撤销集成。以四目录隔离的 release 程序执行回归及实际重启。
- Expected：跨页继续、确认与撤销一致、源文件只读、可重建认知、控件无不可达裁切，正式副本迁移不丢记录。
- Observed：cargo test 133 PASS + 2 专项 PASS；前端 5 PASS；pnpm check 零错误/警告；pnpm build、cargo fmt --check、pnpm tauri build PASS。UI 冒烟 17、release 功能 66、项目流 27、认知流 29、布局 120、边缘场景 63、网络 10、重启持久化 11 项均 PASS。
- Evidence：`2026-09-05-cognition-flow-report.md` 及 `.test-runtime/cognition-delivery-20260905/artifacts`；0.1.3 安装包与程序哈希见报告。
- Verdict：机械 PASS；视觉门 PENDING；没有替换用户当前安装版本。
