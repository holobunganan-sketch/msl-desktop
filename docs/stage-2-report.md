# Stage 2 完成报告

> 日期：2026-08-13
> 项目：msl-desktop
> 依据：`DEEPSEEK_V4_FLASH_MSL_DESKTOP_DEVELOPMENT_GUIDE.md` §16

## 1. 本阶段目标

建立数据层：12 张表、显式 SQL migration、repository/service 最小 CRUD、
unit tests、临时数据库测试、migration 幂等、明确错误、WAL、shutdown flush。

## 2. 实际完成

- **schema（指南 §6）**：`migrations/0001_init.sql` 建立全部 12 张表
  （workspaces / works / resume_points / work_file_refs / tasks / waiting_items /
  inbox_items / calendar_events / activity_events / daily_briefs / provider_settings /
  app_settings）+ 14 个常用索引；时间字段统一 INTEGER（Unix 秒，UTC）；
- **migration 机制**：`schema_migrations` 版本表 + 事务化幂等 runner
  （每个迁移在单事务中执行 SQL + 版本记录，重复执行自动跳过）；
- **数据库核心 `db/mod.rs`**：
  - `Database::open(path)` / `open_in_memory()`（测试用）；
  - WAL、外键约束 ON、busy_timeout 5s；
  - `close()` 执行 `wal_checkpoint(TRUNCATE)` 后关闭（shutdown flush）；
  - `DbError`：Io / Sqlite / Migration / NotFound；
  - 默认路径 `%APPDATA%\MSLDesktop\msl-desktop.db`（指南 §2.3）；
- **repository 最小 CRUD（8 个域模块）**：workspace（workspaces+work_file_refs）、
  work（works+resume_points）、task（tasks+waiting_items）、inbox、calendar、
  activity、brief、provider（provider_settings+app_settings）；
  每个含 insert/get/list/update/delete 及域特定操作；
- **测试**：18 个单元测试全部通过（见 §5）。

## 3. 修改/新增文件

- `src-tauri/migrations/0001_init.sql` — 初始 schema
- `src-tauri/src/db/mod.rs` — Database 核心 + 数据库级测试
- `src-tauri/src/db/migrations.rs` — migration runner
- `src-tauri/src/db/workspace.rs` / `work.rs` / `task.rs` / `inbox.rs` /
  `calendar.rs` / `activity.rs` / `brief.rs` / `provider.rs` — 各域 repository
- `src-tauri/Cargo.toml` — 新增 `rusqlite`（bundled）
- `src-tauri/src/lib.rs` — 注册 `pub mod db;`
- `docs/stage-2-report.md` — 本报告

## 4. 执行的验证

- command: `cargo check`
  result: Finished，无警告无错误
- command: `cargo test`
  result: 18 passed / 0 failed（0.06s）

## 5. 测试结果

| 测试 | 验证点 |
| --- | --- |
| fresh_db_auto_creates_and_migrates | 全新 DB 自动创建、12 张表存在、journal_mode=WAL |
| reopen_preserves_data | 写入→close→reopen 数据完整（WAL checkpoint 生效） |
| migration_is_idempotent | 重复 migrate 不重复应用、版本记录仅 1 条 |
| default_path_uses_appdata | 默认路径含 MSLDesktop、文件名正确 |
| foreign_keys_enforced | 外键约束生效 |
| workspace_crud / work_file_ref_crud | CRUD + NotFound 错误路径 |
| work_crud_and_archive / resume_point_history_and_latest | 归档 + 最新 Resume Point |
| task_crud_and_complete / task_filter_by_work / waiting_crud_and_resolve | Task/Waiting |
| inbox_crud_and_process | Inbox + 转换标记 |
| calendar_crud_and_range | 时间范围查询 |
| activity_insert_and_query | 多条件查询 + dedupe_key |
| brief_latest_for_date | 同日最新 Brief 复用 |
| provider_crud_and_enabled / app_settings_upsert | Provider/设置 |

## 6. 性能

- 测试运行时间 0.06s，无性能问题；
- 数据文件位置 `%APPDATA%\MSLDesktop\msl-desktop.db`，未创建任何用户文件副本
  （work_file_refs 仅存路径引用）。

## 7. 与指南的偏差

无。实现说明：
1. 时间字段统一使用 INTEGER（Unix 秒）而非 TEXT，避免额外时间库依赖，
   应用层展示时再格式化；
2. `app_settings` 采用通用 key-value（key 主键 + value + updated_at），
   指南未细化该表结构；
3. rusqlite 使用 bundled 特性（内置 SQLite），Windows 无需系统 SQLite。

## 8. 已知问题

- 无阻塞问题。activity.query 的 limit 参数直接用 `LIMIT {n}` 拼接（n 为 u32，
  无注入风险，值来自应用内部）；
- repository 层尚未接入 AppState / IPC（属 Stage 3+ 集成工作）。

## 9. 当前本地 Git 状态

- branch: master
- HEAD: 4a3975a（Stage 1）→ 本 Stage 提交后更新
- working tree: 待提交
- remote: 无

## 10. 下一阶段

Stage 3 — Workspace / File Browser / Watcher / File Activity：
- 选择文件夹、保存 workspace、文件/目录列表、lazy loading、文件 metadata、
  打开文件、Reveal in Explorer、最近文件、file watcher、activity recording、
  debounce、Office 临时文件过滤。

停止，不自动进入下一阶段。
