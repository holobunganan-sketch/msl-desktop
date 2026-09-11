use super::{SyncError, SyncResult};
use std::path::{Path, PathBuf};

pub const ROOT_NAME: &str = "MSLDesktop.sync";
pub const MANIFEST_NAME: &str = "msl-workspace.json";
pub(crate) fn valid_component(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceManifest {
    pub application: String,
    pub format_version: u32,
    pub dataset_id: String,
    pub generation: String,
    pub created_at: i64,
    #[serde(default)]
    pub ai_primary_device: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FolderProbe {
    Empty,
    Unrelated {
        entries: usize,
    },
    Existing {
        root: PathBuf,
        manifest: WorkspaceManifest,
    },
    Incomplete {
        root: PathBuf,
    },
}

pub fn protocol_root(folder: &Path) -> PathBuf {
    folder.join(ROOT_NAME)
}

pub fn probe_directory(folder: &Path) -> SyncResult<FolderProbe> {
    crate::storage::paths::reject_link_components(folder)
        .map_err(|e| SyncError::new("SYNC_DIRECTORY_UNSAFE", e))?;
    if !folder.is_absolute() || !folder.is_dir() {
        return Err(SyncError::new(
            "SYNC_INVALID_DIRECTORY",
            "请选择已存在且可访问的同步文件夹",
        ));
    }
    let root = protocol_root(folder);
    if root.exists() {
        if !root.is_dir() {
            return Err(SyncError::new(
                "SYNC_RESERVED_NAME_CONFLICT",
                "同步目录名称已被其他文件占用",
            ));
        }
        let manifest_path = root.join(MANIFEST_NAME);
        crate::storage::paths::reject_link_components(&manifest_path)
            .map_err(|e| SyncError::new("SYNC_MANIFEST_UNSAFE", e))?;
        if !manifest_path.exists() {
            return Ok(FolderProbe::Incomplete { root });
        }
        let meta = std::fs::metadata(&manifest_path)
            .map_err(|error| SyncError::new("SYNC_MANIFEST_READ_FAILED", error.to_string()))?;
        if meta.len() > 64 * 1024 {
            return Err(SyncError::new(
                "SYNC_MANIFEST_TOO_LARGE",
                "同步目录标识异常，未进行任何覆盖",
            ));
        }
        let bytes = std::fs::read(&manifest_path)
            .map_err(|error| SyncError::new("SYNC_MANIFEST_READ_FAILED", error.to_string()))?;
        let manifest: WorkspaceManifest = serde_json::from_slice(&bytes).map_err(|_| {
            SyncError::new(
                "SYNC_MANIFEST_INVALID",
                "同步目录标识无法验证，未进行任何覆盖",
            )
        })?;
        if manifest.application != "MSLDesktop"
            || !matches!(manifest.format_version, 1 | 2)
            || !valid_component(&manifest.dataset_id)
            || !valid_component(&manifest.generation)
            || (!manifest.ai_primary_device.is_empty()
                && !valid_component(&manifest.ai_primary_device))
        {
            return Err(SyncError::new(
                "SYNC_MANIFEST_UNSUPPORTED",
                "同步目录来自不兼容版本，未进行任何覆盖",
            ));
        }
        return Ok(FolderProbe::Existing { root, manifest });
    }
    let entries = std::fs::read_dir(folder)
        .map_err(|error| SyncError::new("SYNC_DIRECTORY_READ_FAILED", error.to_string()))?
        .count();
    if entries == 0 {
        Ok(FolderProbe::Empty)
    } else {
        Ok(FolderProbe::Unrelated { entries })
    }
}
