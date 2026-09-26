//! IPC boundary for durable, evidence-grounded knowledge work.
use super::with_db;
use crate::{
    app_state::AppState,
    db::{self, knowledge::EvidencePack},
};
use serde_json::{json, Value};
use tauri::State;

#[tauri::command]
pub fn list_qa_sessions(state: State<AppState>) -> Result<Vec<Value>, String> {
    with_db(&state, db::qa::sessions)
}
#[tauri::command]
pub fn create_qa_session(
    state: State<AppState>,
    title: String,
    scope: Vec<i64>,
    expert_id: Option<i64>,
) -> Result<Value, String> {
    with_db(&state, |db| {
        db::qa::create_scoped(db, &title, &scope, expert_id)
    })
}
#[tauri::command]
pub fn list_qa_turns(state: State<AppState>, session_id: i64) -> Result<Vec<Value>, String> {
    with_db(&state, |db| db::qa::turns(db, session_id))
}
#[tauri::command]
pub fn queue_qa_question(
    state: State<AppState>,
    session_id: i64,
    question: String,
    scope: Vec<i64>,
    expert_id: Option<i64>,
) -> Result<Value, String> {
    with_db(&state, |db| {
        db::qa::queue_scoped(db, session_id, &question, &scope, expert_id)
    })
}
#[tauri::command]
pub fn delete_qa_session(state: State<AppState>, id: i64) -> Result<(), String> {
    with_db(&state, |db| db::qa::remove(db, id))
}
#[tauri::command]
pub fn list_kol_experts(state: State<AppState>) -> Result<Vec<Value>, String> {
    with_db(&state, db::kol::experts)
}
#[tauri::command]
pub async fn delete_kol_expert(
    id: i64,
    confirmation_name: String,
    expected_revision: i64,
) -> Result<Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let db = db::Database::open(&db::default_db_path()).map_err(|e| e.to_string())?;
        db::kol::delete_expert(&db, id, &confirmation_name, expected_revision)
            .map_err(|e| e.to_string())?;
        let pending = crate::materials::purge_unused(&db, &crate::materials::root())
            .map_err(|e| e.to_string())?;
        Ok(json!({"removed":true,"pending_cleanup":pending}))
    })
    .await
    .map_err(|_| "专家删除任务中断")?
}
#[tauri::command]
pub fn save_kol_expert(
    state: State<AppState>,
    id: Option<i64>,
    revision: Option<i64>,
    name: String,
    institution: String,
    department: Option<String>,
    specialty: String,
    projects: Vec<i64>,
    archived: bool,
) -> Result<Value, String> {
    with_db(&state, |db| {
        db::kol::save_expert(
            db,
            id,
            revision,
            &name,
            &institution,
            department.as_deref(),
            &specialty,
            &projects,
            archived,
        )
    })
}
#[tauri::command]
pub fn capture_kol_note(
    state: State<AppState>,
    expert_id: i64,
    work_id: Option<i64>,
    inbox_id: Option<i64>,
    content: String,
    occurred_at: i64,
) -> Result<Value, String> {
    with_db(&state, |db| {
        db::kol::capture(db, expert_id, work_id, inbox_id, &content, occurred_at)
    })
}
#[tauri::command]
pub fn list_kol_notes(
    state: State<AppState>,
    expert_id: Option<i64>,
) -> Result<Vec<Value>, String> {
    with_db(&state, |db| db::kol::notes(db, expert_id))
}
#[tauri::command]
pub fn list_kol_drafts(
    state: State<AppState>,
    expert_id: Option<i64>,
) -> Result<Vec<Value>, String> {
    with_db(&state, |db| db::kol::drafts(db, expert_id))
}
#[tauri::command]
pub fn list_kol_insights(
    state: State<AppState>,
    expert_id: Option<i64>,
) -> Result<Vec<Value>, String> {
    with_db(&state, |db| db::kol::insights(db, expert_id))
}
#[tauri::command]
pub fn list_kol_followups(
    state: State<AppState>,
    expert_id: Option<i64>,
) -> Result<Vec<Value>, String> {
    with_db(&state, |db| db::kol::followups(db, expert_id))
}
#[tauri::command]
pub fn review_kol_draft(
    state: State<AppState>,
    id: i64,
    revision: i64,
    decision: String,
    payload: Value,
) -> Result<Value, String> {
    with_db(&state, |db| {
        db::kol::review(db, id, revision, &decision, &payload.to_string())
    })
}
#[tauri::command]
pub fn review_kol_insight(
    state: State<AppState>,
    id: i64,
    status: String,
    note: String,
) -> Result<(), String> {
    with_db(&state, |db| {
        db::kol::set_insight_status(db, id, &status, &note)
    })
}

