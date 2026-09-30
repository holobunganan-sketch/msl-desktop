use crate::weixin::{self, LoginView, WeixinStatus};

#[tauri::command]
pub fn get_weixin_status(app: tauri::AppHandle) -> Result<WeixinStatus, String> {
    weixin::status(&app)
}
#[tauri::command]
pub async fn begin_weixin_login(app: tauri::AppHandle) -> Result<LoginView, String> {
    weixin::begin_login(app).await
}
#[tauri::command]
pub async fn poll_weixin_login(app: tauri::AppHandle) -> Result<LoginView, String> {
    weixin::poll_login(app).await
}
#[tauri::command]
pub fn submit_weixin_verifycode(app: tauri::AppHandle, code: String) -> Result<(), String> {
    weixin::submit_code(&app, code)
}
#[tauri::command]
pub fn set_weixin_enabled(app: tauri::AppHandle, enabled: bool) -> Result<WeixinStatus, String> {
    weixin::set_enabled(app, enabled)
}
#[tauri::command]
pub fn unlink_weixin(app: tauri::AppHandle) -> Result<WeixinStatus, String> {
    weixin::unlink(&app)
}
