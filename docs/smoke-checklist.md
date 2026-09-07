# Stage 11 人工 Smoke Checklist（指南 §25）

> 运行 `scripts/smoke-test.ps1` 完成自动化部分后，逐项人工验证以下
> Windows UI 操作。全部通过视为 Stage 11 完成。

## A. Lifecycle（自动化已覆盖 launch/close/reopen/exit，此处复核 UI 行为）

- [ ] 从托盘点击图标 → 主窗口出现，标题 "MSL Desktop"
- [ ] 窗口关闭（×）→ 窗口消失，托盘图标仍在（进程常驻）
- [ ] 托盘右键 → 菜单包含：打开 MSL Desktop / Quick Capture / 退出
- [ ] 托盘 → 退出 → 进程结束，任务管理器无 msl-desktop 进程
- [ ] 双击/再启动一个实例 → 第二个实例静默退出（单实例）

## B. Workspace（文件夹对话框与文件操作）

- [ ] Workspace 页 → "绑定工作目录…" → 系统文件夹选择对话框出现
- [ ] 选择一个真实工作目录（如 `tests\fixtures\workspace-small`）→ 列表显示文件
- [ ] 双击文件 → 用系统默认程序打开（Word/记事本等）
- [ ] "定位"按钮 → 资源管理器打开并选中该文件
- [ ] 在已绑定目录内新建/修改文件 → Activity 页（后续）出现对应记录
- [ ] 面包屑导航可返回上级目录

## C. Work

- [ ] Works 页 → 新建 Work → 详情显示 RESUME POINT（上次做到/下一步/记住）置顶
- [ ] 填写 Resume Point 表单 → 保存 → 立即显示
- [ ] 关联文件（路径输入）→ 显示在 FILES 区块；置顶/取消置顶生效
- [ ] 状态切换 active/paused/waiting/done 生效
- [ ] 归档 → Work 从列表消失（状态 archived）

## D. Today

- [ ] Today 页显示 CONTINUE（进行中 Work + 上次做到/下一步/最近文件）
- [ ] 今日任务/日历/截止显示正确（与 Plan/Calendar 一致）
- [ ] Waiting 区块显示需跟进事项，"解决"按钮生效
- [ ] Inbox 区块显示未处理项

## E. Search（Ctrl+K）

- [ ] 按 Ctrl+K → 搜索框出现并聚焦
- [ ] 输入关键词 → 结果按组显示（Works/Files/Tasks/Calendar/Activity 等）
- [ ] 点击结果 → 跳转到对应视图
- [ ] Esc 关闭搜索

## F. AI（需真实 API Key）

- [ ] 设置 → AI Providers → 新增 → 使用 DeepSeek preset
- [ ] 输入真实 API Key → 保存 → 显示"已配置 Key"
- [ ] "测试连接" → 成功提示
- [ ] Today → "生成今日 Brief" → 内容基于实际数据（不凭空捏造）
- [ ] 无 Key 时 Today 显示"连接 AI 后可生成 Morning Brief"，应用不报错
- [ ] 无效 Key → 测试连接显示明确错误（HTTP 401 等）

## G. Persistence（自动化已覆盖重启数据保持，此处复核 UI 数据）

- [ ] 重启应用后：Work / Task / Waiting / Calendar / Inbox 数据完整
- [ ] 上次绑定的 workspace 自动恢复监听（修改文件产生 Activity）

## H. Safety（文件安全）

> 当前版本 Workspace 仅提供"打开/定位"，**无文件删除/移动操作**，
> 不存在误删路径；如后续增加删除，必须满足：
- [ ] 高风险操作（删除/覆盖/批量移动）出现二次确认对话框
- [ ] 删除优先走回收站（不直接永久删除）
- [ ] 取消确认后文件保持不变

---
记录：日期 ____ 执行人 ____ 结果：全部通过 / 有失败项（详见备注）

## Workbench rebuild 验收补充

- [x] 默认中文，顶部可切换英文；重启后语言保持。
- [x] Dashboard 显示四个指标卡、Continue Work、今日时间线、Waiting、Inbox、最近文件变化和 Brief。
- [x] Work/Task/Waiting/Calendar 均通过 Modal 创建；空标题可见错误，成功后同页刷新。
- [x] Calendar 支持 Day/Week、编辑和删除；结束时间早于开始时间被拒绝。
- [x] Inbox 可转换为 Task/Waiting/Calendar，取消或失败不丢失原 Inbox。
- [x] Quick Capture 写入 Inbox；失败时保留输入。
- [x] Workspace 首次绑定只建立 metadata baseline；停止期间新增/修改/删除在下次启动进入 Activity 和 Brief。
- [x] Brief 支持昨天/7 天/自定义范围；无 Provider 时返回非空本地摘要并展示来源计数。
- [x] 隔离 UI CDP smoke：17/17；release exe smoke：17/17；未触碰正式 DB。
- [x] 1024×640 与 1440×900 截图已保存到 `output/playwright/final-*.png`，视觉成熟度等待用户/更高能力模型审阅。

