# Stage 7 完成报告

> 日期：2026-08-13
> 项目：msl-desktop
> 依据：`DEEPSEEK_V4_FLASH_MSL_DESKTOP_DEVELOPMENT_GUIDE.md` §21

## 1. 本阶段目标

实现轻量 `Ctrl + K` 搜索：Works/Tasks/Waiting/Calendar/Inbox/Resume Points/
Activity/文件名；优先 SQLite；不建向量数据库；结果分类；空闲无后台 CPU。

## 2. 实际完成

### Rust `search(query)` 命令
- 8 组结果：works / files / tasks / waiting / calendar / inbox / resume_points / activity；
- 全部走 SQLite `LIKE ? ESCAPE '\'`（含 `%`/`_`/`\` 转义），每组 LIMIT 8；
- **files**：work_file_refs（label/path）+ activity 中出现的路径，按 path 去重
  ——只搜"已索引"的文件名，**不遍历磁盘**；
- 空查询安全返回空；
- 各 repository 新增 `search(like, limit)` 方法。

### 前端 SearchOverlay
- `Ctrl+K` 打开并聚焦、`Esc`/点击遮罩关闭；
- 150ms debounce 输入即搜；
- 结果按组展示（组标题 + 项：名称 + 附加信息/路径）；
- 点击结果导航到对应视图（Work→Works、Task→Plan、Waiting→Waiting、
  Calendar→Calendar、Inbox→Inbox、File→定位/Works）；
- 无后台轮询/定时器，空闲零 CPU。

## 3. 修改/新增文件

- `src-tauri/src/commands/mod.rs` — search 命令 + SearchResults/FileHit
- `src-tauri/src/db/{workspace,work,task,inbox,calendar,activity}.rs` — 各 repo search 方法
- `src-tauri/src/lib.rs` — 注册 search
- `src/lib/components/SearchOverlay.svelte` — 新增
- `src/routes/+page.svelte` — 接入覆盖层与导航
- `docs/stage-7-report.md` — 本报告

## 4. 执行的验证

验收数据（CDP 驱动）：
- Work「老年破伤风 IIT」；Task「核对老年破伤风入排标准」；
  Resume Point「破伤风方案第二轮修订完成」；文件 `C:\Work\破伤风 IIT\研究方案V3.docx`。

搜索「破伤风」：

| 分组 | 结果 |
| --- | --- |
| works | 老年破伤风 IIT ✓ |
| files | 破伤风 IIT\研究方案V3.docx ✓ |
| tasks | 核对老年破伤风入排标准 ✓ |
| resume_points | 破伤风方案第二轮修订完成 ✓ |
| activity | 创建任务… / 创建 Work… ✓ |

关键词同时出现在 Work、文件名、Task、Activity 中，结果在一个界面清晰分组 ✓。
空查询返回全空 ✓。

## 5. 测试结果

- `cargo test`：21 passed / 0 failed；
- `cargo check`：无警告；
- `pnpm check`：0 errors / 0 warnings。

## 6. 性能

- 搜索为按需 invoke（无后台线程/定时器），空闲时零 CPU（指南 §21 要求）；
- 单次搜索为 8 个带索引的 LIKE 查询，LIMIT 8，毫秒级；
- 前端 150ms debounce 避免输入抖动。

## 7. 与指南的偏差

无。实现说明：
1. 文件名搜索范围限定为"已访问/已索引"的文件（work_file_refs + activity 路径），
   不实时遍历工作目录（指南 §21 允许、§1.2 禁止周期遍历）；
2. 结果分类为 works/files/tasks/waiting/calendar/inbox/resume_points/activity
   （指南要求至少 Works/Files/Tasks/Calendar/Activity 五类，已超集覆盖）。

## 8. 已知问题

- 无阻塞问题；
- 文件名搜索依赖 activity/work_file_refs 已记录的路径，未访问过的文件
  搜索不到（设计如此，符合"不遍历磁盘"约束）。

## 9. 当前本地 Git 状态

- branch: master
- HEAD: 3e2021c（Stage 6）→ 本 Stage 提交后更新
- working tree: 待提交
- remote: 无

## 10. 下一阶段

Stage 8 — AI Provider / Morning Brief：
- provider settings、keyring、DeepSeek preset、Generic OpenAI-compatible、
  connection test、timeout/cancellation、Morning Brief snapshot、prompt、
  response、daily cache、regenerate、error UI；AI 默认关闭。

停止，不自动进入下一阶段。
