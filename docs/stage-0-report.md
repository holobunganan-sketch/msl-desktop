# Stage 0 完成报告

> 日期：2026-08-13
> 项目：msl-desktop（Tauri 2 + Svelte 5 + TypeScript）
> 依据：`DEEPSEEK_V4_FLASH_MSL_DESKTOP_DEVELOPMENT_GUIDE.md` §14

## 1. 本阶段目标

建立最小可运行 Tauri 2 + Svelte + TypeScript 项目，验证 Windows 开发环境与
官方模板可启动、可关闭，产出 `docs/architecture.md` 初稿并做本地 commit。

## 2. 实际完成

- 环境验证：Rust / Cargo / MSVC toolchain / Node / pnpm / WebView2 全部可用（记录见下）；
- 使用官方 `create-tauri-app`（svelte-ts 模板，pnpm 管理器）初始化 `msl-desktop/`；
- `pnpm install` 成功；
- `pnpm check`（TypeScript + Svelte check）通过：0 errors / 0 warnings；
- `cargo check` 通过；
- `pnpm tauri dev` smoke test 通过：窗口正常打开（标题 msl-desktop）、正常关闭、无残留进程；
- 模板 `tauri.conf.json` 的 bundle targets 为 "all"（含 NSIS），本阶段未做 installer；
- `docs/architecture.md` 初稿完成；
- 纯本地 Git 初始化，无 remote。

## 3. 修改/新增文件

- `msl-desktop/` 全部（create-tauri-app 官方 svelte-ts 模板）
- `msl-desktop/package.json` — 增加 `pnpm.onlyBuiltDependencies: ["esbuild"]`（pnpm 10 默认阻止构建脚本，需显式允许 esbuild postinstall）
- `msl-desktop/docs/architecture.md` — 架构初稿
- `msl-desktop/docs/stage-0-report.md` — 本报告

## 4. 执行的验证

- command: `pnpm install`
  result: Done in 14.3s
- command: `pnpm check`
  result: svelte-check found 0 errors and 0 warnings
- command: `cargo check`
  result: Finished `dev` profile target(s) in 2m 13s
- command: `pnpm tauri dev`（后台）+ PowerShell 进程/窗口检查
  result: msl-desktop.exe 运行，MainWindowTitle=msl-desktop，Working Set ≈ 39 MB (dev)
- command: PowerShell `CloseMainWindow()`
  result: True；3 秒后进程确认退出；无 msl-desktop/vite/node 残留进程

## 5. 环境记录

| 组件 | 版本 | 状态 |
| --- | --- | --- |
| Windows | 11（本机） | OK |
| Rust | 1.97.1 (8bab26f4f 2026-07-14) | OK |
| Cargo | 1.97.1 (c980f4866 2026-06-30) | OK |
| Toolchain | stable-x86_64-pc-windows-msvc | OK |
| MSVC linker | link 测试通过 | OK |
| Node.js | v26.7.0 | OK |
| pnpm | 10.34.5 | OK |
| npm | 11.19.0 | OK |
| WebView2 Runtime | 151.0.4129.72 / .78 | OK |
| Tauri CLI | 2.11.4（@tauri-apps/cli） | OK |
| tauri crate | 2.11.5 | OK |
| Svelte | 5.56.9 | OK |

## 6. 性能

- RAM：dev 模式窗口打开 Working Set ≈ 39 MB（仅供参考，指南 §4 要求以 Release 判定，属 Stage 10 工作）；
- CPU：无后台持续任务，模板 idle 正常；
- 说明：本阶段不建立性能脚本（`scripts/measure-memory.ps1` 属 Stage 1 交付）。

## 7. 与指南的偏差

1. 模板结构：指南 §5 示例为纯 Svelte + Vite，官方 svelte-ts 模板使用
   SvelteKit + `@sveltejs/adapter-static`（`src/routes/`）。属于 create-tauri-app
   官方初始化结构，指南 §14 允许"先运行原始模板"，职责边界（前端只做展示/交互，
   核心在 Rust）不受影响。
2. `package.json` 增加 pnpm `onlyBuiltDependencies` 配置，解决 pnpm 10 默认忽略
   esbuild 构建脚本的问题（否则 vite 无法工作）。非架构变更。

## 8. 已知问题

- 无阻塞问题。cargo check 首次两次因 `src-tauri/target` 目录文件锁定
  （os error 32）失败，手动创建目录后重试成功——判断为暂时性文件系统/防病毒
  锁定，后续未再出现。

## 9. 当前本地 Git 状态

- branch: master（初始）
- HEAD: 待首次 commit
- working tree: 全部未跟踪
- remote: 无（符合指南 §13，纯本地 Git）

## 10. 下一阶段

Stage 1 — Resident Core / Tray / UI 生命周期：
- System Tray、Main Window 销毁/重建、Tray Exit、单实例、AppState 骨架；
- `scripts/measure-memory.ps1` 性能脚本；
- 验收：打开→Close→托盘常驻→点击托盘→窗口重建→Exit→进程结束。

停止，不自动进入下一阶段。
