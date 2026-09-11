//! Portable, verified snapshots. No credential-store access or workspace traversal.
use crate::db::DB_FILE_NAME;
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};
pub type Result<T> = std::result::Result<T, String>;
pub(crate) fn err<E: std::fmt::Display>(e: E) -> String {
    format!("备份操作失败：{e}")
}
pub(crate) fn hash(path: &Path) -> Result<(u64, String)> {
    let m = fs::symlink_metadata(path).map_err(err)?;
    if !m.is_file() || m.file_type().is_symlink() {
        return Err("备份资源必须为普通文件".into());
    }
    let mut f = File::open(path).map_err(err)?;
    let mut h = Sha256::new();
    let mut n = 0;
    let mut b = [0u8; 65536];
    loop {
        let r = f.read(&mut b).map_err(err)?;
        if r == 0 {
            break;
        }
        n += r as u64;
        h.update(&b[..r]);
    }
    Ok((n, format!("{:x}", h.finalize())))
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub name: String,
    pub bytes: u64,
    pub sha256: String,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub format: u32,
    pub application: String,
    pub app_version: String,
    pub schema: i64,
    pub created_at: i64,
    pub owner: String,
    pub files: Vec<Entry>,
}
pub(crate) fn read_db(path: &Path) -> Result<Connection> {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(err)
}
pub(crate) fn schema(c: &Connection) -> Result<i64> {
    c.query_row("SELECT MAX(version) FROM schema_migrations", [], |r| {
        r.get(0)
    })
    .map_err(err)
}
fn valid_name(name: &str) -> bool {
    name == DB_FILE_NAME
        || name.strip_prefix("attachments/blobs/").is_some_and(|s| {
            let Some((digest, extension)) = s.split_once('.') else {
                return false;
            };
            digest.len() == 64
                && digest.chars().all(|c| c.is_ascii_hexdigit())
                && !extension.is_empty()
                && extension.len() <= 20
                && extension.chars().all(|c| c.is_ascii_alphanumeric())
        })
}

