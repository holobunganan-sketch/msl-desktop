use super::{SyncError, SyncResult};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::Path};

const MAX_BLOB_BYTES: u64 = 2 * 1024 * 1024 * 1024;

fn validate_name(name: &str) -> SyncResult<&str> {
    let (digest, extension) = name
        .split_once('.')
        .ok_or_else(|| SyncError::new("SYNC_BLOB_NAME", "资料文件名称无效"))?;
    if digest.len() != 64
        || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        || extension.is_empty()
        || extension.len() > 20
        || !extension.bytes().all(|byte| byte.is_ascii_alphanumeric())
    {
        return Err(SyncError::new("SYNC_BLOB_NAME", "资料文件名称无效"));
    }
    Ok(digest)
}

fn hash(path: &Path) -> SyncResult<(u64, String)> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| SyncError::new("SYNC_BLOB_READ", error.to_string()))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > MAX_BLOB_BYTES {
        return Err(SyncError::new(
            "SYNC_BLOB_UNSAFE",
            "资料文件类型或大小无法同步",
        ));
    }
    let mut file = fs::File::open(path)
        .map_err(|error| SyncError::new("SYNC_BLOB_READ", error.to_string()))?;
    let mut digest = Sha256::new();
    let mut bytes = 0u64;
    let mut buffer = [0u8; 65_536];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| SyncError::new("SYNC_BLOB_READ", error.to_string()))?;
        if count == 0 {
            break;
        }
        bytes += count as u64;
        digest.update(&buffer[..count]);
    }
    Ok((bytes, format!("{:x}", digest.finalize())))
}

fn copy_verified(source: &Path, destination: &Path, expected: &str) -> SyncResult<bool> {
    if destination.exists() {
        let (_, digest) = hash(destination)?;
        if digest == expected {
            return Ok(false);
        }
        return Err(SyncError::new(
            "SYNC_BLOB_COLLISION",
            "同名资料的内容校验不一致",
        ));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| SyncError::new("SYNC_BLOB_PATH", "资料同步路径无效"))?;
    fs::create_dir_all(parent)
        .map_err(|error| SyncError::new("SYNC_BLOB_WRITE", error.to_string()))?;
    let temporary = parent.join(format!(".blob-{}.part", uuid::Uuid::new_v4()));
    let result = (|| {
        fs::copy(source, &temporary)
            .map_err(|error| SyncError::new("SYNC_BLOB_WRITE", error.to_string()))?;
        let (_, digest) = hash(&temporary)?;
        if digest != expected {
            return Err(SyncError::new("SYNC_BLOB_INTEGRITY", "资料传输校验失败"));
        }
        fs::rename(&temporary, destination)
            .map_err(|error| SyncError::new("SYNC_BLOB_WRITE", error.to_string()))?;
        Ok(true)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

pub fn publish(conn: &Connection, data: &Path, sync_root: &Path) -> SyncResult<u64> {
    let mut statement = conn
        .prepare("SELECT DISTINCT b.relative_path,b.hash,b.byte_size FROM material_blobs b JOIN kol_materials m ON m.blob_hash=b.hash")
        .map_err(|error| SyncError::new("SYNC_DB_READ", error.to_string()))?;
    let records = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, u64>(2)?,
            ))
        })
        .map_err(|error| SyncError::new("SYNC_DB_READ", error.to_string()))?;
    let mut copied = 0;
    for record in records {
        let (relative, expected, expected_size) =
            record.map_err(|error| SyncError::new("SYNC_DB_READ", error.to_string()))?;
        let digest = validate_name(&relative)?;
        if !expected.eq_ignore_ascii_case(digest) {
            return Err(SyncError::new("SYNC_BLOB_INDEX", "资料索引与文件名不一致"));
        }
        let source = data.join("attachments").join("blobs").join(&relative);
        let (size, actual) = hash(&source)?;
        if size != expected_size || actual != expected {
            return Err(SyncError::new(
                "SYNC_BLOB_INDEX",
                "资料索引与文件内容不一致",
            ));
        }
        if copy_verified(&source, &sync_root.join("blobs").join(&relative), &expected)? {
            copied += 1;
        }
    }
    Ok(copied)
}

pub fn receive(data: &Path, sync_root: &Path) -> SyncResult<u64> {
    let source_root = sync_root.join("blobs");
    if !source_root.exists() {
        return Ok(0);
    }
    let mut copied = 0;
    for entry in fs::read_dir(&source_root)
        .map_err(|error| SyncError::new("SYNC_BLOB_READ", error.to_string()))?
    {
        let entry = entry.map_err(|error| SyncError::new("SYNC_BLOB_READ", error.to_string()))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let expected = validate_name(&name)?;
        let source = entry.path();
        crate::storage::paths::reject_link_components(&source)
            .map_err(|message| SyncError::new("SYNC_BLOB_UNSAFE", message))?;
        let (_, actual) = hash(&source)?;
        if !actual.eq_ignore_ascii_case(expected) {
            return Err(SyncError::new(
                "SYNC_BLOB_INTEGRITY",
                "同步文件夹中的资料校验失败",
            ));
        }
        let destination = data.join("attachments").join("blobs").join(&name);
        if copy_verified(&source, &destination, expected)? {
            copied += 1;
        }
    }
    Ok(copied)
}
