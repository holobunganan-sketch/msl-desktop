use super::with_db;
use crate::app_state::AppState;
use tauri::State;

#[tauri::command]
pub fn capture_work_note(
    state: State<AppState>,
    content: String,
    work_id: Option<i64>,
    entity_kind: Option<String>,
    entity_id: Option<i64>,
) -> Result<crate::db::inbox::InboxItem, String> {
    with_db(&state, |db| {
        crate::db::flow::capture(
            db.conn(),
            &content,
            work_id,
            entity_kind.as_deref(),
            entity_id,
        )
    })
}
#[tauri::command]
pub fn get_entity_location(
    state: State<AppState>,
    kind: String,
    id: i64,
) -> Result<serde_json::Value, String> {
    with_db(&state, |db| {
        crate::db::flow::entity_location(db.conn(), &kind, id)
    })
}
#[tauri::command]
pub fn schedule_work_task(
    state: State<AppState>,
    id: i64,
    start: Option<i64>,
    end: Option<i64>,
) -> Result<(), String> {
    with_db(&state, |db| {
        crate::db::flow::schedule(db.conn(), id, start, end)
    })
}

#[tauri::command]
pub fn get_project_cognition(
    state: State<AppState>,
    scope: String,
    scope_id: Option<i64>,
) -> Result<crate::cognition::CognitionEntry, String> {
    with_db(&state, |db| crate::cognition::refresh(db, &scope, scope_id))
}

pub async fn refresh_project_cognition(
    state: State<'_, AppState>,
    scope: String,
    scope_id: Option<i64>,
) -> Result<crate::cognition::CognitionEntry, String> {
    let ids = with_db(&state, |db| {
        // Validate scope before scheduling a disk read.
        crate::cognition::preview(db, &scope, scope_id)?;
        Ok(if scope == "work" {
            Some(
                crate::db::documents::WorkWorkspaceLinkRepo::new(db.conn())
                    .list_by_work(scope_id.unwrap())?,
            )
        } else if scope == "workspace" {
            Some(vec![scope_id.unwrap()])
        } else {
            None
        })
    })?;
    super::ai_secretary::refresh_evidence(ids).await?;
    with_db(&state, |db| crate::cognition::refresh(db, &scope, scope_id))
}

#[tauri::command]
pub fn list_classification_memories(
    state: State<AppState>,
) -> Result<Vec<crate::db::memory::MemoryCard>, String> {
    with_db(&state, |db| crate::db::memory::list_cards(db.conn()))
}
#[tauri::command]
pub fn edit_classification_memory(
    state: State<AppState>,
    id: i64,
    expected_updated_at: i64,
    preferred_kind: Option<String>,
    work_id: Option<i64>,
) -> Result<(), String> {
    with_db(&state, |db| {
        crate::db::memory::edit_card(
            db.conn(),
            id,
            expected_updated_at,
            preferred_kind.as_deref(),
            work_id,
        )
    })
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectedProposal {
    id: i64,
    expected_updated_at: i64,
}
#[tauri::command]
pub fn confirm_ai_proposal_group(
    state: State<AppState>,
    items: Vec<SelectedProposal>,
) -> Result<crate::ai::receipts::ConfirmationGroup, String> {
    with_db(&state, |db| {
        crate::ai::receipts::confirm_batch(
            db,
            &items
                .into_iter()
                .map(|p| (p.id, p.expected_updated_at))
                .collect::<Vec<_>>(),
        )
    })
}
#[tauri::command]
pub fn list_confirmation_receipts(
    state: State<AppState>,
) -> Result<Vec<crate::ai::receipts::ReceiptCard>, String> {
    with_db(&state, crate::ai::receipts::list)
}
#[tauri::command]
pub fn undo_ai_confirmation(state: State<AppState>, receipt_id: String) -> Result<(), String> {
    with_db(&state, |db| crate::ai::receipts::undo(db, &receipt_id))
}

pub async fn organize_inbox_item(state: State<'_, AppState>, inbox_id: i64) -> Result<i64, String> {
    with_db(&state, |db| {
        crate::db::inbox::InboxRepo::new(db.conn())
            .get(inbox_id)?
            .filter(|item| item.processed_at.is_none())
            .ok_or_else(|| crate::db::DbError::Migration("收件箱事项已处理或不存在".into()))?;
        Ok(())
    })?;
    super::ai_secretary::execute_focused_analysis(&state, "inbox_capture", Some(inbox_id)).await
}
