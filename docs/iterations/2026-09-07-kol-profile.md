# 专家导航与机构/科室拆分

状态：PARTIALLY_COMPLETED（功能机械验收通过，视觉终审待用户）；正式安装替换：COMPLETED；版本 0.3.1。限定范围：将专家入口归入工作台，分开持久化机构与科室，并打通现有编辑、搜索和 AI 上下文。

## 回归复现
- Action：新增旧 schema 14 升级保真测试及原生导航测试，在原有实现上执行。
- Expected：测试能准确捕捉字段合并与导航位置不符的问题。
- Observed：Rust 测试失败于缺少独立 department 字段；原生 0.3.0 测试失败于专家入口未在 Workbench 分组。均为预期断言失败。
- Evidence：kol_department_migration_preserves_legacy_profile_and_notes；scripts/kol-profile-cdp.py --baseline；四目录隔离的 kol-profile。
- Verdict：RED 已验证，进入实现。

## 设计边界
- 导航位于“工作台”的“项目”之后；问答保留在工具。
- 姓名、机构必填；科室和关注领域可稍后补充。机构/科室分别保存、编辑、搜索，并进入交流与 AI 来源包。
- schema 15 仅追加 department 默认空字符串，旧机构原文及关联记录不改动；不猜测自动拆分。
- 兼容未传科室的旧调用；修改旧档案时不清空已有科室，显式空字符串允许用户清空。
- 所有测试隔离 APPDATA、LOCALAPPDATA、TEMP、TMP 与 WebView2；不读取真实 Key、不调用真实 AI、不修改源工作文件。

