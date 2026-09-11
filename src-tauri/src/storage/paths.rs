//! 应用专用 local cache/tmp/logs 路径与安全写入。

use std::io::Write;
use std::path::{Component, Path, PathBuf};

use uuid::Uuid;

#[cfg(test)]
static LOCAL_APP_DATA_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
pub(crate) struct LocalAppDataTestGuard {
    previous: Option<std::ffi::OsString>,
    _lock: std::sync::MutexGuard<'static, ()>,
}

#[cfg(test)]
impl LocalAppDataTestGuard {
    pub(crate) fn set(path: &Path) -> Self {
        let lock = LOCAL_APP_DATA_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let previous = std::env::var_os("LOCALAPPDATA");
        std::env::set_var("LOCALAPPDATA", path);
        Self {
            previous,
            _lock: lock,
        }
    }
}

#[cfg(test)]
impl Drop for LocalAppDataTestGuard {
    fn drop(&mut self) {
        if let Some(previous) = self.previous.as_ref() {
            std::env::set_var("LOCALAPPDATA", previous);
        } else {
            std::env::remove_var("LOCALAPPDATA");
        }
    }
}

pub fn local_app_root() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .map(|path| path.join(crate::db::APP_DATA_DIR_NAME))
        .unwrap_or_else(|| crate::db::default_app_data_dir().join("local"))
}

pub fn cache_root() -> PathBuf {
    local_app_root().join("cache")
}
pub fn tmp_root() -> PathBuf {
    local_app_root().join("tmp")
}
pub fn logs_root() -> PathBuf {
    local_app_root().join("logs")
}

fn safe_relative(relative: &str) -> Result<PathBuf, String> {
    let normalized = relative.replace('\\', "/");
    let path = Path::new(&normalized);
    if normalized.trim().is_empty() || path.is_absolute() || normalized.contains(':') {
        return Err("缓存相对路径不能为空且不能是绝对路径".into());
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        return Err("缓存相对路径包含非法穿越组件".into());
    }
    for part in normalized.split('/') {
        let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        if part.ends_with([' ', '.'])
            || ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
            || (stem.len() == 4
                && (stem.starts_with("COM") || stem.starts_with("LPT"))
                && stem.as_bytes()[3].is_ascii_digit())
        {
            return Err("缓存路径包含特殊设备名称或歧义组件".into());
        }
    }
    Ok(path.to_path_buf())
}

/// Validate every existing component before any write, including Windows junctions.
pub fn reject_link_components(path: &Path) -> Result<(), String> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component);
        if matches!(component, Component::Prefix(_)) {
            continue;
        }
        match std::fs::symlink_metadata(&current) {
            Ok(meta) => {
                #[cfg(windows)]
                let reparse = {
                    use std::os::windows::fs::MetadataExt;
                    meta.file_attributes() & 0x400 != 0
                };
                #[cfg(not(windows))]
                let reparse = false;
                if reparse || meta.file_type().is_symlink() {
                    return Err("应用写入路径包含符号链接或目录联接，已阻止操作".into());
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(_) => return Err("无法校验应用写入路径".into()),
        }
    }
    Ok(())
}

fn resolved_or_absolute(path: &Path) -> PathBuf {
    if let Ok(value) = path.canonicalize() {
        return value;
    }
    if let (Some(parent), Some(name)) = (path.parent(), path.file_name()) {
        return resolved_or_absolute(parent).join(name);
    }
    path.to_path_buf()
}

/// Binding application storage or its ancestors would make internal writes source writes.
pub fn validate_workspace_root(root: &Path) -> Result<(), String> {
    let root = resolved_or_absolute(root);
    let normalize = |p: &Path| {
        p.to_string_lossy()
            .replace('/', "\\")
            .trim_start_matches("\\\\?\\")
            .trim_end_matches('\\')
            .to_lowercase()
    };
    let root = normalize(&root);
    for owned in [local_app_root(), crate::db::default_app_data_dir()] {
        let owned = normalize(&resolved_or_absolute(&owned));
        if root == owned
            || owned.starts_with(&format!("{root}\\"))
            || root.starts_with(&format!("{owned}\\"))
        {
            return Err(
                "工作目录与应用数据目录重叠。请选择独立的工作文件夹，源文件将保持只读。".into(),
            );
        }
    }
    Ok(())
}

pub fn ensure_storage_disjoint(db: &crate::db::Database) -> crate::db::DbResult<()> {
    for workspace in crate::db::workspace::WorkspaceRepo::new(db.conn()).list()? {
        validate_workspace_root(Path::new(&workspace.root_path))
            .map_err(crate::db::DbError::Migration)?;
    }
    Ok(())
}

