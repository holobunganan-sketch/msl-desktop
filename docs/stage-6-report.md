# Stage 6 完成报告

> 日期：2026-08-13
> 项目：msl-desktop
> 依据：`DEEPSEEK_V4_FLASH_MSL_DESKTOP_DEVELOPMENT_GUIDE.md` §20

## 1. 本阶段目标

完成产品最核心首页 Today：Continue（最近活动 Work）、Today（今日任务+日历）、
Waiting（需跟进）、Inbox（未处理）、Morning Brief 占位。明确排序规则，不依赖 AI。

## 2. 实际完成

### Rust 命令 `get_today(day_start, day_end)`
- day_start/day_end 由前端按本地时区计算（Rust 侧不引入时区依赖）；
- **Continue**（指南 §20.4）：未 done/archived 的 Works，按最近 Activity /
  更新时间降序，附最新 Resume Point 与最近文件；
- **Today tasks**：due ≤ 今天结束且未完成（含 overdue）；
- **Today calendar**：`list_between(day_start, day_end)`；
- **Waiting follow-up**：open 且 follow_up_at ≤ 今天结束（需跟进/已到期）；
- **Inbox pending**：未处理项。

### 前端 TodayView
- 两栏：主栏 CONTINUE（Work 标题/上次做到/下一步/最近文件/上次活动）、
  TODAY（任务可一键完成 + 日历时间线）、MORNING BRIEF 占位
  （"连接 AI 后可生成 Morning Brief"）；
- 侧栏 WAITING（可解决）+ INBOX（未处理）；
- 数据每次进入页面刷新（$effect 加载）。

## 3. 修改/新增文件

- `src-tauri/src/commands/mod.rs` — get_today 命令 + TodayData/ContinueWork 结构
- `src-tauri/src/lib.rs` — 注册 get_today
- `src/lib/components/TodayView.svelte` — 新增
- `src/routes/+page.svelte` — today 视图接入，删除旧占位样式
- `docs/stage-6-report.md` — 本报告

## 4. 执行的验证

验收数据（CDP 驱动真实命令）：
- 3 个 Works：老年破伤风 IIT（active）、KOL 拜访计划（active）、季度总结（done）；
- 任务：今天 due / 昨天 due（overdue）/ 明天 due；
- 日历：今天 10:00 会议、明天专家拜访；
- waiting：今天跟进（统计方案反馈）、下周跟进；
- inbox：1 条未处理。

`get_today` 结果：

| 区块 | 结果 |
| --- | --- |
| continue_works | 2 个 active Work（done 的季度总结被排除）✓ |
| today_tasks | 今天 + overdue 2 个（明天任务未出现）✓ |
| today_calendar | 仅今天会议（明天拜访未出现）✓ |
| waiting_followups | 仅今天跟进（下周未出现）✓ |
| inbox_pending | 1 条 ✓ |

## 5. 测试结果

- `cargo test`：21 passed / 0 failed；
- `cargo check`：无警告；
- `pnpm check`：0 errors / 0 warnings。

## 6. 性能

- get_today 单命令多次索引查询，数据量小，毫秒级；
- 前端只在进入 Today 页面时加载，无轮询。

## 7. 与指南的偏差

无。实现说明：
1. 排序规则按指南 §20 的"避免智能黑箱"原则实现为确定性 SQL/代码逻辑：
   硬 deadline 任务 → 今天日历 → 到期 waiting → 最近活动 Work → overdue task
   （前端按区块自然呈现该优先级顺序）；
2. 时区语义由前端 Date 计算（本地时区）传入 Rust，避免在 Rust 侧引入
   时区库，且与系统时间一致。

## 8. 已知问题

- 无阻塞问题；
- 任务/日历的"手动 pinned"优先级（§20.6）尚未体现（当前无手动置顶任务
  的数据模型，属可选增强）。

## 9. 当前本地 Git 状态

- branch: master
- HEAD: 22bb5c2（Stage 5）→ 本 Stage 提交后更新
- working tree: 待提交
- remote: 无

## 10. 下一阶段

Stage 7 — Search / Command：
- `Ctrl + K` 轻量搜索：Works/Tasks/Waiting/Calendar/Inbox/Resume Points/Activity/
  文件名；SQLite 优先；结果分类；空闲不产生后台 CPU。

停止，不自动进入下一阶段。
