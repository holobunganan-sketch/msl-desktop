use super::with_db;
use crate::app_state::AppState;
use serde_json::Value;
use tauri::{Manager, State};

fn mark_analysis_failed(state: &State<AppState>, run_id: i64, code: &str, message: &str) {
    let _ = state.with_database(|db| crate::ai::analysis::fail_run(db, run_id, code, message));
}

#[tauri::command]
pub fn secretary_round_status(
    state: State<AppState>,
    work_id: Option<i64>,
    proposal_id: Option<i64>,
) -> Result<Vec<crate::ai::rounds::RoundStatus>, String> {
    with_db(&state, |db| {
        let scopes = if let Some(w) = work_id {
            crate::db::work::WorkRepo::new(db.conn())
                .get(w)?
                .ok_or_else(|| crate::db::DbError::NotFound("work".into()))?;
            vec![format!("work:{w}")]
        } else if let Some(p) = proposal_id {
            crate::ai::rounds::scopes_for_proposal(db, p)?
        } else {
            return Ok(Vec::new());
        };
        scopes
            .iter()
            .map(|s| crate::ai::rounds::status(db, s))
            .collect()
    })
}

#[tauri::command]
pub fn complete_secretary_round(
    state: State<AppState>,
    scope: String,
    expected_epoch: i64,
) -> Result<(), String> {
    with_db(&state, |db| {
        crate::ai::rounds::complete(db, &scope, expected_epoch)
    })
}

/// Disk scanning uses a separate connection and blocking worker, never the UI's
/// shared database lock. Refresh linked folders before reading cached evidence.
pub(crate) async fn refresh_evidence(workspaces: Option<Vec<i64>>) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let db =
            crate::db::Database::open(&crate::db::default_db_path()).map_err(|e| e.to_string())?;
        let folders = crate::db::workspace::WorkspaceRepo::new(db.conn())
            .list()
            .map_err(|e| e.to_string())?;
        for folder in folders
            .into_iter()
            .filter(|f| f.enabled && workspaces.as_ref().is_none_or(|ids| ids.contains(&f.id)))
        {
            let root = std::path::Path::new(&folder.root_path);
            if !root.is_dir() {
                continue;
            } // Existing index carries last-known evidence.
            crate::workspace::inventory::reconcile(&db, folder.id, root)
                .map_err(|_| "目录扫描失败，请检查目录是否可访问".to_string())?;
            crate::documents::indexer::reindex_workspace(&db, folder.id, root)
                .map_err(|_| "目录文档索引失败，请重试".to_string())?;
        }
        crate::cognition::refresh_all(&db)
            .map_err(|_| "项目认知入口更新失败，请检查应用存储路径".to_string())?;
        Ok(())
    })
    .await
    .map_err(|_| "目录分析线程异常退出".to_string())?
}

pub(crate) async fn execute_global_analysis(
    state: &State<'_, AppState>,
    trigger: &str,
) -> Result<i64, String> {
    execute_focused_analysis(state, trigger, None).await
}

