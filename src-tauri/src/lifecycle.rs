//! 生命周期增强（指南 §23）：Autostart / background 模式 / 窗口状态恢复。

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;

use crate::db::provider::AppSettingsRepo;
use crate::db::{Database, DbResult};

/// 启动参数：autostart/手动后台启动时附加，不创建主窗口。
pub const BACKGROUND_ARG: &str = "--background";

/// HKCU Run 键。
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "MSLDesktop";

fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// 当前是否以 background 模式启动（不弹主窗口，仅 tray 常驻）。
pub fn is_background_start() -> bool {
    std::env::args().any(|a| a == BACKGROUND_ARG)
}

/// 启用/禁用 Windows 自启动（HKCU\...\Run）。
/// 启用时命令行为 `"<exe>" --background`。
pub fn set_autostart(enabled: bool) -> std::io::Result<()> {
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
        KEY_SET_VALUE, REG_OPTION_NON_VOLATILE, REG_SZ,
    };

    let exe = std::env::current_exe()?;
    let key_name = wide(RUN_KEY);
    let value_name = wide(RUN_VALUE);

    unsafe {
        let mut key: HKEY = std::ptr::null_mut();
        let status = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            key_name.as_ptr(),
            0,
            std::ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            std::ptr::null(),
            &mut key,
            std::ptr::null_mut(),
        );
        if status != ERROR_SUCCESS {
            return Err(std::io::Error::other(format!(
                "RegCreateKeyExW failed: {status}"
            )));
        }

        if enabled {
            let cmd = format!("\"{}\" {}", exe.display(), BACKGROUND_ARG);
            let cmd_wide = wide(&cmd);
            let status = RegSetValueExW(
                key,
                value_name.as_ptr(),
                0,
                REG_SZ,
                cmd_wide.as_ptr() as *const u8,
                (cmd_wide.len() * 2) as u32,
            );
            if status != ERROR_SUCCESS {
                RegCloseKey(key);
                return Err(std::io::Error::other(format!(
                    "RegSetValueExW failed: {status}"
                )));
            }
        } else {
            let _ = RegDeleteValueW(key, value_name.as_ptr());
        }

        RegCloseKey(key);
    }
    Ok(())
}

/// 自启动当前是否启用。
pub fn is_autostart_enabled() -> bool {
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegGetValueW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE,
        RRF_RT_REG_SZ,
    };

    let key_name = wide(RUN_KEY);
    let value_name = wide(RUN_VALUE);

    unsafe {
        let mut key: HKEY = std::ptr::null_mut();
        let status = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            key_name.as_ptr(),
            0,
            KEY_QUERY_VALUE,
            &mut key,
        );
        if status != ERROR_SUCCESS {
            return false;
        }
        let mut buf = [0u16; 1024];
        let mut len = (buf.len() * 2) as u32;
        let status = RegGetValueW(
            key,
            std::ptr::null(),
            value_name.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buf.as_mut_ptr() as *mut core::ffi::c_void,
            &mut len,
        );
        RegCloseKey(key);
        status == ERROR_SUCCESS
    }
}

/// 保存主窗口状态（关闭/销毁前调用）。
pub fn save_window_state(db: &Database, x: i32, y: i32, width: u32, height: u32) -> DbResult<()> {
    let repo = AppSettingsRepo::new(db.conn());
    repo.set("window_x", &x.to_string())?;
    repo.set("window_y", &y.to_string())?;
    repo.set("window_width", &width.to_string())?;
    repo.set("window_height", &height.to_string())?;
    Ok(())
}

/// 读取保存的窗口状态。
pub fn load_window_state(db: &Database) -> Option<(i32, i32, u32, u32)> {
    let repo = AppSettingsRepo::new(db.conn());
    let x = repo.get("window_x").ok().flatten()?.parse().ok()?;
    let y = repo.get("window_y").ok().flatten()?.parse().ok()?;
    let w = repo.get("window_width").ok().flatten()?.parse().ok()?;
    let h = repo.get("window_height").ok().flatten()?.parse().ok()?;
    // 防御：极小/超大窗口值直接忽略
    if w < 640 || h < 400 || w > 8000 || h > 8000 {
        return None;
    }
    Some((x, y, w, h))
}

/// 当前 exe 路径（用于自启动命令）。
#[allow(dead_code)]
pub fn current_exe() -> std::io::Result<PathBuf> {
    std::env::current_exe()
}

// ---------- 单元测试 ----------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_arg_detected() {
        // 当前测试进程不带 --background
        assert!(!is_background_start());
    }

    #[test]
    fn window_state_roundtrip() {
        use crate::db::Database;
        let db = Database::open_in_memory().unwrap();
        save_window_state(&db, 120, 80, 1240, 720).unwrap();
        let restored = load_window_state(&db).unwrap();
        assert_eq!(restored, (120, 80, 1240, 720));
    }

    #[test]
    fn window_state_invalid_ignored() {
        use crate::db::Database;
        let db = Database::open_in_memory().unwrap();
        // 极小尺寸应被忽略（防御）
        save_window_state(&db, 0, 0, 100, 100).unwrap();
        assert!(load_window_state(&db).is_none());
    }
}
