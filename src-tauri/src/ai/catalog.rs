//! 固定 Provider 模板与远端模型目录刷新。

use crate::db::provider::{ProviderCatalogRepo, ProviderConnection};
mod provider_presets;

/// Declared model capacities are local metadata, never extra wire parameters.
/// Unknown capacities remain unknown; conflicting declarations use the lower bound.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ModelCapabilities {
    pub max_context_tokens: Option<u32>,
    pub max_input_tokens: Option<u32>,
    pub max_output_tokens: Option<u32>,
}

pub fn model_capabilities(model: &crate::db::provider::ProviderModel) -> ModelCapabilities {
    let value: serde_json::Value =
        serde_json::from_str(&model.capabilities_json).unwrap_or_default();
    let capacity = |paths: &[&str]| {
        paths
            .iter()
            .filter_map(|path| value.pointer(path).and_then(serde_json::Value::as_u64))
            .filter(|limit| *limit > 0)
            .min()
            .map(|limit| limit.min(u64::from(u32::MAX)) as u32)
    };
    ModelCapabilities {
        max_context_tokens: capacity(&[
            "/context_window",
            "/max_context_tokens",
            "/limit/context",
            "/limits/context_window",
        ]),
        max_input_tokens: capacity(&[
            "/max_input_tokens",
            "/limit/input",
            "/limits/max_input_tokens",
        ]),
        max_output_tokens: capacity(&[
            "/max_output_tokens",
            "/limit/output",
            "/limits/max_output_tokens",
        ]),
    }
}

