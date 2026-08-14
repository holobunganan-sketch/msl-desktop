//! MSL Desktop — Resident Core 入口（指南 §3.1 / §15）。
//!
//! Stage 1 目标：证明"UI 可以被销毁，而 Core 继续常驻"。
//! - System Tray（打开 / Quick Capture 占位 / 退出）
//! - 主窗口关闭 → 拦截并销毁 WebView，Core + Tray 继续常驻
//! - 托盘"打开" → 重建主窗口
//! - 托盘"退出" → 真正结束进程
//! - 单实例保护

pub mod app_state;
pub mod commands;
pub mod db;
mod single_instance;
pub mod workspace;

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
/// 托盘重建使用同一份配置。
fn create_main_window(app: &tauri::AppHandle) -> tauri::Result<()> {
    let window = tauri::WebviewWindowBuilder::new(
        app,
        MAIN_WINDOW_LABEL,
        tauri::WebviewUrl::default(), // dev 模式自动使用 devUrl，prod 使用 frontendDist
    )
    .title("MSL Desktop")
    .inner_size(1240.0, 720.0)
    .min_inner_size(1024.0, 640.0)
    .build()?;

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
        .manage(AppState::default())
        .setup(|app| {
            // 打开数据库（默认路径 %APPDATA%\MSLDesktop\msl-desktop.db）。
            // 失败只告警不阻塞启动——后续命令会返回"数据库未初始化"。
            match Database::open(&db::default_db_path()) {
                Ok(db) => app.state::<AppState>().set_database(db),
                Err(e) => eprintln!("[db] failed to open default database: {e}"),
            }

            // 恢复主 workspace 的文件监听（上次绑定过的目录自动继续监控）。
            let state = app.state::<AppState>();
            if let Some(main_ws) = state.with_database(|db| {
                crate::db::provider::AppSettingsRepo::new(db.conn()).get("main_workspace")
            }) {
                if let Ok(Some(path)) = main_ws {
                    match crate::workspace::watcher::FileWatcher::start(
                        app.handle().clone(),
                        std::path::PathBuf::from(&path),
                    ) {
                        Ok(w) => {
                            state.set_watcher(w);
                            eprintln!("[workspace] watching {path}");
                        }
                        Err(e) => eprintln!("[workspace] failed to watch {path}: {e}"),
                    }
                }
            }

            setup_tray(app.handle())?;
            create_main_window(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // 主窗口关闭请求 → 阻止真正退出，销毁 WebView；Core + Tray 继续常驻。
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == MAIN_WINDOW_LABEL {
                    api.prevent_close();
                    if let Some(state) = window.try_state::<AppState>() {
                        state.record_window_destroyed();
                    }
                    let _ = window.destroy();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::bind_workspace,
            commands::list_dir,
            commands::open_file,
            commands::reveal_in_explorer,
            commands::recent_files,
            commands::get_workspaces,
            commands::set_watcher_paused,
            commands::watcher_status,
            commands::create_task,
            commands::update_task,
            commands::complete_task,
            commands::list_tasks,
            commands::delete_task,
            commands::create_waiting,
            commands::resolve_waiting,
            commands::list_waiting,
            commands::delete_waiting,
            commands::create_inbox_item,
            commands::list_inbox,
            commands::convert_inbox_to_task,
            commands::convert_inbox_to_waiting,
            commands::convert_inbox_to_calendar,
            commands::delete_inbox_item,
            commands::create_calendar_event,
            commands::update_calendar_event,
            commands::delete_calendar_event,
            commands::list_calendar_events,
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
            if !should_quit {
                api.prevent_exit();
            }
        }
    });
}
