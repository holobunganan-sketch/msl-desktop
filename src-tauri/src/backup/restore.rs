//! Offline restore promotion. Called before any runtime database connection exists.
use super::archive::{self, err, Result};
use crate::db::DB_FILE_NAME;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
const ITEMS: [&str; 4] = [
    DB_FILE_NAME,
    "msl-desktop.db-wal",
    "msl-desktop.db-shm",
    "attachments",
];
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Journal {
    pub token: String,
    pub phase: String,
    pub originals: Vec<String>,
}
pub(crate) fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let temp = path.with_extension(format!("{}-partial", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut f = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp)
            .map_err(err)?;
        f.write_all(&serde_json::to_vec(value).map_err(err)?)
            .map_err(err)?;
        f.sync_all().map_err(err)?;
        drop(f);
        fs::rename(&temp, path).map_err(err)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
fn folder(data: &Path, token: &str) -> Result<PathBuf> {
    let id = uuid::Uuid::parse_str(token).map_err(|_| "恢复标识已失效")?;
    let p = data.join(format!(".restore-{id}"));
    crate::storage::paths::reject_link_components(&p)?;
    Ok(p)
}
pub fn stage(data: &Path, package: &Path) -> Result<String> {
    archive::inspect(package)?;
    if fs::metadata(package).map_err(err)?.len() > 101 * 1024 * 1024 * 1024 {
        return Err("备份包超过恢复上限".into());
    }
    if data.join("restore-pending.json").exists() {
        return Err("已有待执行恢复，请先重新启动应用".into());
    }
    let token = uuid::Uuid::new_v4().to_string();
    let root = folder(data, &token)?;
    fs::create_dir(&root).map_err(err)?;
    let result = (|| {
        let local = root.join("source.mslbackup");
        fs::copy(package, &local).map_err(err)?;
        fs::OpenOptions::new()
            .write(true)
            .open(&local)
            .map_err(err)?
            .sync_all()
            .map_err(err)?;
        archive::unpack(&local, &root.join("incoming"))?;
        Ok(token)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(root);
    }
    result
}
pub fn confirm(data: &Path, token: &str, name: &str) -> Result<()> {
    if name.trim() != "恢复备份" {
        return Err("请输入“恢复备份”以确认替换工作台数据".into());
    }
    if data.join("restore-pending.json").exists() {
        return Err("已有待执行恢复".into());
    }
    let root = folder(data, token)?;
    let m = archive::inspect(&root.join("source.mslbackup"))?;
    for e in &m.files {
        if archive::hash(&root.join("incoming").join(&e.name))? != (e.bytes, e.sha256.clone()) {
            return Err("恢复预览之后文件发生变化，请重新选择备份".into());
        }
    }
    archive::validate_data(&root.join("incoming"), &m)?;
    write_json(
        &data.join("restore-pending.json"),
        &Journal {
            token: token.into(),
            phase: "prepared".into(),
            originals: Vec::new(),
        },
    )
}
fn prepare_database(path: &Path) -> Result<()> {
    let c = rusqlite::Connection::open(path).map_err(err)?;
    c.execute_batch(
        "PRAGMA foreign_keys=ON; PRAGMA journal_mode=DELETE; BEGIN IMMEDIATE;
 UPDATE analysis_schedule_state SET enabled=0,daily_enabled=0;
 UPDATE report_schedule_state SET weekly_enabled=0,monthly_enabled=0;
 -- Preserve directory/project membership and its deletion guard. Removing the
 -- main watcher target plus disabling schedules stops automatic source reads.
 DELETE FROM app_settings WHERE key='main_workspace';
 INSERT OR REPLACE INTO app_settings(key,value,updated_at) VALUES('restored_data_notice','true',0);
 DELETE FROM cache_entries; DELETE FROM material_import_journal;
 UPDATE document_index SET cache_rel_path=NULL,extract_status='pending';
 UPDATE ai_jobs SET status='interrupted',error='从备份恢复，未自动重试' WHERE status='running';
 UPDATE kol_materials SET status='partial',error='从备份恢复，可重新读取' WHERE status='reading';
 COMMIT;",
    )
    .map_err(err)?;
    drop(c);
    fs::OpenOptions::new()
        .write(true)
        .open(path)
        .map_err(err)?
        .sync_all()
        .map_err(err)
}
pub(crate) fn rollback(data: &Path, journal: &Journal) -> Result<()> {
    let root = folder(data, &journal.token)?;
    let old = root.join("previous");
    for name in ITEMS {
        let saved = old.join(name);
        let live = data.join(name);
        if saved.exists() {
            if live.exists() {
                let failed = root.join(format!("failed-{name}"));
                if failed.exists() {
                    return Err("恢复回滚存在冲突，请保留数据目录并联系维护者".into());
                }
                fs::rename(&live, failed).map_err(err)?;
            }
            fs::rename(saved, live).map_err(err)?;
        } else if !journal.originals.iter().any(|s| s == name) && live.exists() {
            fs::rename(live, root.join(format!("failed-{name}"))).map_err(err)?;
        }
    }
    if journal.originals.iter().any(|s| s == DB_FILE_NAME) && !data.join(DB_FILE_NAME).exists() {
        return Err("旧数据库未能恢复，已阻止启动空数据库".into());
    }
    Ok(())
}
pub fn apply_pending(data: &Path) -> Result<()> {
    let pending = data.join("restore-pending.json");
    if !pending.exists() {
        return Ok(());
    }
    crate::storage::paths::reject_link_components(data)?;
    let mut journal: Journal = serde_json::from_slice(&fs::read(&pending).map_err(err)?)
        .map_err(|_| "恢复日志损坏，已停止初始化以保护现有数据")?;
    let root = folder(data, &journal.token)?;
    if journal
        .originals
        .iter()
        .any(|s| !ITEMS.contains(&s.as_str()))
    {
        return Err("恢复日志含非法文件项".into());
    }
    match journal.phase.as_str() {
        "applying" => {
            rollback(data, &journal)?;
            journal.phase = "rolled_back".into();
            write_json(&pending, &journal)?;
        }
        "prepared" => {
            let m = archive::inspect(&root.join("source.mslbackup"))?;
            for e in &m.files {
                if archive::hash(&root.join("incoming").join(&e.name))?
                    != (e.bytes, e.sha256.clone())
                {
                    return Err("待恢复资料发生变化，已保留现有数据".into());
                }
            }
            archive::validate_data(&root.join("incoming"), &m)?;
            // Keep the confirmed input immutable if the process dies before journaling.
            let ready = root.join(format!("ready-{}", uuid::Uuid::new_v4()));
            archive::unpack(&root.join("source.mslbackup"), &ready)?;
            prepare_database(&ready.join(DB_FILE_NAME))?;
            fs::create_dir_all(root.join("previous")).map_err(err)?;
            if fs::read_dir(root.join("previous"))
                .map_err(err)?
                .next()
                .is_some()
            {
                return Err("恢复回滚目录非空，已阻止覆盖".into());
            }
            journal.originals = ITEMS
                .iter()
                .filter(|name| data.join(name).exists())
                .map(|s| s.to_string())
                .collect();
            journal.phase = "applying".into();
            write_json(&pending, &journal)?;
            let result = (|| {
                for name in &journal.originals {
                    fs::rename(data.join(name), root.join("previous").join(name)).map_err(err)?;
                }
                fs::rename(ready.join(DB_FILE_NAME), data.join(DB_FILE_NAME)).map_err(err)?;
                if ready.join("attachments").exists() {
                    fs::rename(ready.join("attachments"), data.join("attachments")).map_err(err)?;
                }
                // Before publishing success, prove the promoted database can be read.
                let c = archive::read_db(&data.join(DB_FILE_NAME))?;
                let ok: String = c
                    .query_row("PRAGMA integrity_check", [], |r| r.get(0))
                    .map_err(err)?;
                if ok != "ok" {
                    return Err("恢复后数据库校验失败".into());
                }
                drop(c);
                journal.phase = "completed".into();
                write_json(&pending, &journal)
            })();
            if let Err(error) = result {
                journal.phase = "applying".into();
                rollback(data, &journal)?;
                journal.phase = "rolled_back".into();
                write_json(&pending, &journal)?;
                write_json(&data.join("last-restore.json"), &journal)?;
                fs::remove_file(pending).map_err(err)?;
                return Err(error);
            }
        }
        "completed" | "rolled_back" => (),
        _ => return Err("无法识别恢复阶段，已保留现有数据".into()),
    }
    write_json(&data.join("last-restore.json"), &journal)?;
    // Machine-specific backup settings are deliberately outside the portable DB.
    let config = data.join("backup-settings.json");
    if config.exists() {
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&config).map_err(err)?).map_err(err)?;
        value["enabled"] = false.into();
        write_json(&config, &value)?;
    }
    if journal.phase == "completed" {
        // Restoring a snapshot is an explicit baseline change. A previous cloud
        // connection must not resume automatically against the restored records.
        let path = data.join("sync-settings.json");
        let directory = fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
            .and_then(|v| v["directory"].as_str().map(str::to_owned))
            .unwrap_or_default();
        let config = crate::sync::settings::SyncConfig {
            directory,
            enabled: false,
            ai_primary: false,
            ..Default::default()
        };
        write_json(&path, &config)?;
        write_json(
            &data.join("sync-state.json"),
            &serde_json::json!({"phase":"disconnected","last_error":"已恢复备份，请重新检查同步文件夹并确认数据方向"}),
        )?;
    }
    fs::remove_file(pending).map_err(err)?;
    if journal.phase == "completed" {
        if cleanup_completed(data, &journal.token).is_err() {
            let _ = write_json(
                &data.join("restore-cleanup-warning.json"),
                &serde_json::json!({"token":journal.token}),
            );
        }
    }
    Ok(())
}

