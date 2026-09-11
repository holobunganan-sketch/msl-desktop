//! Provider connection、model catalog 与按任务路由的 IPC 边界。

use tauri::State;

use crate::app_state::AppState;
use crate::db::provider::{AiTaskRoute, ProviderCatalogRepo, ProviderConnection, ProviderModel};

fn required(value: String, field: &str) -> Result<String, String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        Err(format!("{field} 不能为空"))
    } else {
        Ok(value)
    }
}

fn optional_trim(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim().to_string();
        (!value.is_empty()).then_some(value)
    })
}

fn with_catalog<T>(
    state: &State<AppState>,
    f: impl FnOnce(&ProviderCatalogRepo<'_>) -> crate::db::DbResult<T>,
) -> Result<T, String> {
    crate::commands::with_db(state, |db| f(&ProviderCatalogRepo::new(db.conn())))
}

#[tauri::command]
pub fn list_provider_connections(
    state: State<AppState>,
) -> Result<Vec<ProviderConnection>, String> {
    with_catalog(&state, |repo| {
        crate::ai::catalog::repair_fixed_templates(repo)?;
        repo.list_connections()
    })
}

#[tauri::command]
pub fn create_provider_template(
    state: State<AppState>,
    template_kind: String,
    api_key: Option<String>,
    enabled: Option<bool>,
) -> Result<ProviderConnection, String> {
    let template = crate::ai::catalog::template(&template_kind)
        .ok_or_else(|| "只支持 DeepSeek 或 OpenCode Go 固定模板".to_string())?;
    let connection = with_catalog(&state, |repo| {
        let connection = repo.insert_connection(
            template.display_name,
            template.kind,
            template.base_url,
            "",
            template.kind,
            template.auth_mode,
            Some(template.models_endpoint),
            enabled.unwrap_or(true),
        )?;
        crate::ai::catalog::seed_template(repo, &connection, template.kind)?;
        Ok(connection)
    })?;
    if let Some(key) = api_key.filter(|key| !key.is_empty()) {
        crate::ai::provider::save_api_key(&connection.credential_ref, &key)
            .map_err(|error| error.to_string())?;
    }
    Ok(connection)
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn save_provider_connection(
    state: State<AppState>,
    id: Option<i64>,
    display_name: String,
    provider_type: Option<String>,
    base_url: String,
    legacy_model: Option<String>,
    template_kind: Option<String>,
    auth_mode: Option<String>,
    models_endpoint: Option<String>,
    enabled: bool,
    api_key: Option<String>,
) -> Result<ProviderConnection, String> {
    let display_name = required(display_name, "Provider 显示名称")?;
    let base_url = required(base_url, "Provider Base URL")?;
    let provider_type = provider_type.unwrap_or_else(|| "custom".into());
    let legacy_model = legacy_model.unwrap_or_default();
    let template_kind = template_kind.unwrap_or_else(|| "custom".into());
    let auth_mode = auth_mode.unwrap_or_else(|| "bearer".into());
    let models_endpoint = optional_trim(models_endpoint);

    let saved = match id {
        Some(id) => with_catalog(&state, |repo| {
            repo.update_connection(
                id,
                &display_name,
                &provider_type,
                &base_url,
                &legacy_model,
                &template_kind,
                &auth_mode,
                models_endpoint.as_deref(),
                enabled,
            )
        })?,
        None => with_catalog(&state, |repo| {
            repo.insert_connection(
                &display_name,
                &provider_type,
                &base_url,
                &legacy_model,
                &template_kind,
                &auth_mode,
                models_endpoint.as_deref(),
                enabled,
            )
        })?,
    };

    // 空 Key 表示保留旧凭据；明文只在此处短暂经过 keyring facade。
    if let Some(key) = api_key.filter(|key| !key.is_empty()) {
        crate::ai::provider::save_api_key(&saved.credential_ref, &key)
            .map_err(|error| error.to_string())?;
    }
    Ok(saved)
}

#[tauri::command]
pub fn list_provider_models(
    state: State<AppState>,
    provider_id: Option<i64>,
) -> Result<Vec<ProviderModel>, String> {
    with_catalog(&state, |repo| repo.list_models(provider_id))
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn save_provider_model(
    state: State<AppState>,
    provider_id: i64,
    model_id: String,
    display_name: String,
    protocol: String,
    endpoint_path: String,
    capabilities_json: Option<String>,
    source: Option<String>,
    enabled: bool,
    available: bool,
) -> Result<ProviderModel, String> {
    let model_id = required(model_id, "模型 ID")?;
    let display_name = required(display_name, "模型显示名称")?;
    let endpoint_path = required(endpoint_path, "模型 endpoint")?;
    if !endpoint_path.starts_with('/') || endpoint_path.contains("..") {
        return Err("模型 endpoint 必须以 / 开头且不得包含 ..".into());
    }
    let source = source.unwrap_or_else(|| "manual".into());
    let capabilities_json = capabilities_json.unwrap_or_else(|| "{}".into());
    with_catalog(&state, |repo| {
        repo.get_connection(provider_id)?
            .ok_or_else(|| crate::db::DbError::NotFound("provider_connection".into()))?;
        repo.upsert_model(
            provider_id,
            &model_id,
            &display_name,
            &protocol,
            &endpoint_path,
            &capabilities_json,
            &source,
            enabled,
            available,
        )
    })
}

#[tauri::command]
pub fn set_provider_model_enabled(
    state: State<AppState>,
    id: i64,
    enabled: bool,
) -> Result<(), String> {
    with_catalog(&state, |repo| repo.set_model_enabled(id, enabled))
}

#[tauri::command]
pub fn list_ai_task_routes(state: State<AppState>) -> Result<Vec<AiTaskRoute>, String> {
    with_catalog(&state, |repo| repo.list_routes())
}

#[tauri::command]
pub fn save_ai_task_route(
    state: State<AppState>,
    task_kind: String,
    provider_model_id: Option<i64>,
) -> Result<(), String> {
    with_catalog(&state, |repo| {
        crate::db::provider::validate_task_kind(&task_kind)?;
        if let Some(model_id) = provider_model_id {
            let model = repo
                .get_model(model_id)?
                .ok_or_else(|| crate::db::DbError::NotFound("provider_model".into()))?;
            if !model.enabled {
                return Err(crate::db::DbError::Migration(
                    "provider model is disabled".into(),
                ));
            }
            let connection = repo
                .get_connection(model.provider_id)?
                .ok_or_else(|| crate::db::DbError::NotFound("provider_connection".into()))?;
            if !connection.enabled {
                return Err(crate::db::DbError::Migration(
                    "provider connection is disabled".into(),
                ));
            }
        }
        repo.upsert_route(&task_kind, provider_model_id)
    })
}

#[tauri::command]
pub async fn refresh_provider_models(
    state: State<'_, AppState>,
    provider_id: i64,
) -> Result<serde_json::Value, String> {
    let connection = with_catalog(&state, |repo| {
        repo.get_connection(provider_id)?
            .ok_or_else(|| crate::db::DbError::NotFound("provider_connection".into()))
    })?;
    let key = crate::ai::provider::get_api_key(&connection.credential_ref)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "未配置 API Key".to_string())?;
    let ids = crate::ai::catalog::fetch_model_ids(&connection, &key).await?;
    let (available, unknown) = with_catalog(&state, |repo| {
        let result = crate::ai::catalog::apply_remote_ids(repo, &connection, &ids)?;
        repo.mark_models_refreshed(provider_id, crate::db::now_unix())?;
        Ok(result)
    })?;
    Ok(serde_json::json!({ "received": ids.len(), "available": available, "unknown": unknown }))
}

#[tauri::command]
pub async fn test_provider_model(
    state: State<'_, AppState>,
    provider_model_id: i64,
) -> Result<String, String> {
    let (connection, model) = with_catalog(&state, |repo| {
        let model = repo
            .get_model(provider_model_id)?
            .ok_or_else(|| crate::db::DbError::NotFound("provider_model".into()))?;
        let connection = repo
            .get_connection(model.provider_id)?
            .ok_or_else(|| crate::db::DbError::NotFound("provider_connection".into()))?;
        Ok((connection, model))
    })?;
    if !connection.enabled || !model.enabled {
        return Err("Provider 或模型未启用".into());
    }
    let key = crate::ai::provider::get_api_key(&connection.credential_ref)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "未配置 API Key".to_string())?;
    crate::ai::provider::probe_model(
        &connection,
        &model,
        &key,
        &crate::ai::provider::AiTextRequest {
            model_id: model.model_id.clone(),
            system: None,
            messages: vec![crate::ai::provider::AiMessage {
                role: "user".into(),
                content: "ping".into(),
            }],
            temperature: Some(0.0),
            max_output_tokens: Some(5),
            output_format: crate::ai::output::OutputFormat::Text,
            budget: Default::default(),
        },
    )
    .await
    .map(|_| {
        format!(
            "连接成功（model={}，协议={}）",
            model.model_id, model.protocol
        )
    })
    .map_err(|error| error.to_string())
}
