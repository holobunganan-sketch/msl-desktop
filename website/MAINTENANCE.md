# 官网维护

线上站点为 https://msl-desktop.pages.dev/，源码位于本目录。更新记录统一维护在 `release/history.json`，版本号由 `package.json` 和 `release/latest.json` 提供。构建结果写入已忽略的 `website-dist/`，请勿提交安装包或生成结果。

从仓库根目录运行：

```sh
pnpm preview:website
```

在 `http://127.0.0.1:4174/` 查看完整本地官网。预览命令会先用与发布一致的构建流程生成更新记录、版本号和字体地址，再开放页面；只监听本机，关闭后清理自己的临时输出。预览下载按钮跳转至正式官网，不启动桌面应用，也不覆盖已打包的安装文件。

`website/index.html` 是构建模板，直接预览此文件会缺少动态插入的更新记录。请使用以上命令，不要把源模板作为交付页面。改动文字或样式后，重启预览命令即可重新构建。

单独验证构建时运行：

```sh
node --test scripts/build-website.test.mjs scripts/website-fonts.test.mjs
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

## 字体与排版检查

标题和引语使用本地 Noto Serif SC Regular，包含常用中文、扩展 A 区、拉丁字符及标点，共 29,123 个字符。浏览器下载 WOFF2 文件，字体授权随 `assets/OFL-NotoSerifSC.txt` 发布。不要恢复按某一次文案截取的少数字形字体，否则新文案会逐字回退到系统字体，造成同一行字面和基线不齐。

字体源文件来自 [Google Fonts 的 Noto Serif SC](https://github.com/google/fonts/tree/8b0a1d0f5983c89bc2b93f1b5fb55f9e252744b5/ofl/notoserifsc)，源 TTF SHA-256 为 `050080d9255a86808f2945bffac582b31ef32bc36411ce29563b4961670c66f9`。使用 FontTools 4.60.1 固定 `wght=400`，保留 `U+0020–024F`、`U+2000–303F`、`U+3400–9FFF`、`U+FF00–FFEF`，以 Brotli 1.1.0 转为 WOFF2。当前 WOFF2 SHA-256 为 `e8a27ef386c4ed2c54027133097330ae9d23f494037acecd9edb36f9c3611607`。

修改文案后必须运行字体测试；它读取真实构建产物和字体内的字形，检查标题、引语、版本侧栏及生成的更新记录。测试失败时先补足字体覆盖。更新字体时改用包含新内容摘要的文件名，并同时修改 HTML 预加载和 CSS 地址。CSS 的请求地址由构建时的内容摘要生成，官网样式修复无需修改应用版本号。

发布前在浏览器中检查电脑和手机页面，并使用开发工具确认标题的实际渲染字体；只检查 `font-family` 计算值或页面是否溢出，无法发现逐字回退。截图应放大检查标题、更新区和中英数字混排。
