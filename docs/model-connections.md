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
- OpenCode 官方 Messages 入口使用 `x-api-key`；Chat Completions 与 Responses 使用 Bearer。连接默认 Bearer 时，Messages 自动使用同一把 Key 生成正确认证头；显式“无需认证”或 API Key 设置保持不变。自定义网关不套用此规则。
- HTTP 重定向不转发密钥，需在连接设置中填入厂商正式的新地址。

## OpenCode 分流依据

v0.3.22 参考 [Reasonix 的协议契约](https://github.com/indielab/Reasonix/blob/6845b6de3c5e079737a3b010ebcdd87f6cd0228c/internal/provider/opencode_contract.go)及[推荐预设](https://github.com/indielab/Reasonix/blob/6845b6de3c5e079737a3b010ebcdd87f6cd0228c/internal/config/provider_presets_opencode_go.go)，将推荐入口与兼容入口分开。实现保持独立，模型默认表以本轮核对的 OpenCode 官方文档为准。

| 协议 | Go 请求路径 | 认证 |
| --- | --- | --- |
| Chat Completions | `/zen/go/v1/chat/completions` | `Authorization: Bearer …` |
| Anthropic Messages | `/zen/go/v1/messages` | `x-api-key: …`，另含 `anthropic-version` |
| Responses | `/zen/go/v1/responses` | `Authorization: Bearer …` |

Zen 对应路径去掉 `/go`，模型分组独立维护。Messages 的认证另核对了 [OpenCode 官方服务端实现](https://github.com/anomalyco/opencode/blob/0112a92c416f5ad833d96e7a8308441f0a875d94/packages/console/app/src/routes/zen/go/v1/messages.ts#L5)。

展开模型并点击“编辑”可选择入口，无需重复填写 Key。Go 的 DeepSeek V4 Flash 提供三种入口；部分 Qwen、MiniMax 模型保留参考项目中的 Chat 兼容入口。仅列出已核对的精确模型 ID，不将一个模型的兼容性外推到整个模型系列或 Zen。DeepSeek Pro 继续推荐 Chat：[Reasonix 使用说明](https://github.com/indielab/Reasonix/blob/6845b6de3c5e079737a3b010ebcdd87f6cd0228c/docs/GUIDE.md#L369)记录了其他入口的上游转换问题，因此本轮未将其加入快捷选项；手动配置仍保留。

这些选项用于明确请求协议，无法保证账户拥有模型权限或上游始终提供相同兼容能力。手动协议会在刷新后保留；一次请求只走选定入口，不会自动向三个入口重复提交。Responses 使用 `store: false`，继续发送本次所需上下文。应用使用自己的 User-Agent，不冒充 Reasonix 或其他客户端。

测试通过本地 Mock 检查实际路径、认证头、请求体和刷新后的持久状态。此验证不包含真实付费模型调用。
