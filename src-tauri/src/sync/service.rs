use super::{
    protocol::{probe_directory, protocol_root, FolderProbe, WorkspaceManifest, MANIFEST_NAME},
    rows::{publish_state, receive_states, SyncSummary},
    settings::SyncConfig,
    SyncError, SyncResult,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

pub static RUNNING: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectMode {
    InitializeFromLocal,
    UseFolder,
    ReplaceWithLocal,
    JoinExisting,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncState {
    pub phase: String,
    pub last_attempt: i64,
    pub last_success: i64,
    pub last_error: String,
    pub last_uploaded: u64,
    pub last_applied: u64,
    pub conflicts: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConnectResult {
    pub phase: String,
    pub dataset_id: String,
    pub generation: String,
    pub summary: SyncSummary,
}

fn config_path(data: &Path) -> PathBuf {
    data.join("sync-settings.json")
}

fn state_path(data: &Path) -> PathBuf {
    data.join("sync-state.json")
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> SyncResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| SyncError::new("SYNC_PATH_INVALID", "同步设置路径无效"))?;
    fs::create_dir_all(parent)
        .map_err(|error| SyncError::new("SYNC_SETTINGS_WRITE", error.to_string()))?;
    let temporary = parent.join(format!(".sync-{}.tmp", uuid::Uuid::new_v4()));
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| SyncError::new("SYNC_SERIALIZE", error.to_string()))?;
    fs::write(&temporary, bytes)
        .map_err(|error| SyncError::new("SYNC_SETTINGS_WRITE", error.to_string()))?;
    fs::rename(&temporary, path)
        .map_err(|error| SyncError::new("SYNC_SETTINGS_WRITE", error.to_string()))?;
    Ok(())
}

pub fn load_config(data: &Path) -> SyncResult<SyncConfig> {
    let path = config_path(data);
    if !path.exists() {
        return Ok(SyncConfig::default());
    }
    let config: SyncConfig = serde_json::from_slice(
        &fs::read(path).map_err(|error| SyncError::new("SYNC_SETTINGS_READ", error.to_string()))?,
    )
    .map_err(|_| SyncError::new("SYNC_SETTINGS_INVALID", "同步设置无法读取"))?;
    config.validate()?;
    Ok(config)
}

pub fn load_state(data: &Path) -> SyncState {
    fs::read(state_path(data))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub fn save_schedule(data: &Path, enabled: bool, interval_minutes: u32) -> SyncResult<SyncConfig> {
    let mut config = load_config(data)?;
    config.enabled = enabled;
    config.interval_minutes = interval_minutes;
    config.validate()?;
    write_json(&config_path(data), &config)?;
    Ok(config)
}

pub fn make_ai_primary(data: &Path) -> SyncResult<SyncConfig> {
    let mut config = load_config(data)?;
    if config.directory.is_empty() || config.dataset_id.is_empty() || config.generation.is_empty() {
        return Err(SyncError::new("SYNC_NOT_CONNECTED", "请先连接同步文件夹"));
    }
    let folder = PathBuf::from(&config.directory);
    crate::backup::service::validate_for_app(data, &folder)
        .map_err(|message| SyncError::new("SYNC_DIRECTORY_UNSAFE", message))?;
    let mut manifest = match probe_directory(&folder)? {
        FolderProbe::Existing { manifest, .. } => manifest,
        _ => {
            return Err(SyncError::new(
                "SYNC_FOLDER_UNAVAILABLE",
                "同步目录当前不可用",
            ))
        }
    };
    if manifest.dataset_id != config.dataset_id || manifest.generation != config.generation {
        return Err(SyncError::new(
            "SYNC_GENERATION_CHANGED",
            "同步数据版本已经变化，请重新连接",
        ));
    }
    manifest.ai_primary_device = config.device_id.clone();
    write_manifest(&protocol_root(&folder), &manifest)?;
    config.ai_primary = true;
    write_json(&config_path(data), &config)?;
    Ok(config)
}

pub fn allows_automatic_ai(data: &Path) -> bool {
    match load_config(data) {
        Ok(config) if !config.dataset_id.is_empty() => {
            // A cached flag can remain true after another machine claims ownership.
            // Re-read the locally available manifest at every scheduling boundary.
            matches!(probe_directory(Path::new(&config.directory)),Ok(FolderProbe::Existing{manifest,..})
                if manifest.dataset_id==config.dataset_id && manifest.generation==config.generation && manifest.ai_primary_device==config.device_id)
        }
        Ok(_) => true,
        Err(_) => false,
    }
}

fn set_database_identity(conn: &rusqlite::Connection, config: &SyncConfig) -> SyncResult<()> {
    conn.execute(
        "UPDATE sync_local_state SET device_id=?1,dataset_id=?2,generation=?3,updated_at=strftime('%s','now') WHERE id=1",
        rusqlite::params![config.device_id, config.dataset_id, config.generation],
    )
    .map_err(|error| SyncError::new("SYNC_DB_WRITE", error.to_string()))?;
    conn.execute(
        "INSERT INTO sync_devices(device_id,display_name,active,first_seen_at,last_seen_at) VALUES(?1,?2,1,strftime('%s','now'),strftime('%s','now')) ON CONFLICT(device_id) DO UPDATE SET active=1,last_seen_at=excluded.last_seen_at",
        rusqlite::params![config.device_id, std::env::var("COMPUTERNAME").unwrap_or_else(|_| "Windows device".into())],
    )
    .map_err(|error| SyncError::new("SYNC_DB_WRITE", error.to_string()))?;
    Ok(())
}

fn write_manifest(root: &Path, manifest: &WorkspaceManifest) -> SyncResult<()> {
    fs::create_dir_all(root)
        .map_err(|error| SyncError::new("SYNC_DIRECTORY_WRITE", error.to_string()))?;
    write_json(&root.join(MANIFEST_NAME), manifest)
}

pub fn connect(data: &Path, folder: &Path, mode: ConnectMode) -> SyncResult<ConnectResult> {
    if RUNNING.swap(true, Ordering::AcqRel) {
        return Err(SyncError::new("SYNC_ALREADY_RUNNING", "同步正在后台进行"));
    }
    let _running = Running;
    crate::storage::paths::reject_link_components(folder)
        .map_err(|message| SyncError::new("SYNC_DIRECTORY_UNSAFE", message))?;
    crate::backup::service::validate_for_app(data, folder)
        .map_err(|message| SyncError::new("SYNC_DIRECTORY_UNSAFE", message))?;
    let probe = probe_directory(folder)?;
    let mut config = load_config(data)?;
    let (dataset_id, generation, ai_primary_device, must_write_manifest) = match (&probe, mode) {
        (FolderProbe::Empty | FolderProbe::Unrelated { .. }, ConnectMode::InitializeFromLocal) => (
            uuid::Uuid::new_v4().to_string(),
            uuid::Uuid::new_v4().to_string(),
            config.device_id.clone(),
            true,
        ),
        (
            FolderProbe::Existing { manifest, .. },
            ConnectMode::UseFolder | ConnectMode::JoinExisting,
        ) => (
            manifest.dataset_id.clone(),
            manifest.generation.clone(),
            manifest.ai_primary_device.clone(),
            false,
        ),
        (FolderProbe::Existing { manifest, .. }, ConnectMode::ReplaceWithLocal) => (
            manifest.dataset_id.clone(),
            uuid::Uuid::new_v4().to_string(),
            config.device_id.clone(),
            true,
        ),
        (FolderProbe::Incomplete { .. }, _) => {
            return Err(SyncError::new(
                "SYNC_FOLDER_INCOMPLETE",
                "同步目录尚未完整到达或已经损坏，请等待云盘完成后重试",
            ))
        }
        _ => {
            return Err(SyncError::new(
                "SYNC_DIRECTION_REQUIRED",
                "当前文件夹状态与选择的数据方向不一致，请重新检查",
            ))
        }
    };
    let root = protocol_root(folder);
    config.directory = folder.to_string_lossy().into_owned();
    config.dataset_id = dataset_id.clone();
    config.generation = generation.clone();
    config.ai_primary = ai_primary_device.is_empty() || ai_primary_device == config.device_id;
    config.enabled = true;
    config.interval_minutes = config.interval_minutes.clamp(5, 10_080);
    config.validate()?;

    let database = crate::db::Database::open(&data.join(crate::db::DB_FILE_NAME))
        .map_err(|error| SyncError::new("SYNC_DB_OPEN", error.to_string()))?;
    let mut summary = SyncSummary::default();
    if mode == ConnectMode::UseFolder {
        let busy:i64=database.conn().query_row("SELECT (SELECT count(*) FROM ai_jobs WHERE status='running') + (SELECT count(*) FROM analysis_runs WHERE status='running') + (SELECT count(*) FROM kol_materials WHERE status='reading')",[],|r|r.get(0)).map_err(|e|SyncError::new("SYNC_DB_READ",e.to_string()))?;
        if busy > 0 {
            return Err(SyncError::new(
                "SYNC_JOBS_ACTIVE",
                "请等待正在运行的分析或资料读取结束，再切换数据基线",
            ));
        }
        let before: i64 = database
            .conn()
            .query_row("PRAGMA data_version", [], |r| r.get(0))
            .map_err(|e| SyncError::new("SYNC_DB_READ", e.to_string()))?;
        let recovery = data.join("sync-recovery");
        fs::create_dir_all(&recovery)
            .map_err(|e| SyncError::new("SYNC_RECOVERY_WRITE", e.to_string()))?;
        crate::backup::archive::create_snapshot(data, &recovery, &config.device_id)
            .map_err(|e| SyncError::new("SYNC_RECOVERY_WRITE", e))?;
        let tx = crate::db::write_transaction(database.conn())
            .map_err(|e| SyncError::new("SYNC_DB_WRITE", e.to_string()))?;
        let after: i64 = tx
            .query_row("PRAGMA data_version", [], |r| r.get(0))
            .map_err(|e| SyncError::new("SYNC_DB_READ", e.to_string()))?;
        if before != after {
            return Err(SyncError::new(
                "SYNC_BASELINE_CHANGED",
                "备份期间本机数据发生变化，请重新确认数据方向",
            ));
        }
        clear_active_dataset(&tx)?;
        set_database_identity(&tx, &config)?;
        super::blobs::receive(data, &root.join("generations").join(&generation))?;
        summary = receive_states(&tx, &root, &config.device_id, &dataset_id, &generation)?;
        if summary.applied == 0 {
            return Err(SyncError::new(
                "SYNC_BASELINE_PENDING",
                "文件夹数据尚未完整到达，本机数据已保持原样",
            ));
        }
        tx.commit()
            .map_err(|e| SyncError::new("SYNC_DB_WRITE", e.to_string()))?;
    } else {
        set_database_identity(database.conn(), &config)?;
    }
    if mode == ConnectMode::JoinExisting {
        super::blobs::receive(data, &root.join("generations").join(&generation))?;
        summary = receive_states(
            database.conn(),
            &root,
            &config.device_id,
            &dataset_id,
            &generation,
        )?;
    }
    super::blobs::publish(
        database.conn(),
        data,
        &root.join("generations").join(&generation),
    )?;
    publish_state(
        database.conn(),
        &root,
        &config.device_id,
        &dataset_id,
        &generation,
    )?;
    if must_write_manifest {
        write_manifest(
            &root,
            &WorkspaceManifest {
                application: "MSLDesktop".into(),
                format_version: 2,
                dataset_id: dataset_id.clone(),
                generation: generation.clone(),
                created_at: crate::db::now_unix(),
                ai_primary_device: ai_primary_device.clone(),
            },
        )?;
    }
    summary.uploaded = 1;
    write_json(&config_path(data), &config)?;
    let state = SyncState {
        phase: "connected".into(),
        last_attempt: crate::db::now_unix(),
        last_success: crate::db::now_unix(),
        last_error: String::new(),
        last_uploaded: summary.uploaded,
        last_applied: summary.applied,
        conflicts: summary.conflicts,
    };
    write_json(&state_path(data), &state)?;
    Ok(ConnectResult {
        phase: state.phase,
        dataset_id,
        generation,
        summary,
    })
}

fn clear_active_dataset(conn: &rusqlite::Connection) -> SyncResult<()> {
    conn.execute(
        "UPDATE sync_local_state SET suppress_capture=1 WHERE id=1",
        [],
    )
    .map_err(|e| SyncError::new("SYNC_DB_WRITE", e.to_string()))?;
    // Explicit business allowlist. Provider credentials and preferences remain local.
    // Deferred constraints permit deleting an entire dataset as one atomic change.
    conn.execute_batch("PRAGMA defer_foreign_keys=ON;")
        .map_err(|e| SyncError::new("SYNC_DB_WRITE", e.to_string()))?;
    for table in [
        "ai_jobs",
        "ai_readable_documents",
        "knowledge_deleted_sources",
        "material_segments",
        "material_readings",
        "kol_materials",
        "kol_actions",
        "kol_insights",
        "kol_drafts",
        "kol_notes",
        "kol_projects",
        "kol_experts",
        "qa_turns",
        "qa_sessions",
        "proposal_receipts",
        "review_decisions",
        "classification_memories",
        "ai_proposals",
        "daily_briefs",
        "analysis_runs",
        "reports",
        "capture_context",
        "cognition_entries",
        "ai_efficiency_state",
        "work_file_refs",
        "work_workspace_links",
        "resume_points",
        "tasks",
        "waiting_items",
        "calendar_events",
        "inbox_items",
        "activity_events",
        "daily_activity_rollups",
        "document_index",
        "workspace_file_state",
        "cache_entries",
        "works",
        "workspaces",
        "sync_entities",
        "sync_row_versions",
        "sync_peer_rows",
        "sync_applied",
        "sync_outbox",
        "sync_conflicts",
        "sync_checkpoints",
        "sync_workspace_bindings",
        "sync_feedback_events",
    ] {
        conn.execute(&format!("DELETE FROM \"{table}\""), [])
            .map_err(|e| SyncError::new("SYNC_BASELINE_RESET", e.to_string()))?;
    }
    conn.execute("DELETE FROM sync_pending_edits", [])
        .map_err(|e| SyncError::new("SYNC_DB_WRITE", e.to_string()))?;
    conn.execute(
        "UPDATE sync_local_state SET suppress_capture=0 WHERE id=1",
        [],
    )
    .map_err(|e| SyncError::new("SYNC_DB_WRITE", e.to_string()))?;
    Ok(())
}

struct Running;
impl Drop for Running {
    fn drop(&mut self) {
        RUNNING.store(false, Ordering::Release);
    }
}

pub fn run(data: &Path) -> SyncResult<SyncSummary> {
    let result = run_inner(data);
    if let Err(error) = &result {
        if error.code != "SYNC_ALREADY_RUNNING" {
            record_failure(data, &error.message);
        }
    }
    result
}
fn run_inner(data: &Path) -> SyncResult<SyncSummary> {
    if RUNNING.swap(true, Ordering::AcqRel) {
        return Err(SyncError::new("SYNC_ALREADY_RUNNING", "同步正在后台进行"));
    }
    let _running = Running;
    let mut state = load_state(data);
    state.phase = "running".into();
    state.last_attempt = crate::db::now_unix();
    state.last_error.clear();
    write_json(&state_path(data), &state)?;
    let config = load_config(data)?;
    if config.directory.is_empty() || config.dataset_id.is_empty() || config.generation.is_empty() {
        return Err(SyncError::new("SYNC_NOT_CONNECTED", "请先连接同步文件夹"));
    }
    let folder = PathBuf::from(&config.directory);
    crate::storage::paths::reject_link_components(&folder)
        .map_err(|message| SyncError::new("SYNC_DIRECTORY_UNSAFE", message))?;
    crate::backup::service::validate_for_app(data, &folder)
        .map_err(|message| SyncError::new("SYNC_DIRECTORY_UNSAFE", message))?;
    let probe = probe_directory(&folder)?;
    let manifest = match probe {
        FolderProbe::Existing { manifest, .. } => manifest,
        _ => {
            return Err(SyncError::new(
                "SYNC_FOLDER_UNAVAILABLE",
                "同步目录当前不可用或尚未完整到达",
            ))
        }
    };
    if manifest.dataset_id != config.dataset_id || manifest.generation != config.generation {
        return Err(SyncError::new(
            "SYNC_GENERATION_CHANGED",
            "同步文件夹的数据版本已经变化，请重新检查并选择数据方向",
        ));
    }
    if config.ai_primary
        != (manifest.ai_primary_device.is_empty() || manifest.ai_primary_device == config.device_id)
    {
        let mut refreshed = config.clone();
        refreshed.ai_primary =
            manifest.ai_primary_device.is_empty() || manifest.ai_primary_device == config.device_id;
        write_json(&config_path(data), &refreshed)?;
    }
    let database = crate::db::Database::open(&data.join(crate::db::DB_FILE_NAME))
        .map_err(|error| SyncError::new("SYNC_DB_OPEN", error.to_string()))?;
    let identity: (String, String, String) = database
        .conn()
        .query_row(
            "SELECT device_id,dataset_id,generation FROM sync_local_state WHERE id=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|e| SyncError::new("SYNC_DB_READ", e.to_string()))?;
    if identity
        != (
            config.device_id.clone(),
            config.dataset_id.clone(),
            config.generation.clone(),
        )
    {
        return Err(SyncError::new("SYNC_LOCAL_BASELINE_CHANGED", "本机数据库与同步设置的基线不一致，可能曾中断切换或恢复。已停止同步，请重新检查并确认数据方向"));
    }
    let generation_root = protocol_root(&folder)
        .join("generations")
        .join(&config.generation);
    super::blobs::receive(data, &generation_root)?;
    let mut summary = receive_states(
        database.conn(),
        &protocol_root(&folder),
        &config.device_id,
        &config.dataset_id,
        &config.generation,
    )?;
    super::blobs::publish(database.conn(), data, &generation_root)?;
    publish_state(
        database.conn(),
        &protocol_root(&folder),
        &config.device_id,
        &config.dataset_id,
        &config.generation,
    )?;
    summary.uploaded = 1;
    let mut state = load_state(data);
    state.phase = "idle".into();
    state.last_attempt = crate::db::now_unix();
    state.last_success = state.last_attempt;
    state.last_error.clear();
    state.last_uploaded = summary.uploaded;
    state.last_applied = summary.applied;
    state.conflicts = summary.conflicts;
    write_json(&state_path(data), &state)?;
    Ok(summary)
}

pub fn record_failure(data: &Path, message: &str) {
    let mut state = load_state(data);
    state.phase = "failed".into();
    state.last_attempt = crate::db::now_unix();
    state.last_error = message.chars().take(1000).collect();
    let _ = write_json(&state_path(data), &state);
}

pub fn spawn() {
    tauri::async_runtime::spawn(async {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            let _ = tauri::async_runtime::spawn_blocking(|| {
                let data = crate::db::default_app_data_dir();
                let Ok(config) = load_config(&data) else {
                    return;
                };
                if !config.enabled || config.directory.is_empty() || RUNNING.load(Ordering::Acquire)
                {
                    return;
                }
                let state = load_state(&data);
                let due = state.last_attempt == 0
                    || crate::db::now_unix() - state.last_attempt
                        >= i64::from(config.interval_minutes) * 60;
                if due {
                    if let Err(error) = run(&data) {
                        record_failure(&data, &error.message);
                    }
                }
            })
            .await;
        }
    });
}