fn cleanup_completed(data: &Path, token: &str) -> Result<()> {
    let root = folder(data, token)?;
    if !root.join("previous").is_dir() {
        return Err("恢复回滚目录不可用，临时副本暂时保留".into());
    }
    // The previous database/attachments remain protected; only duplicate staging is removed.
    for entry in fs::read_dir(&root).map_err(err)? {
        let entry = entry.map_err(err)?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "source.mslbackup"
            || name == "incoming"
            || name
                .strip_prefix("ready-")
                .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok())
        {
            crate::storage::paths::reject_link_components(&entry.path())?;
            if entry.file_type().map_err(err)?.is_dir() {
                fs::remove_dir_all(entry.path()).map_err(err)?;
            } else {
                fs::remove_file(entry.path()).map_err(err)?;
            }
        }
    }
    Ok(())
}

pub fn retry_cleanup(data: &Path) -> Result<()> {
    let marker = data.join("restore-cleanup-warning.json");
    if !marker.exists() {
        return Ok(());
    }
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(&marker).map_err(err)?).map_err(err)?;
    cleanup_completed(data, value["token"].as_str().ok_or("临时清理标识无效")?)?;
    fs::remove_file(marker).map_err(err)
}

pub fn discard(data: &Path, token: &str) -> Result<()> {
    if data.join("restore-pending.json").exists() {
        return Err("已确认的恢复不能作为临时文件删除".into());
    }
    let root = folder(data, token)?;
    if root.join("previous").exists() {
        return Err("恢复前副本受保护".into());
    }
    if root.exists() {
        fs::remove_dir_all(root).map_err(err)?;
    }
    Ok(())
}
