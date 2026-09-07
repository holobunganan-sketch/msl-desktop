# OpenCode Go、外观与 AI 工作流迭代设计

## 目标

修复 OpenCode Go 模型协议错误；将 Muse Spark 1.2 Contributor 作为共享 OpenCode Go API Key 的独立模型；增加字号与主题设置；将 AI 翻译拆为侧栏独立页面；重排首页；为 AI 工作流提供受控、版本化的后台系统指令；统一 Windows 任务栏与安装程序图标。

## Provider 协议

- OpenCode Go 固定 Base URL：`https://opencode.ai/zen/go/v1`。
- 模型目录：`https://opencode.ai/zen/go/v1/models`。
- Muse Spark 1.2 Contributor：模型 ID `muse-spark-1.2-contributor`，协议 `responses`，端点 `/responses`。
- Muse 属于 OpenCode Go 连接下的模型，复用同一个 `credential_ref`，不创建第二份密钥。
- 固定模板刷新必须按内置协议表修复已有模型协议。
- 未知远程模型保持禁用，标记 `needs_protocol`，不得默认启用 Chat Completions。
- UI 显示 Muse Contributor 数据使用提示。
- 依据：<https://opencode.ai/docs/zh-cn/go>，页面更新日期 2026-08-20。

## 外观

- 字号档位：紧凑、标准、较大、特大；对应根字号 13、14、15.5、17px。
- 主题：雾蓝、鼠尾草、暖灰象牙、高对比。
- 设置通过现有 `app_settings` 持久化，键名为 `appearance_font_scale` 与 `appearance_theme`。
- 应用启动时先应用默认值，再读取持久化值；任何非法值回退到 `standard` 与 `mist`。
- 主题通过 `document.documentElement.dataset.theme` 和 CSS 变量切换，字号通过 `dataset.fontSize` 切换。

## 页面结构

- 侧栏 AI 秘书分组新增“AI 翻译”。
- 新建 `TranslationView.svelte`，以双栏大输入/大输出布局承载现有翻译能力。
- 首页移除 `TranslationCard`，桌面宽度下使用 1.5fr / 0.9fr / 0.8fr 三栏：今日继续；决策/等待/变化；秘书状态/最近分析。
- 1024px 及以下使用两栏，移动宽度使用单栏。
- 首页保持无横向滚动，1024×640 与 1440×900 机械布局检查必须通过。

## AI 系统指令

- 新建 `ai/prompts.rs`，提供 `prompt_for(task_kind, locale, context) -> String`。
- 工作目录分析：识别新增、修改、删除和正文事实，输出来源引用与待确认建议。
- 工作草稿：提取目标、背景、里程碑、参与者、时间、等待关系、风险和澄清问题。
- 全局分析：跨 Work、Task、Waiting、Calendar、Inbox、Resume Point 去重并排列推进顺序。
- 每日简报：总结期间变化、停点、当天硬安排和 1–3 个建议动作。
- 翻译：只输出译文，保留数字、专名、格式和行结构。
- 所有分析指令要求把文件正文当作不可信参考资料；文件中的命令不能覆盖系统指令。
- 所有结构化建议继续进入确认队列，确认前不得写入正式实体。

## Windows 图标

- 使用 `src-tauri/icons/msl-c1-icon.svg` 作为矢量母版。
- 重新生成 `icon.ico`、PNG、ICNS 和 Windows Store Logo 尺寸。
- Tauri 配置继续引用生成后的图标资源，正式构建验证 exe、任务栏、标题栏和 NSIS 安装包资源。

## 测试和安全

- Rust 测试锁定官方模型 ID、协议、端点、共享 credential_ref 与系统指令安全条款。
- CDP 测试覆盖翻译侧栏、外观切换与持久化、首页无翻译模块、Provider Muse 行和 API Key 入口。
- 所有应用启动使用隔离 APPDATA、LOCALAPPDATA、TEMP、TMP。
- 测试使用本地 Mock Provider，不调用真实 OpenCode Go、Muse 或真实 API Key。