/// Compatibility defaults belong to the provider catalog, shared by every task.
/// A model name on a custom gateway does not establish that gateway's capacity.
pub fn output_limits(
    connection: &ProviderConnection,
    model: &crate::db::provider::ProviderModel,
    requested: Option<u32>,
) -> (u32, u32) {
    let reasoning_default = connection.template_kind == "deepseek"
        && (model.model_id.starts_with("deepseek-v4-")
            || model.model_id == "deepseek-flash"
            || model.model_id.starts_with("deepseek-flash-"));
    let requested = requested.unwrap_or(8000).max(1);
    let initial = if reasoning_default {
        requested.max(65536)
    } else {
        requested
    };
    let safety_cap = if reasoning_default {
        131072
    } else {
        requested.max(32768)
    };
    let cap = model_capabilities(model)
        .max_output_tokens
        .unwrap_or(safety_cap)
        .min(safety_cap);
    (initial.min(cap), cap)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ModelSeed {
    pub model_id: &'static str,
    pub protocol: &'static str,
    pub endpoint_path: &'static str,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProviderTemplate {
    pub kind: &'static str,
    pub vendor: &'static str,
    pub display_name: &'static str,
    pub service_tier: &'static str,
    pub base_url: &'static str,
    pub models_endpoint: Option<&'static str>,
    pub auth_mode: &'static str,
    pub protocol: Option<&'static str>,
    pub endpoint_path: Option<&'static str>,
    pub discovery: &'static str,
    pub docs_url: &'static str,
    pub note: &'static str,
    pub configurable: bool,
    pub models: Vec<ModelSeed>,
}

const DEEPSEEK_MODELS: &[ModelSeed] = &[
    ModelSeed {
        model_id: "deepseek-v4-pro",
        protocol: "chat_completions",
        endpoint_path: "/chat/completions",
    },
    ModelSeed {
        model_id: "deepseek-v4-flash",
        protocol: "chat_completions",
        endpoint_path: "/chat/completions",
    },
];

const OPENCODE_GO_MODELS: &[ModelSeed] = &[
    ModelSeed {
        model_id: "grok-4.6",
        protocol: "responses",
        endpoint_path: "/responses",
    },
    ModelSeed {
        model_id: "gpt-5.6-luna",
        protocol: "responses",
        endpoint_path: "/responses",
    },
    ModelSeed {
        model_id: "glm-5.3-flash",
        protocol: "chat_completions",
        endpoint_path: "/chat/completions",
    },
    ModelSeed {
        model_id: "glm-5.3",
        protocol: "chat_completions",
        endpoint_path: "/chat/completions",
    },
    ModelSeed {
        model_id: "glm-5.2",
        protocol: "chat_completions",
        endpoint_path: "/chat/completions",
    },
    ModelSeed {
        model_id: "glm-5.1",
        protocol: "chat_completions",
        endpoint_path: "/chat/completions",
    },
    ModelSeed {
        model_id: "kimi-k3",
        protocol: "chat_completions",
        endpoint_path: "/chat/completions",
    },
    ModelSeed {
        model_id: "kimi-k2.7-code",
        protocol: "chat_completions",
        endpoint_path: "/chat/completions",
    },
    ModelSeed {
        model_id: "kimi-k2.6",
        protocol: "chat_completions",
        endpoint_path: "/chat/completions",
    },
    ModelSeed {
        model_id: "longcat-2.0",
        protocol: "chat_completions",
        endpoint_path: "/chat/completions",
    },
    ModelSeed {
        model_id: "deepseek-v4-pro",
        protocol: "chat_completions",
        endpoint_path: "/chat/completions",
    },
    ModelSeed {
        model_id: "deepseek-v4-flash",
        protocol: "chat_completions",
        endpoint_path: "/chat/completions",
    },
    ModelSeed {
        model_id: "deepseek-v4-flash-vision-exp",
        protocol: "chat_completions",
        endpoint_path: "/chat/completions",
    },
    ModelSeed {
        model_id: "mimo-v2.5",
        protocol: "chat_completions",
        endpoint_path: "/chat/completions",
    },
    ModelSeed {
        model_id: "mimo-v2.5-pro",
        protocol: "chat_completions",
        endpoint_path: "/chat/completions",
    },
    ModelSeed {
        model_id: "minimax-m3",
        protocol: "anthropic_messages",
        endpoint_path: "/messages",
    },
    ModelSeed {
        model_id: "minimax-m2.7",
        protocol: "anthropic_messages",
        endpoint_path: "/messages",
    },
    ModelSeed {
        model_id: "minimax-m2.5",
        protocol: "anthropic_messages",
        endpoint_path: "/messages",
    },
    ModelSeed {
        model_id: "muse-spark-1.3-contributor",
        protocol: "responses",
        endpoint_path: "/responses",
    },
    ModelSeed {
        model_id: "muse-spark-1.2-contributor",
        protocol: "responses",
        endpoint_path: "/responses",
    },
    ModelSeed {
        model_id: "qwen3.8-max",
        protocol: "anthropic_messages",
        endpoint_path: "/messages",
    },
    ModelSeed {
        model_id: "qwen3.8-flash",
        protocol: "anthropic_messages",
        endpoint_path: "/messages",
    },
    ModelSeed {
        model_id: "qwen3.7-max",
        protocol: "anthropic_messages",
        endpoint_path: "/messages",
    },
    ModelSeed {
        model_id: "qwen3.7-plus",
        protocol: "anthropic_messages",
        endpoint_path: "/messages",
    },
    ModelSeed {
        model_id: "qwen3.6-plus",
        protocol: "anthropic_messages",
        endpoint_path: "/messages",
    },
    ModelSeed {
        model_id: "hy4-preview",
        protocol: "chat_completions",
        endpoint_path: "/chat/completions",
    },
    ModelSeed {
        model_id: "hy3",
        protocol: "chat_completions",
        endpoint_path: "/chat/completions",
    },
    ModelSeed {
        model_id: "omen-alpha",
        protocol: "chat_completions",
        endpoint_path: "/chat/completions",
    },
];

pub fn template(kind: &str) -> Option<ProviderTemplate> {
    match kind {
        "deepseek" => Some(ProviderTemplate {
            kind: "deepseek",
            vendor: "DeepSeek",
            display_name: "DeepSeek",
            service_tier: "api",
            base_url: "https://api.deepseek.com",
            models_endpoint: Some("https://api.deepseek.com/models"),
            auth_mode: "bearer",
            protocol: Some("chat_completions"),
            endpoint_path: Some("/chat/completions"),
            discovery: "remote",
            docs_url: "https://api-docs.deepseek.com/api/list-models",
            note: "可从账户模型目录刷新；新模型沿用兼容 Chat Completions 协议。",
            configurable: true,
            models: DEEPSEEK_MODELS.to_vec(),
        }),
        "opencode_go" => Some(ProviderTemplate {
            kind: "opencode_go",
            vendor: "OpenCode",
            display_name: "OpenCode Go",
            service_tier: "plan",
            base_url: "https://opencode.ai/zen/go/v1",
            models_endpoint: Some("https://opencode.ai/zen/go/v1/models"),
            auth_mode: "bearer",
            protocol: None,
            endpoint_path: None,
            discovery: "remote",
            docs_url: "https://opencode.ai/docs/go/",
            note: "面向编程代理的套餐；按官方模型逐项匹配协议，未知协议需要手动确认。",
            configurable: true,
            models: OPENCODE_GO_MODELS
                .iter()
                .chain(provider_presets::GO_ADDITIONS.iter())
                .cloned()
                .collect(),
        }),
        _ => provider_presets::template(kind),
    }
}

pub fn list_templates() -> Vec<ProviderTemplate> {
    std::iter::once(template("deepseek").unwrap())
        .chain(std::iter::once(template("opencode_go").unwrap()))
        .chain(provider_presets::templates())
        .collect()
}

pub fn seed_template(
    repo: &ProviderCatalogRepo<'_>,
    provider: &ProviderConnection,
    kind: &str,
) -> crate::db::DbResult<usize> {
    let template = template(kind)
        .ok_or_else(|| crate::db::DbError::Migration("unknown fixed template".into()))?;
    let existing = repo.list_models(Some(provider.id))?;
    for model in &template.models {
        if existing.iter().any(|old| old.model_id == model.model_id) {
            continue;
        }
        let supported = crate::db::provider::validate_protocol(model.protocol).is_ok();
        let capabilities = if supported {
            serde_json::json!({"catalog_source":"curated","docs_url":template.docs_url})
        } else {
            serde_json::json!({"catalog_source":"curated","docs_url":template.docs_url,"needs_protocol":true,"unsupported_protocol":model.protocol})
        };
        repo.upsert_model(
            provider.id,
            model.model_id,
            model.model_id,
            if supported {
                model.protocol
            } else {
                "responses"
            },
            if supported {
                model.endpoint_path
            } else {
                "/responses"
            },
            &capabilities.to_string(),
            "template",
            supported,
            supported,
        )?;
    }
    Ok(template.models.len())
}

pub fn known_model(kind: &str, model_id: &str) -> Option<ModelSeed> {
    template(kind)?
        .models
        .into_iter()
        .find(|model| model.model_id == model_id)
}

/// Reconcile fixed-template rows with the locked protocol table while preserving
/// the user's enabled/available state. This repairs catalogs created by older builds.
pub fn repair_fixed_templates(repo: &ProviderCatalogRepo<'_>) -> crate::db::DbResult<usize> {
    let connections = repo.list_connections()?;
    let mut repaired_connections = 0;
    for connection in connections {
        let Some(template) = template(&connection.template_kind) else {
            continue;
        };
        let existing = repo.list_models(Some(connection.id))?;
        for seed in template.models {
            if crate::db::provider::validate_protocol(seed.protocol).is_err() {
                continue;
            }
            if let Some(model) = existing
                .iter()
                .find(|model| model.model_id == seed.model_id)
            {
                let capabilities: serde_json::Value =
                    serde_json::from_str(&model.capabilities_json).unwrap_or_default();
                if matches!(model.source.as_str(), "manual" | "legacy")
                    || capabilities
                        .get("protocol_from_remote")
                        .and_then(serde_json::Value::as_bool)
                        == Some(true)
                {
                    continue;
                }
                repo.upsert_model(
                    connection.id,
                    seed.model_id,
                    &model.display_name,
                    seed.protocol,
                    seed.endpoint_path,
                    &model.capabilities_json,
                    &model.source,
                    model.enabled,
                    model.available,
                )?;
            }
        }
        repaired_connections += 1;
    }
    Ok(repaired_connections)
}

#[derive(Debug, Clone)]
pub struct RemoteModel {
    pub model_id: String,
    pub protocol: Option<String>,
    pub endpoint_path: Option<String>,
    pub model_type: Option<String>,
}

pub fn validate_endpoint_path(path: &str) -> Result<(), String> {
    if !path.starts_with('/')
        || path.starts_with("//")
        || path.contains("..")
        || path.contains(['?', '#', '\\'])
        || path.chars().any(char::is_control)
    {
        return Err("模型 endpoint 必须是以单个 / 开头的路径，不得包含查询、片段或路径跳转".into());
    }
    Ok(())
}

fn checked_catalog_url(base: &reqwest::Url, endpoint: &str) -> Result<reqwest::Url, String> {
    let url = base
        .join(endpoint)
        .map_err(|_| "模型目录 URL 无效".to_string())?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.origin() != base.origin()
    {
        return Err("模型目录与分页 URL 必须和 Base URL 同源，且不得包含凭据或片段".into());
    }
    if std::env::var("MSL_ISOLATED_TEST").as_deref() == Ok("1")
        && !url.host_str().is_some_and(|host| {
            host.eq_ignore_ascii_case("localhost")
                || host
                    .trim_matches(['[', ']'])
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|address| address.is_loopback())
        })
    {
        return Err("隔离测试只允许本地模拟模型目录".into());
    }
    Ok(url)
}

