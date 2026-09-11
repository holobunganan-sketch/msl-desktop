//! 三种文本协议的最小适配器；业务层只使用统一的 `AiTextRequest/Response`。

use std::time::Duration;

use super::AiAttachment;
use base64::Engine;
use reqwest::header::{HeaderValue, USER_AGENT};
use serde_json::Value;

use super::{AiError, AiMessage, AiTextRequest, AiTextResponse};
use crate::db::provider::{ProviderConnection, ProviderModel};

// Completion includes reasoning + final output. The old 60s probe-sized limit
// repeatedly discarded valid long analyses before they could return a body.
const GENERATION_TIMEOUT: Duration = Duration::from_secs(180);
const PROBE_TIMEOUT: Duration = Duration::from_secs(20);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(12);
const MAX_ATTEMPTS: usize = 2;

fn safe_message(input: &str) -> String {
    let mut value = input
        .replace('\n', " ")
        .replace('\r', " ")
        .replace("Authorization", "[redacted-header]")
        .replace("authorization", "[redacted-header]")
        .replace("x-api-key", "[redacted-header]")
        .replace("Bearer ", "Bearer [redacted]");
    if let Some(index) = value.find("api_key") {
        value.replace_range(index.., "[redacted]");
    }
    value.chars().take(200).collect()
}

fn endpoint_url(
    connection: &ProviderConnection,
    model: &ProviderModel,
) -> Result<reqwest::Url, AiError> {
    if !model.endpoint_path.starts_with('/') || model.endpoint_path.contains("..") {
        return Err(AiError::Config("模型 endpoint 路径非法".into()));
    }
    let mut base = reqwest::Url::parse(connection.base_url.trim_end_matches('/'))
        .map_err(|_| AiError::Config("Provider Base URL 无效".into()))?;
    if !matches!(base.scheme(), "http" | "https")
        || base.username() != ""
        || base.password().is_some()
    {
        return Err(AiError::Config(
            "Provider Base URL 必须是无凭据的 http/https URL".into(),
        ));
    }
    base.set_path(&format!(
        "{}{}",
        base.path().trim_end_matches('/'),
        model.endpoint_path
    ));
    Ok(base)
}

fn endpoint_is_loopback(url: &reqwest::Url) -> bool {
    url.host_str().is_some_and(|host| {
        host.eq_ignore_ascii_case("localhost")
            || host
                .parse::<std::net::IpAddr>()
                .is_ok_and(|address| address.is_loopback())
    })
}

fn http_client(url: &reqwest::Url, timeout: Duration) -> Result<reqwest::Client, AiError> {
    let mut builder = reqwest::Client::builder()
        .timeout(timeout)
        .connect_timeout(CONNECT_TIMEOUT);
    if endpoint_is_loopback(url) {
        builder = builder.no_proxy();
    }
    builder
        .build()
        .map_err(|_| AiError::Config("无法初始化网络连接，请检查系统代理配置".into()))
}

#[derive(Debug, PartialEq, Eq)]
enum TransportFailure {
    ConnectTimeout,
    ResponseTimeout,
    Tls,
    Dns,
    Connect,
    Body,
    InvalidRequest,
    Other,
}

fn transport_failure(error: &reqwest::Error) -> TransportFailure {
    use std::error::Error;
    if error.is_timeout() {
        return if error.is_connect() {
            TransportFailure::ConnectTimeout
        } else {
            TransportFailure::ResponseTimeout
        };
    }
    if error.is_builder() {
        return TransportFailure::InvalidRequest;
    }
    // Inspect the causal chain for classification only. Never persist its raw
    // text: proxy credentials, URL query secrets or provider echoes may occur.
    let mut source = error.source();
    while let Some(cause) = source {
        let text = cause.to_string().to_ascii_lowercase();
        if ["certificate", "tls", "ssl"]
            .iter()
            .any(|word| text.contains(word))
        {
            return TransportFailure::Tls;
        }
        if ["dns", "resolve", "no such host"]
            .iter()
            .any(|word| text.contains(word))
        {
            return TransportFailure::Dns;
        }
        source = cause.source();
    }
    if error.is_connect() {
        TransportFailure::Connect
    } else if error.is_body() || error.is_decode() {
        TransportFailure::Body
    } else {
        TransportFailure::Other
    }
}

fn transport_error(error: &reqwest::Error, model: &ProviderModel, attempt: usize) -> AiError {
    let (code, explanation) = match transport_failure(error) {
        TransportFailure::ConnectTimeout => ("NET_CONNECT_TIMEOUT", "建立连接超时，请检查网络和系统代理后重试"),
        TransportFailure::ResponseTimeout => ("NET_TIMEOUT", "等待模型完整结果超时，服务可能繁忙或本次分析较长。可稍后重试或在设置中换用更快的分析模型；为避免重复生成，本次未自动重试"),
        TransportFailure::Tls => ("NET_TLS", "安全连接验证失败，请检查系统时间、证书或代理配置；连接安全验证仍然开启"),
        TransportFailure::Dns => ("NET_DNS", "无法解析服务地址，请检查网络、DNS 和系统代理"),
        TransportFailure::Connect => ("NET_CONNECT", "无法连接模型服务，请检查网络及系统代理是否运行，再重试"),
        TransportFailure::Body => ("NET_RESPONSE", "模型响应传输中断，本次未自动重复生成，请检查网络后重试"),
        TransportFailure::InvalidRequest => ("NET_CONFIG", "无法构造请求，请检查接口配置和 API Key 格式"),
        TransportFailure::Other => ("NET_REQUEST", "模型请求传输失败，本次未自动重复生成，请检查网络或代理后重试"),
    };
    let message = format!(
        "{code} · {}：{explanation}（请求 {attempt} 次）",
        safe_message(&model.model_id)
    );
    if transport_failure(error) == TransportFailure::InvalidRequest {
        AiError::Config(message)
    } else {
        AiError::Http(message)
    }
}

