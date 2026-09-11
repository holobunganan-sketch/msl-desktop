//! Persistent app-owned copies; source paths are never written or removed.
pub mod reading;
use crate::db::{
    knowledge::{self, Evidence},
    Database, DbError, DbResult,
};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};
pub(crate) static FILE_IO: Mutex<()> = Mutex::new(());
pub const LOCAL_READER: &str = "local-text-v1";
pub fn root() -> PathBuf {
    crate::db::default_app_data_dir().join("attachments")
}
fn err(message: impl Into<String>) -> DbError {
    DbError::Migration(message.into())
}
fn io<T>(result: std::io::Result<T>) -> DbResult<T> {
    result.map_err(|_| err("资料文件访问失败，请检查磁盘空间或文件占用"))
}
fn checked(root: &Path, relative: &str) -> DbResult<PathBuf> {
    if relative.is_empty()
        || relative.starts_with('.')
        || relative.len() > 100
        || !relative
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
    {
        return Err(err("资料存储路径无效"));
    }
    let path = root.join("blobs").join(relative);
    crate::storage::paths::reject_link_components(&path).map_err(err)?;
    Ok(path)
}
pub fn list(db: &Database, expert: Option<i64>) -> DbResult<Vec<Value>> {
    knowledge::rows(db.conn(),"SELECT m.*,b.byte_size,b.media_type FROM kol_materials m JOIN material_blobs b ON b.hash=m.blob_hash WHERE ?1 IS NULL OR m.expert_id=?1 ORDER BY m.created_at DESC,m.id DESC",&[&expert])
}
pub fn get(db: &Database, id: i64) -> DbResult<Value> {
    knowledge::rows(db.conn(),"SELECT m.*,b.byte_size,b.media_type FROM kol_materials m JOIN material_blobs b ON b.hash=m.blob_hash WHERE m.id=?1",&[&id])?.pop().ok_or_else(||DbError::NotFound("资料已移除".into()))
}
/// Evidence points to a reading segment. Embedded quoted JSON may use another device's IDs.
pub fn preview_id(
    db: &Database,
    id: i64,
    segment: Option<i64>,
    hash: Option<&str>,
) -> DbResult<i64> {
    match (segment, hash) {
        (None, None) => Ok(id),
        (Some(segment), Some(hash)) if hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()) => {
            db.conn().query_row(
                "SELECT m.id FROM material_segments s JOIN kol_materials m ON m.id=s.material_id WHERE s.id=?1 AND m.blob_hash=?2",
                params![segment, hash], |r| r.get(0)
            ).optional()?.ok_or_else(|| err("资料依据已变更或尚未同步，请刷新后重试"))
        }
        _ => Err(err("资料依据缺少可验证的版本信息")),
    }
}
pub fn blob_path(db: &Database, root: &Path, record: &Value) -> DbResult<PathBuf> {
    let relative: String = db.conn().query_row(
        "SELECT relative_path FROM material_blobs WHERE hash=?1",
        [record["blob_hash"].as_str()],
        |r| r.get(0),
    )?;
    checked(root, &relative)
}
pub fn mime(extension: &str) -> &'static str {
    match extension {
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "doc" => "application/msword",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "ppt" => "application/vnd.ms-powerpoint",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "xls" => "application/vnd.ms-excel",
        "txt" | "md" | "csv" | "tsv" | "log" => "text/plain",
        "json" => "application/json",
        "html" => "text/html",
        "xml" => "application/xml",
        "rtf" => "application/rtf",
        "odt" => "application/vnd.oasis.opendocument.text",
        "wav" => "audio/wav",
        "mp3" => "audio/mpeg",
        "mp4" => "video/mp4",
        _ => "application/octet-stream",
    }
}
pub fn import_one(db: &Database, root: &Path, expert: i64, source: &Path) -> DbResult<Value> {
    crate::storage::paths::ensure_storage_disjoint(db)?;
    crate::storage::paths::reject_link_components(source).map_err(err)?;
    if !source.is_absolute() || !io(fs::metadata(source))?.is_file() {
        return Err(err("请选择普通文件"));
    }
    crate::storage::paths::reject_link_components(root).map_err(err)?;
    let _guard = FILE_IO.lock().map_err(|_| err("资料存储暂不可用"))?;
    io(fs::create_dir_all(root.join("blobs")))?;
    let canonical_source = io(source.canonicalize())?;
    if canonical_source.starts_with(io(root.canonicalize())?) {
        return Err(err("请选择工作文件，不能重复导入应用内部存储"));
    }
    let filename = source
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| err("文件名无效"))?;
    let extension = source
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("bin")
        .to_ascii_lowercase();
    let extension = if extension.len() <= 20 && extension.chars().all(|c| c.is_ascii_alphanumeric())
    {
        extension
    } else {
        "bin".into()
    };
    let stage_name = format!("import-{}.part", uuid::Uuid::new_v4());
    let stage = checked(root, &stage_name)?;
    db.conn().execute(
        "INSERT INTO material_import_journal(stage,created_at) VALUES(?1,?2)",
        params![stage_name, crate::db::now_unix()],
    )?;
    let result = (|| {
        let mut input = io(File::open(source))?;
        let original = io(input.metadata())?;
        let mut output = io(OpenOptions::new().create_new(true).write(true).open(&stage))?;
        let mut digest = Sha256::new();
        let mut size = 0u64;
        let mut buffer = [0u8; 65536];
        loop {
            let count = io(input.read(&mut buffer))?;
            if count == 0 {
                break;
            }
            io(output.write_all(&buffer[..count]))?;
            digest.update(&buffer[..count]);
            size += count as u64;
        }
        io(output.sync_all())?;
        drop(output);
        let after = io(input.metadata())?;
        if size != original.len()
            || after.len() != original.len()
            || after.modified().ok() != original.modified().ok()
        {
            return Err(err("文件在上传期间发生变化，请稍后重新上传"));
        }
        let hash = format!("{:x}", digest.finalize());
        let relative = format!("{hash}.{extension}");
        let tx = crate::db::write_transaction(db.conn())?;
        if !tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM kol_experts WHERE id=?1)",
            [expert],
            |r| r.get::<_, bool>(0),
        )? {
            return Err(err("专家已删除，请重新选择"));
        }
        let existing = knowledge::rows(
            &tx,
            "SELECT relative_path FROM material_blobs WHERE hash=?1",
            &[&hash],
        )?;
        let relative = existing
            .first()
            .and_then(|v| v["relative_path"].as_str())
            .unwrap_or(&relative)
            .to_string();
        let destination = checked(root, &relative)?;
        // Persist the final candidate before the filesystem rename. Recovery checks live references.
        tx.execute(
            "UPDATE material_import_journal SET destination=?1 WHERE stage=?2",
            params![relative, stage_name],
        )?;
        tx.commit()?;
        let tx = crate::db::write_transaction(db.conn())?;
        if !tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM kol_experts WHERE id=?1)",
            [expert],
            |r| r.get::<_, bool>(0),
        )? {
            return Err(err("专家已删除"));
        }
        if !destination.exists() {
            io(fs::rename(&stage, &destination))?;
        }
        tx.execute("INSERT OR IGNORE INTO material_blobs(hash,relative_path,byte_size,media_type,created_at) VALUES(?1,?2,?3,?4,?5)",params![hash,relative,size as i64,mime(&extension),crate::db::now_unix()])?;
        tx.execute("INSERT OR IGNORE INTO kol_materials(expert_id,blob_hash,filename,created_at) VALUES(?1,?2,?3,?4)",params![expert,hash,filename,crate::db::now_unix()])?;
        let id = tx.query_row(
            "SELECT id FROM kol_materials WHERE expert_id=?1 AND blob_hash=?2",
            params![expert, hash],
            |r| r.get(0),
        )?;
        tx.execute(
            "DELETE FROM material_import_journal WHERE stage=?1",
            [&stage_name],
        )?;
        tx.commit()?;
        get(db, id)
    })();
    if stage.exists() {
        let _ = fs::remove_file(&stage);
    }
    result
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Segment {
    pub text: String,
    pub locator: String,
    pub kind: String,
}
pub fn save_reading(
    db: &Database,
    id: i64,
    revision: i64,
    reader: &str,
    segments: &[Segment],
    partial: bool,
    note: &str,
) -> DbResult<()> {
    if segments.is_empty()
        || segments.len() > 1000
        || segments.iter().any(|s| {
            s.text.is_empty()
                || s.text.chars().count() > 4000
                || s.locator.chars().count() > 200
                || !matches!(s.kind.as_str(), "extracted_text" | "model_interpretation")
        })
    {
        return Err(err("资料读取结果无效或超出限制"));
    }
    let tx = crate::db::write_transaction(db.conn())?;
    let record = knowledge::rows(
        &tx,
        "SELECT * FROM kol_materials WHERE id=?1 AND revision=?2",
        &[&id, &revision],
    )?
    .pop()
    .ok_or_else(|| err("资料已更新或移除，旧读取结果已丢弃"))?;
    let previous = knowledge::rows(
        &tx,
        "SELECT text,locator,kind FROM material_segments WHERE material_id=?1 ORDER BY ordinal",
        &[&id],
    )?;
    let unchanged = previous.len() == segments.len()
        && previous
            .iter()
            .zip(segments)
            .all(|(a, b)| a["text"] == b.text && a["locator"] == b.locator && a["kind"] == b.kind);
    if !unchanged {
        retire_segments(&tx, id)?;
        tx.execute("DELETE FROM material_segments WHERE material_id=?1", [id])?;
        for (n, s) in segments.iter().enumerate() {
            tx.execute("INSERT INTO material_segments(material_id,ordinal,locator,text,kind) VALUES(?1,?2,?3,?4,?5)",params![id,n as i64,s.locator,s.text,s.kind])?;
        }
    }
    let status = if partial { "partial" } else { "ready" };
    tx.execute(
        "UPDATE kol_materials SET status=?1,error=?2,reader_key=?3 WHERE id=?4",
        params![status, note, reader, id],
    )?;
    tx.execute("INSERT OR REPLACE INTO material_readings(blob_hash,reader_key,segments_json,status,note) VALUES(?1,?2,?3,?4,?5)",params![record["blob_hash"].as_str(),reader,serde_json::to_string(segments).unwrap(),status,note])?;
    tx.commit()?;
    Ok(())
}
pub fn reuse_reading(db: &Database, id: i64, reader: &str) -> DbResult<bool> {
    let record = get(db, id)?;
    let row = knowledge::rows(
        db.conn(),
        "SELECT * FROM material_readings WHERE blob_hash=?1 AND reader_key=?2",
        &[&record["blob_hash"].as_str(), &reader],
    )?
    .pop();
    if let Some(row) = row {
        let segments =
            serde_json::from_str::<Vec<Segment>>(row["segments_json"].as_str().unwrap_or(""))
                .map_err(|_| err("资料索引无法读取"))?;
        save_reading(
            db,
            id,
            record["revision"].as_i64().unwrap(),
            reader,
            &segments,
            row["status"] == "partial",
            row["note"].as_str().unwrap_or(""),
        )?;
        return Ok(true);
    }
    Ok(false)
}
pub fn index_local(db: &Database, root: &Path, id: i64) -> DbResult<bool> {
    if reuse_reading(db, id, LOCAL_READER)? {
        return Ok(true);
    }
    let record = get(db, id)?;
    let path = blob_path(db, root, &record)?;
    let extracted = crate::documents::extract_path(&path);
    if extracted.status != crate::documents::ExtractStatus::Ready
        || extracted.text.trim().is_empty()
    {
        return Ok(false);
    }
    let chars: Vec<char> = extracted.text.chars().collect();
    let segments = chars
        .chunks(3500)
        .enumerate()
        .map(|(i, c)| Segment {
            text: c.iter().collect(),
            locator: format!("正文字符 {}–{}", i * 3500 + 1, i * 3500 + c.len()),
            kind: "extracted_text".into(),
        })
        .collect::<Vec<_>>();
    let visual = matches!(
        path.extension().and_then(|s| s.to_str()),
        Some("pdf" | "docx")
    );
    save_reading(
        db,
        id,
        record["revision"].as_i64().unwrap(),
        LOCAL_READER,
        &segments,
        extracted.truncated || visual,
        if visual {
            "已读取可提取正文；图表、扫描页或嵌入图片尚未核对，可尝试模型完整读取。"
        } else if extracted.truncated {
            "长文已截取可用范围，未覆盖全部内容。"
        } else {
            ""
        },
    )?;
    Ok(true)
}
fn retire_segments(conn: &Connection, id: i64) -> DbResult<()> {
    let ids = knowledge::rows(
        conn,
        "SELECT id FROM material_segments WHERE material_id=?1",
        &[&id],
    )?
    .into_iter()
    .map(|r| format!("kol_material:{}", r["id"]))
    .collect::<Vec<_>>();
    crate::db::source_lifecycle::retire(conn, &ids)
}
pub fn detach_expert(conn: &Connection, expert: i64) -> DbResult<()> {
    for row in knowledge::rows(
        conn,
        "SELECT id FROM kol_materials WHERE expert_id=?1",
        &[&expert],
    )? {
        retire_segments(conn, row["id"].as_i64().unwrap())?;
    }
    conn.execute("DELETE FROM kol_materials WHERE expert_id=?1", [expert])?;
    Ok(())
}
pub fn remove(db: &Database, id: i64, revision: i64) -> DbResult<()> {
    let tx = crate::db::write_transaction(db.conn())?;
    if !tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM kol_materials WHERE id=?1 AND revision=?2)",
        params![id, revision],
        |r| r.get::<_, bool>(0),
    )? {
        return Err(err("资料已更新或移除，请刷新"));
    }
    retire_segments(&tx, id)?;
    tx.execute("DELETE FROM kol_materials WHERE id=?1", [id])?;
    tx.commit()?;
    Ok(())
}
pub fn purge_unused(db: &Database, root: &Path) -> DbResult<usize> {
    crate::storage::paths::ensure_storage_disjoint(db)?;
    let _guard = FILE_IO.lock().map_err(|_| err("资料存储暂不可用"))?;
    let tx = crate::db::write_transaction(db.conn())?;
    let mut pending = 0;
    for row in knowledge::rows(
        &tx,
        "SELECT stage,destination FROM material_import_journal",
        &[],
    )? {
        let stage = row["stage"].as_str().unwrap_or("");
        let mut paths = vec![checked(root, stage)?];
        if let Some(destination) = row["destination"].as_str() {
            if !tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM material_blobs WHERE relative_path=?1)",
                [destination],
                |r| r.get::<_, bool>(0),
            )? {
                paths.push(checked(root, destination)?);
            }
        }
        let mut removed = true;
        for path in paths {
            if let Err(e) = fs::remove_file(path) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    removed = false;
                }
            }
        }
        if removed {
            tx.execute(
                "DELETE FROM material_import_journal WHERE stage=?1",
                [stage],
            )?;
        } else {
            pending += 1;
        }
    }
    for row in knowledge::rows(&tx,"SELECT hash,relative_path FROM material_blobs b WHERE NOT EXISTS(SELECT 1 FROM kol_materials m WHERE m.blob_hash=b.hash)",&[])? {
  let path=checked(root,row["relative_path"].as_str().unwrap_or(""))?;
  match fs::remove_file(&path){Ok(())=>(),Err(e) if e.kind()==std::io::ErrorKind::NotFound=>(),Err(_)=>{pending+=1;tx.execute("UPDATE material_blobs SET cleanup_error='文件被占用或无法访问，将在下次维护时重试' WHERE hash=?1",[row["hash"].as_str()])?;continue;}}
  tx.execute("DELETE FROM material_blobs WHERE hash=?1",[row["hash"].as_str()])?;
 }
    tx.commit()?;
    Ok(pending)
}
pub fn evidence_count(db: &Database, expert: Option<i64>, scope: &[i64]) -> DbResult<usize> {
    let scope = serde_json::to_string(scope).unwrap();
    Ok(db.conn().query_row("SELECT COUNT(*) FROM material_segments s JOIN kol_materials m ON m.id=s.material_id WHERE m.status IN ('ready','partial') AND (?1 IS NULL OR m.expert_id=?1) AND (?2='[]' OR m.expert_id IN(SELECT expert_id FROM kol_projects WHERE work_id IN(SELECT value FROM json_each(?2))))",params![expert,scope],|r|r.get::<_,i64>(0))? as usize)
}
pub fn evidence(
    db: &Database,
    expert: Option<i64>,
    scope: &[i64],
    query: &str,
) -> DbResult<Vec<Evidence>> {
    let scope = serde_json::to_string(scope).unwrap();
    let sql="SELECT s.id,s.material_id,s.text,s.locator,s.kind,m.expert_id,m.filename AS title,m.blob_hash,m.revision,m.created_at,m.description FROM material_segments s JOIN kol_materials m ON m.id=s.material_id WHERE m.status IN ('ready','partial') AND (?1 IS NULL OR m.expert_id=?1) AND (?2='[]' OR m.expert_id IN(SELECT expert_id FROM kol_projects WHERE work_id IN(SELECT value FROM json_each(?2)))) ORDER BY m.created_at DESC,s.ordinal LIMIT 1000";
    let mut rows = knowledge::rows(db.conn(), sql, &[&expert, &scope])?;
    rows.sort_by_key(|r| {
        std::cmp::Reverse(crate::cognition::relevance(
            query,
            r["text"].as_str().unwrap_or(""),
        ))
    });
    Ok(rows
        .iter()
        .take(100)
        .map(|r| {
            let trust = if r["kind"] == "model_interpretation" {
                "model_reading"
            } else {
                "document_text"
            };
            let mut source = knowledge::evidence("kol_material", r, trust, query);
            source.text = serde_json::to_string(r).unwrap();
            source.hash = crate::cognition::digest(&source.text);
            source
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{kol, Database};
    #[test]
    fn material_preview_uses_mapped_segment_and_verified_file_identity() {
        let (db, base) = fixture();
        let source = base.join("source.txt");
        fs::write(&source, "synthetic evidence").unwrap();
        let record = import_one(&db, &base.join("owned"), 1, &source).unwrap();
        let id = record["id"].as_i64().unwrap();
        index_local(&db, &base.join("owned"), id).unwrap();
        let segment = evidence(&db, Some(1), &[], "").unwrap()[0].entity_id;
        let hash = record["blob_hash"].as_str().unwrap();
        assert_eq!(
            preview_id(&db, 98765, Some(segment), Some(hash)).unwrap(),
            id
        );
        assert!(preview_id(&db, id, Some(segment), Some(&"0".repeat(64))).is_err());
        assert!(preview_id(&db, id, Some(segment), None).is_err());
        assert_eq!(preview_id(&db, id, None, None).unwrap(), id);
        remove(&db, id, 1).unwrap();
        assert!(preview_id(&db, id, Some(segment), Some(hash)).is_err());
    }
    #[test]
    fn material_cleanup_tracks_locked_copies_until_retry() {
        use std::os::windows::fs::OpenOptionsExt;
        let (db, base) = fixture();
        let root = base.join("owned");
        let source = base.join("source.txt");
        fs::write(&source, "synthetic").unwrap();
        let row = import_one(&db, &root, 1, &source).unwrap();
        let blob = blob_path(&db, &root, &row).unwrap();
        let locked = OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&blob)
            .unwrap();
        remove(&db, row["id"].as_i64().unwrap(), 1).unwrap();
        assert_eq!(purge_unused(&db, &root).unwrap(), 1);
        assert!(blob.exists());
        assert!(
            !knowledge::rows(db.conn(), "SELECT cleanup_error FROM material_blobs", &[]).unwrap()
                [0]["cleanup_error"]
                .as_str()
                .unwrap()
                .is_empty()
        );
        drop(locked);
        assert_eq!(purge_unused(&db, &root).unwrap(), 0);
        assert!(!blob.exists());
        assert!(source.exists());
    }
    #[test]
    fn material_interrupted_import_cleanup_only_touches_journal_owned_files() {
        let (db, base) = fixture();
        let root = base.join("owned");
        let source = base.join("source.txt");
        fs::write(&source, "synthetic").unwrap();
        let record = import_one(&db, &root, 1, &source).unwrap();
        let blob = blob_path(&db, &root, &record).unwrap();
        let stage = "import-synthetic.part";
        fs::write(checked(&root, stage).unwrap(), "interrupted").unwrap();
        db.conn()
            .execute(
                "INSERT INTO material_import_journal VALUES(?1,?2,0)",
                params![stage, blob.file_name().unwrap().to_str()],
            )
            .unwrap();
        assert_eq!(purge_unused(&db, &root).unwrap(), 0);
        assert!(blob.exists());
        assert!(source.exists());
        assert!(!checked(&root, stage).unwrap().exists());
        let orphan = "orphan-synthetic.pdf";
        fs::write(checked(&root, orphan).unwrap(), "partial").unwrap();
        db.conn()
            .execute(
                "INSERT INTO material_import_journal VALUES(?1,?2,0)",
                params!["import-other.part", orphan],
            )
            .unwrap();
        purge_unused(&db, &root).unwrap();
        assert!(!checked(&root, orphan).unwrap().exists());
        assert!(blob.exists());
    }
    #[test]
    fn material_reading_is_version_guarded_and_keeps_stable_evidence() {
        let (db, base) = fixture();
        let source = base.join("source.txt");
        fs::write(&source, "synthetic evidence").unwrap();
        let record = import_one(&db, &base.join("owned"), 1, &source).unwrap();
        let id = record["id"].as_i64().unwrap();
        index_local(&db, &base.join("owned"), id).unwrap();
        let first = evidence(&db, Some(1), &[], "").unwrap();
        reuse_reading(&db, id, LOCAL_READER).unwrap();
        assert_eq!(first[0].id, evidence(&db, Some(1), &[], "").unwrap()[0].id);
        let wrong = serde_json::json!({"files":[{"id":999,"segments":["wrong"],"limitations":[]}]})
            .to_string();
        assert!(reading::accept_model_reading(&db, &[record.clone()], "mock", &wrong).is_err());
        let stale = serde_json::json!({"files":[{"id":id,"segments":["late"],"limitations":[]}]})
            .to_string();
        db.conn()
            .execute("UPDATE kol_materials SET revision=2 WHERE id=?1", [id])
            .unwrap();
        reading::accept_model_reading(&db, &[record.clone()], "mock", &stale).unwrap();
        assert_eq!(first[0].id, evidence(&db, Some(1), &[], "").unwrap()[0].id);
        remove(&db, id, 2).unwrap();
        reading::accept_model_reading(&db, &[record], "mock", &stale).unwrap();
        assert!(evidence(&db, Some(1), &[], "").unwrap().is_empty());
        let pack = knowledge::EvidencePack {
            sources: first,
            ..Default::default()
        };
        assert!(crate::db::source_lifecycle::ensure_live(db.conn(), &pack).is_err());
    }
    fn fixture() -> (Database, std::path::PathBuf) {
        let db = Database::open_in_memory().unwrap();
        kol::save_expert(
            &db,
            None,
            None,
            "专家甲",
            "机构甲",
            Some("科室"),
            "",
            &[],
            false,
        )
        .unwrap();
        kol::save_expert(
            &db,
            None,
            None,
            "专家乙",
            "机构乙",
            Some("科室"),
            "",
            &[],
            false,
        )
        .unwrap();
        let root = std::env::temp_dir().join(format!("materials-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        (db, root)
    }
    #[test]
    fn material_import_is_durable_deduplicated_scoped_and_never_changes_source() {
        let (db, root) = fixture();
        let source = root.join("合成资料.txt");
        std::fs::write(&source, "合成专家关注随访证据。\n需要核对研究终点。").unwrap();
        let storage = root.join("owned");
        let first = import_one(&db, &storage, 1, &source).unwrap();
        let second = import_one(&db, &storage, 1, &source).unwrap();
        assert_eq!(first["id"], second["id"]);
        let shared = import_one(&db, &storage, 2, &source).unwrap();
        assert_eq!(first["blob_hash"], shared["blob_hash"]);
        index_local(&db, &storage, first["id"].as_i64().unwrap()).unwrap();
        assert_eq!(list(&db, Some(1)).unwrap()[0]["status"], "ready");
        assert!(!evidence(&db, Some(1), &[], "").unwrap().is_empty());
        assert!(evidence(&db, Some(2), &[], "").unwrap().is_empty());
        remove(&db, first["id"].as_i64().unwrap(), 1).unwrap();
        purge_unused(&db, &storage).unwrap();
        assert!(blob_path(&db, &storage, &shared).unwrap().is_file());
        assert_eq!(
            std::fs::read_to_string(&source).unwrap(),
            "合成专家关注随访证据。\n需要核对研究终点。"
        );
        remove(&db, shared["id"].as_i64().unwrap(), 1).unwrap();
        purge_unused(&db, &storage).unwrap();
        assert_eq!(
            db.conn()
                .query_row("SELECT COUNT(*) FROM material_blobs", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert!(source.is_file());
    }
    #[test]
    fn material_keeps_unknown_formats_without_claiming_to_read() {
        let (db, root) = fixture();
        let source = root.join("synthetic.unknown");
        std::fs::write(&source, [0u8, 1, 2, 3]).unwrap();
        let storage = root.join("owned");
        let record = import_one(&db, &storage, 1, &source).unwrap();
        assert!(!index_local(&db, &storage, record["id"].as_i64().unwrap()).unwrap());
        assert_eq!(list(&db, Some(1)).unwrap().len(), 1);
        assert!(evidence(&db, Some(1), &[], "").unwrap().is_empty());
    }
}