pub fn discovery_endpoint(connection: &ProviderConnection) -> Option<String> {
    if let Some(endpoint) = &connection.models_endpoint {
        return Some(endpoint.clone());
    }
    if connection.template_kind == "aliyun" {
        let base = reqwest::Url::parse(&connection.base_url).ok()?;
        let host = base.host_str()?;
        if base.scheme() == "https" && host.ends_with(".cn-beijing.maas.aliyuncs.com") {
            return base
                .join("/api/v1/models?capabilities=TG&page_no=1&page_size=100")
                .ok()
                .map(|url| url.to_string());
        }
    }
    None
}

pub async fn fetch_models(
    connection: &ProviderConnection,
    api_key: &str,
) -> Result<Vec<RemoteModel>, String> {
    let base =
        reqwest::Url::parse(&connection.base_url).map_err(|_| "Base URL 无效".to_string())?;
    let endpoint =
        discovery_endpoint(connection).ok_or_else(|| "未配置经过确认的模型目录地址".to_string())?;
    let mut url = checked_catalog_url(&base, &endpoint)?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|_| "无法创建模型目录客户端".to_string())?;
    let mut models = Vec::new();
    let mut seen_pages = std::collections::HashSet::new();
    let mut seen_models = std::collections::HashSet::new();
    let mut expected_total = None;
    loop {
        if seen_pages.len() >= 50 || !seen_pages.insert(url.to_string()) {
            return Err("模型目录分页循环或超过安全上限，未修改已有目录".into());
        }
        let request = client.get(url.clone());
        let request = match connection.auth_mode.as_str() {
            "bearer" => request.bearer_auth(api_key),
            "api_key" => request.header("x-api-key", api_key),
            "none" => request,
            _ => return Err("不支持的模型目录认证方式".into()),
        };
        let response = request
            .send()
            .await
            .map_err(|_| "模型目录请求失败，请检查网络和地址".to_string())?;
        if !response.status().is_success() {
            return Err(format!(
                "模型目录 HTTP {}，未修改已有目录",
                response.status().as_u16()
            ));
        }
        if response
            .content_length()
            .is_some_and(|length| length > 4 * 1024 * 1024)
        {
            return Err("模型目录响应过大，未修改已有目录".into());
        }
        let body = response
            .bytes()
            .await
            .map_err(|_| "模型目录读取失败".to_string())?;
        if body.len() > 4 * 1024 * 1024 {
            return Err("模型目录响应过大".into());
        }
        let value: serde_json::Value =
            serde_json::from_slice(&body).map_err(|_| "模型目录响应格式无效".to_string())?;
        let aliyun = connection.template_kind == "aliyun" && value.get("output").is_some();
        let data = if aliyun {
            value.pointer("/output/models")
        } else {
            value.get("data")
        }
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "模型目录缺少 data 数组".to_string())?;
        for item in data {
            let id = item
                .get(if aliyun { "model" } else { "id" })
                .and_then(serde_json::Value::as_str)
                .filter(|id| {
                    !id.trim().is_empty() && id.trim() == *id && !id.chars().any(char::is_control)
                })
                .ok_or_else(|| "模型目录包含无效模型 ID，未修改已有目录".to_string())?;
            if !seen_models.insert(id.to_string()) {
                return Err("模型目录包含重复模型 ID".into());
            }
            let read_optional = |field: &str| -> Result<Option<String>, String> {
                match item.get(field) {
                    None | Some(serde_json::Value::Null) => Ok(None),
                    Some(serde_json::Value::String(value)) if !value.trim().is_empty() => {
                        Ok(Some(value.clone()))
                    }
                    _ => Err(format!("模型目录的 {field} 字段格式无效")),
                }
            };
            let protocol = read_optional("protocol")?;
            let endpoint_path = read_optional("endpoint_path")?;
            let model_type = read_optional("type")?;
            if let Some(path) = endpoint_path.as_deref() {
                validate_endpoint_path(path)?;
            }
            models.push(RemoteModel {
                model_id: id.into(),
                protocol,
                endpoint_path,
                model_type,
            });
        }
        if aliyun {
            let read_count = |field: &str| {
                value
                    .pointer(&format!("/output/{field}"))
                    .and_then(serde_json::Value::as_u64)
                    .ok_or_else(|| format!("阿里云模型目录缺少有效 {field}"))
            };
            let total = read_count("total")?;
            let page = read_count("page_no")?;
            let size = read_count("page_size")?;
            let requested_page = url
                .query_pairs()
                .find(|(key, _)| key == "page_no")
                .and_then(|(_, value)| value.parse::<u64>().ok())
                .unwrap_or(1);
            if size == 0
                || page != requested_page
                || expected_total.is_some_and(|previous| previous != total)
            {
                return Err("阿里云模型目录分页状态不一致".into());
            }
            expected_total = Some(total);
            if models.len() as u64 >= total {
                break;
            }
            if data.len() as u64 != size || page.checked_mul(size) != Some(models.len() as u64) {
                return Err("阿里云模型目录分页不完整，未修改已有目录".into());
            }
            let mut query: Vec<(String, String)> = url
                .query_pairs()
                .filter(|(key, _)| key != "page_no")
                .map(|(key, value)| (key.into_owned(), value.into_owned()))
                .collect();
            query.push(("page_no".into(), (page + 1).to_string()));
            url.query_pairs_mut().clear().extend_pairs(query);
            continue;
        }
        if let Some(total) = value
            .get("total")
            .or_else(|| value.pointer("/pagination/total"))
        {
            let total = total
                .as_u64()
                .ok_or_else(|| "模型目录 total 无效".to_string())?;
            if expected_total.is_some_and(|previous| previous != total) {
                return Err("分页模型总数发生变化".into());
            }
            expected_total = Some(total);
        }
        let has_more = value
            .get("has_more")
            .or_else(|| value.pointer("/pagination/has_more"));
        if has_more.is_some_and(|v| !v.is_boolean()) {
            return Err("模型目录分页标记无效".into());
        }
        let next = value
            .pointer("/links/next")
            .or_else(|| value.get("next_url"))
            .or_else(|| value.get("next_page_url"))
            .or_else(|| value.get("next"));
        match next {
            Some(serde_json::Value::String(next)) if !next.is_empty() => {
                url = checked_catalog_url(
                    &base,
                    url.join(next)
                        .map_err(|_| "分页 URL 无效".to_string())?
                        .as_str(),
                )?;
            }
            None | Some(serde_json::Value::Null) | Some(serde_json::Value::String(_)) => {
                if has_more.and_then(serde_json::Value::as_bool) == Some(true)
                    || value
                        .get("next_cursor")
                        .is_some_and(|v| !v.is_null() && v.as_str() != Some(""))
                    || value
                        .get("next_page")
                        .is_some_and(|v| !v.is_null() && v.as_u64() != Some(0))
                {
                    return Err(
                        "模型目录还有后续分页，但未提供可验证的下一页 URL，未修改已有目录".into(),
                    );
                }
                break;
            }
            _ => return Err("模型目录下一页格式无效".into()),
        }
    }
    if expected_total.is_some_and(|total| total != models.len() as u64) {
        return Err("模型目录条数与 total 不一致，未修改已有目录".into());
    }
    Ok(models)
}

