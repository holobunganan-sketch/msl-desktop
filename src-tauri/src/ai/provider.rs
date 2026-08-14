//! AI Provider 适配层（指南 §2.5 / §8.1）。
//!
//! - Provider 抽象：HTTP 统一走 OpenAI-compatible `/chat/completions`；
//! - DeepSeek preset 即 `provider_type = "openai_compatible"` +
//!   `base_url = https://api.deepseek.com`；
//! - API Key 存 Windows Credential Manager（keyring），不落 SQLite 明文；
//! - 按需调用，默认关闭，无 key 不影响其他功能。

use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Keyring 服务名。
pub const KEYRING_SERVICE: &str = "MSLDesktop";
/// AI 请求超时。
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiMessage {
    pub role: String, // system | user
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiRequest {
    pub model: String,
    pub messages: Vec<AiMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResponse {
    pub content: String,
    pub model: Option<String>,
}

#[derive(Debug)]
pub enum AiError {
    Config(String),
    Keyring(String),
    Http(String),
    Api(String),
}

impl fmt::Display for AiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AiError::Config(m) => write!(f, "配置错误: {m}"),
            AiError::Keyring(m) => write!(f, "凭据存储错误: {m}"),
            AiError::Http(m) => write!(f, "网络错误: {m}"),
            AiError::Api(m) => write!(f, "API 错误: {m}"),
        }
    }
}

impl std::error::Error for AiError {}

fn keyring_user(provider_id: i64) -> String {
    format!("provider-{provider_id}")
}

/// 保存 API Key（Windows Credential Manager）。
pub fn save_api_key(provider_id: i64, key: &str) -> Result<(), AiError> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, &keyring_user(provider_id))
        .map_err(|e| AiError::Keyring(e.to_string()))?;
    entry
        .set_password(key)
        .map_err(|e| AiError::Keyring(e.to_string()))
}

/// 读取 API Key；不存在返回 None。
pub fn get_api_key(provider_id: i64) -> Result<Option<String>, AiError> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, &keyring_user(provider_id))
        .map_err(|e| AiError::Keyring(e.to_string()))?;
    match entry.get_password() {
        Ok(k) => Ok(Some(k)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AiError::Keyring(e.to_string())),
    }
}

/// 删除 API Key。
pub fn delete_api_key(provider_id: i64) -> Result<(), AiError> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, &keyring_user(provider_id))
        .map_err(|e| AiError::Keyring(e.to_string()))?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AiError::Keyring(e.to_string())),
    }
}

/// 是否已配置 API Key（用于前端判断"已配置/未配置"）。
pub fn has_api_key(provider_id: i64) -> bool {
    get_api_key(provider_id).map(|k| k.is_some()).unwrap_or(false)
}

/// 调用 OpenAI-compatible chat completion。
pub async fn complete(
    provider: &crate::db::provider::ProviderSetting,
    api_key: &str,
    request: &AiRequest,
) -> Result<AiResponse, AiError> {
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| AiError::Http(e.to_string()))?;

    let url = format!(
        "{}/chat/completions",
        provider.base_url.trim_end_matches('/')
    );

    let mut body = serde_json::json!({
        "model": request.model,
        "messages": request.messages,
    });
    if let Some(t) = request.temperature {
        body["temperature"] = serde_json::json!(t);
    }
    if let Some(mt) = request.max_tokens {
        body["max_tokens"] = serde_json::json!(mt);
    }

    let resp = client
        .post(&url)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| AiError::Http(e.to_string()))?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| AiError::Http(e.to_string()))?;

    if !status.is_success() {
        let snippet: String = text.chars().take(200).collect();
        return Err(AiError::Api(format!("HTTP {status}: {snippet}")));
    }

    let parsed: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| AiError::Api(format!("响应解析失败: {e}")))?;
    let content = parsed["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| AiError::Api("响应缺少 choices[0].message.content".into()))?
        .to_string();

    Ok(AiResponse {
        content,
        model: parsed["model"].as_str().map(str::to_string),
    })
}

/// 连接测试：发送最小请求验证 key/base_url/model 是否可用。
pub async fn test_connection(
    provider: &crate::db::provider::ProviderSetting,
    api_key: &str,
) -> Result<AiResponse, AiError> {
    complete(
        provider,
        api_key,
        &AiRequest {
            model: provider.model.clone(),
            messages: vec![AiMessage {
                role: "user".into(),
                content: "ping".into(),
            }],
            temperature: Some(0.0),
            max_tokens: Some(5),
        },
    )
    .await
}
