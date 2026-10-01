//! Official provider metadata. Subscription services retain their published scope.
use super::{ModelSeed, ProviderTemplate};

macro_rules! chat_models {
    ($($id:literal),* $(,)?) => { &[$(ModelSeed { model_id: $id, protocol: "chat_completions", endpoint_path: "/chat/completions" }),*] };
}

const ALI: &[ModelSeed] = chat_models![
    "qwen3.8-max",
    "qwen3.7-plus",
    "qwen3.8-flash",
    "deepseek-v4-pro-0813",
    "deepseek-v4.1-flash",
    "kimi-k3",
    "glm-5.2",
    "ZHIPU/GLM-5.3",
    "MiniMax-M3",
    "mimo-v2.5-pro"
];
const KIMI_CODE: &[ModelSeed] = chat_models![
    "k3",
    "k3-256k",
    "kimi-for-coding",
    "kimi-for-coding-highspeed"
];
const MINIMAX_PLAN: &[ModelSeed] = chat_models!["MiniMax-M3.1-Flash-Preview"];
const MINIMAX_CHAT: &[ModelSeed] = chat_models![
    "MiniMax-M3",
    "MiniMax-M3.1-Flash-Preview",
    "MiniMax-M2.7",
    "MiniMax-M2.5"
];
const TENCENT_CHAT: &[ModelSeed] =
    chat_models!["hy3", "glm-5.3", "kimi-k3", "minimax-m3", "mimo-v2.6-pro"];
const BAIDU_CHAT: &[ModelSeed] = chat_models![
    "deepseek-v3.2",
    "qwen3-32b",
    "qianfan-toytalk",
    "minimax-m2.5"
];
const TENCENT_PLAN: &[ModelSeed] = chat_models![
    "tc-code-latest",
    "deepseek/deepseek-flash",
    "deepseek-v4-flash-202605",
    "deepseek-v4-pro-202606",
    "minimax-m2.7",
    "minimax-m3",
    "glm-5.2",
    "glm-5.3",
    "glm-5.3-flash",
    "hy4-preview",
    "kimi-k2.7-code",
    "kimi-k3",
    "mimo-v2.6-flash"
];
const BAIDU_PLAN: &[ModelSeed] = chat_models![
    "qianfan-code-latest",
    "deepseek-v4.1-flash",
    "deepseek-v4-pro",
    "deepseek-v4-pro-0813",
    "deepseek-v4-flash-0731",
    "glm-5.3",
    "glm-5.3-flash",
    "glm-5.2",
    "glm-5.1"
];
const MIMO: &[ModelSeed] = chat_models!["mimo-v2.6-pro", "mimo-v2.6-flash"];
const ZHIPU: &[ModelSeed] = chat_models!["glm-5.3", "glm-5.3-flash", "glm-5.3-flashx", "glm-5.2"];
const ZHIPU_CODING: &[ModelSeed] = chat_models!["glm-5.3", "glm-5.3-flash"];
macro_rules! routed_models {
    ($($protocol:literal => $path:literal : [$($id:literal),* $(,)?]),* $(,)?) => {
        &[$($(ModelSeed { model_id: $id, protocol: $protocol, endpoint_path: $path }),*),*]
    };
}

// Exact documented IDs, not prefix inference: Go and Zen differ for several families.
const ZEN: &[ModelSeed] = routed_models![
    "responses" => "/responses": [
        "gpt-6-astra", "gpt-6.1-sol", "gpt-6-sol", "gpt-6-luna", "gpt-5.6-sol", "gpt-5.6-terra", "gpt-5.6-luna", "gpt-5.5", "gpt-5.5-pro", "gpt-5.4", "gpt-5.4-pro", "gpt-5.4-mini", "gpt-5.4-nano", "gpt-5.3-codex", "gpt-5.3-codex-spark", "gpt-5.2", "gpt-5.2-codex", "gpt-5.1", "gpt-5.1-codex", "gpt-5.1-codex-max", "gpt-5.1-codex-mini", "gpt-5", "gpt-5-codex", "gpt-5-nano", "grok-4.7", "grok-4.6", "grok-4.5", "grok-build-0.1", "muse-spark-1.3", "muse-spark-1.2", "muse-spark-1.3-contributor-free"
    ],
    "anthropic_messages" => "/messages": [
        "claude-fable-5-1", "claude-fable-5", "claude-opus-5-5", "claude-opus-5", "claude-opus-4-8", "claude-opus-4-7", "claude-opus-4-6", "claude-opus-4-5", "claude-sonnet-5", "claude-sonnet-4-6", "claude-sonnet-4-5", "claude-haiku-4-5", "qwen3.8-flash", "qwen3.7-max", "qwen3.7-plus", "qwen3.6-plus", "qwen3.5-plus"
    ],
    "chat_completions" => "/chat/completions": [
        "qwen3.8-max", "deepseek-v4.1-flash", "deepseek-v4-pro", "deepseek-v4-flash", "deepseek-v4-flash-vision-exp", "minimax-m3", "minimax-m2.7", "minimax-m2.5", "glm-5.3-flash", "glm-5.3", "glm-5.2", "glm-5.1", "glm-5", "kimi-k2.5", "kimi-k2.6", "kimi-k2.7-code", "kimi-k3", "big-pickle", "space-bunny-free", "longcat-2.5-preview-free", "mimo-v2.6-flash-free", "mimo-v2.5-free", "ling-3.0-flash-fin-free", "nemotron-3-ultra-free", "nemotron-3.5-lightning-free"
    ],
    "gemini" => "/models": ["gemini-3.8-flash", "gemini-3.7-flash", "gemini-3.6-flash", "gemini-3.5-flash", "gemini-3.5-flash-lite", "gemini-3.1-pro", "gemini-3-flash"],
    "systemone" => "/systemone": ["jev-1.13", "jev-1.13-free"]
];

