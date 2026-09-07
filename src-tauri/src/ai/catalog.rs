//! 固定 Provider 模板与远端模型目录刷新。

use crate::db::provider::{ProviderCatalogRepo, ProviderConnection};

#[derive(Debug, Clone)]
pub struct ModelSeed {
    pub model_id: &'static str,
    pub protocol: &'static str,
    pub endpoint_path: &'static str,
}

#[derive(Debug, Clone)]
pub struct ProviderTemplate {
    pub kind: &'static str,
    pub display_name: &'static str,
    pub base_url: &'static str,
    pub models_endpoint: &'static str,
    pub auth_mode: &'static str,
    pub models: &'static [ModelSeed],
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
            display_name: "DeepSeek",
            base_url: "https://api.deepseek.com",
            models_endpoint: "https://api.deepseek.com/models",
            auth_mode: "bearer",
            models: DEEPSEEK_MODELS,
        }),
        "opencode_go" => Some(ProviderTemplate {
            kind: "opencode_go",
            display_name: "OpenCode Go",
            base_url: "https://opencode.ai/zen/go/v1",
            models_endpoint: "https://opencode.ai/zen/go/v1/models",
            auth_mode: "bearer",
            models: OPENCODE_GO_MODELS,
        }),
        _ => None,
    }
}

pub fn seed_template(
    repo: &ProviderCatalogRepo<'_>,
    provider: &ProviderConnection,
    kind: &str,
) -> crate::db::DbResult<usize> {
    let template = template(kind)
        .ok_or_else(|| crate::db::DbError::Migration("unknown fixed template".into()))?;
    for model in template.models {
        repo.upsert_model(
            provider.id,
            model.model_id,
            model.model_id,
            model.protocol,
            model.endpoint_path,
            "{}",
            "template",
            true,
            true,
        )?;
    }
    Ok(template.models.len())
}

pub fn known_model(kind: &str, model_id: &str) -> Option<&'static ModelSeed> {
    template(kind)?
        .models
        .iter()
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
            if let Some(model) = existing
                .iter()
                .find(|model| model.model_id == seed.model_id)
            {
                repo.upsert_model(
                    connection.id,
                    seed.model_id,
                    &model.display_name,
                    seed.protocol,
                    seed.endpoint_path,
                    "{}",
                    &model.source,
                    model.enabled,
                    model.available,
                )?;
            } else {
                repo.upsert_model(
                    connection.id,
                    seed.model_id,
                    seed.model_id,
                    seed.protocol,
                    seed.endpoint_path,
                    "{}",
                    "template",
                    true,
                    true,
                )?;
            }
        }
        repaired_connections += 1;
    }
    Ok(repaired_connections)
}

pub async fn fetch_model_ids(
    connection: &ProviderConnection,
    api_key: &str,
) -> Result<Vec<String>, String> {
    let endpoint = connection
        .models_endpoint
        .clone()
        .unwrap_or_else(|| format!("{}/models", connection.base_url.trim_end_matches('/')));
    let url = reqwest::Url::parse(&endpoint).map_err(|_| "模型目录 URL 无效".to_string())?;
    if !matches!(url.scheme(), "http" | "https") || url.username() != "" || url.password().is_some()
    {
        return Err("模型目录 URL 必须是无凭据的 http/https URL".into());
    }
    let response = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|error| format!("HTTP client error: {}", error))?
        .get(url)
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|error| format!("模型目录请求失败: {}", error))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("模型目录读取失败: {}", error))?;
    if !status.is_success() {
        return Err(format!("模型目录 HTTP {}", status.as_u16()));
    }
    let value: serde_json::Value =
        serde_json::from_str(&body).map_err(|_| "模型目录响应格式无效".to_string())?;
    let ids = value
        .get("data")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "模型目录缺少 data".to_string())?
        .iter()
        .filter_map(|item| item.get("id").and_then(serde_json::Value::as_str))
        .map(str::to_string)
        .collect();
    Ok(ids)
}

pub fn apply_remote_ids(
    repo: &ProviderCatalogRepo<'_>,
    connection: &ProviderConnection,
    ids: &[String],
) -> crate::db::DbResult<(usize, usize)> {
    let existing = repo.list_models(Some(connection.id))?;
    let mut available = 0;
    let mut unknown = 0;
    for id in ids {
        if let Some(seed) = known_model(&connection.template_kind, id) {
            repo.upsert_model(
                connection.id,
                id,
                id,
                seed.protocol,
                seed.endpoint_path,
                "{}",
                "remote",
                true,
                true,
            )?;
            available += 1;
        } else if let Some(model) = existing.iter().find(|model| model.model_id == *id) {
            repo.upsert_model(
                connection.id,
                id,
                &model.display_name,
                &model.protocol,
                &model.endpoint_path,
                r#"{"needs_protocol":true}"#,
                "remote",
                false,
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
                r#"{"needs_protocol":true}"#,
                "remote",
                false,
                true,
            )?;
            unknown += 1;
        }
    }
    for model in existing {
        if !ids.iter().any(|id| id == &model.model_id) {
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
    Ok((available, unknown))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{provider::ProviderCatalogRepo, Database};

    #[test]
    fn fixed_templates_match_locked_snapshot() {
        let deepseek = template("deepseek").unwrap();
        assert_eq!(deepseek.base_url, "https://api.deepseek.com");
        assert_eq!(deepseek.models.len(), 2);
        let go = template("opencode_go").unwrap();
        assert_eq!(go.models_endpoint, "https://opencode.ai/zen/go/v1/models");
        let muse = known_model("opencode_go", "muse-spark-1.2-contributor").unwrap();
        assert_eq!(muse.protocol, "responses");
        assert_eq!(muse.endpoint_path, "/responses");
        let grok = known_model("opencode_go", "grok-4.6").unwrap();
        assert_eq!(grok.protocol, "responses");
        assert_eq!(grok.endpoint_path, "/responses");
        assert!(known_model("opencode_go", "grok-4.5").is_none());
        assert_eq!(go.models.len(), 28);
        assert_eq!(
            go.models
                .iter()
                .filter(|m| m.protocol == "responses")
                .count(),
            4
        );
        assert_eq!(
            go.models
                .iter()
                .filter(|m| m.protocol == "chat_completions")
                .count(),
            16
        );
        assert_eq!(
            go.models
                .iter()
                .filter(|m| m.protocol == "anthropic_messages")
                .count(),
            8
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
            "{}",
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
    }
}