pub(crate) async fn execute_focused_analysis(
    state: &State<'_, AppState>,
    trigger: &str,
    inbox_id: Option<i64>,
) -> Result<i64, String> {
    use chrono::Timelike;

    let now = chrono::Local::now();
    let period_end = now.timestamp();
    let period_start = period_end.saturating_sub(24 * 60 * 60);
    let today_start = period_end.saturating_sub(i64::from(now.num_seconds_from_midnight()));
    let today_end = today_start.saturating_add(24 * 60 * 60);
    let date = now.format("%Y-%m-%d").to_string();
    let run_id = with_db(state, |db| {
        Ok(crate::db::ai::AnalysisRunRepo::new(db.conn())
            .create(trigger, Some(period_start), Some(period_end))?
            .id)
    })?;

    let tickets = with_db(state, |db| {
        let scopes = crate::ai::rounds::candidates(db.conn(), inbox_id, None, None)?;
        crate::ai::rounds::reserve(db, run_id, &scopes)
    })?;
    if tickets.is_empty() {
        return Ok(run_id);
    }
    if let Err(error) = refresh_evidence(None).await {
        mark_analysis_failed(state, run_id, "scan_failed", &error);
        return Err(format!("分析运行 #{run_id} 失败：{error}"));
    }
    let snapshot = match with_db(state, |db| {
        let mut snapshot = crate::ai::analysis_snapshot::build_for_round(
            db,
            "global_analysis",
            &date,
            period_start,
            period_end,
            today_start,
            today_end,
            "zh-CN",
            &tickets,
        )?;
        if let Some(id) = inbox_id {
            let item = crate::db::inbox::InboxRepo::new(db.conn())
                .get(id)?
                .ok_or_else(|| crate::db::DbError::NotFound("inbox".into()))?;
            snapshot.focused_inbox = Some(serde_json::to_value(&item).unwrap_or_default());
            if let Some(context) = crate::db::flow::capture_context(db.conn(), id)? {
                if let Some(focused) = snapshot.focused_inbox.as_mut() {
                    focused["capture_context"] = context;
                }
            }
            snapshot
                .source_refs
                .push(crate::ai::analysis_snapshot::AnalysisSourceRef {
                    source_type: "inbox".into(),
                    entity_id: Some(id),
                    workspace_id: None,
                    relative_path: None,
                    content_hash: None,
                    timestamp: Some(item.created_at),
                });
            snapshot.snapshot_hash =
                crate::cognition::digest(&serde_json::to_string(&snapshot).unwrap_or_default());
        }
        Ok(snapshot)
    }) {
        Ok(value) => value,
        Err(error) => {
            mark_analysis_failed(state, run_id, "snapshot_failed", &error);
            return Err(format!("分析运行 #{run_id} 失败：{error}"));
        }
    };

    let resolved = match with_db(state, |db| {
        let repo = crate::db::provider::ProviderCatalogRepo::new(db.conn());
        crate::ai::router::resolve(
            &repo,
            &crate::ai::router::KeyringCredentialSource,
            "global_analysis",
        )
        .map_err(|error| crate::db::DbError::Migration(error.to_string()))
    }) {
        Ok(value) => value,
        Err(error) => {
            mark_analysis_failed(state, run_id, "route_unavailable", &error);
            return Err(format!("分析运行 #{run_id} 失败：{error}"));
        }
    };
    if let Err(error) = with_db(state, |db| {
        crate::ai::analysis::record_context(db, run_id, resolved.model.id, &snapshot)
    }) {
        mark_analysis_failed(state, run_id, "context_write_failed", &error);
        return Err(format!("分析运行 #{run_id} 失败：{error}"));
    }

    let fingerprint = crate::ai::efficiency::reuse_fingerprint(
        &snapshot,
        &serde_json::json!({"connection":resolved.connection,"model":resolved.model}),
        period_end,
    );
    if inbox_id.is_none() && crate::ai::efficiency::complete_coverage(&snapshot) {
        if let Some(original) = with_db(state, |db| {
            crate::ai::efficiency::reusable_run(db, &fingerprint, trigger)
        })
        .unwrap_or(None)
        {
            with_db(state, |db| {
                crate::ai::analysis::finish_run(
                    db,
                    run_id,
                    "reused",
                    Some("已核查，无新变化；沿用已有建议"),
                    None,
                )?;
                let _ = crate::ai::efficiency::record_check(db, true, 0);
                Ok(())
            })?;
            return Ok(original);
        }
    }
    let key = match crate::ai::provider::get_api_key(&resolved.connection.credential_ref) {
        Ok(Some(value)) => value,
        Ok(None) => {
            let error = "全局分析模型未配置 API Key";
            mark_analysis_failed(state, run_id, "missing_api_key", error);
            return Err(format!("分析运行 #{run_id} 失败：{error}"));
        }
        Err(error) => {
            let message = error.to_string();
            mark_analysis_failed(state, run_id, "credential_failed", &message);
            return Err(format!("分析运行 #{run_id} 失败：{message}"));
        }
    };
    let request = crate::ai::analysis::build_request(&resolved.model.model_id, &snapshot);
    let original_chars = serde_json::to_string(&snapshot)
        .map(|s| s.chars().count())
        .unwrap_or(0);
    let compact_chars = request.messages[0].content.chars().count();
    let _ = with_db(state, |db| {
        crate::ai::efficiency::record_check(db, false, original_chars.saturating_sub(compact_chars))
    });
    let response = match crate::ai::analysis::complete_validated(
        &resolved.connection,
        &resolved.model,
        &key,
        &request,
        &snapshot,
    )
    .await
    {
        Ok(value) => value,
        Err(error) => {
            let message = error.to_string();
            mark_analysis_failed(state, run_id, "provider_failed", &message);
            return Err(format!("分析运行 #{run_id} 失败：{message}"));
        }
    };
    match state.with_database(|db| {
        crate::ai::analysis::apply_output(db, run_id, &snapshot, &response.content)
    }) {
        Some(Ok(_)) => {
            if inbox_id.is_none() {
                let _ = with_db(state, |db| {
                    crate::ai::efficiency::remember_success(db, &fingerprint, run_id)
                });
            }
            Ok(run_id)
        }
        Some(Err(error)) => Err(format!("分析运行 #{run_id} 失败：{error}")),
        None => Err("数据库未初始化".into()),
    }
}