#[cfg(test)]
pub async fn fetch_model_ids(
    connection: &ProviderConnection,
    api_key: &str,
) -> Result<Vec<String>, String> {
    Ok(fetch_models(connection, api_key)
        .await?
        .into_iter()
        .map(|model| model.model_id)
        .collect())
}

pub fn apply_remote_ids(
    repo: &ProviderCatalogRepo<'_>,
    connection: &ProviderConnection,
    ids: &[String],
) -> crate::db::DbResult<(usize, usize)> {
    let models: Vec<_> = ids
        .iter()
        .map(|id| RemoteModel {
            model_id: id.clone(),
            protocol: None,
            endpoint_path: None,
            model_type: None,
        })
        .collect();
    apply_remote_models(repo, connection, &models)
}

pub fn apply_remote_models(
    repo: &ProviderCatalogRepo<'_>,
    connection: &ProviderConnection,
    models: &[RemoteModel],
) -> crate::db::DbResult<(usize, usize)> {
    repo.transaction(|repo| {
    let existing = repo.list_models(Some(connection.id))?;
    let mut available = 0;
    let mut unknown = 0;
    for remote in models {
        let id = &remote.model_id;
        let old = existing.iter().find(|model| model.model_id == *id);
        let template = template(&connection.template_kind);
        let uniform = template.as_ref().and_then(|template| template.protocol.zip(template.endpoint_path));
        let declared = remote.protocol.as_deref().and_then(|protocol| match protocol {
            "chat_completions" | "openai" => Some(("chat_completions", "/chat/completions")),
            "responses" => Some(("responses", "/responses")),
            "anthropic_messages" | "anthropic" => Some(("anthropic_messages", "/messages")),
            _ => None,
        }).map(|(protocol, default_path)| (protocol, remote.endpoint_path.as_deref().unwrap_or(default_path)))
          .or_else(|| (connection.template_kind == "baidu" && remote.model_type.as_deref() == Some("chat")).then_some(("chat_completions", "/chat/completions")));
        let known = known_model(&connection.template_kind, id);
        let unsupported_protocol = remote.model_type.as_deref().filter(|kind| connection.template_kind == "baidu" && *kind != "chat")
            .or_else(|| remote.protocol.as_deref().filter(|_| declared.is_none()))
            .or_else(|| known.as_ref().filter(|seed| crate::db::provider::validate_protocol(seed.protocol).is_err()).map(|seed| seed.protocol));
        let unsupported = unsupported_protocol.is_some();
        if let Some((protocol, endpoint)) = old.filter(|model| matches!(model.source.as_str(), "manual" | "legacy"))
            .map(|model| (model.protocol.as_str(), model.endpoint_path.as_str()))
            .or(declared)
            .or_else(|| known.as_ref().map(|seed| (seed.protocol, seed.endpoint_path)))
            .or(uniform).filter(|(protocol, _)| (!unsupported || old.is_some_and(|model| matches!(model.source.as_str(), "manual" | "legacy"))) && crate::db::provider::validate_protocol(protocol).is_ok()) {
            let mut capabilities: serde_json::Value = serde_json::from_str(old.map_or("{}", |model| model.capabilities_json.as_str())).unwrap_or_else(|_| serde_json::json!({}));
            if let Some(map) = capabilities.as_object_mut() {
                map.remove("needs_protocol");
                map.remove("unsupported_protocol");
                if declared.is_some() { map.insert("protocol_from_remote".into(), serde_json::Value::Bool(true)); }
                if let Some(model_type) = &remote.model_type { map.insert("model_type".into(), serde_json::json!(model_type)); }
            }
            repo.upsert_model(
                connection.id,
                id,
                old.map_or(id.as_str(), |model| model.display_name.as_str()),
                protocol,
                endpoint,
                &capabilities.to_string(),
                old.filter(|model| matches!(model.source.as_str(), "manual" | "legacy")).map_or("remote", |model| model.source.as_str()),
                old.map_or(true, |model| model.enabled),
                true,
            )?;
            available += 1;
        } else if let Some(model) = old {
            let mut capabilities =
                serde_json::from_str::<serde_json::Value>(&model.capabilities_json)
                    .ok()
                    .and_then(|value| value.as_object().cloned())
                    .unwrap_or_default();
            capabilities.insert("needs_protocol".into(), serde_json::Value::Bool(true));
            if unsupported { capabilities.insert("unsupported_protocol".into(), serde_json::json!(unsupported_protocol)); }
            if let Some(model_type) = &remote.model_type { capabilities.insert("model_type".into(), serde_json::json!(model_type)); }
            repo.upsert_model(
                connection.id,
                id,
                &model.display_name,
                &model.protocol,
                &model.endpoint_path,
                &serde_json::Value::Object(capabilities).to_string(),
                "remote",
                model.enabled,
                true,
            )?;
            unknown += 1;
        } else {
            repo.upsert_model(
                connection.id,
                id,
                id,
                "responses",
                "/responses",
                &serde_json::json!({"needs_protocol":true,"unsupported_protocol":unsupported_protocol,"model_type":remote.model_type}).to_string(),
                "remote",
                false,
                true,
            )?;
            unknown += 1;
        }
    }
    for model in existing {
        if model.source != "manual" && model.source != "legacy" && !models.iter().any(|remote| remote.model_id == model.model_id) {
            repo.upsert_model(
                connection.id,
                &model.model_id,
                &model.display_name,
                &model.protocol,
                &model.endpoint_path,
                &model.capabilities_json,
                &model.source,
                model.enabled,
                false,
            )?;
        }
    }
    repo.mark_models_refreshed(connection.id, crate::db::now_unix())?;
    Ok((available, unknown))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{provider::ProviderCatalogRepo, Database};

    fn synthetic_connection(repo: &ProviderCatalogRepo<'_>, kind: &str) -> ProviderConnection {
        repo.insert_connection(
            "Synthetic",
            "custom",
            "http://127.0.0.1",
            "",
            kind,
            "bearer",
            None,
            true,
        )
        .unwrap()
    }

    fn catalog_server(body: &str) -> (String, std::thread::JoinHandle<String>) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let body = body.to_string();
        let thread = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut buffer = [0; 8192];
            let size = stream.read(&mut buffer).unwrap();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
            String::from_utf8_lossy(&buffer[..size]).to_string()
        });
        (base, thread)
    }

    #[test]
    fn aliyun_discovery_collects_all_numbered_pages_before_reconciliation() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for (page, id) in [(1, "model-one"), (2, "model-two")] {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                    .unwrap();
                let mut buffer = [0; 8192];
                let size = stream.read(&mut buffer).unwrap();
                requests.push(String::from_utf8_lossy(&buffer[..size]).to_string());
                let body = serde_json::json!({"output":{"models":[{"model":id}],"total":2,"page_no":page,"page_size":1}}).to_string();
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            }
            requests
        });
        let db = Database::open_in_memory().unwrap();
        let repo = ProviderCatalogRepo::new(db.conn());
        let mut provider = synthetic_connection(&repo, "aliyun");
        provider.base_url = base.clone();
        provider.models_endpoint = Some(format!(
            "{base}/api/v1/models?capabilities=TG&page_no=1&page_size=1"
        ));
        let result = tauri::async_runtime::block_on(fetch_model_ids(&provider, "synthetic-key"));
        assert_eq!(result.unwrap(), ["model-one", "model-two"]);
        let requests = server.join().unwrap();
        assert!(requests[1].contains("page_no=2"));
        assert!(requests[1].contains("capabilities=TG"));
    }

    #[test]
    fn malformed_catalog_entry_rejects_the_entire_response() {
        let db = Database::open_in_memory().unwrap();
        let repo = ProviderCatalogRepo::new(db.conn());
        let mut provider = synthetic_connection(&repo, "deepseek");
        let (base, server) = catalog_server(r#"{"data":[{"id":"good"},{"id":42}]}"#);
        provider.base_url = base.clone();
        provider.models_endpoint = Some(format!("{base}/models"));
        let result = tauri::async_runtime::block_on(fetch_model_ids(&provider, "synthetic-key"));
        server.join().unwrap();
        assert!(
            result.is_err(),
            "a malformed row must not silently shrink the remote catalog"
        );
    }

    #[test]
    fn incomplete_catalog_without_continuation_is_rejected() {
        let db = Database::open_in_memory().unwrap();
        let repo = ProviderCatalogRepo::new(db.conn());
        let mut provider = synthetic_connection(&repo, "deepseek");
        let (base, server) = catalog_server(r#"{"data":[{"id":"first"}],"has_more":true}"#);
        provider.base_url = base.clone();
        provider.models_endpoint = Some(format!("{base}/models"));
        let result = tauri::async_runtime::block_on(fetch_model_ids(&provider, "synthetic-key"));
        server.join().unwrap();
        assert!(
            result.is_err(),
            "an incomplete page must not mark remaining models unavailable"
        );
    }

    #[test]
    fn discovery_sends_the_configured_api_key_header() {
        let db = Database::open_in_memory().unwrap();
        let repo = ProviderCatalogRepo::new(db.conn());
        let mut provider = synthetic_connection(&repo, "deepseek");
        let (base, server) = catalog_server(r#"{"data":[{"id":"first"}]}"#);
        provider.base_url = base.clone();
        provider.models_endpoint = Some(format!("{base}/models"));
        provider.auth_mode = "api_key".into();
        tauri::async_runtime::block_on(fetch_model_ids(&provider, "synthetic-key")).unwrap();
        let request = server.join().unwrap().to_lowercase();
        assert!(request.contains("x-api-key: synthetic-key\r\n"));
        assert!(!request.contains("authorization:"));
    }

    #[test]
    fn every_configurable_template_creates_independent_connections() {
        let db = Database::open_in_memory().unwrap();
        let repo = ProviderCatalogRepo::new(db.conn());
        let kinds: Vec<_> = list_templates()
            .iter()
            .map(|template| template.kind)
            .collect();
        for required in [
            "opencode_zen",
            "aliyun",
            "volcengine",
            "zhipu",
            "kimi",
            "minimax",
            "tencent",
            "baidu",
            "mimo",
            "siliconflow",
        ] {
            assert!(
                kinds.contains(&required),
                "missing vendor template {required}"
            );
        }
        for template in list_templates()
            .into_iter()
            .filter(|template| template.configurable)
        {
            let first = synthetic_connection(&repo, template.kind);
            let second = synthetic_connection(&repo, template.kind);
            assert_ne!(first.id, second.id);
            assert_ne!(first.credential_ref, second.credential_ref);
            seed_template(&repo, &first, template.kind).unwrap();
            assert!(repo.list_models(Some(second.id)).unwrap().is_empty());
        }
    }

    #[test]
    fn curated_refresh_preserves_existing_model_choices() {
        let db = Database::open_in_memory().unwrap();
        let repo = ProviderCatalogRepo::new(db.conn());
        let provider = synthetic_connection(&repo, "deepseek");
        let model = repo
            .upsert_model(
                provider.id,
                "deepseek-v4-flash",
                "Keep label",
                "chat_completions",
                "/custom/chat",
                r#"{"max_output_tokens":12000}"#,
                "manual",
                false,
                false,
            )
            .unwrap();
        seed_template(&repo, &provider, "deepseek").unwrap();
        let updated = repo.get_model(model.id).unwrap().unwrap();
        assert!(!updated.enabled);
        assert!(!updated.available);
        assert_eq!(updated.endpoint_path, "/custom/chat");
        assert_eq!(updated.display_name, "Keep label");
        assert_eq!(model_capabilities(&updated).max_output_tokens, Some(12000));
    }

    #[test]
    fn unsupported_zen_models_are_listed_with_their_actual_protocol() {
        let db = Database::open_in_memory().unwrap();
        let repo = ProviderCatalogRepo::new(db.conn());
        let provider = synthetic_connection(&repo, "opencode_zen");
        assert_eq!(
            apply_remote_ids(&repo, &provider, &["gemini-3.8-flash".into()]).unwrap(),
            (0, 1)
        );
        let models = repo.list_models(Some(provider.id)).unwrap();
        assert!(!models[0].enabled);
        let capabilities: serde_json::Value =
            serde_json::from_str(&models[0].capabilities_json).unwrap();
        assert_eq!(capabilities["unsupported_protocol"], "gemini");
        assert!(repo.set_model_enabled(models[0].id, true).is_err());
    }

    #[test]
    fn catalog_repair_preserves_remotely_declared_protocol() {
        let db = Database::open_in_memory().unwrap();
        let repo = ProviderCatalogRepo::new(db.conn());
        let provider = synthetic_connection(&repo, "opencode_go");
        apply_remote_models(
            &repo,
            &provider,
            &[RemoteModel {
                model_id: "minimax-m3".into(),
                protocol: Some("chat_completions".into()),
                endpoint_path: Some("/chat/completions".into()),
                model_type: None,
            }],
        )
        .unwrap();
        repair_fixed_templates(&repo).unwrap();
        let models = repo.list_models(Some(provider.id)).unwrap();
        assert_eq!(
            models.len(),
            1,
            "reading connections must not seed additional models"
        );
        assert_eq!(models[0].protocol, "chat_completions");
    }

    #[test]
    fn refresh_preserves_disabled_model_and_route() {
        let db = Database::open_in_memory().unwrap();
        let repo = ProviderCatalogRepo::new(db.conn());
        let provider = synthetic_connection(&repo, "deepseek");
        let model = repo
            .upsert_model(
                provider.id,
                "deepseek-v4-flash",
                "My label",
                "chat_completions",
                "/chat/completions",
                r#"{"max_output_tokens":12000}"#,
                "remote",
                false,
                true,
            )
            .unwrap();
        repo.upsert_route("general", Some(model.id)).unwrap();
        apply_remote_ids(&repo, &provider, &["deepseek-v4-flash".into()]).unwrap();
        let refreshed = repo.get_model(model.id).unwrap().unwrap();
        assert!(
            !refreshed.enabled,
            "refresh must preserve an explicit user disable"
        );
        assert_eq!(refreshed.display_name, "My label");
        assert_eq!(
            repo.get_route("general")
                .unwrap()
                .unwrap()
                .provider_model_id,
            Some(model.id)
        );
    }

    #[test]
    fn legacy_models_retain_user_protocol_source_and_route_after_discovery_and_repair() {
        let db = Database::open_in_memory().unwrap();
        let repo = ProviderCatalogRepo::new(db.conn());
        for kind in ["custom", "opencode_go"] {
            let provider = synthetic_connection(&repo, kind);
            let model = repo
                .upsert_model(
                    provider.id,
                    "minimax-m3",
                    "Legacy user model",
                    "chat_completions",
                    "/custom/chat",
                    r#"{"max_output_tokens":17000}"#,
                    "legacy",
                    true,
                    true,
                )
                .unwrap();
            repo.upsert_route("general", Some(model.id)).unwrap();
            apply_remote_ids(&repo, &provider, &["minimax-m3".into()]).unwrap();
            repair_fixed_templates(&repo).unwrap();
            let result = repo.get_model(model.id).unwrap().unwrap();
            assert_eq!(result.source, "legacy");
            assert_eq!(result.protocol, "chat_completions");
            assert_eq!(result.endpoint_path, "/custom/chat");
            assert!(result.enabled && result.available);
            assert!(!result.capabilities_json.contains("needs_protocol"));
            assert_eq!(
                repo.get_route("general")
                    .unwrap()
                    .unwrap()
                    .provider_model_id,
                Some(model.id)
            );
        }
    }

    #[test]
    fn cross_modal_provider_catalogs_do_not_guess_chat_for_unknown_models() {
        let db = Database::open_in_memory().unwrap();
        let repo = ProviderCatalogRepo::new(db.conn());
        for kind in ["tencent", "baidu", "minimax", "minimax_tokenplan"] {
            let provider = synthetic_connection(&repo, kind);
            assert_eq!(
                apply_remote_ids(&repo, &provider, &["synthetic-unknown-modality".into()]).unwrap(),
                (0, 1),
                "provider {kind}"
            );
            let model = repo.list_models(Some(provider.id)).unwrap().remove(0);
            assert!(!model.enabled);
            assert!(model.capabilities_json.contains("needs_protocol"));
        }
    }

    #[test]
    fn baidu_catalog_routes_only_explicit_chat_types() {
        let db = Database::open_in_memory().unwrap();
        let repo = ProviderCatalogRepo::new(db.conn());
        let mut provider = synthetic_connection(&repo, "baidu");
        let (base, server) = catalog_server(
            r#"{"data":[{"id":"new-chat","type":"chat"},{"id":"image","type":"text2image"},{"id":"embed","type":"embeddings"}]}"#,
        );
        provider.base_url = base.clone();
        provider.models_endpoint = Some(format!("{base}/models"));
        let models =
            tauri::async_runtime::block_on(fetch_models(&provider, "synthetic-key")).unwrap();
        server.join().unwrap();
        assert_eq!(
            apply_remote_models(&repo, &provider, &models).unwrap(),
            (1, 2)
        );
        let stored = repo.list_models(Some(provider.id)).unwrap();
        assert!(
            stored
                .iter()
                .find(|model| model.model_id == "new-chat")
                .unwrap()
                .enabled
        );
        assert!(
            !stored
                .iter()
                .find(|model| model.model_id == "image")
                .unwrap()
                .enabled
        );
        assert!(
            !stored
                .iter()
                .find(|model| model.model_id == "embed")
                .unwrap()
                .enabled
        );
    }

    #[test]
    fn isolated_catalog_discovery_rejects_external_urls_before_any_network_call() {
        assert_eq!(std::env::var("MSL_ISOLATED_TEST").as_deref(), Ok("1"));
        let base = reqwest::Url::parse("https://synthetic.invalid/v1").unwrap();
        let error = checked_catalog_url(&base, "https://synthetic.invalid/v1/models").unwrap_err();
        assert!(error.contains("隔离"));
    }

    #[test]
    fn uniform_provider_accepts_new_model_with_shared_protocol() {
        let db = Database::open_in_memory().unwrap();
        let repo = ProviderCatalogRepo::new(db.conn());
        let provider = synthetic_connection(&repo, "deepseek");
        assert_eq!(
            apply_remote_ids(&repo, &provider, &["future-synthetic-model".into()]).unwrap(),
            (1, 0)
        );
        let models = repo.list_models(Some(provider.id)).unwrap();
        assert!(models[0].enabled);
        assert_eq!(models[0].protocol, "chat_completions");
    }

    #[test]
    fn refresh_preserves_custom_models_absent_from_remote_catalog() {
        let db = Database::open_in_memory().unwrap();
        let repo = ProviderCatalogRepo::new(db.conn());
        let provider = synthetic_connection(&repo, "deepseek");
        let model = repo
            .upsert_model(
                provider.id,
                "private-deployment",
                "Private",
                "responses",
                "/responses",
                "{}",
                "manual",
                true,
                true,
            )
            .unwrap();
        apply_remote_ids(&repo, &provider, &["deepseek-v4-flash".into()]).unwrap();
        assert!(repo.get_model(model.id).unwrap().unwrap().available);
    }

    #[test]
    fn refresh_rolls_back_all_rows_on_database_failure() {
        let db = Database::open_in_memory().unwrap();
        let repo = ProviderCatalogRepo::new(db.conn());
        let provider = synthetic_connection(&repo, "deepseek");
        db.conn().execute_batch("CREATE TRIGGER reject_second_model BEFORE INSERT ON provider_models WHEN NEW.model_id = 'deepseek-v4-pro' BEGIN SELECT RAISE(ABORT, 'synthetic failure'); END;").unwrap();
        assert!(apply_remote_ids(
            &repo,
            &provider,
            &["deepseek-v4-flash".into(), "deepseek-v4-pro".into()]
        )
        .is_err());
        assert!(
            repo.list_models(Some(provider.id)).unwrap().is_empty(),
            "failed refresh must not leave the first row written"
        );
    }

    #[test]
    fn catalog_refresh_preserves_declared_capacity_for_existing_models() {
        let db = Database::open_in_memory().unwrap();
        let repo = ProviderCatalogRepo::new(db.conn());
        let provider = repo
            .insert_connection(
                "Synthetic",
                "custom",
                "http://127.0.0.1",
                "",
                "deepseek",
                "bearer",
                None,
                true,
            )
            .unwrap();
        let capabilities = serde_json::json!({"context_window":64000,"max_output_tokens":12000});
        for id in ["deepseek-v4-flash", "independent-model"] {
            repo.upsert_model(
                provider.id,
                id,
                id,
                "chat_completions",
                "/chat/completions",
                &capabilities.to_string(),
                "manual",
                true,
                true,
            )
            .unwrap();
        }
        apply_remote_ids(
            &repo,
            &provider,
            &["deepseek-v4-flash".into(), "independent-model".into()],
        )
        .unwrap();
        for model in repo.list_models(Some(provider.id)).unwrap() {
            assert_eq!(model_capabilities(&model).max_context_tokens, Some(64000));
            assert_eq!(model_capabilities(&model).max_output_tokens, Some(12000));
        }
    }

    #[test]
    fn fixed_templates_match_locked_snapshot() {
        let deepseek = template("deepseek").unwrap();
        assert_eq!(deepseek.base_url, "https://api.deepseek.com");
        assert_eq!(deepseek.models.len(), 2);
        let go = template("opencode_go").unwrap();
        assert_eq!(
            go.models_endpoint,
            Some("https://opencode.ai/zen/go/v1/models")
        );
        let muse = known_model("opencode_go", "muse-spark-1.2-contributor").unwrap();
        assert_eq!(muse.protocol, "responses");
        assert_eq!(muse.endpoint_path, "/responses");
        let grok = known_model("opencode_go", "grok-4.6").unwrap();
        assert_eq!(grok.protocol, "responses");
        assert_eq!(grok.endpoint_path, "/responses");
        assert!(known_model("opencode_go", "grok-4.5").is_none());
        assert_eq!(
            known_model("opencode_go", "minimax-m3").unwrap().protocol,
            "anthropic_messages"
        );
        assert_eq!(
            known_model("opencode_zen", "minimax-m3").unwrap().protocol,
            "chat_completions"
        );
    }

    #[test]
    fn unknown_remote_models_are_disabled_and_missing_are_unavailable() {
        let db = Database::open_in_memory().unwrap();
        let repo = ProviderCatalogRepo::new(db.conn());
        let provider = repo
            .insert_connection(
                "OpenCode Go",
                "custom",
                "http://mock",
                "",
                "opencode_go",
                "bearer",
                Some("http://mock/models"),
                true,
            )
            .unwrap();
        seed_template(&repo, &provider, "opencode_go").unwrap();
        let ids = vec!["gpt-5.6-luna".into(), "new-model".into()];
        let result = apply_remote_ids(&repo, &provider, &ids).unwrap();
        assert_eq!(result, (1, 1));
        let models = repo.list_models(Some(provider.id)).unwrap();
        assert!(models
            .iter()
            .any(|m| m.model_id == "new-model" && !m.enabled && m.protocol == "responses"));
        assert!(models
            .iter()
            .any(|m| m.model_id == "glm-5.3" && !m.available));
    }

    #[test]
    fn repair_fixed_templates_corrects_persisted_muse_protocol() {
        let db = Database::open_in_memory().unwrap();
        let repo = ProviderCatalogRepo::new(db.conn());
        let provider = repo
            .insert_connection(
                "OpenCode Go",
                "custom",
                "http://mock",
                "",
                "opencode_go",
                "bearer",
                Some("http://mock/models"),
                true,
            )
            .unwrap();
        repo.upsert_model(
            provider.id,
            "muse-spark-1.2-contributor",
            "Muse Spark 1.2 Contributor",
            "chat_completions",
            "/chat/completions",
            r#"{"max_output_tokens":10000,"vision":true}"#,
            "remote",
            true,
            true,
        )
        .unwrap();
        assert_eq!(repair_fixed_templates(&repo).unwrap(), 1);
        let repaired = repo
            .list_models(Some(provider.id))
            .unwrap()
            .into_iter()
            .find(|model| model.model_id == "muse-spark-1.2-contributor")
            .unwrap();
        assert_eq!(repaired.protocol, "responses");
        assert_eq!(repaired.endpoint_path, "/responses");
        assert_eq!(
            repaired.capabilities_json,
            r#"{"max_output_tokens":10000,"vision":true}"#
        );
    }
}