fn http_status_error(
    status: reqwest::StatusCode,
    model: &ProviderModel,
    attempt: usize,
) -> AiError {
    let explanation = match status.as_u16() {
        400 | 422 => "模型不接受本次请求参数，请检查模型与协议设置",
        401 => "API Key 未通过验证，请在设置中更新该接口的 API Key",
        403 => "接口拒绝访问，请检查账户权限、订阅及模型地区限制",
        404 => "接口或模型不存在，请检查模型名称和接口地址",
        408 => "服务端处理超时，请稍后重试",
        413 => "本次分析内容超出服务端限制，请缩小分析范围",
        429 => "服务限流或额度不足，请稍后重试并检查订阅额度",
        500..=599 => "模型服务暂时异常，请稍后重试或切换分析模型",
        _ => "模型服务拒绝了本次请求，请检查接口配置或联系服务提供商",
    };
    AiError::Api(format!(
        "HTTP {} · {}：{explanation}（请求 {attempt} 次）",
        status.as_u16(),
        safe_message(&model.model_id)
    ))
}

fn messages_with_system(request: &AiTextRequest) -> Vec<AiMessage> {
    let mut messages = Vec::with_capacity(request.messages.len() + 1);
    if let Some(system) = request.system.as_deref().filter(|value| !value.is_empty()) {
        messages.push(AiMessage {
            role: "system".into(),
            content: system.into(),
        });
    }
    messages.extend(request.messages.clone());
    messages
}

fn chat_completions_body(request: &AiTextRequest) -> Value {
    let mut body = serde_json::json!({
        "model": request.model_id,
        "messages": messages_with_system(request),
    });
    if let Some(value) = request.temperature {
        body["temperature"] = serde_json::json!(value);
    }
    if let Some(value) = request.max_output_tokens {
        body["max_tokens"] = serde_json::json!(value);
    }
    body
}

fn responses_body(request: &AiTextRequest) -> Value {
    let mut body = serde_json::json!({
        "model": request.model_id,
        "input": request.messages,
    });
    if let Some(system) = request.system.as_deref().filter(|value| !value.is_empty()) {
        body["instructions"] = serde_json::json!(system);
    }
    if let Some(value) = request.max_output_tokens {
        body["max_output_tokens"] = serde_json::json!(value);
    }
    body
}

fn anthropic_messages_body(request: &AiTextRequest) -> Value {
    let messages: Vec<AiMessage> = request
        .messages
        .iter()
        .filter(|message| message.role != "system")
        .cloned()
        .collect();
    let mut body = serde_json::json!({ "model": request.model_id, "messages": messages, "max_tokens": request.max_output_tokens.unwrap_or(800) });
    if let Some(system) = request.system.as_deref().filter(|value| !value.is_empty()) {
        body["system"] = serde_json::json!(system);
    }
    if let Some(value) = request.temperature {
        body["temperature"] = serde_json::json!(value);
    }
    body
}

pub fn attachment_supported(model: &ProviderModel, mime: &str, size: u64) -> Result<(), AiError> {
    let caps: Value = serde_json::from_str(&model.capabilities_json).unwrap_or_default();
    let limit = caps["max_file_bytes"]
        .as_u64()
        .unwrap_or(20 * 1024 * 1024)
        .min(24 * 1024 * 1024);
    if size > limit {
        return Err(AiError::Config(
            "文件已保存，但超过本接口单次读取限制；请分段导入或提供较小版本".into(),
        ));
    }
    let image = matches!(
        mime,
        "image/png" | "image/jpeg" | "image/webp" | "image/gif"
    );
    let audio = matches!(mime, "audio/wav" | "audio/mpeg");
    let modality = if image {
        "image"
    } else if audio {
        "audio"
    } else {
        "file"
    };
    if let Some(modes) = caps["input_modalities"].as_array() {
        if !modes.iter().any(|m| m == modality) {
            return Err(AiError::Config(
                "所选模型声明不支持此类资料，文件已保存".into(),
            ));
        }
    }
    let supported = match model.protocol.as_str() {
        "chat_completions" => image || audio || mime == "application/pdf",
        "responses" => {
            image
                || mime == "application/pdf"
                || mime.starts_with("text/")
                || matches!(
                    mime,
                    "application/msword"
                        | "application/rtf"
                        | "application/json"
                        | "application/xml"
                        | "application/vnd.oasis.opendocument.text"
                        | "application/vnd.ms-powerpoint"
                        | "application/vnd.ms-excel"
                )
                || mime.starts_with("application/vnd.openxmlformats-officedocument.")
        }
        "anthropic_messages" => image || mime == "application/pdf",
        _ => false,
    };
    if supported {
        Ok(())
    } else {
        Err(AiError::Config(
            "当前接口没有此格式的文件读取通道，资料已保存；可更换专家分析模型后重试".into(),
        ))
    }
}
fn request_body(
    protocol: &str,
    request: &AiTextRequest,
    parts: &[AiAttachment],
) -> Result<Value, AiError> {
    let mut body = match protocol {
        "chat_completions" => chat_completions_body(request),
        "responses" => responses_body(request),
        "anthropic_messages" => anthropic_messages_body(request),
        _ => return Err(AiError::Config("不支持的文件接口协议".into())),
    };
    crate::ai::output::apply_output_format(protocol, &mut body, &request.output_format)
        .map_err(|error| AiError::Config(error.to_string()))?;
    if parts.is_empty() {
        return Ok(body);
    }
    let key = if protocol == "responses" {
        "input"
    } else {
        "messages"
    };
    let message = body[key]
        .as_array_mut()
        .and_then(|messages| messages.iter_mut().rev().find(|m| m["role"] == "user"))
        .ok_or_else(|| AiError::Config("文件读取缺少用户请求".into()))?;
    let text = message["content"].as_str().unwrap_or("").to_string();
    let mut content = vec![
        serde_json::json!({"type":if protocol=="responses"{"input_text"}else{"text"},"text":text}),
    ];
    for part in parts {
        let encoded = base64::engine::general_purpose::STANDARD.encode(&part.data);
        let data_url = format!("data:{};base64,{encoded}", part.media_type);
        let image = part.media_type.starts_with("image/");
        let value = match protocol {
            "responses" if image => serde_json::json!({"type":"input_image","image_url":data_url}),
            "responses" => {
                serde_json::json!({"type":"input_file","filename":part.filename,"file_data":data_url})
            }
            "chat_completions" if image => {
                serde_json::json!({"type":"image_url","image_url":{"url":data_url}})
            }
            "chat_completions" if part.media_type.starts_with("audio/") => {
                serde_json::json!({"type":"input_audio","input_audio":{"data":encoded,"format":if part.media_type=="audio/wav"{"wav"}else{"mp3"}}})
            }
            "chat_completions" => {
                serde_json::json!({"type":"file","file":{"filename":part.filename,"file_data":data_url}})
            }
            _ => {
                serde_json::json!({"type":if image{"image"}else{"document"},"source":{"type":"base64","media_type":part.media_type,"data":encoded}})
            }
        };
        content.push(value);
    }
    message["content"] = Value::Array(content);
    Ok(body)
}

