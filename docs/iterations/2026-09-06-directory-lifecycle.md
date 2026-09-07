# 项目目录同步与安全移除

状态：PARTIALLY_COMPLETED（代码、机械验收和安装包完成；最终视觉审阅待用户确认）。范围为用户要求的目录绑定生命周期、可见清单和安全移除，保留现有脏工作树；不替换正式安装，不测试正式数据或真实 Provider。

## 规则与实施顺序

1. 目录清单按项目关联汇总，同一目录显示一次，同时列出所有关联项目。旧的独立主目录记录没有项目关联时不再出现在清单或后续扫描中。
2. 从一个项目解绑只移除该项目的关系；仍被其他项目关联的目录保留。最后一条关联解除或最后一个项目删除后，目录停用并从清单移出。
3. 任一关联项目状态非 archived（含已完成、暂停、等待）均禁止从目录页移除。只有全部关联项目已归档时允许移除，需明确确认；移除关联和监控入口，保存历史证据，不操作源文件。
4. 重新绑定恢复相同目录记录。SQLite 生命周期约束覆盖手动操作、AI 确认及撤销路径。
5. 页面选择、同步信息、扫描、文档索引必须指向同一 workspace ID。解绑与移除后刷新选中项，不退回浏览整盘。

依次执行：复现与失败测试 → 数据层生命周期与后端防护 → 目录清单/移除确认/刷新 → 全量与隔离 UI 测试 → 持久化/备份副本迁移 → release 构建。仅在机械验收通过后交付；视觉成熟度待用户审阅。

## 排查

- Action：沿项目解绑、目录总表、页面选中项和扫描入口追踪。
- Expected：项目解绑会同步目录清单，扫描使用选中目录。
- Observed：unlink 仅删除 work_workspace_links；页面直接读所有 workspaces；文档与扫描取 watcher 的主目录，workspace_rescan 即使传入别的 ID 也使用主路径。
- Evidence：db/documents.rs、db/workspace.rs、commands/mod.rs、WorkspaceView.svelte。
- Verdict：根因确认，已加入 3 个回归测试，等待 RED 结果。

## 数据层与界面实施

- Action：执行回归测试；加入 schema 13、目录停用保护、按项目汇总查询、所选目录扫描和界面移除确认。
- Expected：孤立目录退出清单与后续 AI 文档输入；有未归档项目时禁止移除；历史身份和证据保留。
- Observed：生命周期 3 个测试按预期 RED；补充归档目录保留证据测试触发旧 DELETE 的外键失败，修改为停用后 GREEN。6 个 workspace 测试全部通过。AI 输入排除测试先 RED 后 GREEN。原生旧版目录页基线测试 RED。新版 pnpm check 为 0 errors / 0 warnings；Rust 首轮全量 157 PASS、2 个专项待显式执行。
- Evidence：db/workspace.rs、ai/analysis_snapshot.rs、本轮终端输出、directory-lifecycle-cdp.py --baseline。
- Verdict：核心 PASS，原生交互与最终构建待验收。发布版本提升到 0.2.2 以区分正式 0.2.1。

## 原生操作验收 — PASS

- Action：以四个隔离数据目录和独立 WebView2 目录启动实际 0.2.2 release，执行 directory-lifecycle-cdp.py 的主流程、edges、layout，并真实停止、重启后执行 reopen。
- Expected：界面按项目去重显示；移除保护、取消/确认、共享关系、选中目录扫描、持久化和源文件安全符合规则。
- Observed：
  - 活跃项目的移除按钮禁用；后台直接调用也拒绝。共享目录展示两个项目，仅出现一张卡片。
  - 选中 B 时，文档列表只显示 B 的文件；存在独立主监听 C 时，传入 D 的扫描只统计 D 的两个文件，主监听仍在 C。
  - 全部关联项目归档后按钮可用。取消保留关联；确认移出清单并解除归档项目关联。
  - 确认窗口打开后新增未归档项目关联，确认操作被拒绝；直接 SQL 停用也被保护触发器阻止。
  - 部分解绑保留共享目录；最后解绑清空清单及浏览/文档面板；重新绑定复用原 ID；删除最后一个项目同步停用目录。
  - 最后解绑立即停止对应主监听并清除其启动设置；移除后扫描、状态查询和重新索引均拒绝该 ID。
  - 5 个合成文本文件保持存在，主流程 A/B 的内容逐字比对不变；测试没有源文件删除操作。
  - 重启后目录清单仍为空，历史目录记录仍保留为停用状态；主监听未恢复；SQLite integrity_check=ok，foreign_key_check 为空。
  - 目录页 1440×1000 与 1100×720、大字号，卡片不横向裁切，最后操作可滚动到达；已查看顶部、底部和空状态截图。
  - 全应用 11 个视图、中英文首页/审阅、两种尺寸的布局冒烟 63 PASS，无 UI 运行时异常。
- Evidence：本轮终端输出；scripts/directory-lifecycle-cdp.py；.test-runtime/directory-ui/artifacts/directory-final-1440.png、directory-final-1100.png、directory-final-bottom-1100.png、directory-empty.png 及 workflow 系列截图。
- Verdict：PASS。edges 脚本首轮存在辅助函数参数重名，修正测试辅助函数后完整重跑通过，应用无需因此修改。

## 构建与数据安全 — PASS

- Action：最终全量检查、release/NSIS 构建、既有正式备份的独立副本迁移与慢响应 Mock 测试。
- Expected：产物版本一致，迁移保留业务实体，隔离数据测试结束后关闭测试实例。
- Observed：pnpm check 0 errors / 0 warnings；pnpm build、pnpm tauri build 成功；cargo fmt --check PASS；cargo test 157 PASS、2 项默认忽略；显式执行 ignored 为 2/2 PASS，总计 159；Node 24/24 PASS。构建保留既有 SSR 未使用导入和 Windows 链接信息提示。
- Observed：独立 formal-migration-copy.db 迁移至 schema 13 后 Work/Task/Waiting/Calendar/Inbox 数量不变。原备份 DB 哈希保持 97B2C683A0BCDE84FE707698879D1EBBB48D6B3A486ED2B4AB2EC751391D2AEF。测试进程 18872、3752、28296 已按核对后的路径关闭；未关闭用户正式程序。
- Evidence：.test-runtime/directory-lifecycle/formal-migration-copy.db；本轮构建/测试终端输出与安装包指纹。
- Verdict：PASS。UI 测试中没有调用 AI；后端 Provider 相关测试仅使用 loopback Mock。

## 交付

- 安装包：src-tauri/target/release/bundle/nsis/msl-desktop_0.2.2_x64-setup.exe
- 大小：5,428,825 bytes
- SHA256：DF028647999431C009864E98849A37B3EA5CBFC1BC5DC72858BA61B091AC639E
- 正式安装仍为 0.2.1；本轮没有执行卸载或安装。
- 源文件只读约束保持，未读取真实 API Key、调用真实 Provider 或操作真实工作目录。历史孤立目录在升级迁移时停用，保留原目录 ID、文件索引和引用；不会在源目录生成、移动或删除文件。
- 系统排查和测试驱动技能使修复覆盖了关联生命周期、历史证据保护和扫描 ID/路径错配；完成前验证技能用于核对最终安装包及重启结果。最终视觉成熟度尚待用户审阅，因此报告保持 PARTIALLY_COMPLETED。
