# MSL Desktop 0.1.3 — 自然流转、源文件只读与项目认知

日期：2026-09-05。状态：PARTIALLY_COMPLETED。实施、机械验收及 release 构建已通过；最终视觉成熟度待用户或更高能力模型明确审阅。未覆盖用户已安装版本。

## 已实现的产品行为

1. 工作区源文件只读。绑定、取消关联、项目归档、AI 确认及撤销只作用于工作台记录。缓存写入校验提前，阻止路径穿越、Windows 特殊设备路径、目录链接越界及应用存储与工作区重叠。
2. 总体、每个已绑定目录、每个工作项目具有认知入口。参考用户指定的 project-cognition 技能，采用增量事实、版本指纹、覆盖说明、分层入口、按任务检索与失效重建。产品使用原生 Rust，无 Python 安装要求。
3. 收件箱支持一句话“记下并整理”。输入先保存在收件箱，再后台生成关联的项目进展、任务、等待等建议；确认前不写正式实体。
4. 建议去重保护已经确认、拒绝和用户编辑的内容；暂缓建议保留。首页展示最近批次，并提供早前/暂缓建议入口。
5. 多项建议可勾选共同确认，SQLite 事务确保整组成功或整组回滚。首页快捷确认保留完成、解决、归档和工作进展字段。确认后可撤销；检测到后续修改时拒绝覆盖。
6. 反馈区分明确分类错误和普通拒绝。重复、已完成、暂缓不会累计为分类错误；分类偏好可查看、更正、忘记。这属于本地检索记忆，不改变模型权重。
7. 项目页面增加阻塞信息和折叠的认知入口，详细资料按需展开。认知刷新、收件箱整理及原有 AI 分析沿用后台任务队列，切页不会取消。

## 项目认知文件与读取范围

产品派生文件位于 `%LOCALAPPDATA%\MSLDesktop\cache\cognition`，不会写入绑定的工作目录：

```text
cognition/
  global/PROJECT.md
  global/DOCUMENTS.md
  workspace-<目录ID>/PROJECT.md
  workspace-<目录ID>/DOCUMENTS.md
  work-<项目ID>/PROJECT.md
  work-<项目ID>/DOCUMENTS.md
```

- `PROJECT.md`：范围与权限、项目状态、下一步、任务/等待/日程、关联目录、近期资料入口、版本与覆盖。
- `DOCUMENTS.md`：详细文档地图，按目录和文档编号定位资料。默认不整体发送给 AI。
- SQLite `cognition_entries` 保存事实入口、版本与指纹。Markdown 属于可重建派生数据，可由缓存清理移除并恢复。
- 日常全局分析先读认知与结构化记录，变化正文上限为 4 个文件、12,000 字符。项目深入整理按相关性选择最多 8 个文件，总正文预算 40,000 字符；长文选择相关窗口并说明省略。
- 预算按字符设置，实际 token 数取决于模型和语言；未承诺固定 token 节省百分比。
- 目录可能仍做元数据扫描；无变化文件复用提取缓存。不会每次重读并发送全部文件正文。
- 不可访问目录、未提取/不支持/OCR 文件及未展开内容明确披露。模型需依据已有证据作答；没有证据时保留未知。
- 源资料视为不可信数据，不能赋予模型文件写入权限。任务跟进与归档均只改变工作台记录。

## 发现并修复的问题

### 并发关联目录导致 SQLite 锁定

- Action：在后台索引进行时连续关联多个目录，增加两连接 WAL 回归测试。
- Expected：前台关联与后台索引可顺序写入，无读快照升级失败。
- Observed：旧 DEFERRED 事务先读后写，遇到并发写入报 `database is locked`；测试先失败。写事务改为在读取前预留写入权，磁盘扫描保持在短事务外。
- Evidence：`db::tests::write_transaction_reserves_writer_before_reading_shared_state`；项目流程 CDP 的幂等关联检查。
- Verdict：PASS。

### 首页快捷确认丢失更新字段

- Action：为首页提取独立建议载荷函数，测试任务完成、等待解决、项目归档、进展字段和跨分类日期行为。
- Expected：首页确认与深入审阅保持一致，不凭空增加日期，不丢失状态。
- Observed：旧逻辑 3 项测试均失败；修复后 3 项通过。保持类别时保留原建议补丁，改类时仅构造相应字段；新增首页“工作进展”选项。
- Evidence：`tests/proposalPayload.test.mjs`；终版认知 CDP 额外覆盖首页确认与撤销。
- Verdict：核心 PASS；终版端到端结果见下。

### 工作进展建议无法保存草稿

- Action：通过真实首页确认进展建议，定位 `invalid proposal kind`；添加可编辑进展草稿单测。
- Expected：工作进展可在首页和审阅页面编辑、确认。
- Observed：后端分类白名单遗漏 `resume_point`，回归先失败，补齐后通过；首页及审阅组确认的端到端测试均通过。
- Evidence：`db::ai::tests::progress_draft_can_be_edited_before_confirmation`、`cognition-flow-cdp.py`。
- Verdict：PASS。

### 测试场景校准

- 旧回归把任意拒绝都算作分类错误，与本轮已批准规则冲突，改用明确的 `wrong_category` 测试。
- JSON 修复 Mock 与已确认基线建议重名，被正确去重；改为独立的格式修复测试建议，不取消产品去重保护。
- 开发热更新与发布构建同时写入前端生成目录，导致一次可见性矩阵中断。终版改用静态 release 程序独立运行，测试期间不构建或热更新。
- 网络面板测试使用直接 IPC 创建任务，却仅等待 0.5 秒，早于后台状态的 1.8 秒刷新周期。改为有上限地等待可见失败状态；重新执行为 10 / 10 PASS。
- 以上中间失败均保留说明，不能据此声称终版机械验收通过；以最新完整运行结果为准。