async fn send_json(
    client: &reqwest::Client,
    url: reqwest::Url,
    connection: &ProviderConnection,
    model: &ProviderModel,
    api_key: &str,
    body: Value,
    anthropic: bool,
    budget: &std::sync::Mutex<crate::ai::output::RequestBudget>,
) -> Result<Value, AiError> {
    let opencode_session = format!("msl-desktop-{}", uuid::Uuid::new_v4());
    for attempt in 1..=MAX_ATTEMPTS {
        {
            use crate::ai::output::AttemptKind;
            let mut budget = budget
                .lock()
                .map_err(|_| AiError::Api("请求预算不可用".into()))?;
            let kind = if attempt > 1 {
                AttemptKind::TransportRetry
            } else if budget.used() == 0 {
                AttemptKind::Initial
            } else {
                AttemptKind::FormatRepair
            };
            budget
                .claim(kind)
                .map_err(|e| AiError::Api(e.to_string()))?;
        }
        let mut request = client
            .post(url.clone())
            .bearer_auth(api_key)
            .header(
                USER_AGENT,
                HeaderValue::from_static(concat!("msl-desktop/", env!("CARGO_PKG_VERSION"))),
            )
            .json(&body);
        if connection.template_kind == "opencode_go" {
            request = request.header("x-opencode-session", &opencode_session);
        }
        if anthropic {
            request = request
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01");
        }
        let mut response = match request.send().await {
            Ok(value) => value,
            // Only retry failed connection establishment. A response timeout
            // may already have consumed generation tokens; do not start over.
            Err(error)
                if attempt < MAX_ATTEMPTS
                    && matches!(
                        transport_failure(&error),
                        TransportFailure::Connect | TransportFailure::ConnectTimeout
                    ) =>
            {
                tokio::time::sleep(Duration::from_millis(250)).await;
                continue;
            }
            Err(error) => return Err(transport_error(&error, model, attempt)),
        };
        let status = response.status();
        if !status.is_success() {
            if status.as_u16() == 429 {
                let retry = response
                    .headers()
                    .get("retry-after")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok());
                return Err(AiError::Api(match retry {
                    Some(seconds) => {
                        format!("模型接口限流，请在 {seconds} 秒后重试；本次未自动重复请求")
                    }
                    None => "模型接口限流，请稍后重试；本次未自动重复请求".into(),
                }));
            }
            if attempt < MAX_ATTEMPTS
                && (status.as_u16() == 408 || status.as_u16() == 429 || status.is_server_error())
            {
                tokio::time::sleep(Duration::from_millis(250)).await;
                continue;
            }
            // Error bodies may echo the submitted document or credential.
            return Err(http_status_error(status, model, attempt));
        }
        // Bound decoded bytes as they arrive, including chunked responses without Content-Length.
        const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
        let too_large = || {
            AiError::Api(
                "模型响应超过 4 MiB 安全上限，请缩小本次分析范围后重试；未保存不完整结果".into(),
            )
        };
        if response
            .content_length()
            .is_some_and(|n| n > MAX_RESPONSE_BYTES as u64)
        {
            return Err(too_large());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| transport_error(&error, model, attempt))?
        {
            if chunk.len() > MAX_RESPONSE_BYTES.saturating_sub(bytes.len()) {
                return Err(too_large());
            }
            bytes.extend_from_slice(&chunk);
        }
        return serde_json::from_slice(&bytes).map_err(|error| {
            AiError::Api(format!(
                "Provider={} model={} 响应解析失败: {}",
                connection.display_name,
                model.model_id,
                safe_message(&error.to_string())
            ))
        });
    }
    Err(AiError::Http("网络请求失败".into()))
}

