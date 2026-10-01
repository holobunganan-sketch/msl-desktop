//! Provider connection、model catalog 与按任务路由的 IPC 边界。

use tauri::State;

use crate::app_state::AppState;
use crate::db::provider::{AiTaskRoute, ProviderCatalogRepo, ProviderConnection, ProviderModel};

#[tauri::command]
pub fn list_provider_templates() -> Vec<crate::ai::catalog::ProviderTemplate> {
    crate::ai::catalog::list_templates()
}

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
    display_name: Option<String>,
) -> Result<ProviderConnection, String> {
    let template = crate::ai::catalog::template(&template_kind)
        .ok_or_else(|| "未找到提供商模板".to_string())?;
    if !template.configurable {
        return Err(format!("{}：{}", template.display_name, template.note));
    }
    let display_name = optional_trim(display_name);
    let connection = with_catalog(&state, |repo| {
        repo.transaction(|repo| {
            let connection = repo.insert_connection(
                display_name.as_deref().unwrap_or(template.display_name),
                template.kind,
                template.base_url,
                "",
                template.kind,
                template.auth_mode,
                template.models_endpoint,
                enabled.unwrap_or(true),
            )?;
            crate::ai::catalog::seed_template(repo, &connection, template.kind)?;
            Ok(connection)
        })
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
    confirm_available: Option<bool>,
) -> Result<ProviderModel, String> {
    let model_id = required(model_id, "模型 ID")?;
    let display_name = required(display_name, "模型显示名称")?;
    let endpoint_path = required(endpoint_path, "模型 endpoint")?;
    crate::ai::catalog::validate_endpoint_path(&endpoint_path)?;
    let _ = source;
    let capabilities_json = capabilities_json.unwrap_or_else(|| "{}".into());
    let mut capabilities: serde_json::Value = serde_json::from_str(&capabilities_json)
        .map_err(|_| "模型能力必须是有效 JSON 对象".to_string())?;
    let map = capabilities
        .as_object_mut()
        .ok_or_else(|| "模型能力必须是有效 JSON 对象".to_string())?;
    map.remove("needs_protocol");
    map.remove("unsupported_protocol");
    map.remove("protocol_from_remote");
    with_catalog(&state, |repo| {
        save_manual_model(
            repo,
            provider_id,
            &model_id,
            &display_name,
            &protocol,
            &endpoint_path,
            &capabilities.to_string(),
            enabled,
            available,
            confirm_available.unwrap_or(false),
        )
    })
}

#[allow(clippy::too_many_arguments)]
fn save_manual_model(
    repo: &ProviderCatalogRepo<'_>,
    provider_id: i64,
    model_id: &str,
    display_name: &str,
    protocol: &str,
    endpoint_path: &str,
    capabilities_json: &str,
    enabled: bool,
    available: bool,
    confirm_available: bool,
) -> crate::db::DbResult<ProviderModel> {
    repo.get_connection(provider_id)?
        .ok_or_else(|| crate::db::DbError::NotFound("provider_connection".into()))?;
    let existing = repo
        .list_models(Some(provider_id))?
        .into_iter()
        .find(|model| model.model_id == model_id);
    repo.upsert_model(
        provider_id,
        model_id,
        display_name,
        protocol,
        endpoint_path,
        capabilities_json,
        "manual",
        enabled,
        confirm_available || existing.map_or(available, |model| model.available),
    )
}

fn finish_successful_probe(
    repo: &ProviderCatalogRepo<'_>,
    connection: &ProviderConnection,
    model: &ProviderModel,
) -> crate::db::DbResult<()> {
    repo.transaction(|repo| {
        let current_connection = repo
            .get_connection(connection.id)?
            .ok_or_else(|| crate::db::DbError::NotFound("provider_connection".into()))?;
        let current_model = repo
            .get_model(model.id)?
            .ok_or_else(|| crate::db::DbError::NotFound("provider_model".into()))?;
        if current_connection != *connection || current_model != *model {
            return Err(crate::db::DbError::Migration(
                "连接或模型设置在测试期间发生变化，请重试；未改变当前可用状态".into(),
            ));
        }
        repo.mark_model_available(model.id)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_manual_confirmation_restores_missing_model_without_changing_identity_or_route() {
        let db = crate::db::Database::open_in_memory().unwrap();
        let repo = ProviderCatalogRepo::new(db.conn());
        let provider = repo
            .insert_connection(
                "Synthetic",
                "custom",
                "http://127.0.0.1",
                "",
                "custom",
                "none",
                None,
                true,
            )
            .unwrap();
        let model = repo
            .upsert_model(
                provider.id,
                "missing",
                "Missing",
                "chat_completions",
                "/chat/completions",
                "{}",
                "remote",
                true,
                false,
            )
            .unwrap();
        repo.upsert_route("general", Some(model.id)).unwrap();
        let ordinary_edit = save_manual_model(
            &repo,
            provider.id,
            "missing",
            "Edited",
            "chat_completions",
            "/chat/completions",
            "{}",
            true,
            true,
            false,
        )
        .unwrap();
        assert!(
            !ordinary_edit.available,
            "ordinary editing must preserve discovery availability"
        );
        let restored = save_manual_model(
            &repo,
            provider.id,
            "missing",
            "Confirmed",
            "chat_completions",
            "/chat/completions",
            "{}",
            true,
            true,
            true,
        )
        .unwrap();
        assert!(
            restored.available,
            "explicit manual confirmation must restore usability"
        );
        assert_eq!(restored.id, model.id);
        assert_eq!(restored.source, "manual");
        assert_eq!(
            repo.get_route("general")
                .unwrap()
                .unwrap()
                .provider_model_id,
            Some(model.id)
        );
    }

    #[test]
    fn successful_probe_restores_availability_only_for_unchanged_snapshot() {
        let db = crate::db::Database::open_in_memory().unwrap();
        let repo = ProviderCatalogRepo::new(db.conn());
        let provider = repo
            .insert_connection(
                "Synthetic",
                "custom",
                "http://127.0.0.1",
                "",
                "custom",
                "none",
                None,
                true,
            )
            .unwrap();
        let model = repo
            .upsert_model(
                provider.id,
                "missing",
                "Missing",
                "chat_completions",
                "/chat/completions",
                "{}",
                "remote",
                true,
                false,
            )
            .unwrap();
        finish_successful_probe(&repo, &provider, &model).unwrap();
        let restored = repo.get_model(model.id).unwrap().unwrap();
        assert!(restored.available);
        assert!(restored.enabled);
        assert_eq!(restored.source, "remote");
        repo.set_model_enabled(model.id, false).unwrap();
        assert!(finish_successful_probe(&repo, &provider, &restored).is_err());
        assert!(!repo.get_model(model.id).unwrap().unwrap().enabled);
    }

    #[test]
    fn successful_probe_does_not_overwrite_a_changed_connection_or_deleted_model() {
        let db = crate::db::Database::open_in_memory().unwrap();
        let repo = ProviderCatalogRepo::new(db.conn());
        let provider = repo
            .insert_connection(
                "Synthetic",
                "custom",
                "http://127.0.0.1",
                "",
                "custom",
                "none",
                None,
                true,
            )
            .unwrap();
        let model = repo
            .upsert_model(
                provider.id,
                "missing",
                "Missing",
                "chat_completions",
                "/chat/completions",
                "{}",
                "remote",
                true,
                false,
            )
            .unwrap();
        repo.update_connection(
            provider.id,
            "Changed",
            "custom",
            "http://localhost",
            "",
            "custom",
            "none",
            None,
            true,
        )
        .unwrap();
        assert!(finish_successful_probe(&repo, &provider, &model).is_err());
        assert!(!repo.get_model(model.id).unwrap().unwrap().available);
        db.conn()
            .execute("DELETE FROM provider_models WHERE id=?1", [model.id])
            .unwrap();
        assert!(finish_successful_probe(&repo, &provider, &model).is_err());
    }
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
            let capabilities: serde_json::Value =
                serde_json::from_str(&model.capabilities_json).unwrap_or_default();
            if !model.enabled
                || !model.available
                || capabilities
                    .get("needs_protocol")
                    .and_then(serde_json::Value::as_bool)
                    == Some(true)
            {
                return Err(crate::db::DbError::Migration(
                    "provider model is disabled, unavailable, or awaiting protocol selection"
                        .into(),
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
    if let Some(template) =
        crate::ai::catalog::template(&connection.template_kind).filter(|template| {
            template.discovery == "curated"
                && crate::ai::catalog::discovery_endpoint(&connection).is_none()
        })
    {
        let (count, available) = with_catalog(&state, |repo| {
            repo.transaction(|repo| {
                let count = crate::ai::catalog::seed_template(repo, &connection, template.kind)?;
                let available = repo
                    .list_models(Some(provider_id))?
                    .into_iter()
                    .filter(|model| model.available)
                    .count();
                Ok((count, available))
            })
        })?;
        let message = if count == 0 {
            "该服务的模型目录需要额外配置或独立认证；请按官方文档手动添加账户可用模型。尚未进行远程目录验证。"
        } else {
            "已载入官方文档预置列表；这不代表网络验证成功或账户已开通全部模型。"
        };
        return Ok(
            serde_json::json!({"received":count,"available":available,"unknown":0,"source":"curated","message":message,"docs_url":template.docs_url}),
        );
    }
    let key = if connection.auth_mode == "none" {
        String::new()
    } else {
        crate::ai::provider::get_api_key(&connection.credential_ref)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "未配置 API Key".to_string())?
    };
    let models = crate::ai::catalog::fetch_models(&connection, &key).await?;
    let (available, unknown) = with_catalog(&state, |repo| {
        let current = repo
            .get_connection(provider_id)?
            .ok_or_else(|| crate::db::DbError::NotFound("provider_connection".into()))?;
        if current.base_url != connection.base_url
            || current.models_endpoint != connection.models_endpoint
            || current.auth_mode != connection.auth_mode
            || current.template_kind != connection.template_kind
        {
            return Err(crate::db::DbError::Migration(
                "连接设置在刷新期间发生变化，请重试".into(),
            ));
        }
        crate::ai::catalog::apply_remote_models(repo, &connection, &models)
    })?;
    let message = if unknown > 0 {
        Some("部分模型需要手动选择兼容协议后才可使用")
    } else if connection.template_kind == "minimax_tokenplan" {
        Some("目录包含平台模型；具体套餐权益仍以账户为准")
    } else {
        None
    };
    Ok(
        serde_json::json!({ "received": models.len(), "available": available, "unknown": unknown, "source":"remote", "message": message }),
    )
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
    let key = if connection.auth_mode == "none" {
        String::new()
    } else {
        crate::ai::provider::get_api_key(&connection.credential_ref)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "未配置 API Key".to_string())?
    };
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
    .map_err(|error| error.to_string())?;
    with_catalog(&state, |repo| {
        finish_successful_probe(repo, &connection, &model)
    })?;
    Ok(format!(
        "连接成功（model={}，协议={}）",
        model.model_id, model.protocol
    ))
}
