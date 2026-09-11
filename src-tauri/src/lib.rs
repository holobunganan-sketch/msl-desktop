//! MSL Desktop — Resident Core 入口（指南 §3.1 / §15）。
//!
//! Stage 1 目标：证明"UI 可以被销毁，而 Core 继续常驻"。
//! - System Tray（打开 / Quick Capture 占位 / 退出）
//! - 主窗口关闭 → 拦截并销毁 WebView，Core + Tray 继续常驻
//! - 托盘"打开" → 重建主窗口
//! - 托盘"退出" → 真正结束进程
//! - 单实例保护

pub mod ai;
pub mod app_state;
pub mod backup;
pub mod cognition;
pub mod commands;
pub mod db;
pub mod documents;
mod lifecycle;
pub mod materials;
pub mod notifications;
pub mod scheduler;
mod single_instance;
pub mod storage;
pub mod sync;
pub mod workspace;

#[cfg(test)]
mod sync_contract_tests;

#[cfg(test)]
mod ai_output_contract_tests;

use app_state::AppState;
use db::Database;
use tauri::{Emitter, Manager};

const MAIN_WINDOW_LABEL: &str = "main";
const TRAY_ID: &str = "msl-desktop-tray";
const TRAY_MENU_OPEN: &str = "open";
const TRAY_MENU_QUICK_CAPTURE: &str = "quick_capture";
const TRAY_MENU_QUIT: &str = "quit";

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// 创建（或重建）主窗口。窗口属性集中在此处定义，保证初次创建与
/// 托盘重建使用同一份配置。若已保存窗口状态则恢复位置与大小。
fn create_main_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    let mut builder = tauri::WebviewWindowBuilder::new(
        app,
        MAIN_WINDOW_LABEL,
        tauri::WebviewUrl::default(), // dev 模式自动使用 devUrl，prod 使用 frontendDist
    )
    .title(format!("MSL Desktop · {}", app.package_info().version))
    .inner_size(1240.0, 720.0)
    .min_inner_size(1024.0, 640.0);

    // 恢复上次窗口位置/大小（指南 §23 window state restore）
    let restored = app
        .try_state::<AppState>()
        .and_then(|s| s.with_database(lifecycle::load_window_state))
        .flatten();
    if let Some((x, y, w, h)) = restored {
        builder = builder
            .position(x as f64, y as f64)
            .inner_size(w as f64, h as f64);
    }

    let window = builder.build()?;

    if let Some(state) = app.try_state::<AppState>() {
        state.record_window_created();
    }
    let _ = window.set_focus();
    Ok(())
}

/// 展示主窗口：已存在则显示+聚焦，否则重建（指南 §3.2 托盘点击逻辑）。
fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    } else if let Err(err) = create_main_window(app) {
        eprintln!("[lifecycle] failed to recreate main window: {err}");
    }
}

/// 构建托盘菜单：打开 / Quick Capture / 分隔线 / 退出。
fn build_tray_menu(app: &tauri::AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{Menu, MenuItemBuilder, PredefinedMenuItem};

    let open = MenuItemBuilder::with_id(TRAY_MENU_OPEN, "打开 MSL Desktop").build(app)?;
    let quick_capture =
        MenuItemBuilder::with_id(TRAY_MENU_QUICK_CAPTURE, "Quick Capture").build(app)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItemBuilder::with_id(TRAY_MENU_QUIT, "退出").build(app)?;

    Menu::with_items(app, &[&open, &quick_capture, &separator, &quit])
}