fn text_content(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str().filter(|text| !text.trim().is_empty()) {
        return Some(text.to_string());
    }
    if let Some(object) = value.as_object() {
        return object
            .get("text")
            .and_then(|text| {
                text.as_str().map(str::to_string).or_else(|| {
                    text.get("value")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
            })
            .filter(|text| !text.trim().is_empty());
    }
    value.as_array().and_then(|parts| {
        let combined = parts
            .iter()
            .filter_map(text_content)
            .collect::<Vec<_>>()
            .join("");
        (!combined.trim().is_empty()).then_some(combined)
    })
}

fn response_model(body: &Value, fallback: &str) -> Option<String> {
    body.get("model")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| Some(fallback.into()))
}

fn completion_status(body: &Value, protocol: &str) -> Result<super::Completion, AiError> {
    let reason = match protocol {
        "chat_completions" => body.pointer("/choices/0/finish_reason"),
        "responses" => body.get("status"),
        _ => body.get("stop_reason"),
    }
    .and_then(Value::as_str);
    let refusal = body
        .pointer("/choices/0/message/refusal")
        .is_some_and(|v| !v.is_null())
        || body
            .get("output")
            .and_then(Value::as_array)
            .is_some_and(|a| {
                a.iter().any(|i| {
                    i.get("content")
                        .and_then(Value::as_array)
                        .is_some_and(|c| c.iter().any(|p| p["type"] == "refusal"))
                })
            });
    if refusal || matches!(reason, Some("content_filter" | "refusal")) {
        return Err(AiError::Api(
            "模型未提供此内容，本次已停止；未进行格式修复或重复生成".into(),
        ));
    }
    if matches!(
        reason,
        Some(
            "length"
                | "max_tokens"
                | "incomplete"
                | "failed"
                | "cancelled"
                | "in_progress"
                | "queued"
                | "tool_calls"
                | "tool_use"
                | "pause_turn"
        )
    ) {
        return Err(AiError::Api(
            "模型输出尚未完整结束，未保存为成功结果；可调整输出上限后重试".into(),
        ));
    }
    Ok(
        if matches!(
            reason,
            Some("stop" | "end_turn" | "stop_sequence" | "completed")
        ) {
            super::Completion::Complete
        } else {
            super::Completion::Unknown
        },
    )
}

async fn chat_completions(
    client: &reqwest::Client,
    connection: &ProviderConnection,
    model: &ProviderModel,
    api_key: &str,
    request: &AiTextRequest,
    parts: &[AiAttachment],
) -> Result<AiTextResponse, AiError> {
    let body = send_json(
        client,
        endpoint_url(connection, model)?,
        connection,
        model,
        api_key,
        request_body("chat_completions", request, parts)?,
        false,
        &request.budget,
    )
    .await?;
    let completion = completion_status(&body, "chat_completions")?;
    let content = body
        .pointer("/choices/0/message/content")
        .and_then(text_content)
        .or_else(|| body.pointer("/choices/0/text").and_then(text_content))
        .ok_or_else(|| {
            if body
                .pointer("/choices/0/message/reasoning_content")
                .is_some_and(|value| !value.is_null())
            {
                AiError::Api(
                    "模型只返回了推理过程，没有给出最终正文；请提高输出上限或更换模型".into(),
                )
            } else {
                AiError::Api("模型响应中没有可用正文（content）".into())
            }
        })?;
    Ok(AiTextResponse {
        content,
        completion,
        model: response_model(&body, &model.model_id),
        usage: body.get("usage").cloned(),
        request_id: body.get("id").and_then(Value::as_str).map(str::to_string),
    })
}

async fn responses(
    client: &reqwest::Client,
    connection: &ProviderConnection,
    model: &ProviderModel,
    api_key: &str,
    request: &AiTextRequest,
    parts: &[AiAttachment],
) -> Result<AiTextResponse, AiError> {
    let body = send_json(
        client,
        endpoint_url(connection, model)?,
        connection,
        model,
        api_key,
        request_body("responses", request, parts)?,
        false,
        &request.budget,
    )
    .await?;
    let completion = completion_status(&body, "responses")?;
    let content = body
        .get("output_text")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| {
            body.get("output")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.get("content").and_then(Value::as_array))
                        .flat_map(|parts| parts.iter())
                        .filter_map(|part| part.get("text").and_then(Value::as_str))
                        .collect::<Vec<_>>()
                        .join("")
                })
                .filter(|value| !value.is_empty())
        })
        .ok_or_else(|| {
            AiError::Api("Responses 响应缺少 output_text 或 output[].content[].text".into())
        })?;
    Ok(AiTextResponse {
        content,
        completion,
        model: response_model(&body, &model.model_id),
        usage: body.get("usage").cloned(),
        request_id: body.get("id").and_then(Value::as_str).map(str::to_string),
    })
}

async fn anthropic_messages(
    client: &reqwest::Client,
    connection: &ProviderConnection,
    model: &ProviderModel,
    api_key: &str,
    request: &AiTextRequest,
    parts: &[AiAttachment],
) -> Result<AiTextResponse, AiError> {
    let body = send_json(
        client,
        endpoint_url(connection, model)?,
        connection,
        model,
        api_key,
        request_body("anthropic_messages", request, parts)?,
        true,
        &request.budget,
    )
    .await?;
    let completion = completion_status(&body, "anthropic_messages")?;
    let content = body
        .get("content")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter(|item| item.get("type").and_then(Value::as_str) == Some("text"))
                .filter_map(|item| item.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("")
        })
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AiError::Api("Anthropic 响应缺少 content[].text".into()))?;
    Ok(AiTextResponse {
        content,
        completion,
        model: response_model(&body, &model.model_id),
        usage: body.get("usage").cloned(),
        request_id: body.get("id").and_then(Value::as_str).map(str::to_string),
    })
}

pub async fn complete(
    connection: &ProviderConnection,
    model: &ProviderModel,
    api_key: &str,
    request: &AiTextRequest,
) -> Result<AiTextResponse, AiError> {
    complete_multimodal(connection, model, api_key, request, &[]).await
}

