# 模型连接

最后核对官方文档：2026-10-01。模型与套餐会变化，服务资格、地区、价格和限额以厂商账户及最新条款为准。

## 设置步骤

1. 在设置页点击“添加连接”，下拉选择厂商和接入方式。
2. 填写连接昵称与对应 API Key。普通 API 与订阅可能使用不同凭据；即使地址相同，也应分别保存连接。
3. 如账户使用专属工作空间、不同地域或代理入口，在“端点与认证设置”中填写对应基础地址和模型发现地址。
4. 保存后展开厂商，点击“刷新模型”。模型按接口协议分组，可继续折叠；更改启停状态或刷新不会重新展开全部目录。
5. 在 AI 任务分工中设置全局默认模型，必要时为某一任务单独指定。

“远程目录已列出”只说明接口返回了模型。部分服务返回全平台目录，套餐权益、地区和访问权限需另外确认。连接测试会提交一条简短请求，可能产生少量费用；添加或刷新连接不会自动执行推理。

## 目录与官方来源

| 厂商 | 常规 API 基础地址 | 目录策略 / 官方文档 |
| --- | --- | --- |
| DeepSeek | `https://api.deepseek.com` | `/models`；[官方文档](https://api-docs.deepseek.com/) |
| 阿里云百炼 | `https://dashscope.aliyuncs.com/compatible-mode/v1` 或业务空间专属地址 | 公共旧地址保留兼容；业务空间地址可用 `/api/v1/models` 分页目录。[目录 API](https://help.aliyun.com/zh/model-studio/list-models)、[域名说明](https://help.aliyun.com/zh/model-studio/regions/) |
| 火山方舟 | `https://ark.cn-beijing.volces.com/api/v3` | 已查到的管理目录需要 AK 签名，不能用模型 API Key 代替。按控制台填写模型或推理接入点 ID。[目录 API](https://docs.volcengine.com/docs/ark/list-model-activations-api?lang=en) |
| 智谱 | `https://open.bigmodel.cn/api/paas/v4` | 本轮未找到官方文档化的 GET 模型列表；提供参考名单及手动配置。[模型文档](https://docs.bigmodel.cn/cn/guide/start/model-overview) |
| Kimi | `https://api.moonshot.cn/v1` | `/models`；[模型列表](https://platform.kimi.com/docs/api/list-models) |
| MiniMax | `https://api.minimax.cn/v1` | `/models`，平台目录不代表订阅权益。[模型列表](https://platform.minimaxi.com/docs/api-reference/models/openai/list-models) |
| 腾讯 TokenHub | `https://tokenhub.tencentmaas.com/v1` | `/models` 返回跨模态目录；仅为已确认的对话模型配置聊天协议。[API 文档](https://cloud.tencent.com/document/product/1823/130078) |
| 百度千帆 | `https://qianfan.baidubce.com/v2` | `/models` 包含聊天、向量、图像和视频等类型，按返回类型识别。[模型列表](https://cloud.baidu.com/doc/qianfan-api/s/Dmba8k71y) |
| 小米 MiMo | `https://api.xiaomimimo.com/v1` | `/models`；[模型列表](https://mimo.mi.com/docs/en-US/api/model/list-models) |
| 硅基流动 | `https://api.siliconflow.cn/v1` | `/models?sub_type=chat` 只获取当前工作台使用的对话模型。[目录 API](https://api-docs.siliconflow.cn/docs/api/models-get) |
| OpenCode Go / Zen | `https://opencode.ai/zen/go/v1` / `https://opencode.ai/zen/v1` | 分别调用 `/models`；按各自官方表匹配协议。[Go](https://opencode.ai/docs/go/)、[Zen](https://opencode.ai/docs/zen/) |

阿里百炼北京业务空间示例：基础地址填写 `https://你的业务空间ID.cn-beijing.maas.aliyuncs.com/compatible-mode/v1`，目录地址为同一域名下的 `/api/v1/models?capabilities=TG`。不要将文档中的占位符原样保存，也不要把需要管理凭据的部署 API 当成模型发现入口。

## Coding / Token Plan 的边界

下拉菜单同时保留套餐选项及官方链接。对官方明确禁止自建应用或限定封闭客户端名单的方案，预设显示限制说明，无法直接创建新连接。不会篡改 User-Agent、绕过客户端限制，或把订阅 Key 发送至另一个付费入口。

- 阿里：[接口与套餐范围](https://help.aliyun.com/en/model-studio/base-url)
- 腾讯：[Token Plan 使用范围](https://cloud.tencent.com/document/product/1823/130060)
- 百度：[个人套餐说明](https://cloud.baidu.com/doc/qianfan/s/Dmrabu8b6)
- 智谱：[指定工具名单](https://docs.bigmodel.cn/cn/coding-plan/tool/others)
- 小米：[订阅说明](https://mimo.mi.com/docs/tokenplan/subscription)
- MiniMax：[M Plan 通用工具配置](https://platform.minimaxi.com/docs/m-plan/other-tools)
- Kimi：[Code 与普通 API 区别](https://www.kimi.com/code/docs/)

已有连接不会因新增模板而被删除。请自行核对当前服务允许的使用场景；套餐介绍和兼容协议均不构成厂商授权。

## 协议、状态与安全

- OpenCode 的 Responses、Chat Completions、Anthropic Messages 使用各自端点。Go 和 Zen 不共享同一套模型映射。
- Gemini 原生、System One 等当前未适配协议可以出现在发现目录中，标为待配置，不会冒充兼容协议发起请求。
- 目录中的图片生成、视频、向量及重排模型不会自动变成秘书的聊天模型；无法确认用途的新条目保留待配置状态。
- 刷新先完整读取和校验目录，再通过事务更新。认证失败、网络故障、分页损坏时保留原目录。
- 模型刷新保留模型 ID、任务绑定、已停用状态、手动协议及能力设置；手动条目不会因远程目录缺失被自动删除。
- API Key 保存在系统凭据库。隔离测试使用进程内合成凭据，不访问用户凭据库；测试仅访问回环 Mock 服务。
- 应用只使用实际配置的认证方式。HTTP 重定向不转发密钥，需在连接设置中填入厂商正式的新地址。
