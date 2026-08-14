# Stage 12 完成报告 — 本地 NSIS setup.exe（最终阶段）

> 日期：2026-08-14
> 项目：msl-desktop
> 依据：`DEEPSEEK_V4_FLASH_MSL_DESKTOP_DEVELOPMENT_GUIDE.md` §26

## 1. 本阶段目标

产生本地可安装程序（NSIS setup.exe），本机完成安装/卸载验证与
Defender 扫描，产出最终交付报告。不签名、不上 GitHub、不上传、不 Release。

## 2. 实际完成

- `pnpm tauri build`（Release + NSIS）成功；
- 本机静默安装验证：安装 → 开始菜单 → 启动 → 数据 → 关闭进 tray → 重启 → 卸载；
- Defender 实时保护检查（自定义扫描脚本已交付，需管理员运行）；
- 最终报告与产物哈希。

## 3. 产物

| 项 | 值 |
| --- | --- |
| App executable | `C:\Myfolder\MSL cowork\msl-desktop\src-tauri\target\release\msl-desktop.exe` |
| setup.exe | `C:\Myfolder\MSL cowork\msl-desktop\src-tauri\target\release\bundle\nsis\msl-desktop_0.1.0_x64-setup.exe` |
| exe 大小 | 15,974,400 B（~15.2 MB） |
| setup 大小 | 4,130,237 B（~3.9 MB） |
| exe SHA-256 | `3A90C862A75B6E883D6E65DABF898F86C1C10E459F0038CEE03D149D5D80B375` |
| setup SHA-256 | `A632FCD5123B09DDE456DA89F44869386AFD999B8F9B695384650123CD2A7C34` |

## 4. 安装测试结果

| 步骤 | 结果 |
| --- | --- |
| 从 setup.exe 安装（/S 静默，currentUser 无需管理员） | ✅ 安装至 `%LOCALAPPDATA%\msl-desktop` |
| 开始菜单入口 | ✅ `...\Start Menu\Programs\msl-desktop.lnk` |
| 启动 | ✅ 窗口 "MSL Desktop"，页面 `http://tauri.localhost/` |
| 数据（Work/Task/Inbox 创建） | ✅ |
| 关闭窗口进入 tray | ✅ 进程常驻 |
| 重启后数据保持 | ✅ works=1 / tasks=1 / inbox=1 完整 |
| 卸载 | ✅ 开始菜单移除、程序文件清理 |
| 用户数据保留 | ✅ `%APPDATA%\MSLDesktop` 数据库完整保留（指南 §26 策略） |
| 卸载残留 | WebView2 用户数据目录（`msl-desktop.exe.WebView2`）保留——浏览器规范行为，避免丢失状态 |

## 5. Defender 结果

- **实时保护（RealTimeProtection）开启**；检测历史中无任何 msl-desktop
  相关记录（历史检测均为其他第三方文件）——产物构建与多次运行期间未被拦截；
- **自定义完整扫描**：`scripts/defender-scan.ps1`（Start-MpScan / MpCmdRun）
  已交付，需以**管理员**身份运行（UAC 提权无法在自动化中确认）；
- SmartScreen "未知发布者" 提示属 unsigned local build 预期限制（指南 §26）。

## 6. 性能（Release，Stage 10 数据）

- tray RAM：Working Set 28.7–36.5 MB（目标 ≤ 80 MB）✅
- 窗口打开（Today）：Private 141–155 MB（目标 ≤ 220 MB）✅
  （Working Set ~425 MB 含 WebView2 共享内存重复计数，performance.md 已说明）
- idle CPU：0.017%（180s 采样，目标 < 0.5%）✅
- 10× open/close：tray 增长 +7.56 MB（目标 ≤ 15 MB）✅
- cold start：0.4–1.1 s

## 7. 修改/新增文件

- `src-tauri/tauri.conf.json` — bundle.targets 改为 ["nsis"]
- `scripts/defender-scan.ps1` — 新增 Defender 扫描
- `docs/stage-12-report.md` — 本报告

## 8. 已知限制

- 未签名（SmartScreen 未知发布者提示，本地阶段预期）；
- Defender 自定义完整扫描需管理员手动运行（脚本已交付）；
- WebView2 用户数据目录卸载后保留（规范行为）；
- 真实 DeepSeek 成功调用需用户提供 API Key 后验证（协议已由 mock + 401 验证）。

## 9. 当前本地 Git 状态

- branch: master
- HEAD: a914a75（Stage 11）→ 本 Stage 提交后更新
- working tree: 待提交
- remote: 无（从未添加）

## 10. 开发阶段结束

按指南 §33：setup.exe 已 build / scan（实时保护无检测）/ install / smoke test /
uninstall 全部完成，性能达到预算。本地 Alpha 开发阶段结束，等待用户真实使用反馈。
不自动进入"企业化/知识库/云同步/GitHub Release"等后续工作。
