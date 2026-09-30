# 官网维护

线上站点为 https://msl-desktop.pages.dev/，源码位于本目录。更新记录统一维护在 `release/history.json`，版本号由 `package.json` 和 `release/latest.json` 提供。构建结果写入已忽略的 `website-dist/`，请勿提交安装包或生成结果。

从仓库根目录运行：

```sh
node --test scripts/build-website.test.mjs
node scripts/build-website.mjs --source-only
```

`--source-only` 用于页面预览，不包含安装包，不能作为正式交付目录。更新正文在构建时直接写入 HTML，版本号和折叠记录在关闭 JavaScript 时仍可阅读。

完整构建默认读取 `src-tauri/target/release/bundle/nsis/MSL Desktop_<version>_x64-setup.exe`。已有经过验证的发布安装包时，可显式指定路径：

```sh
node scripts/build-website.mjs
node scripts/build-website.mjs --installer "path/to/MSL-Desktop-Windows-x64.exe"
```

显式指定安装包时，由发布流程确认其版本，并与 GitHub Release 的校验值核对。脚本将给定文件原样复制到固定下载地址 `/downloads/MSL-Desktop-Windows-x64.exe`，重新计算 `/downloads/SHA256SUMS.txt`，并写入 `/release/latest.json`。脚本不安装、启动应用，也不访问应用资料目录。

每次更新，在 `release/history.json` 顶部添加一个版本。`released` 配合 `dateKind: published` 表示可验证的公开发布；`local` 配合 `dateKind: local` 表示本地历史里程碑。当前待发布版本使用 `current` 和 `local`，完成公开发布后可改为 `released` / `published`，并补入真实发布链接及时间。网页日期按 Asia/Shanghai 表示，不把本地提交时间写成公开发布时间。

构建只收集列明的 HTML、CSS、JavaScript、站点配置与静态媒体，跳过源码中的下载目录、数据库、环境文件和其他文件类型。发布前核对页面、安装包校验值、公开历史来源，再上传 `website-dist/` 到现有 Cloudflare Pages 项目。