pub async fn complete_multimodal(
    connection: &ProviderConnection,
    model: &ProviderModel,
    api_key: &str,
    request: &AiTextRequest,
    parts: &[AiAttachment],
) -> Result<AiTextResponse, AiError> {
    let url = endpoint_url(connection, model)?;
    if std::env::var("MSL_ISOLATED_TEST").as_deref() == Ok("1") && !endpoint_is_loopback(&url) {
        return Err(AiError::Config("隔离测试只允许本地模拟接口".into()));
    }
    let mut size = 0u64;
    for part in parts {
        size += part.data.len() as u64;
        attachment_supported(model, &part.media_type, part.data.len() as u64)?;
    }
    if size > 24 * 1024 * 1024 {
        return Err(AiError::Config(
            "本次文件批量超过读取限制，请分批读取".into(),
        ));
    }
    let client = http_client(&endpoint_url(connection, model)?, GENERATION_TIMEOUT)?;
    // The budget includes the automatic connection/status retry. Two validated
    // generations (including a JSON repair) remain below the 10-minute stale-run
    // threshold; a slow retry cannot reset the entire generation's deadline.
    let completion = async {
        match model.protocol.as_str() {
            "chat_completions" => {
                chat_completions(&client, connection, model, api_key, request, parts).await
            }
            "responses" => responses(&client, connection, model, api_key, request, parts).await,
            "anthropic_messages" => {
                anthropic_messages(&client, connection, model, api_key, request, parts).await
            }
            other => Err(AiError::Config(format!("未知模型协议: {other}"))),
        }
    };
    tokio::time::timeout(GENERATION_TIMEOUT, completion).await.unwrap_or_else(|_| {
        Err(AiError::Http(format!("NET_TIMEOUT · {}：本次生成等待已超过 180 秒，任务已停止等待。可稍后重试或切换分析模型；未自动重新生成", safe_message(&model.model_id))))
    })
}

