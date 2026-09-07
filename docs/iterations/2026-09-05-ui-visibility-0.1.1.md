# MSL Desktop 0.1.1：全应用可见性修复

日期：2026-09-05

状态：PARTIALLY_COMPLETED。实施、机械验收和本机更新均已完成；最终视觉成熟度提交截图供用户复核。

## 本轮范围与根因

本轮处理用户指出的页面底部不可达、卡片裁切、通知叠放和优先级标签缺字，并排查全部 11 个页面。保留原有业务功能和未提交成果。

1. 全局样式会根据字号补偿应用高度，但页面局部样式再次设置 `height:100vh`，其优先级更高。在 1280×720、特大字号下，实际应用高度达到 835.2 像素，后台任务入口从 745.4 像素开始，落在窗口之外。根节点隐藏溢出，使滚动无法补救。
2. 首页叠加了多轮互相覆盖的高度规则。支持卡片在部分断点被固定为 88 或 106 像素，分析周期控件高度超过卡片后被截掉。
3. 通知无限追加，并固定在窗口右下方，造成多条错误遮挡操作区。
4. 任务优先级使用固定 34 像素宽度，并直接显示 `normal` 等内部枚举。
5. 多列页面依照整个窗口宽度选择布局，未考虑侧栏和字号缩放后实际可用宽度。
6. Tauri 依赖无条件启用 `custom-protocol`，开发启动也加载旧打包资源。首次回归复测发现界面仍旧，随后修复开发与生产资源选择，确认开发窗口使用 devUrl、正式构建使用打包资源。

## 实施记录

### 01 · 复现

- Action：在四个环境目录均隔离的 WebView 中，写入合成的长项目名、任务、等待事项、建议和报告，测试实际屏幕坐标及滚动祖先。
- Expected：复现底部越界和标签缺字，记录红测。
- Observed：1280×720 特大字号下所有页面外框越界；标准字号下首页支持卡片也存在控件不可达。
- Evidence：`.test-runtime/ui-visibility-20260905/artifacts/before/visibility-results.json` 及同目录截图；外框高度 835.2 > 720。
- Verdict：PASS，根因复现。

### 02 · 布局与通知修复

- Action：统一缩放后的可视窗口尺寸；重写首页样式，移除强制压缩卡片的高度规则；根据内容区域宽度重排工作、审阅、报告、设置；通知移入顶部正常文档流并只展示当前一条；修正任务和等待事项的标题、元数据及操作区；优先级本地化并按内容宽度显示；弹窗正文可滚动，关闭按钮保持可见。
- Expected：长内容允许正常滚动，卡片内按钮可达，通知不遮挡操作区，中英文标签完整。
- Observed：开发界面 120/120 可见性检查通过。
- Evidence：`scripts/ui-visibility-cdp.py`、`tests/toast.test.mjs`；通知测试先失败后通过。
- Verdict：PASS。

### 03 · 功能与安全回归

- Action：执行静态检查、构建、Rust 测试、真实 WebView 操作、当地 Mock AI、重启持久化和缓存安全测试；对正式数据库的可丢弃副本单独运行迁移测试。
- Expected：保留业务行为，AI 建议继续经过确认，禁止触及正式数据或真实 Provider。
- Observed：下表全部通过。
- Evidence：本轮工具输出及 `.test-runtime/ui-visibility-20260905/artifacts/functional`、`artifacts/release`。
- Verdict：PASS。

| 验证 | 结果 |
| --- | --- |
| pnpm check | 0 errors / 0 warnings |
| pnpm build | PASS |
| cargo fmt --check | PASS |
| cargo test | 116 passed；专用副本迁移测试默认忽略 |
| 正式数据库副本迁移，显式运行 ignored 测试 | 1/1 PASS |
| 通知回归 | 2/2 PASS |
| 基本 UI 操作 | 17/17 PASS |
| 项目流转、Mock AI、后台任务、缓存安全 | 27/27 PASS |
| 进程重启后的持久化 | 4/4 PASS |
| pnpm tauri build | PASS，生成 0.1.1 NSIS 安装包 |
| 正式构建可见性矩阵 | 120/120 PASS |
| 中英文、大字号、弹窗、空审阅、搜索、后台面板、AI 错误通知 | 63/63 PASS |
| 正式构建功能回归 | 66/66 PASS |
| 本机安装后可见性复测 | 48/48 PASS |
| 本机安装后基本操作 | 17/17 PASS |
| 本机安装后持久化 | 4/4 PASS |
| 真实鼠标滚轮触达首页底部、设置版本读取 | PASS |