## 验证记录

所有写入和应用运行同时隔离 APPDATA、LOCALAPPDATA、TEMP、TMP；仅使用 localhost Mock 和合成文件。未读取真实 API Key、未调用真实 AI Provider、未让测试程序打开正式数据库。

| 验证 | 已观测结果 |
|---|---|
| pnpm check | 0 errors / 0 warnings |
| pnpm build | PASS |
| cargo fmt --check | PASS |
| cargo test | 133 PASS；2 项专项测试单独执行并通过 |
| 65 秒长请求回归 | PASS |
| 正式数据库副本迁移 | PASS；正式实体数量保持一致 |
| 前端状态与通知测试 | 5 PASS |
| release UI CDP 冒烟 | 17 / 17 PASS |
| release 功能与 Mock 验证 | 66 / 66 PASS |
| 项目关联、跨页、JSON 恢复、缓存安全 | 27 / 27 PASS |
| 三层认知、首页/审阅确认、撤销、记忆、源文件哈希 | 29 / 29 PASS |
| release 多尺寸及字号布局 | 120 / 120 PASS |
| release 中英文、弹窗、错误通知、边缘布局 | 63 / 63 PASS |
| 网络长请求、认证错误、隐私与重试 | 10 / 10 PASS |
| 退出进程并重启后的项目持久化 | 4 / 4 PASS |
| 退出进程并重启后的认知与撤销持久化 | 7 / 7 PASS |
| pnpm tauri build | 终版 PASS；Windows x64 NSIS 安装包 |

终版应用证据目录：`C:\Myfolder\MSL cowork\msl-desktop\.test-runtime\cognition-delivery-20260905\artifacts`。可见性明细为 `visibility-results.json`，其他脚本的逐项观测在本次执行工具输出中。

终版综合 Action：对打包使用的 release 程序执行 UI 创建、Mock AI、首页确认/撤销、后台跨页、缓存清理、边缘布局和实际进程重启。

Expected：各入口可用、确认才写入、源资料不变、异常不留下部分正式结果、重启保留本机记录。

Observed：上表机械项目全部通过；没有访问真实 Provider。源文件清单及 SHA-256 前后一致。正式数据库迁移仅在一次性副本上进行。

Evidence：`scripts/cognition-flow-cdp.py`、`scripts/project-flow-cdp.py`、`scripts/cognition-persistence-cdp.py`、`scripts/project-flow-persistence.py`、`scripts/ui-visibility-cdp.py`、`scripts/ui-edge-cases-cdp.py`、`scripts/ai-network-cdp.py` 及上述截图目录。

Verdict：机械 PASS；视觉门 PENDING。

## 发布物

- 版本：0.1.3，Windows 文件属性 ProductVersion/FileVersion 与源码版本一致。
- 安装包：`C:\Myfolder\MSL cowork\msl-desktop\src-tauri\target\release\bundle\nsis\msl-desktop_0.1.3_x64-setup.exe`
- 安装包 SHA-256：`EBE5B773092578EBBC88AC7BBC7C22519CA37C6BED261DE68BB25A95E35AF23B`
- release 程序 SHA-256：`84069A32ED06DAAA785A49657B0451DFA5481AFE667353B9A33CBD2D5113C0D7`
- 构建保留了此前存在的 SSR 未使用导入提示和 Windows 链接器信息；没有编译错误，Svelte 检查为零错误、零警告。

## 新功能入口

1. “工作”选择项目并关联目录；展开“项目认知”查看覆盖与版本，可更新索引、定位 Markdown。工作目录页提供总体及所选目录的认知。
2. 收件箱输入一段工作进展，点击“记下并整理”，可以立即切回工作页面。
3. 首页确认最近建议；需要细改时进入 AI 审阅。“一起处理相关建议”支持勾选已审阅条目共同确认。
4. “分类记忆与撤销”可更正/忘记记忆及撤销新版确认。后续修改冲突会阻止撤销，保护已经推进的工作。
5. 设置内“智能清理”包含派生认知缓存；清理不会改变源资料和正式工作记录。

## 视觉证据

均为隔离合成数据截图，非用户正式工作内容：

- `cognition-project-entry.png`：项目认知入口与项目推进区。
- `cognition-review-decisions.png`：建议审阅、状态与分组确认入口。
- `cognition-memory-controls.png`：分类记忆维护。
- `visibility-today-1280-xlarge.png`、`visibility-review-960-xlarge.png`：大字号、小窗口。
- `edge-dashboard-bottom-1280.png`：首页底部可达性。

截图由执行器检查过，机械几何验收通过。视觉成熟度仍待用户或更高能力模型明确审阅。

## 交付边界

- 用户正式数据库、工作文件和已安装程序未作为测试目标，也未被本轮卸载或覆盖。
- 发布版本号为 0.1.3。必须通过安装包安装才能替换当前已安装版本；仅更新源码不会让旧程序自行更新。
- 缓存重建会进行必要的本地读取，首次索引和大量变更时仍需处理时间。
- 本轮实现有预算的本地检索上下文，不承诺模型已完整阅读所有资料。真实供应商的可用性与实际生成质量未使用真实 Key 验证。
- 视觉审阅由用户或更高能力模型明确通过前，最终报告最多为 PARTIALLY_COMPLETED。
