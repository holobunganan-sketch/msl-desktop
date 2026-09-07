# MSL Desktop 秘书决策与周期报告设计

## 目标

本轮将秘书分析拆分为简洁事实摘要、最近一次分析的首页决策、最近七天的深度审阅，以及可调度的周报与月报。所有实体写入继续经过用户确认。周报和月报必须调用用户选定的大模型，失败时保留失败记录并允许重试。

## 产品规则

### 工作层级

- `Work` 表示长期项目或持续性工作。
- `Task` 与 `Waiting` 可以关联某个 Work，也可以将 `work_id` 留空并标记为临时事务。
- `Calendar` 可以关联 Work，也可以作为独立安排。
- AI 在建议 Task 或 Waiting 时必须给出推荐归属及理由；证据不足时推荐临时事务。

### 首页简报

- 首页只显示 5–7 条事实摘要。
- 展示文本清除 `source_type`、`entity_id`、`workspace_id` 等内部标记。
- 完整来源保留在“查看来源”抽屉中。
- 首页摘要不承载建议确认操作。

### 需要您决定

- 只显示最近一次已完成全局分析产生的待确认建议。
- 每行提供标题、理由、推荐去向、工作归属和确认操作。
- 支持去向：Work、Task、Waiting、Calendar、Inbox。
- Task 与 Waiting 支持选择已有 Work 或临时事务。
- Calendar 确认前必须具备日期和时间。
- 用户可确认、暂不处理、转到收件箱或进入 AI 审阅深入编辑。
- 暂不处理不会写入正式实体；原本来自收件箱的事项继续留在收件箱。

### 秘书状态

- 首页状态卡只显示监听状态、上次分析时间和当前分析周期。
- 周期可在卡片内选择 60、180、360、720 分钟或输入 30–1440 分钟的自定义值。
- 首页不显示最近分析记录。

### AI 审阅

- 默认展示最近七天的建议记录，覆盖 pending、confirmed、rejected、deferred 和 superseded。
- 支持状态与分析批次筛选。
- Pending 与 deferred 条目可修改标题、目标类型、Work 归属和结构化字段。
- Confirmed、rejected、superseded 只读展示审计信息。

## 周报与月报

### 周报

- 默认周期为生成时刻向前七天。
- 默认每周日 17:00 自动生成；星期、小时和分钟可调整。
- 支持用户输入开始与结束时间生成额外周报。
- 正文使用序号列表。
- 报告区分长期 Work 进展和临时事务，并标注长期项目名称。

### 月报

- 默认周期为上一个自然月。
- 默认每月 1 日 09:00 自动生成。
- 提供手动生成按钮和自定义起止时间。
- 输入包括周期内 Work、Resume Point、Task、Waiting、Calendar、Inbox、工作目录变化、受支持文档证据和该周期周报。
- 输出比周报更详细，执行跨项目横向关联和“历史变化—当前状态—阻塞—下一阶段”的纵向分析。

### 报告边界

- AI 翻译输入和输出不进入周报或月报。
- 报告正文、来源计数、模型、周期、状态和错误信息持久化。
- 报告可保留、重新生成和查看历史。
- 未配置报告模型或 Provider 调用失败时，报告状态为 failed；系统不得生成伪 AI 报告。

## 数据设计

新增 migration 0007：

- `ai_proposals.deferred_at INTEGER`，用于首页暂不处理。
- `reports`：`kind`、`period_start`、`period_end`、`status`、`content`、`provider_model_id`、`snapshot_hash`、`source_counts_json`、`source_report_ids_json`、`error_code`、`error_message`、`retention_state`、时间字段。
- `report_schedule_state`：周报启用状态、星期、时间、最近周期键；月报启用状态、日期、时间、最近周期键。

ProposalRepo 增加：

- 最近一次已完成分析的首页建议查询。
- 最近七天历史查询。
- 可修改 kind、work_id、title、payload 的乐观锁更新。
- deferred 状态切换。

ReportRepo 增加报告创建、完成、失败、查询、保留和重试所需方法。

## AI 与数据流

1. 调度器或手动按钮建立 AnalysisSnapshot。
2. 全局分析提示词要求事实摘要不含内部来源标记，并生成结构化建议。
3. 建议保存到确认队列。
4. 首页按最新 completed run 读取建议。
5. 用户修改目的地或 Work 归属后确认，现有事务应用层原子写入目标实体。
6. 报告生成建立 ReportSnapshot；月报附带周期内周报正文。
7. 报告调用 weekly_report 或 monthly_report 路由对应模型并持久化结果。

## 调度与并发

- 后台低频轮询统一评估全局分析、周报和月报到期状态。
- 自动运行前先持久化本次尝试键，防止同一分钟重复执行。
- running 记录存在时不重复启动相同类型任务。
- 手动报告允许自定义周期，不改变自动周期键。

## 验收

- 首页看不到内部来源标记。
- 最新分析建议可在首页改去向、改 Work 归属并确认。
- AI 审阅可查看最近七天全部状态并编辑待确认项。
- 首页秘书状态卡可直接调整周期，最近分析卡消失。
- 周报默认七天且使用序号列表。
- 月报默认上个自然月并包含周期周报证据。
- 报告生成必须经过 Mock AI 验证；不得调用真实 Provider。
- pnpm check/build、cargo fmt --check/test、隔离 UI/CDP、持久化、迁移副本和 tauri build 全部通过。