## 实现及专项验证
- Action：追加 schema 15，打通独立 department 字段的保存/清空、查询、搜索、交流和模型证据；调整工作台导航及中英文表单。
- Expected：旧数据原样保留；机构、科室、关注领域不会相互替代；所有入口使用一致信息。
- Observed：两项后端专项测试通过，涵盖旧 schema 14 迁移、重复迁移、独立值回传、空值/遗漏字段、长度限制、全局/专家 AI 证据；前端 check 为 0 errors / 0 warnings，27 项 Node 测试全部通过。
- Evidence：cargo test kol_department --lib；pnpm check；node --test tests/*.test.mjs。
- Verdict：专项 PASS，完整回归与原生 UI 继续验证。

## 完整回归
- Action：执行全部 Rust 测试（含 ignored 专项）、前端构建与格式检查。
- Expected：既有工作流、模型适配、迁移、缓存和新字段全部通过。
- Observed：首轮 174 passed / 1 failed，唯一失败为旧迁移测试遗漏更新的版本数断言（预期 14，实际已正确升级到 15）。定位并只更新该断言后，完整重跑为 175 passed / 0 failed / 0 ignored；cargo fmt --check、pnpm build 通过。构建存在既有未使用 import 及 Windows 链接器信息警告，前端 check 无警告。
- Evidence：cargo test -- --include-ignored --test-threads=1；pnpm build；正式备份的独立迁移副本测试。
- Verdict：PASS，继续 release 构建和桌面测试。

## 桌面发布版验证
- Action：pnpm tauri build；隔离启动发布版，测试导航、创建/编辑/清空科室、搜索、重启与中英文大字号布局。
- Expected：字段操作真实持久化，保存按钮可滚动到且可点击，专家入口位于工作台。
- Observed：release 构建成功。首轮行为检查全部通过；1100 宽度的 nearest 滚动将按钮底部放在 720.2679（视口 720），精确坐标断言失败。诊断证实为 1.16 缩放的边界舍入；测试改为居中滚动后验证按钮完整位于滚动区内且实际命中可点击元素，未放宽阈值。随后重启检查 18 PASS，另用全新 kol-profile-final 从零执行整个脚本 21 PASS / 0 异常，4 张大字号中英文截图已保存。
- Evidence：scripts/kol-profile-cdp.py；.test-runtime/kol-profile/artifacts/kol-profile-debug-1100.png 与 debug-scrolled；.test-runtime/kol-profile-final/artifacts/。已目视检查 1440 中文及 1100 英文布局。
- Verdict：机械与操作 PASS，继续既有页面回归及正式替换。最终视觉审阅待用户。

## 替换前检查
- Action：既有页面布局回归后，停止测试 PID 20888 和正式 PID 19708，冷备份旧程序、卸载器、快捷方式、DB/WAL/SHM，复制第二份用于升级测试。
- Expected：旧功能保留，退出后才复制数据，不打开正式数据库。
- Observed：旧页面 63 PASS / 26 张截图。首次退出后即时检查命中进程退出延迟，备份保护条件停止了该次操作；后续迁移检查因副本尚未创建而失败，未触及正式数据。确认进程均退出后重新完成冷备份、逐项哈希验证和最新副本迁移，迁移测试 1 passed。
- Evidence：.test-runtime/reinstall-0.3.1-20260907/backup/ 和 formal-migration-copy/。
- Verdict：PASS，可替换正式安装。

替换前 SHA256：
- 安装包：61447690DE6882167543E69A529EF8364B39595B79AF238BA72EFE458266C06B
- 旧 EXE：C2D04299052A72A1F1032AF21EED73401136607C3D90024DB989EFAB3648278B
- 旧卸载器：FCBD1C6322A49E7299B9342A1AEF754D31D0975327E2711C0CC6617C3CA938B2
- 正式 DB：B4254CC4634364AFCAD418EA1608AB74CF1F8F416669765D2796A784026061FF
- 正式 WAL：C4E19D0FEF17BBE7B6B10E9EEE07D80E0E2753CB22A6804A81FE69B478EC7A13
- 正式 SHM：B35A1B96B23D38D2F2E8CC6A97EBAD142025314CF3084E4B32484AB69F7CAE65

## 卸载旧版
- Action：旧卸载器 `/S /UPDATE`，保留数据。
- Expected：仅移除旧安装，正式数据库保留。
- Observed：PID 3968 退出码 0；旧 EXE、卸载注册项均移除；DB/WAL/SHM 哈希均与冷备份一致。
- Evidence：卸载进程结果、文件与注册项断言、三件套指纹对比；旧程序备份可恢复。
- Verdict：PASS。

## 新版安装与最终复核
- Action：验证安装包 SHA256，执行 `/S /D=C:\Users\ZhouNan\Apps\MSL Desktop`，无自动运行参数；核对产物、快捷方式后，隔离启动实际安装 EXE 检查运行时版本和重启资料。
- Expected：原快捷方式打开 0.3.1；实际安装版符合本轮实现，正式数据保留。
- Observed：安装 PID 27776 退出码 0；文件和注册项为 0.3.1。仅内存归一化 Tauri UNK/NSS 包类型标记后，安装 EXE 与构建逐字节一致；桌面及开始菜单快捷方式目标正确，无自动启动。实际安装测试 PID 21000 使用 kol-profile-final 隔离四目录和 WebView2，运行时版本为 0.3.1，18 项重启/导航/中英文大字号表单检查全部通过，4 张截图已保存，无 UI 异常。
- Observed：停止精确测试 PID 后无遗留 MSL 进程；正式 DB/WAL/SHM 哈希仍与冷备份一致。测试未使用真实 AI 或真实 Key，未读取工作源文件正文。
- Evidence：.test-runtime/reinstall-0.3.1-20260907/artifacts/；已安装 EXE SHA256 `1326E5A63E3E6539E76EE033534F8698F89667373B11F39EC405C21F3FC58459`；构建 EXE SHA256 `AE3A9DFDD51EB5E7379B6962CFFF17D9203E53737A489C852F7A3ADCD6915B72`；最终进程/版本/指纹断言。
- Verdict：功能和安装 PASS；最终视觉成熟度等待用户查看截图和实际使用。备份保留，可恢复旧安装及数据。

## 使用变化
- 左侧“工作台”内，“项目”下面为“专家与洞察”；“工作台问答”仍在“工具”。
- 在专家“资料”中分别填写机构与科室；科室可留空，也可清空，搜索支持科室。
- 旧机构原文保留，合并写过机构与科室的旧档案请在编辑时分别调整；系统不会猜测拆分。
- 截图：`.test-runtime/reinstall-0.3.1-20260907/artifacts/kol-profile-zh-1440.png`（合成资料）。