#[tauri::command]
pub fn list_ai_proposals(
    state: State<AppState>,
    status: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<crate::db::ai::AiProposal>, String> {
    with_db(&state, |db| {
        crate::db::ai::ProposalRepo::new(db.conn()).list(status.as_deref(), limit.unwrap_or(50))
    })
}

#[tauri::command]
pub fn list_latest_analysis_proposals(
    state: State<AppState>,
    limit: Option<usize>,
) -> Result<Vec<crate::db::ai::AiProposal>, String> {
    with_db(&state, |db| {
        crate::db::ai::ProposalRepo::new(db.conn()).list_latest_run_pending(limit.unwrap_or(20))
    })
}

#[tauri::command]
pub fn list_recent_ai_proposals(
    state: State<AppState>,
    cutoff_created_at: i64,
    status: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<crate::db::ai::AiProposal>, String> {
    with_db(&state, |db| {
        crate::db::ai::ProposalRepo::new(db.conn()).list_since(
            cutoff_created_at,
            status.as_deref(),
            limit.unwrap_or(200),
        )
    })
}

#[tauri::command]
pub fn get_classification_memory_stats(
    state: State<AppState>,
) -> Result<crate::db::memory::ClassificationMemoryStats, String> {
    with_db(&state, |db| crate::db::memory::stats(db.conn()))
}

#[tauri::command]
pub fn update_ai_proposal_draft(
    state: State<AppState>,
    id: i64,
    expected_updated_at: i64,
    title: String,
    payload: Value,
) -> Result<crate::db::ai::AiProposal, String> {
    with_db(&state, |db| {
        crate::db::ai::ProposalRepo::new(db.conn()).update_draft(
            id,
            expected_updated_at,
            &title,
            &payload.to_string(),
        )
    })
}

#[tauri::command]
pub fn update_ai_proposal_classification(
    state: State<AppState>,
    id: i64,
    expected_updated_at: i64,
    kind: String,
    work_id: Option<i64>,
    title: String,
    payload: Value,
) -> Result<crate::db::ai::AiProposal, String> {
    with_db(&state, |db| {
        crate::db::ai::ProposalRepo::new(db.conn()).update_classification(
            id,
            expected_updated_at,
            &kind,
            work_id,
            &title,
            &payload.to_string(),
        )
    })
}

#[tauri::command]
pub fn defer_ai_proposal(
    state: State<AppState>,
    id: i64,
    expected_updated_at: i64,
) -> Result<crate::db::ai::AiProposal, String> {
    with_db(&state, |db| {
        crate::db::ai::ProposalRepo::new(db.conn()).defer(id, expected_updated_at)
    })
}

#[tauri::command]
pub fn delete_ai_proposal(
    state: State<AppState>,
    id: i64,
    expected_updated_at: i64,
) -> Result<(), String> {
    with_db(&state, |db| {
        crate::ai::lifecycle::delete(db, id, expected_updated_at)
    })
}

#[tauri::command]
pub fn resolve_ai_proposal(
    state: State<AppState>,
    id: i64,
    expected_updated_at: i64,
) -> Result<crate::db::ai::AiProposal, String> {
    with_db(&state, |db| {
        crate::ai::lifecycle::resolve(db, id, expected_updated_at)
    })
}

#[tauri::command]
pub fn confirm_ai_proposal(
    state: State<AppState>,
    id: i64,
    expected_updated_at: i64,
    edited_payload: Option<Value>,
) -> Result<crate::ai::apply::ApplyResult, String> {
    with_db(&state, |db| {
        crate::ai::apply::confirm_proposal(db, id, expected_updated_at, edited_payload)
    })
}

#[tauri::command]
pub fn reject_ai_proposal(
    state: State<AppState>,
    id: i64,
    expected_updated_at: i64,
    reason: Option<String>,
) -> Result<(), String> {
    with_db(&state, |db| {
        crate::ai::apply::reject_proposal(db, id, expected_updated_at, reason.as_deref())
    })
}

#[tauri::command]
pub async fn start_workspace_work_draft(
    state: State<'_, AppState>,
    workspace_id: Option<i64>,
    work_id: Option<i64>,
) -> Result<i64, String> {
    use chrono::Timelike;

    with_db(&state, |db| {
        if let Some(workspace_id) = workspace_id {
            crate::db::workspace::WorkspaceRepo::new(db.conn())
                .get(workspace_id)?
                .ok_or_else(|| crate::db::DbError::NotFound("workspace".into()))?;
        } else if work_id.is_none() {
            return Err(crate::db::DbError::Migration("请选择项目或工作目录".into()));
        }
        if let Some(work_id) = work_id {
            crate::db::work::WorkRepo::new(db.conn())
                .get(work_id)?
                .filter(|work| work.status != "archived")
                .ok_or_else(|| crate::db::DbError::NotFound("active work".into()))?;
        }
        Ok(())
    })?;

    let now = chrono::Local::now();
    let period_end = now.timestamp();
    let period_start = period_end.saturating_sub(30 * 24 * 60 * 60);
    let today_start = period_end.saturating_sub(i64::from(now.num_seconds_from_midnight()));
    let today_end = today_start.saturating_add(24 * 60 * 60);
    let date = now.format("%Y-%m-%d").to_string();
    let run_id = with_db(&state, |db| {
        Ok(crate::db::ai::AnalysisRunRepo::new(db.conn())
            .create("workspace_import", Some(period_start), Some(period_end))?
            .id)
    })?;

    let workspace_ids = with_db(&state, |db| {
        let mut ids = if let Some(id) = work_id {
            crate::db::documents::WorkWorkspaceLinkRepo::new(db.conn()).list_by_work(id)?
        } else {
            Vec::new()
        };
        if let Some(id) = workspace_id {
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        Ok(ids)
    })?;
    let tickets = with_db(&state, |db| {
        let scopes = crate::ai::rounds::candidates(db.conn(), None, work_id, workspace_id)?;
        crate::ai::rounds::reserve(db, run_id, &scopes)
    })?;
    if tickets.is_empty() {
        return Ok(run_id);
    }
    if let Err(error) = refresh_evidence(Some(workspace_ids.clone())).await {
        mark_analysis_failed(&state, run_id, "scan_failed", &error);
        return Err(format!("项目整理 #{run_id} 失败：{error}"));
    }
    let snapshot = match with_db(&state, |db| {
        let eligible_ids = crate::ai::rounds::eligible_workspaces(db, &tickets)?;
        let selected_ids = workspace_ids
            .iter()
            .copied()
            .filter(|id| eligible_ids.contains(id))
            .collect::<Vec<_>>();
        let snapshot = crate::ai::analysis_snapshot::build_scoped(
            db,
            "work_draft",
            &date,
            period_start,
            period_end,
            today_start,
            today_end,
            "zh-CN",
            Some(&selected_ids),
        )?;
        let snapshot = if let Some(id) = work_id {
            crate::ai::analysis_snapshot::focus_work(db, snapshot, id)?
        } else {
            snapshot
        };
        crate::ai::rounds::restrict(db, snapshot, tickets.clone())
    }) {
        Ok(value) => value,
        Err(error) => {
            mark_analysis_failed(&state, run_id, "snapshot_failed", &error);
            return Err(format!("项目整理 #{run_id} 失败：{error}"));
        }
    };
    let resolved = match with_db(&state, |db| {
        let repo = crate::db::provider::ProviderCatalogRepo::new(db.conn());
        crate::ai::router::resolve(
            &repo,
            &crate::ai::router::KeyringCredentialSource,
            "work_draft",
        )
        .map_err(|error| crate::db::DbError::Migration(error.to_string()))
    }) {
        Ok(value) => value,
        Err(error) => {
            mark_analysis_failed(&state, run_id, "route_unavailable", &error);
            return Err(format!("项目整理 #{run_id} 失败：{error}"));
        }
    };
    if let Err(error) = with_db(&state, |db| {
        crate::ai::analysis::record_context(db, run_id, resolved.model.id, &snapshot)
    }) {
        mark_analysis_failed(&state, run_id, "context_write_failed", &error);
        return Err(format!("项目整理 #{run_id} 失败：{error}"));
    }
    let key = match crate::ai::provider::get_api_key(&resolved.connection.credential_ref) {
        Ok(Some(value)) => value,
        Ok(None) => {
            let error = "项目整理模型未配置 API Key";
            mark_analysis_failed(&state, run_id, "missing_api_key", error);
            return Err(format!("项目整理 #{run_id} 失败：{error}"));
        }
        Err(error) => {
            let message = error.to_string();
            mark_analysis_failed(&state, run_id, "credential_failed", &message);
            return Err(format!("项目整理 #{run_id} 失败：{message}"));
        }
    };
    let request = crate::ai::analysis::build_workspace_intake_request(
        &resolved.model.model_id,
        &snapshot,
        work_id,
    );
    let response = match crate::ai::analysis::complete_validated(
        &resolved.connection,
        &resolved.model,
        &key,
        &request,
        &snapshot,
    )
    .await
    {
        Ok(value) => value,
        Err(error) => {
            let message = error.to_string();
            mark_analysis_failed(&state, run_id, "provider_failed", &message);
            return Err(format!("项目整理 #{run_id} 失败：{message}"));
        }
    };
    match state.with_database(|db| {
        crate::ai::analysis::apply_output(db, run_id, &snapshot, &response.content)
    }) {
        Some(Ok(_)) => Ok(run_id),
        Some(Err(error)) => Err(format!("项目整理 #{run_id} 失败：{error}")),
        None => Err("数据库未初始化".into()),
    }
}

#[tauri::command]
pub fn get_analysis_schedule(
    state: State<AppState>,
) -> Result<crate::db::ai::AnalysisSchedule, String> {
    with_db(&state, |db| {
        crate::db::ai::ScheduleRepo::new(db.conn()).get()
    })
}

#[tauri::command]
pub fn save_analysis_schedule(
    state: State<AppState>,
    schedule: crate::db::ai::AnalysisSchedule,
) -> Result<crate::db::ai::AnalysisSchedule, String> {
    with_db(&state, |db| {
        crate::db::ai::ScheduleRepo::new(db.conn()).save(&schedule)?;
        crate::db::ai::ScheduleRepo::new(db.conn()).get()
    })
}

#[tauri::command]
pub async fn run_analysis_now(
    state: State<'_, AppState>,
    trigger: Option<String>,
) -> Result<i64, String> {
    execute_global_analysis(&state, trigger.as_deref().unwrap_or("manual")).await
}

#[tauri::command]
pub fn list_analysis_runs(
    state: State<AppState>,
    limit: Option<usize>,
) -> Result<Vec<crate::db::ai::AnalysisRun>, String> {
    with_db(&state, |db| {
        let repo = crate::db::ai::AnalysisRunRepo::new(db.conn());
        repo.expire_stale_running(crate::db::now_unix().saturating_sub(10 * 60))?;
        repo.list(limit.unwrap_or(50))
    })
}

#[tauri::command]
pub async fn retry_analysis_run(state: State<'_, AppState>, run_id: i64) -> Result<i64, String> {
    with_db(&state, |db| {
        crate::db::ai::AnalysisRunRepo::new(db.conn())
            .get(run_id)?
            .ok_or_else(|| crate::db::DbError::NotFound("analysis_run".into()))?;
        Ok(())
    })?;
    execute_global_analysis(&state, "retry").await
}

#[tauri::command]
pub fn keep_daily_brief(state: State<AppState>, id: i64) -> Result<(), String> {
    with_db(&state, |db| {
        crate::db::brief::BriefRepo::new(db.conn()).keep(id)
    })
}

#[tauri::command]
pub async fn translate_text(
    state: State<'_, AppState>,
    input: String,
    style: String,
) -> Result<String, String> {
    if input.trim().is_empty() {
        return Err("翻译内容不能为空".into());
    }
    if input.chars().count() > 20_000 {
        return Err("翻译内容不能超过 20000 个字符".into());
    }
    let resolved = with_db(&state, |db| {
        let repo = crate::db::provider::ProviderCatalogRepo::new(db.conn());
        crate::ai::router::resolve(
            &repo,
            &crate::ai::router::KeyringCredentialSource,
            "translation",
        )
        .map_err(|error| crate::db::DbError::Migration(error.to_string()))
    })?;
    let key = crate::ai::provider::get_api_key(&resolved.connection.credential_ref)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "翻译模型未配置 API Key".to_string())?;
    let request = crate::ai::translation::build_request(&resolved.model.model_id, &input, &style)?;
    let direction = crate::ai::translation::detect_direction(&input);
    let response =
        crate::ai::provider::complete_model(&resolved.connection, &resolved.model, &key, &request)
            .await
            .map_err(|error| error.to_string())?;
    match crate::ai::translation::parse_output(&input, direction, &response.content) {
        Ok(value) => Ok(value),
        Err(first_error) => {
            let mut repair = request;
            repair.system.get_or_insert_default().push_str(
                "<format_repair>The previous response failed validation. Return one msl.translation.v1 JSON object with the complete translation and correct language direction. Do not add conversation or explanation.</format_repair>",
            );
            let repaired = crate::ai::provider::complete_model(
                &resolved.connection,
                &resolved.model,
                &key,
                &repair,
            )
            .await
            .map_err(|error| error.to_string())?;
            crate::ai::translation::parse_output(&input, direction, &repaired.content)
                .map_err(|_| format!("翻译输出格式无法验证：{first_error}"))
        }
    }
}

#[tauri::command]
pub fn get_storage_usage(
    state: State<AppState>,
) -> Result<crate::storage::usage::StorageUsage, String> {
    with_db(&state, |db| crate::storage::usage::get_usage(db))
}

#[tauri::command]
pub fn preview_storage_cleanup(
    state: State<AppState>,
    categories: Option<Vec<String>>,
) -> Result<crate::storage::cleanup::CleanupPlan, String> {
    with_db(&state, |db| {
        crate::storage::cleanup::preview(db, &categories.unwrap_or_default())
    })
}

#[tauri::command]
pub fn execute_storage_cleanup(
    state: State<AppState>,
    plan_id: String,
) -> Result<crate::storage::cleanup::CleanupResult, String> {
    with_db(&state, |db| crate::storage::cleanup::execute(db, &plan_id))
}

#[tauri::command]
pub fn compact_storage_history(state: State<AppState>) -> Result<(), String> {
    with_db(&state, |db| {
        crate::storage::cleanup::compact_history(db, crate::db::now_unix())
    })
}

#[tauri::command]
pub fn clear_webview_data(app: tauri::AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "主窗口不存在".to_string())?;
    window
        .clear_all_browsing_data()
        .map_err(|e| format!("WebView 数据清理失败：{e}"))
}

#[tauri::command]
pub fn get_ai_efficiency_stats(
    state: State<AppState>,
) -> Result<crate::ai::efficiency::EfficiencyStats, String> {
    with_db(&state, crate::ai::efficiency::stats)
}