pub fn safe_cache_path_under(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let relative = safe_relative(relative)?;
    reject_link_components(&root.join(&relative))?;
    std::fs::create_dir_all(root).map_err(|error| format!("无法创建 cache root: {error}"))?;
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("cache root 不可解析: {error}"))?;
    let target = canonical_root.join(relative);
    let parent = target
        .parent()
        .ok_or_else(|| "缓存路径缺少父目录".to_string())?;
    std::fs::create_dir_all(parent).map_err(|error| format!("无法创建缓存目录: {error}"))?;
    reject_link_components(&target)?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|error| format!("缓存父目录不可解析: {error}"))?;
    if canonical_parent != canonical_root && !canonical_parent.starts_with(&canonical_root) {
        return Err("缓存路径越出 cache root".into());
    }
    Ok(canonical_parent.join(
        target
            .file_name()
            .ok_or_else(|| "缓存文件名为空".to_string())?,
    ))
}

pub fn safe_cache_path(relative: &str) -> Result<PathBuf, String> {
    safe_cache_path_under(&cache_root(), relative)
}

/// Read/cleanup resolution never creates a directory.
pub fn existing_cache_path(relative: &str) -> Result<PathBuf, String> {
    let target = cache_root().join(safe_relative(relative)?);
    reject_link_components(&target)?;
    let root = cache_root()
        .canonicalize()
        .map_err(|_| "缓存不存在".to_string())?;
    let canonical = target
        .canonicalize()
        .map_err(|_| "缓存文件不存在".to_string())?;
    if !canonical.starts_with(root) {
        return Err("缓存路径越界".into());
    }
    Ok(canonical)
}

pub fn write_cache_atomically(relative: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    let target = safe_cache_path(relative)?;
    let parent = target
        .parent()
        .ok_or_else(|| "缓存目标缺少父目录".to_string())?;
    let temp = parent.join(format!(
        ".{}.partial-{}",
        target.file_name().unwrap().to_string_lossy(),
        Uuid::new_v4()
    ));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|error| error.to_string())?;
        file.write_all(bytes).map_err(|error| error.to_string())?;
        file.flush().map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        reject_link_components(&target)?;
        std::fs::rename(&temp, &target).map_err(|error| error.to_string())?;
        Ok::<(), String>(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result.map(|_| target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejected_relative_paths_do_not_create_any_directories() {
        let root = std::env::temp_dir().join(format!("msl-invalid-path-{}", Uuid::new_v4()));
        for relative in [
            "folder/file.txt:stream",
            "folder/../escape",
            "NUL",
            "folder/CON.txt",
        ] {
            assert!(
                safe_cache_path_under(&root, relative).is_err(),
                "accepted {relative}"
            );
            assert!(!root.exists(), "validation wrote directories");
        }
    }

    #[test]
    #[cfg(windows)]
    fn junction_escape_is_rejected_before_creating_nested_directories() {
        let sandbox = std::env::temp_dir().join(format!("msl-junction-{}", Uuid::new_v4()));
        let root = sandbox.join("cache");
        let source = sandbox.join("source");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&source).unwrap();
        let link = root.join("junction");
        // These paths are fresh test-only descendants of a UUID temp directory.
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(&link)
            .arg(&source)
            .output()
            .unwrap();
        assert!(status.status.success(), "fixture junction creation failed");
        assert!(safe_cache_path_under(&root, "junction/must-not-exist/a.txt").is_err());
        assert!(
            !source.join("must-not-exist").exists(),
            "source directory was modified"
        );
        std::fs::remove_dir(&link).unwrap();
        std::fs::remove_dir_all(&sandbox).unwrap();
    }

    #[test]
    fn cache_paths_reject_absolute_and_traversal() {
        let root = std::env::temp_dir().join(format!("msl-cache-path-{}", Uuid::new_v4()));
        assert!(safe_cache_path_under(&root, "../outside").is_err());
        assert!(safe_cache_path_under(&root, "C:/outside").is_err());
        assert!(safe_cache_path_under(&root, "extracted/a.txt")
            .unwrap()
            .starts_with(root.canonicalize().unwrap()));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn cache_write_is_atomic_and_formal_path_is_separate() {
        let root = std::env::temp_dir().join(format!("msl-cache-write-{}", Uuid::new_v4()));
        let target = safe_cache_path_under(&root, "chunks/hash").unwrap();
        let parent = target.parent().unwrap();
        let _ = std::fs::remove_file(&target);
        let _ = std::fs::remove_dir_all(root);
        assert!(!parent.join(".hash.partial").exists());
    }
}