## AI 秘书与存储治理第二阶段（隔离自动化）

- [x] 使用 `scripts/run-isolated-audit.ps1` 同时隔离 APPDATA、LOCALAPPDATA、TEMP、TMP。
- [x] DeepSeek 与 OpenCode Go 固定模板出现在 Settings；六类任务可分别选择 Provider/Model。
- [x] 绑定目录的 DOCX、文本、PDF 文本层索引走 LOCALAPPDATA cache；扫描版 PDF 显示 needs OCR。
- [x] `scripts/ui-smoke-cdp.py`：Work/Task/Waiting/Calendar/Inbox/Quick Capture、语言切换和 local Brief 17/17。
- [x] `scripts/dashboard-layout-cdp.py`：中英文 × 1024×640/1440×900 四组无 body/document/dashboard 溢出。
- [x] AI Review Center 支持待确认建议编辑、保存草稿、确认/拒绝；未经确认不写业务表。
- [x] 调度默认 180 分钟与每日 06:00，可在 Settings 修改并手动运行；翻译支持自动方向与书面/口语。
- [x] 存储与清理先预览再确认；缓存、活动 rollup、过期草稿和 WebView 数据均有独立保护边界。
- [x] `scripts/release-functional-cdp.py`：release Dashboard/Review/Workspace/Provider/Schedule/Storage 11/11，截图保存于隔离 artifacts。
- [x] 正式数据库只读元数据保持不变；正式副本迁移与 release 验证详见 `docs/luna-ai-secretary-iteration/EXECUTION_REPORT.md`。

## 首页决策与周报/月报迭代（隔离自动化）

- [x] 首页 Brief 保持 5–7 条以内的项目符号摘要，并清除 `source_type`、`entity_id` 等内部标记。
- [x] 首页“需要您决定”只展示最近一次已完成分析的建议，可选择 Work、Task、Waiting、Calendar 或 Inbox。
- [x] Task/Waiting 可关联长期 Work，也可设置为临时事务；暂缓不会写入业务实体。
- [x] 秘书状态卡保持紧凑，可直接选择常用分析周期或输入自定义分钟数。
- [x] AI 审阅展示最近 7 天记录，可按状态、分析批次筛选，并可深入编辑待确认或已暂缓建议。
- [x] 周报默认覆盖生成日前 7 天并使用序号列表；支持自定义起止日期和用户设定自动生成时间。
- [x] 月报默认覆盖上一个自然月，结合任务变动、项目进展和周期重叠周报进行综合分析。
- [x] 报告运行状态、正文、失败信息、重试与保留标记持久化；缓存清理将保留报告列为保护项。
- [x] 本地 Mock Provider 验证 Brief、全局分析、翻译、周报和月报，全程未调用真实 Provider。
- [x] `scripts/release-functional-cdp.py`：50/50；`scripts/layout-audit-cdp.py`：11 页 × 3 视口无页面溢出。
- [x] 报告与建议重启后保持；缓存安全验证后报告数量不变；正式数据库副本迁移无实体丢失。
- [x] 中文最终截图保存到 `.test-runtime/luna-ai-secretary/artifacts/ui-c1/final-*.png`。

## AI 审阅工作流与系统代理修复

- [x] 正式数据库只读诊断确认：7 次分析均在 Provider 连接阶段失败，`ai_proposals` 为 0，模型路由和启用状态正常。
- [x] HTTP 客户端编译启用 `system-proxy`，可读取 Windows 当前用户代理设置。
- [x] 本地回环地址强制直连，Mock Provider 测试不会被系统代理转发。
- [x] AI 审阅默认显示待确认事项，并展示最近一次分析的完成、运行或失败状态。
- [x] 分析失败时显示原因和重新分析按钮；分析成功后刷新并显示最新待确认建议。
- [x] 最新完成批次没有建议时不再重新展示旧批次待确认事项。
- [x] `scripts/release-functional-cdp.py`：56/56；失败状态和成功生成建议两条路径均通过。
- [x] `scripts/layout-audit-cdp.py`：11 页 × 3 视口无页面溢出。