pub const GO_ADDITIONS: &[ModelSeed] = routed_models![
    "responses" => "/responses": ["gpt-6-luna", "grok-4.7"],
    "chat_completions" => "/chat/completions": ["deepseek-v4.1-flash", "mimo-v2.6-flash", "mimo-v2.6-pro", "space-bunny-free", "longcat-2.5-preview-free"]
];

#[allow(clippy::too_many_arguments)]
fn preset(
    kind: &'static str,
    vendor: &'static str,
    display_name: &'static str,
    service_tier: &'static str,
    base_url: &'static str,
    models_endpoint: Option<&'static str>,
    docs_url: &'static str,
    note: &'static str,
    configurable: bool,
    models: &'static [ModelSeed],
) -> ProviderTemplate {
    ProviderTemplate {
        kind,
        vendor,
        display_name,
        service_tier,
        base_url,
        models_endpoint,
        auth_mode: "bearer",
        protocol: Some("chat_completions"),
        endpoint_path: Some("/chat/completions"),
        discovery: if models_endpoint.is_some() {
            "remote"
        } else {
            "curated"
        },
        docs_url,
        note,
        configurable,
        models: models.to_vec(),
    }
}

pub fn templates() -> Vec<ProviderTemplate> {
    let mut zen = preset(
        "opencode_zen",
        "OpenCode",
        "OpenCode Zen",
        "api",
        "https://opencode.ai/zen/v1",
        Some("https://opencode.ai/zen/v1/models"),
        "https://opencode.ai/docs/zen/",
        "按官方模型逐项匹配协议；Gemini 等未支持的协议保留在目录中，需确认兼容入口。",
        true,
        ZEN,
    );
    zen.protocol = None;
    zen.endpoint_path = None;
    let mut templates = vec![
        zen,
        preset("aliyun", "阿里云百炼", "阿里云百炼 API", "api", "https://dashscope.aliyuncs.com/compatible-mode/v1", None, "https://help.aliyun.com/zh/model-studio/list-models", "公共地址使用官方预置名单；填写业务空间专属地址后可刷新模型目录。按账户地域和权限配置。", true, ALI),
        preset("aliyun_coding", "阿里云百炼", "阿里云百炼 Coding Plan", "plan", "https://coding.dashscope.aliyuncs.com/v1", None, "https://help.aliyun.com/zh/model-studio/coding-plan", "官方限定交互式编程工具，禁止自定义应用后端使用；本工作台不提供该套餐连接。请使用普通 API。", false, ALI),
        preset("aliyun_tokenplan", "阿里云百炼", "阿里云百炼 Token Plan", "plan", "https://token-plan.cn-beijing.maas.aliyuncs.com/compatible-mode/v1", None, "https://help.aliyun.com/en/model-studio/base-url", "官方限定交互式编程工具，禁止自定义应用后端使用；本工作台不提供该套餐连接。请使用普通 API。", false, ALI),
        preset("volcengine", "火山引擎", "火山方舟 API", "api", "https://ark.cn-beijing.volces.com/api/v3", None, "https://www.volcengine.com/docs/82379/1330310", "API Key 无法调用需 AK 签名的模型管理目录；请从方舟控制台填写已开通的模型 ID 或推理接入点 ID。", true, &[]),
        preset("volcengine_coding", "火山引擎", "火山方舟 Coding Plan", "plan", "https://ark.cn-beijing.volces.com/api/coding/v3", None, "https://www.volcengine.com/docs/82379/1928261", "Coding Plan 专属凭据和模型范围；使用前确认套餐适用工具。模型管理目录需要独立 AK 签名，当前提供手动配置。", true, &[]),
        preset("volcengine_agent_plan", "火山引擎", "火山方舟 Agent Plan", "plan", "https://ark.cn-beijing.volces.com/api/plan/v3", None, "https://docs.volcengine.com/docs/ark/list-ark-agent-plan-model-api?lang=en", "Agent Plan 与 Coding Plan 入口独立；模型管理目录需要 AK 签名，当前请从控制台填写可用模型 ID。", true, &[]),
        preset("zhipu", "智谱", "智谱 BigModel API", "api", "https://open.bigmodel.cn/api/paas/v4", None, "https://docs.bigmodel.cn/cn/guide/start/model-overview", "使用普通 API Key；模型列表以官方文档预置为基础，可手动添加账户已开通模型。", true, ZHIPU),
        preset("zhipu_coding", "智谱", "智谱 GLM Coding Plan", "plan", "https://open.bigmodel.cn/api/coding/paas/v4", None, "https://docs.bigmodel.cn/cn/coding-plan/latest-model", "官方限定指定兼容工具，MSL Desktop 未列入适配名单；本工作台不提供该套餐连接。请使用普通 API。", false, ZHIPU_CODING),
        preset("kimi", "Kimi", "Kimi Moonshot API", "api", "https://api.moonshot.cn/v1", Some("https://api.moonshot.cn/v1/models"), "https://platform.kimi.com/docs/api/list-models", "普通 API 按量计费；请使用开放平台 Key 和账户已开通的模型。", true, &[]),
        preset("kimi_code", "Kimi", "Kimi Code", "plan", "https://api.kimi.com/coding/v1", None, "https://www.kimi.com/code/docs/", "面向编程工具的会员权益；产品集成建议使用普通 API，请确认当前会员适用范围。", true, KIMI_CODE),
        preset("minimax", "MiniMax", "MiniMax API", "api", "https://api.minimax.cn/v1", Some("https://api.minimax.cn/v1/models"), "https://platform.minimaxi.com/docs/api-reference/models/openai/list-models", "普通 API Key；目录返回平台模型，具体调用权限以账户开通情况为准。", true, &[]),
        preset("minimax_tokenplan", "MiniMax", "MiniMax M Plan / Token Plan", "plan", "https://api.minimax.cn/v1", Some("https://api.minimax.cn/v1/models"), "https://platform.minimaxi.com/docs/m-plan/other-tools", "使用订阅专属 Key；可刷新平台模型目录，具体套餐权益以账户为准，旧 Token Plan 以续订权益为准。", true, MINIMAX_PLAN),
        preset("tencent", "腾讯云", "腾讯 TokenHub API", "api", "https://tokenhub.tencentmaas.com/v1", Some("https://tokenhub.tencentmaas.com/v1/models"), "https://cloud.tencent.com/document/product/1823/130078", "普通 API 按量入口；使用 TokenHub API Key，可刷新账户模型目录。", true, &[]),
        preset("tencent_tokenplan", "腾讯云", "腾讯 Token Plan", "plan", "https://api.lkeap.cloud.tencent.com/plan/v3", None, "https://cloud.tencent.com/document/product/1823/130060", "官方禁止自动化脚本、自定义应用后端与非交互批处理；本工作台不提供该套餐连接。请使用普通 API。", false, TENCENT_PLAN),
        preset("baidu", "百度千帆", "百度千帆 API", "api", "https://qianfan.baidubce.com/v2", Some("https://qianfan.baidubce.com/v2/models"), "https://cloud.baidu.com/doc/qianfan-api/s/Dmba8k71y", "普通 API Key；可从官方模型目录刷新，调用权限以账户为准。", true, &[]),
        preset("baidu_tokenplan", "百度千帆", "百度千帆 Token Plan", "plan", "https://qianfan.baidubce.com/v2/tokenplan/personal", None, "https://cloud.baidu.com/doc/qianfan/s/Dmrabu8b6", "官方限定兼容编程和智能体工具交互使用，禁止自动化脚本与应用后端；本工作台不提供该套餐连接。", false, BAIDU_PLAN),
        preset("mimo", "小米 MiMo", "小米 MiMo API", "api", "https://api.xiaomimimo.com/v1", Some("https://api.xiaomimimo.com/v1/models"), "https://mimo.mi.com/docs/en-US/api/model/list-models", "普通 API 使用 sk- Key；可以刷新模型目录。", true, MIMO),
        preset("mimo_tokenplan", "小米 MiMo", "小米 MiMo Token Plan", "plan", "https://token-plan-cn.xiaomimimo.com/v1", None, "https://mimo.mi.com/docs/tokenplan/subscription", "官方限定兼容编程与智能体工具并禁止自定义应用后端；本工作台不提供该套餐连接。请使用普通 API。", false, MIMO),
        preset("siliconflow", "硅基流动", "硅基流动 API", "api", "https://api.siliconflow.cn/v1", Some("https://api.siliconflow.cn/v1/models?sub_type=chat"), "https://api-docs.siliconflow.cn/docs/api/models-get", "目录限定聊天模型；新模型使用兼容 Chat Completions 协议。", true, &[]),
    ];
    // These catalogs cover multiple product capabilities or make no guarantee
    // that every future model supports conversational text generation.
    for template in &mut templates {
        if matches!(
            template.kind,
            "tencent" | "baidu" | "minimax" | "minimax_tokenplan"
        ) {
            template.protocol = None;
            template.endpoint_path = None;
        }
        if matches!(template.kind, "minimax" | "minimax_tokenplan") {
            template.models = MINIMAX_CHAT.to_vec();
        }
        if template.kind == "tencent" {
            template.models = TENCENT_CHAT.to_vec();
        }
        if template.kind == "baidu" {
            template.models = BAIDU_CHAT.to_vec();
        }
    }
    templates
}

pub fn template(kind: &str) -> Option<ProviderTemplate> {
    templates()
        .into_iter()
        .find(|template| template.kind == kind)
}
