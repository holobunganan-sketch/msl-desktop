use crate::{
    backup::{archive, service},
    db,
};
use std::sync::atomic::Ordering;
#[tauri::command]
pub fn backup_status() -> Result<serde_json::Value, String> {
    let data = db::default_app_data_dir();
    let mut state = service::state(&data)?;
    if data.join("restore-cleanup-warning.json").exists() {
        state.warning = "恢复已完成，临时副本尚未清理完，后台将重试；恢复前副本保持保留。".into();
    }
    Ok(
        serde_json::json!({"data_directory":data,"cache_directory":crate::storage::paths::local_app_root(),"config":service::config(&data)?,"state":state,"running":service::RUNNING.load(Ordering::Acquire),"restore_pending":data.join("restore-pending.json").exists(),"has_rollback":data.join("last-restore.json").exists()}),
    )
}
#[tauri::command]
pub fn save_backup_settings(
    directory: String,
    enabled: bool,
    interval_minutes: u32,
    keep_count: usize,
) -> Result<service::Config, String> {
    service::save(
        &db::default_app_data_dir(),
        directory,
        enabled,
        interval_minutes,
        keep_count,
    )
}
#[tauri::command]
pub async fn create_backup() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(|| {
        service::run(&db::default_app_data_dir()).map(|p| p.to_string_lossy().into())
    })
    .await
    .map_err(|_| "备份后台任务中断")?
}
#[tauri::command]
pub async fn prepare_private_cloud_folder(directory: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        service::prepare_private_cloud_folder(
            &db::default_app_data_dir(),
            std::path::Path::new(&directory),
        )
        .map(|path| path.to_string_lossy().into_owned())
    })
    .await
    .map_err(|_| "私有云盘目录准备任务中断")?
}
#[tauri::command]
pub async fn preview_backup_restore(path: String) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let data = db::default_app_data_dir();
        let token = service::stage(&data, std::path::Path::new(&path))?;
        let manifest = archive::inspect(&data.join(format!(".restore-{token}/source.mslbackup")))?;
        Ok(serde_json::json!({"token":token,"manifest":manifest}))
    })
    .await
    .map_err(|_| "备份校验任务中断")?
}
#[tauri::command]
pub fn confirm_backup_restore(token: String, confirmation_name: String) -> Result<(), String> {
    service::confirm(&db::default_app_data_dir(), &token, &confirmation_name)
}
#[tauri::command]
pub fn discard_backup_preview(token: String) -> Result<(), String> {
    crate::backup::restore::discard(&db::default_app_data_dir(), &token)
}
#[tauri::command]
pub fn restart_after_restore(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    if !db::default_app_data_dir()
        .join("restore-pending.json")
        .exists()
    {
        return Err("没有待执行恢复".into());
    }
    let exe = std::env::current_exe().map_err(|_| "无法定位当前应用")?;
    let mut command = std::process::Command::new(exe);
    command.arg(format!("--restore-wait-pid={}", std::process::id()));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
        .spawn()
        .map_err(|_| "无法重新启动，恢复仍保留在下一次启动时执行")?;
    app.state::<crate::app_state::AppState>().request_quit();
    app.exit(0);
    Ok(())
}
