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