#[cfg(test)]
#[test]
fn windows_device_paths_are_rejected_before_extraction() {
    for name in [
        "COM1.txt",
        "LPT1.pdf",
        "file.",
        "file.txt ",
        "folder/file.txt",
        "abc.txt",
    ] {
        assert!(
            !valid_name(&format!("attachments/blobs/{name}")),
            "unsafe blob name {name}"
        );
    }
    assert!(valid_name(&format!(
        "attachments/blobs/{}.pdf",
        "a".repeat(64)
    )));
}
fn validate_manifest(m: &Manifest) -> Result<()> {
    if m.format != 1
        || m.application != "MSLDesktop"
        || m.schema < 17
        || m.schema > crate::db::latest_schema_version()
    {
        return Err("备份格式或数据库版本不兼容，请使用匹配的新版本应用".into());
    }
    if m.files.is_empty() || m.files.len() > 100_000 || m.owner.len() > 100 {
        return Err("备份清单无效".into());
    }
    let mut seen = BTreeSet::new();
    let mut size = 0u64;
    for f in &m.files {
        if !valid_name(&f.name)
            || !seen.insert(f.name.clone())
            || f.sha256.len() != 64
            || !f.sha256.chars().all(|c| c.is_ascii_hexdigit())
        {
            return Err("备份包含重复、非法路径或校验值".into());
        }
        size = size.checked_add(f.bytes).ok_or("备份大小无效")?;
    }
    if !seen.contains(DB_FILE_NAME) || size > 100 * 1024 * 1024 * 1024 {
        return Err("备份缺少数据库或超过 100 GB 恢复上限".into());
    }
    Ok(())
}
pub fn inspect(path: &Path) -> Result<Manifest> {
    let mut z = ZipArchive::new(File::open(path).map_err(err)?).map_err(err)?;
    let mut text = String::new();
    z.by_name("manifest.json")
        .map_err(err)?
        .take(32 * 1024 * 1024 + 1)
        .read_to_string(&mut text)
        .map_err(err)?;
    if text.len() > 32 * 1024 * 1024 {
        return Err("备份清单过大".into());
    }
    let m: Manifest = serde_json::from_str(&text).map_err(|_| "备份清单无法读取")?;
    validate_manifest(&m)?;
    if z.len() != m.files.len() + 1 {
        return Err("备份文件数量不一致".into());
    }
    let mut names = BTreeSet::new();
    for i in 0..z.len() {
        let f = z.by_index(i).map_err(err)?;
        if !names.insert(f.name().to_string())
            || f.is_dir()
            || f.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
        {
            return Err("备份含重复项或链接".into());
        }
        if f.name() != "manifest.json"
            && !m
                .files
                .iter()
                .any(|e| e.name == f.name() && e.bytes == f.size())
        {
            return Err("备份包含未登记内容".into());
        }
    }
    Ok(m)
}
pub(crate) fn validate_data(data: &Path, m: &Manifest) -> Result<()> {
    let db = read_db(&data.join(DB_FILE_NAME))?;
    if schema(&db)? != m.schema {
        return Err("备份数据库版本与清单不一致".into());
    }
    let integrity: String = db
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(err)?;
    if integrity != "ok"
        || db
            .prepare("PRAGMA foreign_key_check")
            .map_err(err)?
            .query([])
            .map_err(err)?
            .next()
            .map_err(err)?
            .is_some()
    {
        return Err("备份数据库完整性检查失败".into());
    }
    let mut q=db.prepare("SELECT DISTINCT b.relative_path,b.byte_size,b.hash FROM material_blobs b JOIN kol_materials m ON m.blob_hash=b.hash").map_err(err)?;
    let records = q
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, u64>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .map_err(err)?;
    let mut expected = BTreeSet::from([DB_FILE_NAME.to_string()]);
    for r in records {
        let (relative, size, sha) = r.map_err(err)?;
        let name = format!("attachments/blobs/{relative}");
        if !m
            .files
            .iter()
            .any(|e| e.name == name && e.bytes == size && e.sha256 == sha)
        {
            return Err("专家资料缺失或与数据库引用不一致".into());
        }
        expected.insert(name);
    }
    if m.files.iter().any(|f| !expected.contains(&f.name)) {
        return Err("备份包含未引用资料".into());
    }
    Ok(())
}
pub fn unpack(path: &Path, destination: &Path) -> Result<()> {
    crate::storage::paths::reject_link_components(destination)?;
    let m = inspect(path)?;
    fs::create_dir(destination).map_err(err)?;
    let result = (|| {
        let mut z = ZipArchive::new(File::open(path).map_err(err)?).map_err(err)?;
        for e in &m.files {
            let target = destination.join(&e.name);
            fs::create_dir_all(target.parent().unwrap()).map_err(err)?;
            let mut out = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&target)
                .map_err(err)?;
            let mut input = z.by_name(&e.name).map_err(err)?.take(e.bytes + 1);
            let bytes = std::io::copy(&mut input, &mut out).map_err(err)?;
            out.sync_all().map_err(err)?;
            drop(out);
            if bytes != e.bytes || hash(&target)?.1 != e.sha256 {
                return Err("备份文件校验失败，当前数据未修改".into());
            }
        }
        validate_data(destination, &m)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(destination);
    }
    result
}
pub fn create_snapshot(data: &Path, destination: &Path, owner: &str) -> Result<PathBuf> {
    crate::storage::paths::reject_link_components(data)?;
    let _guard = crate::materials::FILE_IO
        .lock()
        .map_err(|_| "资料正在处理")?;
    let id = uuid::Uuid::new_v4();
    let stage = data.join(format!(".backup-{id}"));
    fs::create_dir(&stage).map_err(err)?;
    let result = (|| {
        let source = read_db(&data.join(DB_FILE_NAME))?;
        source
            .busy_timeout(std::time::Duration::from_secs(10))
            .map_err(err)?;
        source
            .execute(
                "VACUUM INTO ?1",
                [stage.join(DB_FILE_NAME).to_string_lossy().as_ref()],
            )
            .map_err(err)?;
        let snapshot = read_db(&stage.join(DB_FILE_NAME))?;
        let version = schema(&snapshot)?;
        let mut names = vec![DB_FILE_NAME.to_string()];
        let mut q=snapshot.prepare("SELECT DISTINCT b.relative_path FROM material_blobs b JOIN kol_materials m ON m.blob_hash=b.hash").map_err(err)?;
        for relative in q.query_map([], |r| r.get::<_, String>(0)).map_err(err)? {
            let name = format!("attachments/blobs/{}", relative.map_err(err)?);
            if !valid_name(&name) {
                return Err("资料索引存在非法路径，备份已停止".into());
            }
            names.push(name);
        }
        let mut files = Vec::new();
        for name in &names {
            let path = if name == DB_FILE_NAME {
                stage.join(name)
            } else {
                data.join(name)
            };
            crate::storage::paths::reject_link_components(&path)?;
            let (bytes, sha256) = hash(&path)?;
            files.push(Entry {
                name: name.clone(),
                bytes,
                sha256,
            });
        }
        let m = Manifest {
            format: 1,
            application: "MSLDesktop".into(),
            app_version: env!("CARGO_PKG_VERSION").into(),
            schema: version,
            created_at: crate::db::now_unix(),
            owner: owner.into(),
            files,
        };
        validate_manifest(&m)?;
        validate_data(&stage, &m)?;
        let staged_archive = stage.join("snapshot.mslbackup");
        let out = File::create(&staged_archive).map_err(err)?;
        let mut z = ZipWriter::new(out);
        let opts = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o600);
        z.start_file("manifest.json", opts).map_err(err)?;
        z.write_all(&serde_json::to_vec(&m).map_err(err)?)
            .map_err(err)?;
        for e in &m.files {
            z.start_file(&e.name, opts).map_err(err)?;
            let path = if e.name == DB_FILE_NAME {
                stage.join(&e.name)
            } else {
                data.join(&e.name)
            };
            std::io::copy(&mut File::open(path).map_err(err)?, &mut z).map_err(err)?;
        }
        z.finish().map_err(err)?.sync_all().map_err(err)?;
        unpack(&staged_archive, &stage.join("verification"))?;
        let final_path = destination.join(format!("msl-backup-{}-{id}.mslbackup", m.created_at));
        let partial = destination.join(format!(".{id}.partial-backup"));
        let published = (|| {
            let mut src = File::open(&staged_archive).map_err(err)?;
            let mut dest = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&partial)
                .map_err(err)?;
            std::io::copy(&mut src, &mut dest).map_err(err)?;
            dest.sync_all().map_err(err)?;
            drop(dest);
            if hash(&partial)? != hash(&staged_archive)? {
                return Err("目标目录写入校验失败".into());
            }
            fs::rename(&partial, &final_path).map_err(err)?;
            Ok(final_path)
        })();
        if published.is_err() {
            let _ = fs::remove_file(&partial);
        }
        published
    })();
    let _ = fs::remove_dir_all(&stage);
    result
}
