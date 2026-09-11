use crate::sync::{
    protocol::{probe_directory, FolderProbe},
    rows::{SyncConflict, SyncSummary},
    service::{self, ConnectMode, ConnectResult, SyncState},
    settings::SyncConfig,
};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
pub struct SyncStatusResponse {
    pub config: SyncConfig,
    pub state: SyncState,
    pub running: bool,
}

#[derive(Serialize)]
pub struct SyncFolderProbe {
    pub kind: String,
    pub entries: usize,
    pub dataset_id: String,
    pub generation: String,
}

#[tauri::command]
pub fn sync_status() -> Result<SyncStatusResponse, String> {
    let data = crate::db::default_app_data_dir();
    let mut state = service::load_state(&data);
    if let Ok(database) = crate::db::Database::open(&data.join(crate::db::DB_FILE_NAME)) {
        state.conflicts = database
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sync_conflicts WHERE resolved_at IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap_or(state.conflicts);
    }
    Ok(SyncStatusResponse {
        config: service::load_config(&data).map_err(|error| error.to_string())?,
        state,
        running: service::RUNNING.load(std::sync::atomic::Ordering::Acquire),
    })
}

#[tauri::command]
pub fn probe_sync_folder(directory: String) -> Result<SyncFolderProbe, String> {
    let path = PathBuf::from(directory);
    crate::backup::service::validate_for_app(&crate::db::default_app_data_dir(), &path)?;
    let result = match probe_directory(&path).map_err(|error| error.to_string())? {
        FolderProbe::Empty => SyncFolderProbe {
            kind: "empty".into(),
            entries: 0,
            dataset_id: String::new(),
            generation: String::new(),
        },
        FolderProbe::Unrelated { entries } => SyncFolderProbe {
            kind: "unrelated".into(),
            entries,
            dataset_id: String::new(),
            generation: String::new(),
        },
        FolderProbe::Incomplete { .. } => SyncFolderProbe {
            kind: "incomplete".into(),
            entries: 0,
            dataset_id: String::new(),
            generation: String::new(),
        },
        FolderProbe::Existing { manifest, .. } => SyncFolderProbe {
            kind: "existing".into(),
            entries: 0,
            dataset_id: manifest.dataset_id,
            generation: manifest.generation,
        },
    };
    Ok(result)
}

#[tauri::command]
pub async fn connect_sync_folder(
    directory: String,
    mode: ConnectMode,
) -> Result<ConnectResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        service::connect(
            &crate::db::default_app_data_dir(),
            &PathBuf::from(directory),
            mode,
        )
        .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn save_sync_schedule(enabled: bool, interval_minutes: u32) -> Result<SyncConfig, String> {
    service::save_schedule(
        &crate::db::default_app_data_dir(),
        enabled,
        interval_minutes,
    )
    .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn run_sync_now() -> Result<SyncSummary, String> {
    let result = tauri::async_runtime::spawn_blocking(|| {
        service::run(&crate::db::default_app_data_dir()).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?;
    if let Err(error) = &result {
        service::record_failure(&crate::db::default_app_data_dir(), error);
    }
    result
}

#[tauri::command]
pub fn list_sync_conflicts() -> Result<Vec<SyncConflict>, String> {
    let database = crate::db::Database::open(&crate::db::default_db_path())
        .map_err(|error| error.to_string())?;
    crate::sync::rows::list_conflicts(database.conn()).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn resolve_sync_conflict(id: String, choice: String) -> Result<(), String> {
    let database = crate::db::Database::open(&crate::db::default_db_path())
        .map_err(|error| error.to_string())?;
    crate::sync::rows::resolve_conflict(database.conn(), &id, &choice)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn make_sync_ai_primary() -> Result<SyncConfig, String> {
    service::make_ai_primary(&crate::db::default_app_data_dir()).map_err(|error| error.to_string())
}