/// Validate authentication, endpoint and wire protocol without requiring a
/// completed answer. Reasoning models can legitimately consume a tiny probe's
/// entire output budget before emitting assistant content.
pub async fn probe(
    connection: &ProviderConnection,
    model: &ProviderModel,
    api_key: &str,
    request: &AiTextRequest,
) -> Result<(), AiError> {
    let url = endpoint_url(connection, model)?;
    let client = http_client(&url, PROBE_TIMEOUT)?;
    let body = match model.protocol.as_str() {
        "chat_completions" => {
            send_json(
                &client,
                url,
                connection,
                model,
                api_key,
                chat_completions_body(request),
                false,
                &request.budget,
            )
            .await?
        }
        "responses" => {
            send_json(
                &client,
                url,
                connection,
                model,
                api_key,
                responses_body(request),
                false,
                &request.budget,
            )
            .await?
        }
        "anthropic_messages" => {
            send_json(
                &client,
                url,
                connection,
                model,
                api_key,
                anthropic_messages_body(request),
                true,
                &request.budget,
            )
            .await?
        }
        other => return Err(AiError::Config(format!("未知模型协议: {other}"))),
    };
    let valid = match model.protocol.as_str() {
        "chat_completions" => body.pointer("/choices/0/message").is_some(),
        "responses" => body.get("output").and_then(Value::as_array).is_some(),
        "anthropic_messages" => body.get("content").and_then(Value::as_array).is_some(),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(AiError::Api(format!(
            "Provider={} model={} 返回的 {} 协议结构无效",
            connection.display_name, model.model_id, model.protocol
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;

    fn server(
        status: u16,
        body: &'static str,
    ) -> (String, Arc<Mutex<String>>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let request = Arc::new(Mutex::new(String::new()));
        let captured = Arc::clone(&request);
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 16_384];
            let size = stream.read(&mut buffer).unwrap();
            *captured.lock().unwrap() = String::from_utf8_lossy(&buffer[..size]).to_string();
            let status_text = if status == 200 { "OK" } else { "Unauthorized" };
            let response = format!(
                "HTTP/1.1 {status} {status_text}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(), body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        (format!("http://{addr}"), request, handle)
    }

    fn connection(base_url: &str) -> ProviderConnection {
        ProviderConnection {
            id: 1,
            display_name: "Mock".into(),
            provider_type: "custom".into(),
            base_url: base_url.into(),
            legacy_model: String::new(),
            enabled: true,
            credential_ref: "provider-test".into(),
            template_kind: "custom".into(),
            auth_mode: "bearer".into(),
            models_endpoint: None,
            last_models_refresh_at: None,
            created_at: 0,
            updated_at: 0,
        }
    }

    fn opencode_connection(base_url: &str) -> ProviderConnection {
        let mut value = connection(base_url);
        value.display_name = "OpenCode Go".into();
        value.template_kind = "opencode_go".into();
        value
    }

    fn sequence_server(
        responses: Vec<(u16, &'static str)>,
    ) -> (String, Arc<AtomicUsize>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let addr = listener.local_addr().unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let captured = Arc::clone(&count);
        let handle = thread::spawn(move || {
            let deadline = std::time::Instant::now() + Duration::from_secs(3);
            while captured.load(Ordering::SeqCst) < responses.len()
                && std::time::Instant::now() < deadline
            {
                let Ok((mut stream, _)) = listener.accept() else {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                };
                // Winsock may inherit the listener's nonblocking mode. A client can
                // connect before its first bytes arrive; wait instead of panicking.
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let index = captured.fetch_add(1, Ordering::SeqCst);
                let mut buffer = [0_u8; 16_384];
                let _ = stream.read(&mut buffer).unwrap();
                let (status, body) = responses[index];
                let response = format!(
                    "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(), body
                );
                stream.write_all(response.as_bytes()).unwrap();
            }
        });
        (format!("http://{addr}"), count, handle)
    }

    fn model(protocol: &str, endpoint_path: &str) -> ProviderModel {
        ProviderModel {
            id: 1,
            provider_id: 1,
            model_id: "mock-model".into(),
            display_name: "Mock model".into(),
            protocol: protocol.into(),
            endpoint_path: endpoint_path.into(),
            capabilities_json: "{}".into(),
            source: "manual".into(),
            enabled: true,
            available: true,
            created_at: 0,
            updated_at: 0,
        }
    }

    fn request() -> AiTextRequest {
        AiTextRequest {
            model_id: "mock-model".into(),
            system: Some("system".into()),
            messages: vec![AiMessage {
                role: "user".into(),
                content: "hello".into(),
            }],
            temperature: Some(0.0),
            max_output_tokens: Some(20),
            output_format: crate::ai::output::OutputFormat::Text,
            budget: Default::default(),
        }
    }
    #[test]
    fn logical_generation_and_repair_share_three_http_attempts() {
        tauri::async_runtime::block_on(async {
            let (base, count, handle) = sequence_server(vec![
                (500, "{}"),
                (
                    200,
                    r#"{"choices":[{"message":{"content":"invalid format"},"finish_reason":"stop"}]}"#,
                ),
                (500, "{}"),
                (
                    200,
                    r#"{"choices":[{"message":{"content":"must not be called"},"finish_reason":"stop"}]}"#,
                ),
            ]);
            let request = request();
            let m = model("chat_completions", "/chat/completions");
            let c = connection(&base);
            assert!(complete(&c, &m, "synthetic", &request).await.is_ok());
            let repair = request.clone();
            assert!(complete(&c, &m, "synthetic", &repair).await.is_err());
            handle.join().unwrap();
            assert_eq!(count.load(Ordering::SeqCst), 3);
        });
    }
    #[test]
    fn oversized_chunked_provider_response_stops_at_the_transport_boundary() {
        tauri::async_runtime::block_on(async {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let handle = thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut request = [0u8; 16384];
                stream.read(&mut request).unwrap();
                stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").unwrap();
                let body = serde_json::json!({"choices":[{"message":{"content":"x".repeat(5*1024*1024)},"finish_reason":"stop"}]}).to_string();
                for chunk in body.as_bytes().chunks(32768) {
                    if write!(stream, "{:x}\r\n", chunk.len())
                        .and_then(|_| stream.write_all(chunk))
                        .and_then(|_| stream.write_all(b"\r\n"))
                        .is_err()
                    {
                        return;
                    }
                }
                let _ = stream.write_all(b"0\r\n\r\n");
            });
            let result = complete(
                &connection(&format!("http://{address}")),
                &model("chat_completions", "/chat/completions"),
                "synthetic",
                &request(),
            )
            .await;
            handle.join().unwrap();
            assert!(
                result.is_err(),
                "Oversized output must be rejected before allocating an unbounded JSON document"
            );
            assert!(result.unwrap_err().to_string().contains("4 MiB"));
        });
    }
    #[test]
    fn explicit_truncation_and_refusal_never_become_completed_content() {
        tauri::async_runtime::block_on(async {
            for (protocol, path, response) in [
                (
                    "chat_completions",
                    "/chat/completions",
                    r#"{"choices":[{"message":{"content":"partial"},"finish_reason":"length"}]}"#,
                ),
                (
                    "responses",
                    "/responses",
                    r#"{"status":"incomplete","output_text":"partial"}"#,
                ),
                (
                    "anthropic_messages",
                    "/messages",
                    r#"{"stop_reason":"max_tokens","content":[{"type":"text","text":"partial"}]}"#,
                ),
                (
                    "chat_completions",
                    "/chat/completions",
                    r#"{"choices":[{"message":{"content":"refused","refusal":"cannot answer"},"finish_reason":"stop"}]}"#,
                ),
            ] {
                let (base, _, handle) = server(200, response);
                assert!(
                    complete(
                        &connection(&base),
                        &model(protocol, path),
                        "synthetic",
                        &request()
                    )
                    .await
                    .is_err(),
                    "incomplete {protocol} response was accepted"
                );
                handle.join().unwrap();
            }
        });
    }
    #[test]
    fn multimodal_mock_receives_real_file_bytes_in_all_three_protocols() {
        tauri::async_runtime::block_on(async {
            for (protocol, path, response, expected) in [
                (
                    "chat_completions",
                    "/chat/completions",
                    r#"{"choices":[{"message":{"content":"read"}}]}"#,
                    "file_data",
                ),
                (
                    "responses",
                    "/responses",
                    r#"{"output_text":"read"}"#,
                    "input_file",
                ),
                (
                    "anthropic_messages",
                    "/messages",
                    r#"{"content":[{"type":"text","text":"read"}]}"#,
                    "document",
                ),
            ] {
                let (base, capture, handle) = server(200, response);
                let part = super::super::AiAttachment {
                    filename: "synthetic.pdf".into(),
                    media_type: "application/pdf".into(),
                    data: b"%PDF-test".to_vec(),
                };
                let result = complete_multimodal(
                    &connection(&base),
                    &model(protocol, path),
                    "synthetic-key",
                    &request(),
                    &[part],
                )
                .await
                .unwrap();
                assert_eq!(result.content, "read");
                handle.join().unwrap();
                let wire = capture.lock().unwrap();
                assert!(
                    wire.contains("JVBERi10ZXN0"),
                    "Actual bytes must reach provider"
                );
                assert!(wire.contains(expected));
            }
        });
    }

    #[test]
    fn attachment_capabilities_and_limits_do_not_depend_on_model_name() {
        let mut m = model("responses", "/responses");
        assert!(attachment_supported(&m, "application/pdf", 1024).is_ok());
        assert!(attachment_supported(&m, "application/octet-stream", 10).is_err());
        assert!(attachment_supported(&m, "application/pdf", 25 * 1024 * 1024).is_err());
        m.capabilities_json = r#"{"input_modalities":["text"],"max_file_bytes":100}"#.into();
        assert!(attachment_supported(&m, "image/png", 10).is_err());
        m.capabilities_json = r#"{"input_modalities":["image"],"max_file_bytes":100}"#.into();
        assert!(attachment_supported(&m, "image/png", 10).is_ok());
        assert!(attachment_supported(&m, "image/png", 101).is_err());
        m.protocol = "anthropic_messages".into();
        m.capabilities_json = "{}".into();
        assert!(attachment_supported(
            &m,
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            100
        )
        .is_err());
    }

    #[test]
    fn mock_protocol_adapters_return_text_and_expected_headers() {
        tauri::async_runtime::block_on(async {
            let (base, request_capture, handle) = server(
                200,
                r#"{"id":"chat-1","model":"mock-model","choices":[{"message":{"content":"chat ok"}}]}"#,
            );
            let response = complete(
                &connection(&base),
                &model("chat_completions", "/chat/completions"),
                "synthetic-key",
                &request(),
            )
            .await
            .unwrap();
            assert_eq!(response.content, "chat ok");
            let request_text = request_capture.lock().unwrap().clone();
            assert!(request_text.starts_with("POST /chat/completions HTTP/1.1"));
            assert!(request_text.to_ascii_lowercase().contains("authorization:"));
            handle.join().unwrap();

            let (base, request_capture, handle) = server(
                200,
                r#"{"id":"resp-1","output":[{"content":[{"text":"responses ok"}]}]}"#,
            );
            let response = complete(
                &connection(&base),
                &model("responses", "/responses"),
                "synthetic-key",
                &request(),
            )
            .await
            .unwrap();
            assert_eq!(response.content, "responses ok");
            let response_request = request_capture.lock().unwrap().clone();
            assert!(response_request.starts_with("POST /responses HTTP/1.1"));
            assert!(response_request.contains("\"instructions\":\"system\""));
            assert!(!response_request.contains("\"role\":\"system\""));
            handle.join().unwrap();

            let (base, request_capture, handle) = server(
                200,
                r#"{"id":"anth-1","content":[{"type":"text","text":"messages ok"}]}"#,
            );
            let response = complete(
                &connection(&base),
                &model("anthropic_messages", "/messages"),
                "synthetic-key",
                &request(),
            )
            .await
            .unwrap();
            assert_eq!(response.content, "messages ok");
            let request_text = request_capture.lock().unwrap().clone().to_ascii_lowercase();
            assert!(request_text.starts_with("post /messages http/1.1"));
            assert!(request_text.contains("x-api-key:"));
            assert!(request_text.contains("anthropic-version: 2023-06-01"));
            handle.join().unwrap();
        });
    }

    #[test]
    fn opencode_requests_identify_the_app_and_session() {
        tauri::async_runtime::block_on(async {
            let (base, request_capture, handle) = server(
                200,
                r#"{"id":"chat-1","choices":[{"message":{"content":"ok"}}]}"#,
            );
            complete(
                &opencode_connection(&base),
                &model("chat_completions", "/chat/completions"),
                "synthetic-key",
                &request(),
            )
            .await
            .unwrap();
            let request_text = request_capture.lock().unwrap().to_ascii_lowercase();
            assert!(request_text.contains("user-agent: msl-desktop/"));
            assert!(request_text.contains("x-opencode-session:"));
            handle.join().unwrap();
        });
    }

    #[test]
    fn chat_completions_accepts_structured_text_content() {
        tauri::async_runtime::block_on(async {
            let (base, _capture, handle) = server(
                200,
                r#"{"id":"chat-1","choices":[{"message":{"content":[{"type":"text","text":"structured ok"}]}}]}"#,
            );
            let response = complete(
                &connection(&base),
                &model("chat_completions", "/chat/completions"),
                "synthetic-key",
                &request(),
            )
            .await
            .unwrap();
            assert_eq!(response.content, "structured ok");
            handle.join().unwrap();
        });
    }

    #[test]
    fn transient_server_failure_is_retried_once() {
        tauri::async_runtime::block_on(async {
            let (base, count, handle) = sequence_server(vec![
                (500, r#"{"error":{"message":"temporary"}}"#),
                (
                    200,
                    r#"{"id":"chat-2","choices":[{"message":{"content":"recovered"}}]}"#,
                ),
            ]);
            let response = complete(
                &opencode_connection(&base),
                &model("chat_completions", "/chat/completions"),
                "synthetic-key",
                &request(),
            )
            .await
            .unwrap();
            handle.join().unwrap();
            assert_eq!(response.content, "recovered");
            assert_eq!(count.load(Ordering::SeqCst), 2);
        });
    }

    #[test]
    fn mock_error_is_redacted_and_bad_content_is_rejected() {
        tauri::async_runtime::block_on(async {
            let (base, _capture, handle) =
                server(401, r#"{"error":"Authorization: Bearer synthetic-key"}"#);
            let error = complete(
                &connection(&base),
                &model("chat_completions", "/chat/completions"),
                "synthetic-key",
                &request(),
            )
            .await
            .unwrap_err();
            assert!(!error.to_string().contains("synthetic-key"));
            handle.join().unwrap();

            let (base, _capture, handle) = server(200, r#"{"choices":[{"message":{}}]}"#);
            let error = complete(
                &connection(&base),
                &model("chat_completions", "/chat/completions"),
                "synthetic-key",
                &request(),
            )
            .await
            .unwrap_err();
            assert!(error.to_string().contains("content"));
            handle.join().unwrap();
        });
    }

    #[test]
    fn connection_probe_accepts_reasoning_only_chat_completion() {
        tauri::async_runtime::block_on(async {
            let (base, _capture, handle) = server(
                200,
                r#"{"id":"chat-1","model":"mock-model","choices":[{"message":{"role":"assistant","content":"","reasoning_content":"thinking"},"finish_reason":"length"}]}"#,
            );
            probe(
                &connection(&base),
                &model("chat_completions", "/chat/completions"),
                "synthetic-key",
                &request(),
            )
            .await
            .unwrap();
            handle.join().unwrap();
        });
    }

    #[test]
    fn endpoint_validation_rejects_credentials_and_traversal() {
        let invalid_connection = connection("http://user:pass@localhost:1");
        let invalid_model = model("chat_completions", "/chat/completions");
        let error = tauri::async_runtime::block_on(complete(
            &invalid_connection,
            &invalid_model,
            "key",
            &request(),
        ))
        .unwrap_err();
        assert!(error.to_string().contains("URL"));
        let valid_connection = connection("http://localhost:1");
        let traversal_model = model("chat_completions", "/../secret");
        let error = tauri::async_runtime::block_on(complete(
            &valid_connection,
            &traversal_model,
            "key",
            &request(),
        ))
        .unwrap_err();
        assert!(error.to_string().contains("endpoint"));
    }

    #[allow(dead_code)]
    fn _keep_stream_type(_: TcpStream) {}

    /// Accept all requests so an accidental retry is observable; never contacts a provider.
    fn delayed_server(delay: Duration) -> (String, Arc<AtomicUsize>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let observed = count.clone();
        let handle = thread::spawn(move || {
            let deadline = std::time::Instant::now() + delay + Duration::from_secs(2);
            let mut workers = Vec::new();
            while std::time::Instant::now() < deadline {
                let Ok((mut stream, _)) = listener.accept() else {
                    thread::sleep(Duration::from_millis(5));
                    continue;
                };
                observed.fetch_add(1, Ordering::SeqCst);
                workers.push(thread::spawn(move || {
                    stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
                    let mut received = Vec::new();
                    let mut buffer = [0_u8; 4096];
                    loop {
                        let Ok(n) = stream.read(&mut buffer) else { break };
                        if n == 0 { break; }
                        received.extend_from_slice(&buffer[..n]);
                        if let Some(end) = received.windows(4).position(|v| v == b"\r\n\r\n") {
                            let header = String::from_utf8_lossy(&received[..end]).to_lowercase();
                            let length = header.lines().find_map(|l| l.strip_prefix("content-length:")?.trim().parse::<usize>().ok()).unwrap_or(0);
                            if received.len() >= end + 4 + length { break; }
                        }
                    }
                    thread::sleep(delay);
                    let body = r#"{"choices":[{"message":{"content":"slow analysis completed"}}]}"#;
                    let _ = write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                }));
            }
            for worker in workers {
                worker.join().unwrap();
            }
        });
        (format!("http://{address}"), count, handle)
    }

    #[test]
    fn transport_timeout_names_the_cause_without_restarting_generation() {
        tauri::async_runtime::block_on(async {
            let (base, count, handle) = delayed_server(Duration::from_millis(150));
            let client = reqwest::Client::builder()
                .no_proxy()
                .timeout(Duration::from_millis(50))
                .build()
                .unwrap();
            let result = send_json(
                &client,
                reqwest::Url::parse(&base).unwrap(),
                &connection(&base),
                &model("chat_completions", "/chat/completions"),
                "synthetic-private-key",
                chat_completions_body(&request()),
                false,
                &std::sync::Mutex::new(crate::ai::output::RequestBudget::default()),
            )
            .await;
            handle.join().unwrap();
            let error = result.unwrap_err().to_string();
            assert!(
                error.contains("NET_TIMEOUT"),
                "Timeout must survive error wrapping: {error}"
            );
            assert_eq!(
                count.load(Ordering::SeqCst),
                1,
                "Do not repeat a generation whose result timed out"
            );
            assert!(!error.contains("synthetic-private-key"));
        });
    }

    #[test]
    fn transport_connection_refused_has_actionable_safe_diagnostics() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!(
            "http://{}?token=synthetic-url-secret",
            listener.local_addr().unwrap()
        );
        drop(listener);
        let error = tauri::async_runtime::block_on(complete(
            &connection(&base),
            &model("chat_completions", "/chat/completions"),
            "synthetic-key",
            &request(),
        ))
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("NET_CONNECT"),
            "Connection failure must be distinguished: {error}"
        );
        assert!(error.contains("代理"));
        assert!(!error.contains("synthetic-url-secret"));
        assert!(!error.contains("synthetic-key"));
    }

    #[test]
    fn transport_auth_error_does_not_echo_provider_body_or_retry() {
        tauri::async_runtime::block_on(async {
            let (base, count, handle) =
                sequence_server(vec![(401, r#"{"error":"echo: synthetic-work-document"}"#)]);
            let error = complete(
                &connection(&base),
                &model("chat_completions", "/chat/completions"),
                "synthetic-key",
                &request(),
            )
            .await
            .unwrap_err()
            .to_string();
            handle.join().unwrap();
            assert!(error.contains("401") && error.contains("API Key"));
            assert!(!error.contains("synthetic-work-document"));
            assert_eq!(count.load(Ordering::SeqCst), 1);
        });
    }

    #[test]
    #[ignore = "65-second loopback regression for the former fixed 60-second limit"]
    fn transport_long_analysis_survives_the_former_sixty_second_cutoff() {
        tauri::async_runtime::block_on(async {
            let (base, count, handle) = delayed_server(Duration::from_secs(65));
            let mut long = request();
            long.max_output_tokens = Some(8000);
            let result = complete(
                &connection(&base),
                &model("chat_completions", "/chat/completions"),
                "synthetic-key",
                &long,
            )
            .await;
            handle.join().unwrap();
            assert_eq!(result.unwrap().content, "slow analysis completed");
            assert_eq!(
                count.load(Ordering::SeqCst),
                1,
                "Long analysis must complete with one generation"
            );
        });
    }
}
