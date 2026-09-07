use crate::{app_state::AppState, db::jobs::AiJob};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{Emitter, Manager, State};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(
    tag = "command",
    content = "args",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum JobRequest {
    AskWorkbench {
        turn_id: i64,
        locale: Option<String>,
    },
    AnalyzeKol {
        expert_id: Option<i64>,
        purpose: String,
        locale: Option<String>,
    },
    RefreshProjectCognition {
        scope: String,
        scope_id: Option<i64>,
    },
    OrganizeInboxItem {
        inbox_id: i64,
    },
    RunAnalysisNow {
        trigger: Option<String>,
    },
    StartWorkspaceWorkDraft {
        workspace_id: Option<i64>,
        work_id: Option<i64>,
    },
    RetryAnalysisRun {
        run_id: i64,
    },
    TranslateText {
        input: String,
        style: String,
    },
    GenerateReport {
        kind: String,
        period_start: Option<i64>,
        period_end: Option<i64>,
    },
    RetryReport {
        report_id: i64,
    },
    GenerateBrief {
        date: String,
        period_start: i64,
        period_end: i64,
        today_start: i64,
        today_end: i64,
        locale: Option<String>,
        force: Option<bool>,
    },
}
impl JobRequest {
    async fn execute(self, app: &tauri::AppHandle) -> Result<Value, String> {
        let s = app.state::<AppState>();
        match self {
            Self::AskWorkbench { turn_id, locale } => {
                super::knowledge::ask_workbench(s, turn_id, locale).await
            }
            Self::AnalyzeKol {
                expert_id,
                purpose,
                locale,
            } => super::knowledge::analyze_kol(s, expert_id, purpose, locale).await,
            Self::RefreshProjectCognition { scope, scope_id } => {
                super::flow::refresh_project_cognition(s, scope, scope_id)
                    .await
                    .and_then(|v| serde_json::to_value(v).map_err(|e| e.to_string()))
            }
            Self::OrganizeInboxItem { inbox_id } => super::flow::organize_inbox_item(s, inbox_id)
                .await
                .map(Value::from),
            Self::RunAnalysisNow { trigger } => super::ai_secretary::run_analysis_now(s, trigger)
                .await
                .map(Value::from),
            Self::StartWorkspaceWorkDraft {
                workspace_id,
                work_id,
            } => super::ai_secretary::start_workspace_work_draft(s, workspace_id, work_id)
                .await
                .map(Value::from),
            Self::RetryAnalysisRun { run_id } => super::ai_secretary::retry_analysis_run(s, run_id)
                .await
                .map(Value::from),
            Self::TranslateText { input, style } => {
                super::ai_secretary::translate_text(s, input, style)
                    .await
                    .map(Value::from)
            }
            Self::GenerateReport {
                kind,
                period_start,
                period_end,
            } => super::reports::generate_report(s, kind, period_start, period_end)
                .await
                .map(Value::from),
            Self::RetryReport { report_id } => super::reports::retry_report(s, report_id)
                .await
                .map(Value::from),
            Self::GenerateBrief {
                date,
                period_start,
                period_end,
                today_start,
                today_end,
                locale,
                force,
            } => super::generate_brief(
                s,
                date,
                period_start,
                period_end,
                today_start,
                today_end,
                locale,
                force,
            )
            .await
            .and_then(|v| serde_json::to_value(v).map_err(|e| e.to_string())),
        }
    }
}
#[tauri::command]
pub fn start_ai_job(
    app: tauri::AppHandle,
    state: State<AppState>,
    request: JobRequest,
) -> Result<AiJob, String> {
    let value = serde_json::to_value(&request).map_err(|e| e.to_string())?;
    let command = value["command"].as_str().ok_or("任务类型无效")?;
    let (job, created) = super::with_db(&state, |db| {
        crate::db::jobs::start(db.conn(), command, &value["args"])
    })?;
    if created {
        let id = job.id;
        tauri::async_runtime::spawn(async move {
            let result = request.execute(&app).await;
            let _ = super::with_db(&app.state::<AppState>(), |db| {
                crate::db::jobs::finish(db.conn(), id, result)
            });
            let _ = app.emit("ai-job-finished", id);
        });
    }
    Ok(job)
}
#[tauri::command]
pub fn list_ai_jobs(state: State<AppState>) -> Result<Vec<AiJob>, String> {
    super::with_db(&state, |db| crate::db::jobs::list(db.conn()))
}
#[tauri::command]
pub fn get_ai_job(state: State<AppState>, id: i64) -> Result<AiJob, String> {
    super::with_db(&state, |db| crate::db::jobs::get(db.conn(), id))
}
