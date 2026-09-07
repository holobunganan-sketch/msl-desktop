# 全局问答与专家洞察 Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans to implement this plan task-by-task. 用户要求当前会话直接实施，不使用子代理，不创建提交。

**Goal:** 发布 0.3.0，提供可追溯的 @ 项目连续问答和交流驱动的专家洞察，并安全替换正式安装。
**Architecture:** 在现有 Rust/SQLite 内增加 knowledge 检索、qa 会话、kol 档案与审阅模块。复用后台 JobRequest、模型路由、工作台实体与日历投影；前端新增两个视图。
**Tech Stack:** Rust、rusqlite、serde、Tauri 2、Svelte 5、TypeScript；无外部服务依赖。
**Spec:** docs/superpowers/specs/2026-09-07-knowledge-kol.md

## Global Constraints
- 真实工作目录源文件只读；测试四目录和 WebView2 隔离、Mock Provider。
- 保留脏工作树，不提交/重置/清理，不修改历史执行报告。
- AI 生成结果为草稿；确认才写正式业务实体；版本 0.3.0。

### Task 1 — 持久化与范围隔离
Files: migrations/0014_knowledge_kol.sql; db/{migrations,mod,knowledge,qa,kol}.rs。
Interfaces: knowledge::collect(db, work_ids, query) -> EvidencePack；qa 创建/查询/占用/完成轮次；kol 创建专家、原话、草稿、确认。
- [x] RED：内存数据库断言支持 qa_sessions/qa_turns/kol_experts/kol_notes/kol_drafts/kol_insights/kol_actions/kol_projects 八表，现有 schema 应失败。
```rust
let db=Database::open_in_memory().unwrap();
let found:i64=db.conn().query_row("SELECT COUNT(*) FROM sqlite_master WHERE name IN ('qa_sessions','qa_turns','kol_experts','kol_notes','kol_drafts','kol_insights','kol_actions','kol_projects')",[],|r|r.get(0)).unwrap();
assert_eq!(found,8);
```
- [x] 追加 migration、注册新路由，更新 schema 数量断言；执行 cargo test knowledge_schema。
- [x] RED/GREEN：两个合成项目，任务及原收件箱按项目范围过滤；删除的 ID 返回错误；全局保留独立事项；credentials 不出现在 pack；来源引用原文匹配，未知引用失败。
- [x] 覆盖当前业务表与项目目录，后台只读查询计数、排序相关片段、显式遗漏；模型无任意 SQL 权限。

### Task 2 — 连续问答
Files: ai/knowledge_contract.rs、ai/qa-spec.md、commands/knowledge.rs、commands/jobs.rs、db/qa.rs、lib.rs。
Interfaces: create_qa_session、list_qa_sessions、list_qa_turns、queue_qa_question、ask_workbench job；回答 {claims:[{text,basis,citations:[{source_id,quote}]}],gaps:[]}。
- [x] RED/GREEN：同轮重复请求只认领一次；重试失败轮保留问题；同项目历史保留、换项目不泄漏；恢复进程将 running 标 interrupted。
- [x] 模型请求只含范围内证据、覆盖统计和匹配范围历史，强制事实/推断及引用校验，最多一次修复，不把坏 JSON 显示为成功。
- [x] Svelte 问答页：@ 下拉、多项目标签、会话列表、连续输入、来源展开、任务状态/重试、删除会话确认；项目页加入聚焦问答入口。

### Task 3 — KOL 整理与工作流
Files: db/kol.rs、ai/kol-spec.md、commands/knowledge.rs、KolView.svelte、KolDraft.svelte。
Interfaces: list/save experts、capture/list notes、analyze_kol job、list/review drafts、list/update insights、list followups。
- [x] RED/GREEN：记录先保存原话、AI 草稿不创建正式任务；校验三类 insight 与源记录、日期和工作归属。
- [x] RED/GREEN：确认事务生成正式 tasks/waiting/calendar 并保存引用；重复确认不重复；非法日期/项目回滚；去掉动作不落库；专家主页读取真实状态。
- [x] 专家三页签与详情、最少必填档案、自由文本记录、来源/项目选择、可编辑审阅、会前准备及问答跳转。
- [x] 将已确认 KOL 来源纳入全局问答及秘书上下文；保留多对多项目身份，按问题汇总洞察并展示来源数。

### Task 4 — 集成、持久化、安全和布局
Files: navigation.ts、aiJobs.ts、BackgroundJobs.svelte、+page.svelte、ProviderSettings.svelte、i18n、tests、scripts/knowledge-kol-cdp.py、scripts/mock-knowledge-provider.py。
- [x] 新路由配置和导航双语；后台结果可返回指定会话/专家；无固定高度裁切。
- [x] pnpm check、Node tests、cargo fmt --check、cargo test（含 ignored 专项）、pnpm build。
- [x] 全隔离原生 UI：创建项目/专家、@ 多轮、切页后台、篡改来源被拒、KOL 原话→草稿→编辑确认→项目/日历同实体、重试、重启持久化。
- [ ] 测试真实备份的独立副本迁移、SQLite integrity/foreign keys、缓存清理保留新实体与源文件；1440×1000/1100×720 大字号中英文截图逐张审查。

机械迁移、完整性、缓存及布局检查已通过；截图已保存并抽查。逐张视觉审阅和最终用户视觉门保持待完成，实施报告为 PARTIALLY_COMPLETED。

### Task 5 — 构建与正式替换
- [x] 版本同步 package.json/Cargo.toml/Cargo.lock/tauri.conf.json；pnpm tauri build；记录 installer SHA256。
- [x] 关闭精确正式 PID；冷备份 EXE、卸载器、快捷方式、DB/WAL/SHM，校验哈希。
- [x] 原卸载器 /S /UPDATE，检查旧 EXE/注册项移除及数据保留；新版 /S /D 指向原独立安装目录，禁止 /R。
- [x] 校验文件/注册/运行时版本及快捷方式；启动实际安装 EXE 的隔离配置验证；停止测试并复核正式数据不变。
- [x] 本轮 EXECUTION_REPORT 写 Action、Expected、Observed、Evidence、Verdict。部署完成与视觉审阅状态分别报告。
