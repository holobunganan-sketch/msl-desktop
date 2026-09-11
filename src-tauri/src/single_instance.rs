//! 进程级单实例保护（指南 §15 要求）。
//!
//! 使用 Win32 named mutex 判断是否已有实例在运行：
//! - 第一个实例创建 mutex 并持有句柄直到进程结束；
//! - 后续实例创建时得到 `ERROR_ALREADY_EXISTS`，应静默退出。
//!
//! 命名空间使用 `Local\`（当前会话），桌面应用场景下足够；
//! 不引入额外插件依赖，符合"精巧"约束。

use std::io;
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE};
use windows_sys::Win32::System::Threading::CreateMutexW;

const MUTEX_NAME: &str = "Local\\MSLDesktop_SingleInstance_v1";

/// A replacement process waits before acquiring the single-instance mutex.
pub fn wait_for_previous_process() -> io::Result<()> {
    let Some(arg) = std::env::args().find(|a| a.starts_with("--restore-wait-pid=")) else {
        return Ok(());
    };
    let pid = arg
        .trim_start_matches("--restore-wait-pid=")
        .parse::<u32>()
        .map_err(|_| io::Error::other("Invalid restart process"))?;
    if pid == std::process::id() {
        return Err(io::Error::other("Cannot wait for own process"));
    }
    use windows_sys::Win32::Foundation::WAIT_OBJECT_0;
    use windows_sys::Win32::System::Threading::{
        OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE,
    };
    unsafe {
        let handle = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
        if handle.is_null() {
            let e = io::Error::last_os_error();
            return if e.raw_os_error() == Some(87) {
                Ok(())
            } else {
                Err(e)
            };
        }
        let result = WaitForSingleObject(handle, 30_000);
        CloseHandle(handle);
        if result != WAIT_OBJECT_0 {
            return Err(io::Error::other(
                "Previous application did not exit; restore remains pending",
            ));
        }
    }
    Ok(())
}

/// 持有单实例 mutex 的守卫；Drop 时释放句柄。
pub struct SingleInstanceGuard(HANDLE);

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

/// 尝试获取单实例锁。
///
/// - `Ok(Some(guard))`：本进程是唯一实例，调用方必须持有 `guard` 直到进程结束；
/// - `Ok(None)`：已有实例在运行，调用方应立即退出；
/// - `Err(_)`：系统调用失败，由调用方决定如何处理。
pub fn acquire() -> io::Result<Option<SingleInstanceGuard>> {
    let name = if std::env::var("MSL_ISOLATED_TEST").as_deref() == Ok("1") {
        let paths = ["APPDATA", "LOCALAPPDATA", "TEMP", "TMP"]
            .map(|key| std::env::var(key).unwrap_or_default());
        isolated_mutex_name(&paths).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Test instance requires four isolated directories",
            )
        })?
    } else {
        MUTEX_NAME.into()
    };
    let wide_name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        // owned = FALSE：不要求所有权，只用于检测已存在
        let handle = CreateMutexW(std::ptr::null(), 0, wide_name.as_ptr());
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        if GetLastError() == ERROR_ALREADY_EXISTS {
            CloseHandle(handle);
            return Ok(None);
        }
        Ok(Some(SingleInstanceGuard(handle)))
    }
}

fn isolated_mutex_name(paths: &[String; 4]) -> Option<String> {
    let mut profiles = Vec::new();
    for path in paths {
        let path = std::path::Path::new(path);
        if !path.is_absolute() {
            return None;
        }
        let mut prefix = std::path::PathBuf::new();
        let mut next = false;
        for part in path.components() {
            prefix.push(part);
            if next {
                break;
            }
            if part.as_os_str() == ".test-runtime" {
                next = true;
            }
        }
        if !next {
            return None;
        }
        profiles.push(prefix.to_string_lossy().to_lowercase());
    }
    if profiles.iter().any(|p| p != &profiles[0]) {
        return None;
    }
    Some(format!(
        "{MUTEX_NAME}_test_{}",
        crate::cognition::digest(&profiles[0])
    ))
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_instance_requires_all_four_paths_in_one_explicit_test_profile() {
        let valid = ["appdata", "localappdata", "temp", "tmp"]
            .map(|p| format!("C:/synthetic/.test-runtime/profile/{p}"));
        assert!(super::isolated_mutex_name(&valid).is_some());
        let mut invalid = valid.clone();
        invalid[2] = "C:/Users/synthetic/AppData/Temp".into();
        assert!(super::isolated_mutex_name(&invalid).is_none());
        invalid = valid;
        invalid[3] = "C:/synthetic/.test-runtime/other/tmp".into();
        assert!(super::isolated_mutex_name(&invalid).is_none());
    }
}
