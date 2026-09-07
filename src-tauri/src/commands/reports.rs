use super::with_db;
use crate::app_state::AppState;
use tauri::State;

fn mark_failed(state: &State<'_, AppState>, report_id: i64, code: &str, message: &str) {
    let _ = state.with_database(|db| {
        crate::db::reports::ReportRepo::new(db.conn()).fail(report_id, code, message)
    });
}

pub(crate) async fn execute_report(
    state: &State<'_, AppState>,
    kind: &str,
    period_start: i64,
    period_end: i64,
) -> Result<i64, String> {
    let report = with_db(state, |db| {
        crate::db::reports::ReportRepo::new(db.conn()).create(kind, period_start, period_end)
    })?;
    let report_id = report.id;
    let snapshot = match with_db(state, |db| {
        crate::ai::reports::build_report_snapshot(db, kind, period_start, period_end, "zh-CN")
    }) {
        Ok(value) => value,
        Err(error) => {
            mark_failed(state, report_id, "snapshot_failed", &error);
            return Err(format!("报告 #{report_id} 生成失败：{error}"));
        }
    };
    let task_kind = if kind == "weekly" {
        "weekly_report"
    } else {
        "monthly_report"
    };
    let resolved = match with_db(state, |db| {
        let repo = crate::db::provider::ProviderCatalogRepo::new(db.conn());
        let explicit = repo.get_route(task_kind)?;
        if explicit.and_then(|route| route.provider_model_id).is_none() {
            return Err(crate::db::DbError::Migration(format!(
                "任务 {task_kind} 未绑定专用 Provider 模型"
            )));
        }
        crate::ai::router::resolve(
            &repo,
            &crate::ai::router::KeyringCredentialSource,
            task_kind,
        )
        .map_err(|error| crate::db::DbError::Migration(error.to_string()))
    }) {
        Ok(value) => value,
        Err(error) => {
            mark_failed(state, report_id, "route_unavailable", &error);
            return Err(format!("报告 #{report_id} 生成失败：{error}"));
        }
    };
    let key = match crate::ai::provider::get_api_key(&resolved.connection.credential_ref) {
        Ok(Some(value)) => value,
        Ok(None) => {
            let message = format!("{task_kind} 模型未配置 API Key");
            mark_failed(state, report_id, "missing_api_key", &message);
            return Err(format!("报告 #{report_id} 生成失败：{message}"));
        }
        Err(error) => {
            let message = error.to_string();
            mark_failed(state, report_id, "credential_failed", &message);
            return Err(format!("报告 #{report_id} 生成失败：{message}"));
        }
    };
    let request =
        match crate::ai::reports::build_report_request(&resolved.model.model_id, &snapshot) {
            Ok(value) => value,
            Err(error) => {
                mark_failed(state, report_id, "request_failed", &error);
                return Err(format!("报告 #{report_id} 生成失败：{error}"));
            }
        };
    let content = match crate::ai::report_contract::complete_report(
        &resolved.connection,
        &resolved.model,
        &key,
        &request,
        &snapshot,
    )
    .await
    {
        Ok(content) => content,
        Err(error) => {
            mark_failed(
                state,
                report_id,
                "report_validation_or_provider_failed",
                &error,
            );
            return Err(format!("报告 #{report_id} 生成失败：{error}"));
        }
    };
    let source_counts =
        serde_json::to_string(&snapshot.source_counts).unwrap_or_else(|_| "{}".into());
    let source_ids = serde_json::to_string(
        &snapshot
            .weekly_reports
            .iter()
            .map(|item| item.report_id)
            .collect::<Vec<_>>(),
    )
    .unwrap_or_else(|_| "[]".into());
    with_db(state, |db| {
        crate::db::reports::ReportRepo::new(db.conn()).complete(
            report_id,
            resolved.model.id,
            &content,
            &snapshot.snapshot_hash,
            &source_counts,
            &source_ids,
        )?;
        Ok(())
    })?;
    Ok(report_id)
}

#[tauri::command]
pub fn list_reports(
    state: State<AppState>,
    kind: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<crate::db::reports::Report>, String> {
    with_db(&state, |db| {
        crate::db::reports::ReportRepo::new(db.conn()).list(kind.as_deref(), limit.unwrap_or(100))
    })
}

#[tauri::command]
pub async fn generate_report(
    state: State<'_, AppState>,
    kind: String,
    period_start: Option<i64>,
    period_end: Option<i64>,
) -> Result<i64, String> {
    let now = crate::db::now_unix();
    let (default_start, default_end) = if kind == "weekly" {
        crate::ai::reports::default_weekly_period(now)
    } else if kind == "monthly" {
        crate::ai::reports::default_monthly_period(now)?
    } else {
        return Err("报告类型无效".into());
    };
    let start = period_start.unwrap_or(default_start);
    let end = period_end.unwrap_or(default_end);
    if end <= start {
        return Err("报告结束时间必须晚于开始时间".into());
    }
    execute_report(&state, &kind, start, end).await
}

#[tauri::command]
pub async fn retry_report(state: State<'_, AppState>, report_id: i64) -> Result<i64, String> {
    let report = with_db(&state, |db| {
        crate::db::reports::ReportRepo::new(db.conn())
            .get(report_id)?
            .ok_or_else(|| crate::db::DbError::NotFound("report".into()))
    })?;
    execute_report(&state, &report.kind, report.period_start, report.period_end).await
}

#[tauri::command]
pub fn keep_report(state: State<AppState>, report_id: i64) -> Result<(), String> {
    with_db(&state, |db| {
        crate::db::reports::ReportRepo::new(db.conn()).keep(report_id)
    })
}

#[tauri::command]
pub fn clear_report_history(state: State<AppState>, kind: String) -> Result<usize, String> {
    with_db(&state, |db| {
        crate::db::reports::ReportRepo::new(db.conn()).clear_history(&kind)
    })
}

#[tauri::command]
pub fn get_report_schedule(
    state: State<AppState>,
) -> Result<crate::db::reports::ReportSchedule, String> {
    with_db(&state, |db| {
        crate::db::reports::ReportScheduleRepo::new(db.conn()).get()
    })
}

#[tauri::command]
pub fn save_report_schedule(
    state: State<AppState>,
    schedule: crate::db::reports::ReportSchedule,
) -> Result<crate::db::reports::ReportSchedule, String> {
    with_db(&state, |db| {
        let repo = crate::db::reports::ReportScheduleRepo::new(db.conn());
        repo.save(&schedule)?;
        repo.get()
    })
}
