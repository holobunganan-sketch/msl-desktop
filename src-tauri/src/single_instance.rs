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
    let wide_name: Vec<u16> = MUTEX_NAME.encode_utf16().chain(std::iter::once(0)).collect();
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