async fn model_output(
    state: &State<'_, AppState>,
    kind: &str,
    spec: &str,
    input: Value,
    pack: &EvidencePack,
    kol: bool,
) -> Result<String, String> {
    let resolved = with_db(state, |db| {
        let repo = db::provider::ProviderCatalogRepo::new(db.conn());
        crate::ai::router::resolve(&repo, &crate::ai::router::KeyringCredentialSource, kind)
            .map_err(|e| db::DbError::Migration(e.to_string()))
    })?;
    // Test mode never consults the real credential manager, even with bad routing.
    let key = if std::env::var("MSL_ISOLATED_TEST").as_deref() == Ok("1") {
        "synthetic-knowledge-key".to_string()
    } else if resolved.connection.auth_mode == "none" {
        String::new()
    } else {
        crate::ai::provider::get_api_key(&resolved.connection.credential_ref)
            .map_err(|e| e.to_string())?
            .ok_or("请在模型设置中填写 API Key")?
    };
    let mut request = crate::ai::provider::AiTextRequest {
        model_id: resolved.model.model_id.clone(),
        system: Some(spec.into()),
        messages: vec![crate::ai::provider::AiMessage {
            role: "user".into(),
            content: input.to_string(),
        }],
        temperature: Some(0.1),
        max_output_tokens: Some(8000),
        output_format: crate::ai::output::OutputFormat::PromptJson,
        budget: Default::default(),
    };
    for attempt in 0..2 {
        let response = crate::ai::provider::complete_model(
            &resolved.connection,
            &resolved.model,
            &key,
            &request,
        )
        .await
        .map_err(|e| e.to_string())?;
        let validated = if kol {
            crate::ai::knowledge_contract::parse_kol(&response.content, pack).and_then(|output| {
                if input["purpose"] == "prepare" && !output.actions.is_empty() {
                    return Err("会前准备的 actions 必须为空".into());
                }
                if output.actions.iter().filter_map(|a| a.work_id).any(|id| {
                    !pack
                        .sources
                        .iter()
                        .any(|s| s.kind == "work" && s.entity_id == id)
                }) {
                    return Err("后续动作引用了未提供的项目".into());
                }
                Ok(())
            })
        } else {
            crate::ai::knowledge_contract::parse_answer(&response.content, pack).map(|_| ())
        };
        match validated {
            Ok(()) => return Ok(response.content),
            Err(error) if attempt == 0 => {
                request.system=Some(format!("{spec}\nYour previous response failed this check: {error}. Reconstruct from supplied evidence. Return only valid JSON with real source IDs and exact quotes. Do not reproduce unsupported claims."));
            }
            Err(error) => {
                return Err(format!(
                    "来源或结构校验未通过：{error}。已进行一次校正，可修改问题后重试。"
                ))
            }
        }
    }
    Err("问答未完成".into())
}

pub async fn ask_workbench(
    state: State<'_, AppState>,
    turn_id: i64,
    locale: Option<String>,
) -> Result<Value, String> {
    let existing = with_db(&state, |db| db::qa::get(db, turn_id))?;
    if existing["status"] == "completed" {
        return Ok(existing);
    }
    let turn = with_db(&state, |db| db::qa::claim(db, turn_id))?;
    let result=async {
        let scope:Vec<i64>=serde_json::from_str(turn["scope_json"].as_str().ok_or("问答范围损坏")?).map_err(|_|"问答范围损坏")?;
        let history=with_db(&state,|db|db::qa::history(db,&turn))?;
        let question=turn["question"].as_str().ok_or("问题不存在")?.to_string();
        let retrieval_query=format!("{} {}",history.last().and_then(|h|h["question"].as_str()).unwrap_or(""),question);
        let expert=turn["expert_id"].as_i64();let expert_scoped=turn["expert_scoped"]==1;
        let pack=tauri::async_runtime::spawn_blocking(move ||{
            let db=db::Database::open(&db::default_db_path()).map_err(|e|e.to_string())?;
            db::knowledge::collect_scoped(&db,&scope,&retrieval_query,expert,expert_scoped).map_err(|e|e.to_string())
        }).await.map_err(|_|"证据检索任务中断")??;
        with_db(&state,|db|db::qa::save_evidence(db,turn_id,&pack))?;
        let input=json!({"question":question,"locale":locale.unwrap_or_else(||"zh-CN".into()),"local_time":chrono::Local::now().to_rfc3339(),"history_context_only":history,"evidence":pack});
        let raw=model_output(&state,"workbench_qa",include_str!("../ai/qa-spec.md"),input,&pack,false).await?;
        crate::ai::knowledge_contract::parse_answer(&raw,&pack)
    }.await;
    with_db(&state, |db| {
        db::qa::finish(db, turn_id, result.as_ref().map_err(|s| s.as_str()))
    })?;
    result?;
    with_db(&state, |db| db::qa::get(db, turn_id))
}
pub async fn analyze_kol(
    state: State<'_, AppState>,
    expert_id: Option<i64>,
    purpose: String,
    locale: Option<String>,
) -> Result<Value, String> {
    if !["organize", "prepare", "synthesize"].contains(&purpose.as_str()) {
        return Err("整理方式无效".into());
    }
    let pack = with_db(&state, |db| db::kol::evidence_pack(db, expert_id))?;
    let input = json!({"expert_id":expert_id,"purpose":purpose,"locale":locale.unwrap_or_else(||"zh-CN".into()),"local_time":chrono::Local::now().to_rfc3339(),"evidence":pack});
    let raw = model_output(
        &state,
        "kol_analysis",
        include_str!("../ai/kol-spec.md"),
        input,
        &pack,
        true,
    )
    .await?;
    let output = crate::ai::knowledge_contract::parse_kol(&raw, &pack)?;
    let draft_id = with_db(&state, |db| {
        db::kol::insert_draft(db, expert_id, &purpose, &output, &pack)
    })?;
    Ok(json!({"draft_id":draft_id,"expert_id":expert_id}))
}
