# GitHub 推送核查报告

**时间：** 2026-09-07
**仓库：** https://github.com/holobunganan-sketch/msl-desktop.git
**分支：** master（默认分支）
**本地 HEAD：** `2bca1bc`（与远程 `origin/master` 完全同步）

## 结论

✅ 推送已完成，本地与云端一致，公开仓库无敏感信息与构建产物泄漏。

## 核查明细

| 核查项 | 结果 |
|---|---|
| 云端分支存在 | ✅ 远程 `master` = `2bca1bc`，与本地一致 |
| 本地工作区 | ✅ 干净（0 个未提交变更） |
| 已跟踪文件 | 360 个 |
| 构建产物 | ✅ 未跟踪：`target/`、`build/`、`.svelte-kit/`、`node_modules/` 均被 `.gitignore` 排除 |
| 磁盘大文件 >500KB | ✅ 无 |
| 密钥/token（sk-/ghp_/github_pat/AKIA） | ✅ 无 |
| 凭证赋值（api_key/secret/password=值） | ✅ 无 |
| 测试样本数据 | ✅ 均为 UTF-8 占位文本，无真实临床内容 |

## 关于本地路径的提示

`docs/iterations/` 等多份文档含开发机绝对路径 `C:\Users\ZhouNan\...`（安装/测试记录），属正常开发日志，非敏感凭据；若追求整洁可后续清理，不构成风险。

## 测试样本说明（tests/fixtures/workspace-small/）

`数据表.xlsx`、`研究方案V3.docx`、`统计计划.sas7bdat` 实际是内容为纯文本的占位假文件（如"入组数据"），仅模拟目录结构供测试，**不含真实患者或单位信息**。