/// 创建 System Tray。
fn setup_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::image::Image;
    use tauri::tray::TrayIconBuilder;

    let menu = build_tray_menu(app)?;
    // 32x32 PNG 作为托盘图标（Windows 托盘会按需缩放）
    let icon = Image::from_bytes(include_bytes!("../icons/32x32.png"))?;

    TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .icon(icon)
        .tooltip("MSL Desktop")
        .on_menu_event(|app, event| match event.id().as_ref() {
            TRAY_MENU_OPEN => show_main_window(app),
            // Quick Capture：显示主窗口并通知前端聚焦输入框（指南 §7.8）。
            TRAY_MENU_QUICK_CAPTURE => {
                show_main_window(app);
                let _ = app.emit("quick-capture", ());
            }
            TRAY_MENU_QUIT => {
                if let Some(state) = app.try_state::<AppState>() {
                    state.request_quit();
                }
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // 左键单击托盘图标 → 打开/重建主窗口（指南 §3.2）
            use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

/// 启动应用。先做单实例检查：已有实例时静默退出。
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Err(error) = single_instance::wait_for_previous_process() {
        eprintln!("[restart] {error}");
        return;
    }
    // 性能：优化 WebView2 内存占用（指南 §4 预算）。
    // - 禁用 GPU 进程（简单 UI 不受影响）；
    // - 限制 renderer 进程数为 1（单窗口应用）。
    // 必须在 WebView2 运行时创建前设置；保留用户已有参数（如调试端口）。
    let mut wv_args = std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").unwrap_or_default();
    if !wv_args.contains("--disable-gpu") {
        wv_args.push_str(" --disable-gpu");
    }
    if !wv_args.contains("--renderer-process-limit") {
        wv_args.push_str(" --renderer-process-limit=1");
    }
    std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", wv_args.trim());

    match single_instance::acquire() {
        Ok(Some(_guard)) => run_app(),
        Ok(None) => {
            // 已有实例在运行，静默退出
        }
        Err(err) => {
            eprintln!("[single-instance] check failed: {err}");
            std::process::exit(1);
        }
    }
}

fn run_app() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(AppState::default())
        .setup(|app| {
            let data=db::resolve_data_directory(std::env::var_os("APPDATA")).map_err(std::io::Error::other)?;
            backup::restore::apply_pending(&data).map_err(std::io::Error::other)?;
            // 打开数据库（默认路径 %APPDATA%\MSLDesktop\msl-desktop.db）。
            // 失败只告警不阻塞启动——后续命令会返回"数据库未初始化"。
            match Database::open(&db::default_db_path()) {
                Ok(db) => {
                    let _ = crate::db::jobs::recover(db.conn());
                    let _=db.conn().execute("UPDATE kol_materials SET status=CASE WHEN EXISTS(SELECT 1 FROM material_segments WHERE material_id=kol_materials.id) THEN 'partial' ELSE 'failed' END,error='上次读取中断，资料已保存，可重新读取' WHERE status='reading'",[]);
                    app.state::<AppState>().set_database(db);
                    tauri::async_runtime::spawn_blocking(||{if let Ok(db)=Database::open(&db::default_db_path()){let _=crate::materials::purge_unused(&db,&crate::materials::root());}});
                }
                Err(e) => eprintln!("[db] failed to open default database: {e}"),
            }

            // 恢复主 workspace 的文件监听（上次绑定过的目录自动继续监控）。
            let state = app.state::<AppState>();
            if let Some(main_ws) = state.with_database(|db| {
                crate::db::provider::AppSettingsRepo::new(db.conn()).get("main_workspace")
            }) {
                if let Ok(Some(path)) = main_ws {
                    let workspace_id = state
                        .with_database(|db| {
                            crate::db::workspace::WorkspaceRepo::new(db.conn())
                                .list()
                                .ok()
                                .and_then(|items| {
                                    items
                                        .into_iter()
                                        .find(|ws| ws.enabled && ws.root_path == path)
                                        .map(|ws| ws.id)
                                })
                        })
                        .flatten();
                    if workspace_id.is_some() {
                        match crate::workspace::watcher::FileWatcher::start(
                            app.handle().clone(),
                            std::path::PathBuf::from(&path),
                        ) {
                            Ok(w) => {
                                state.set_watcher(w);
                                eprintln!("[workspace] watching {path}");
                                if let Some(id) = workspace_id {
                                    crate::commands::spawn_workspace_reconcile(
                                        app.handle().clone(),
                                        id,
                                        path.clone(),
                                    );
                                }
                            }
                            Err(e) => eprintln!("[workspace] failed to watch {path}: {e}"),
                        }
                    }
                }
            }

            // 提醒调度（Resident Core 常驻，低频轮询）
            crate::notifications::spawn(app.handle().clone());
            crate::scheduler::spawn(app.handle().clone());
            crate::backup::service::spawn();
            crate::sync::service::spawn();

            setup_tray(app.handle())?;

            // background 启动（自启动/后台模式）不创建主窗口，仅 tray 常驻
            if !crate::lifecycle::is_background_start() {
                create_main_window(app.handle())?;
            } else {
                eprintln!("[lifecycle] background start: window suppressed");
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // 主窗口关闭请求 → 阻止真正退出，销毁 WebView；Core + Tray 继续常驻。
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == MAIN_WINDOW_LABEL {
                    api.prevent_close();
                    // 保存窗口位置/大小（window state restore）
                    if let (Ok(pos), Ok(size)) = (window.outer_position(), window.inner_size()) {
                        if let Some(state) = window.try_state::<AppState>() {
                            state.with_database(|db| {
                                let _ = lifecycle::save_window_state(
                                    db,
                                    pos.x,
                                    pos.y,
                                    size.width,
                                    size.height,
                                );
                            });
                        }
                    }
                    if let Some(state) = window.try_state::<AppState>() {
                        state.record_window_destroyed();
                    }
                    let _ = window.destroy();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::sync::sync_status,
            commands::sync::probe_sync_folder,
            commands::sync::connect_sync_folder,
            commands::sync::save_sync_schedule,
            commands::sync::run_sync_now,
            commands::sync::list_sync_conflicts,
            commands::sync::resolve_sync_conflict,
            commands::sync::make_sync_ai_primary,
            commands::backup::backup_status,
            commands::backup::save_backup_settings,
            commands::backup::prepare_private_cloud_folder,
            commands::backup::create_backup,
            commands::backup::preview_backup_restore,
            commands::backup::confirm_backup_restore,
            commands::backup::discard_backup_preview,
            commands::backup::restart_after_restore,
            commands::knowledge::list_qa_sessions,
            commands::knowledge::create_qa_session,
            commands::knowledge::list_qa_turns,
            commands::knowledge::queue_qa_question,
            commands::knowledge::delete_qa_session,
            commands::knowledge::list_kol_experts,
            commands::knowledge::save_kol_expert,
            commands::knowledge::capture_kol_note,
            commands::knowledge::list_kol_notes,
            commands::knowledge::list_kol_drafts,
            commands::knowledge::list_kol_insights,
            commands::knowledge::list_kol_followups,
            commands::knowledge::review_kol_draft,
            commands::knowledge::review_kol_insight,
            commands::flow::get_project_cognition,
            commands::flow::capture_work_note,
            commands::flow::get_entity_location,
            commands::flow::schedule_work_task,
            commands::flow::list_classification_memories,
            commands::flow::edit_classification_memory,
            commands::flow::confirm_ai_proposal_group,
            commands::flow::list_confirmation_receipts,
            commands::flow::undo_ai_confirmation,
            greet,
            commands::jobs::start_ai_job,
            commands::jobs::list_ai_jobs,
            commands::jobs::get_ai_job,
            commands::bind_workspace,
            commands::get_project_directories,
            commands::remove_workspace,
            commands::list_dir,
            commands::open_file,
            commands::reveal_in_explorer,
            commands::recent_files,
            commands::get_workspaces,
            commands::set_watcher_paused,
            commands::watcher_status,
            commands::workspace_sync_status,
            commands::workspace_rescan,
            commands::list_workspace_documents,
            commands::workspace_document_status,
            commands::reindex_workspace_documents,
            commands::link_work_workspace,
            commands::attach_work_folder,
            commands::unlink_work_workspace,
            commands::list_work_workspaces,
            commands::list_workspace_works,
            commands::create_task,
            commands::update_task,
            commands::complete_task,
            commands::list_tasks,
            commands::delete_task,
            commands::create_waiting,
            commands::resolve_waiting,
            commands::update_waiting,
            commands::list_waiting,
            commands::delete_waiting,
            commands::create_inbox_item,
            commands::list_inbox,
            commands::convert_inbox_to_task,
            commands::convert_inbox_to_resume_point,
            commands::convert_inbox_to_waiting,
            commands::convert_inbox_to_calendar,
            commands::delete_inbox_item,
            commands::create_calendar_event,
            commands::update_calendar_event,
            commands::delete_calendar_event,
            commands::list_calendar_events,
            commands::create_work,
            commands::update_work,
            commands::archive_work,
            commands::delete_work,
            commands::knowledge::delete_kol_expert,
            commands::materials::list_kol_materials,
            commands::materials::update_kol_material,
            commands::materials::remove_kol_material,
            commands::materials::get_kol_material_preview,
            commands::materials::reveal_kol_material,
            commands::list_works,
            commands::get_work_detail,
            commands::create_resume_point,
            commands::list_resume_points,
            commands::delete_resume_point,
            commands::add_work_file_ref,
            commands::update_work_file_ref,
            commands::remove_work_file_ref,
            commands::get_today,
            commands::search,
            commands::list_providers,
            commands::save_provider,
            commands::provider_catalog::list_provider_connections,
            commands::provider_catalog::create_provider_template,
            commands::provider_catalog::save_provider_connection,
            commands::provider_catalog::list_provider_models,
            commands::provider_catalog::save_provider_model,
            commands::provider_catalog::set_provider_model_enabled,
            commands::provider_catalog::list_ai_task_routes,
            commands::provider_catalog::save_ai_task_route,
            commands::provider_catalog::refresh_provider_models,
            commands::provider_catalog::test_provider_model,
            commands::ai_secretary::list_ai_proposals,
            commands::ai_secretary::list_latest_analysis_proposals,
            commands::ai_secretary::list_recent_ai_proposals,
            commands::ai_secretary::get_classification_memory_stats,
            commands::ai_secretary::update_ai_proposal_draft,
            commands::ai_secretary::update_ai_proposal_classification,
            commands::ai_secretary::defer_ai_proposal,
            commands::ai_secretary::confirm_ai_proposal,
            commands::ai_secretary::reject_ai_proposal,
            commands::ai_secretary::start_workspace_work_draft,
            commands::ai_secretary::get_analysis_schedule,
            commands::ai_secretary::save_analysis_schedule,
            commands::ai_secretary::run_analysis_now,
            commands::ai_secretary::list_analysis_runs,
            commands::ai_secretary::get_ai_efficiency_stats,
            commands::ai_secretary::retry_analysis_run,
            commands::ai_secretary::keep_daily_brief,
            commands::ai_secretary::translate_text,
            commands::ai_secretary::get_storage_usage,
            commands::ai_secretary::preview_storage_cleanup,
            commands::ai_secretary::execute_storage_cleanup,
            commands::ai_secretary::compact_storage_history,
            commands::ai_secretary::clear_webview_data,
            commands::reports::list_reports,
            commands::reports::generate_report,
            commands::reports::retry_report,
            commands::reports::keep_report,
            commands::reports::clear_report_history,
            commands::reports::get_report_schedule,
            commands::reports::save_report_schedule,
            commands::delete_provider,
            commands::provider_has_key,
            commands::test_provider_connection,
            commands::generate_morning_brief,
            commands::preview_brief_snapshot,
            commands::generate_brief,
            commands::list_briefs,
            commands::get_brief,
            commands::get_morning_brief,
            commands::set_autostart,
            commands::autostart_status,
            commands::set_notifications_enabled,
            commands::set_reminder_lead_minutes,
            commands::check_reminders_now,
            commands::app_settings_get,
            commands::app_settings_set,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // 拦截退出请求：仅当用户通过托盘"退出"明确请求时才真正退出；
    // 最后一个窗口被销毁触发的 ExitRequested 一律阻止，保持 Core 常驻。
    app.run(|app_handle, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            let should_quit = app_handle
                .try_state::<AppState>()
                .map(|s| s.quit_requested())
                .unwrap_or(false);
            if should_quit {
                // graceful shutdown：WAL checkpoint 后关闭数据库（指南 §23）
                if let Some(state) = app_handle.try_state::<AppState>() {
                    if let Some(db) = state.take_database() {
                        if let Err(e) = db.close() {
                            eprintln!("[db] close failed: {e}");
                        }
                    }
                }
            } else {
                api.prevent_exit();
            }
        }
    });
}
