//! AI Provider 适配层（指南 §2.5 / §8.1）。
//!
//! - Provider 抽象：HTTP 统一走 OpenAI-compatible `/chat/completions`；
//! - DeepSeek preset 即 `provider_type = "openai_compatible"` +
//!   `base_url = https://api.deepseek.com`；
//! - API Key 存 Windows Credential Manager（keyring），不落 SQLite 明文；
//! - 按需调用，默认关闭，无 key 不影响其他功能。

use std::fmt;

use serde::{Deserialize, Serialize};

#[path = "adapters.rs"]
pub mod adapters;

/// Keyring 服务名。
pub const KEYRING_SERVICE: &str = "MSLDesktop";
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiTextRequest {
    pub model_id: String,
    pub system: Option<String>,
    pub messages: Vec<AiMessage>,
    pub temperature: Option<f32>,
    pub max_output_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiTextResponse {
    pub content: String,
    pub model: Option<String>,
    pub usage: Option<serde_json::Value>,
    pub request_id: Option<String>,
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

pub fn new_credential_ref() -> String {
    format!("provider-{}", uuid::Uuid::new_v4())
}

fn keyring_user(credential_ref: &str) -> &str {
    credential_ref
}

/// 保存 API Key（Windows Credential Manager）。
pub fn save_api_key(credential_ref: &str, key: &str) -> Result<(), AiError> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, keyring_user(credential_ref))
        .map_err(|e| AiError::Keyring(e.to_string()))?;
    entry
        .set_password(key)
        .map_err(|e| AiError::Keyring(e.to_string()))
}

/// 读取 API Key；不存在返回 None。
pub fn get_api_key(credential_ref: &str) -> Result<Option<String>, AiError> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, keyring_user(credential_ref))
        .map_err(|e| AiError::Keyring(e.to_string()))?;
    match entry.get_password() {
        Ok(k) => Ok(Some(k)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AiError::Keyring(e.to_string())),
    }
}

/// 删除 API Key。
pub fn delete_api_key(credential_ref: &str) -> Result<(), AiError> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, keyring_user(credential_ref))
        .map_err(|e| AiError::Keyring(e.to_string()))?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AiError::Keyring(e.to_string())),
    }
}

/// 是否已配置 API Key（用于前端判断"已配置/未配置"）。
pub fn has_api_key(credential_ref: &str) -> bool {
    get_api_key(credential_ref)
        .map(|k| k.is_some())
        .unwrap_or(false)
}

/// 统一按 Provider model 协议完成文本请求。
pub async fn complete_model(
    connection: &crate::db::provider::ProviderConnection,
    model: &crate::db::provider::ProviderModel,
    api_key: &str,
    request: &AiTextRequest,
) -> Result<AiTextResponse, AiError> {
    validate_test_endpoint(&connection.base_url)?;
    adapters::complete(connection, model, api_key, request).await
}

fn validate_test_endpoint(base_url: &str) -> Result<(), AiError> {
    if std::env::var("MSL_ISOLATED_TEST").as_deref() == Ok("1") {
        let url = reqwest::Url::parse(base_url)
            .map_err(|_| AiError::Api("测试 Provider 地址无效".into()))?;
        if !matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]")) {
            return Err(AiError::Api("隔离测试仅允许本地 Mock Provider".into()));
        }
    }
    Ok(())
}

/// 连接探测只验证认证、端点与协议结构，允许推理模型尚未生成最终文本。
pub async fn probe_model(
    connection: &crate::db::provider::ProviderConnection,
    model: &crate::db::provider::ProviderModel,
    api_key: &str,
    request: &AiTextRequest,
) -> Result<(), AiError> {
    validate_test_endpoint(&connection.base_url)?;
    adapters::probe(connection, model, api_key, request).await
}

/// 兼容旧调用方：把旧 ProviderSetting 的 model 视为 Chat Completions 模型。
pub async fn complete(
    provider: &crate::db::provider::ProviderSetting,
    api_key: &str,
    request: &AiRequest,
) -> Result<AiResponse, AiError> {
    let connection = crate::db::provider::ProviderConnection {
        id: provider.id,
        display_name: provider.display_name.clone(),
        provider_type: provider.provider_type.clone(),
        base_url: provider.base_url.clone(),
        legacy_model: provider.model.clone(),
        enabled: provider.enabled,
        credential_ref: provider.credential_ref.clone(),
        template_kind: "custom".into(),
        auth_mode: "bearer".into(),
        models_endpoint: None,
        last_models_refresh_at: None,
        created_at: provider.created_at,
        updated_at: provider.updated_at,
    };
    let model = crate::db::provider::ProviderModel {
        id: 0,
        provider_id: provider.id,
        model_id: request.model.clone(),
        display_name: request.model.clone(),
        protocol: "chat_completions".into(),
        endpoint_path: "/chat/completions".into(),
        capabilities_json: "{}".into(),
        source: "legacy".into(),
        enabled: true,
        available: true,
        created_at: provider.created_at,
        updated_at: provider.updated_at,
    };
    let response = complete_model(
        &connection,
        &model,
        api_key,
        &AiTextRequest {
            model_id: request.model.clone(),
            system: None,
            messages: request.messages.clone(),
            temperature: request.temperature,
            max_output_tokens: request.max_tokens,
        },
    )
    .await?;
    Ok(AiResponse {
        content: response.content,
        model: response.model,
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

#[cfg(test)]
mod tests {
    use super::new_credential_ref;

    #[test]
    fn credential_refs_are_random_and_prefixed() {
        let a = new_credential_ref();
        let b = new_credential_ref();
        assert!(a.starts_with("provider-"));
        assert!(b.starts_with("provider-"));
        assert_ne!(a, b);
    }
}