可见性矩阵：960×600/特大、1280×720/特大、1440×900/大、1920×1080/标准、1280×720/紧凑。边界测试另覆盖中英文及 1.5 倍截图像素密度。所有界面内容都是合成测试数据。

测试方法会逐层滚动 `auto/scroll` 容器，计算可视裁切边界，并使用命中测试检查控件是否受到遮挡；不会通过滚动 `overflow:hidden` 元素伪造可达。关闭的详情区不作为当前可见控件，展开的表单单独验证。

过程中修正了两项测试夹具问题：调试脚本顶层变量重复声明，以及业务回归生成已确认建议后，空审阅测试仍假设 confirmed 为空。修正夹具后完整重跑 63/63；未跳过失败检查。

### 04 · 本机更新与版本核对

- Action：使用正式安装包的保留数据更新方式安装到原目录，不传自动启动参数；之后以隔离环境启动安装后的程序验证。
- Expected：版本明确为 0.1.1，快捷方式指向该程序，正式数据库保持原样。
- Observed：安装退出码 0；注册表 DisplayVersion、文件 ProductVersion、窗口标题和设置中的版本均为 0.1.1；快捷方式正确。
- Evidence：`output/ui-0.1.1/installed-verification.json`、`installed-version.png`；正式数据库及 WAL、SHM 校验 3/3 一致。
- Verdict：PASS。

安装位置：`C:\Users\ZhouNan\AppData\Local\msl-desktop\msl-desktop.exe`

桌面快捷方式目标同上，参数为空。测试实例与本地 Mock 服务已关闭；用户从桌面打开时使用原有正式工作数据。

安装包：`src-tauri/target/release/bundle/nsis/msl-desktop_0.1.1_x64-setup.exe`

安装包 SHA256：`03615AFE3B9CB4ABC29F0CC62B270A8DC8B3D47F494A9166446C12C6D6F5851F`

安装后主程序 SHA256：`1FA009D9E788445165DD773C923FD0DF7F25514E04FC59A4C4941F521C34A51A`

## 正式数据保护

- 正式程序停止后留存数据库及 WAL、SHM 备份，未读取业务正文。
- 每次应用启动与写入测试同时隔离 APPDATA、LOCALAPPDATA、TEMP、TMP。
- 未读取、打印或调用真实 API Key；AI 网络测试均使用本地 Mock。
- 正式数据库副本迁移只处理 `.test-runtime/ui-visibility-20260905/formal-migration-copy`。
- 未清理或批量修改正式数据目录；未执行任何 Git 丢弃、提交或历史改写操作。
- 本轮未测试真实 OpenCode Go 的网络可用性，也未声称解决供应商网络故障。

最终正式文件 SHA256 与启动测试前一致：

| 文件 | SHA256 |
| --- | --- |
| msl-desktop.db | AE8F7F2E59A50B1857DB473DD73D8EC8F4A5AAD88D92081429782D1A3851FC25 |
| msl-desktop.db-wal | D3510975321DCD488FBFA2018CE4C04F01A77AB4C6BDBC9F4AE1D94FB40CFD48 |
| msl-desktop.db-shm | 01AE5E89F89FA96155EFEBB6099D299917347C28B57B4E49B8C50D9C94275816 |

## 视觉复核材料

主执行器已逐张查看首页底部、任务列表、弹窗底部和设置版本截图。最终用户视觉审阅尚待反馈。

- `output/ui-0.1.1/dashboard-top.png`
- `output/ui-0.1.1/dashboard-bottom.png`
- `output/ui-0.1.1/tasks-readable.png`
- `output/ui-0.1.1/modal-bottom.png`
- `output/ui-0.1.1/installed-version.png`

长内容和较小窗口允许页面滚动；重点内容字号保持可读，不再为追求一屏显示而裁掉按钮。首页建议卡片的理由仍保留两行预览，完整修改入口为“深入修改”。
