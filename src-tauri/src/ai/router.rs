//! 按任务选择 Provider model 的唯一路由入口。

use crate::db::provider::{AiTaskRoute, ProviderCatalogRepo, ProviderConnection, ProviderModel};

pub trait CredentialSource {
    fn get(&self, credential_ref: &str) -> Result<Option<String>, super::provider::AiError>;
}

pub struct KeyringCredentialSource;

impl CredentialSource for KeyringCredentialSource {
    fn get(&self, credential_ref: &str) -> Result<Option<String>, super::provider::AiError> {
        super::provider::get_api_key(credential_ref)
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedModel {
    pub task_kind: String,
    pub route: AiTaskRoute,
    pub connection: ProviderConnection,
    pub model: ProviderModel,
}

pub struct AiRouter<'a, C: CredentialSource> {
    repo: &'a ProviderCatalogRepo<'a>,
    credentials: &'a C,
}

impl<'a, C: CredentialSource> AiRouter<'a, C> {
    pub fn new(repo: &'a ProviderCatalogRepo<'a>, credentials: &'a C) -> Self {
        Self { repo, credentials }
    }

    pub fn resolve(&self, task_kind: &str) -> Result<ResolvedModel, super::provider::AiError> {
        crate::db::provider::validate_task_kind(task_kind)
            .map_err(|error| super::provider::AiError::Config(error.to_string()))?;
        let route = self
            .repo
            .get_route(task_kind)
            .map_err(|error| super::provider::AiError::Config(error.to_string()))?
            .or(self
                .repo
                .get_route("general")
                .map_err(|error| super::provider::AiError::Config(error.to_string()))?)
            .ok_or_else(|| {
                super::provider::AiError::Config(format!("未配置任务路由: {task_kind}"))
            })?;
        let model_id = route.provider_model_id.ok_or_else(|| {
            super::provider::AiError::Config(format!("任务 {task_kind} 未绑定 Provider 模型"))
        })?;
        let model = self
            .repo
            .get_model(model_id)
            .map_err(|error| super::provider::AiError::Config(error.to_string()))?
            .ok_or_else(|| {
                super::provider::AiError::Config(format!(
                    "任务 {task_kind} 的 Provider 模型不存在: {model_id}"
                ))
            })?;
        if !model.enabled {
            return Err(super::provider::AiError::Config(format!(
                "任务 {task_kind} 的模型 {} 已禁用",
                model.model_id
            )));
        }
        if !model.available {
            return Err(super::provider::AiError::Config(format!(
                "任务 {task_kind} 的模型 {} 当前不可用",
                model.model_id
            )));
        }
        let connection = self
            .repo
            .get_connection(model.provider_id)
            .map_err(|error| super::provider::AiError::Config(error.to_string()))?
            .ok_or_else(|| super::provider::AiError::Config("Provider connection 不存在".into()))?;
        if !connection.enabled {
            return Err(super::provider::AiError::Config(format!(
                "任务 {task_kind} 的 Provider {} 已禁用",
                connection.display_name
            )));
        }
        crate::db::provider::validate_protocol(&model.protocol)
            .map_err(|error| super::provider::AiError::Config(error.to_string()))?;
        Ok(ResolvedModel {
            task_kind: task_kind.into(),
            route,
            connection,
            model,
        })
    }

    pub async fn complete(
        &self,
        task_kind: &str,
        request: &super::provider::AiTextRequest,
    ) -> Result<super::provider::AiTextResponse, super::provider::AiError> {
        let resolved = self.resolve(task_kind)?;
        let key = self
            .credentials
            .get(&resolved.connection.credential_ref)?
            .ok_or_else(|| {
                super::provider::AiError::Config(format!("任务 {task_kind} 未配置 API Key"))
            })?;
        super::provider::complete_model(&resolved.connection, &resolved.model, &key, request).await
    }
}

pub fn resolve<'a, C: CredentialSource>(
    repo: &'a ProviderCatalogRepo<'a>,
    credentials: &'a C,
    task_kind: &str,
) -> Result<ResolvedModel, super::provider::AiError> {
    AiRouter::new(repo, credentials).resolve(task_kind)
}

pub async fn complete<C: CredentialSource>(
    repo: &ProviderCatalogRepo<'_>,
    credentials: &C,
    task_kind: &str,
    request: &super::provider::AiTextRequest,
) -> Result<super::provider::AiTextResponse, super::provider::AiError> {
    AiRouter::new(repo, credentials)
        .complete(task_kind, request)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::provider::ProviderCatalogRepo;
    use crate::db::Database;

    struct FakeCredentials {
        value: Option<String>,
    }

    impl CredentialSource for FakeCredentials {
        fn get(&self, _: &str) -> Result<Option<String>, super::super::provider::AiError> {
            Ok(self.value.clone())
        }
    }

    fn fixture() -> (Database, i64, i64) {
        let db = Database::open_in_memory().unwrap();
        let repo = ProviderCatalogRepo::new(db.conn());
        let connection = repo
            .insert_connection(
                "Mock",
                "custom",
                "http://127.0.0.1",
                "",
                "custom",
                "bearer",
                None,
                true,
            )
            .unwrap();
        let model = repo
            .upsert_model(
                connection.id,
                "mock-model",
                "Mock",
                "chat_completions",
                "/chat/completions",
                "{}",
                "manual",
                true,
                true,
            )
            .unwrap();
        (db, connection.id, model.id)
    }

    #[test]
    fn exact_route_precedes_general_and_missing_is_error() {
        let (db, _provider_id, model_id) = fixture();
        let repo = ProviderCatalogRepo::new(db.conn());
        repo.upsert_route("general", Some(model_id)).unwrap();
        repo.upsert_route("translation", Some(model_id)).unwrap();
        let credentials = FakeCredentials {
            value: Some("synthetic".into()),
        };
        let resolved = resolve(&repo, &credentials, "translation").unwrap();
        assert_eq!(resolved.task_kind, "translation");
        assert!(resolve(&repo, &credentials, "daily_brief").is_ok());
        assert!(resolve(&repo, &credentials, "unknown").is_err());
    }

    #[test]
    fn disabled_unavailable_and_missing_key_are_named_errors() {
        let (db, _provider_id, model_id) = fixture();
        let repo = ProviderCatalogRepo::new(db.conn());
        repo.upsert_route("general", Some(model_id)).unwrap();
        let credentials = FakeCredentials { value: None };
        let error = tauri::async_runtime::block_on(complete(
            &repo,
            &credentials,
            "general",
            &super::super::provider::AiTextRequest {
                model_id: "mock-model".into(),
                system: None,
                messages: Vec::new(),
                temperature: None,
                max_output_tokens: None,
                output_format: crate::ai::output::OutputFormat::Text,
                budget: Default::default(),
            },
        ))
        .unwrap_err();
        assert!(error.to_string().contains("general"));
        repo.set_model_enabled(model_id, false).unwrap();
        let error = resolve(&repo, &credentials, "general").unwrap_err();
        assert!(error.to_string().contains("禁用"));
        assert!(!error.to_string().contains("synthetic"));
    }
}
