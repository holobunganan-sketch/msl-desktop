use super::archive;
use crate::db::{provider::AppSettingsRepo, Database};
use std::{fs, path::PathBuf};
struct Fixture {
    root: PathBuf,
    data: PathBuf,
    out: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("msl-backup-test-{}", uuid::Uuid::new_v4()));
        let data = root.join("data");
        let out = root.join("cloud");
        fs::create_dir_all(&data).unwrap();
        fs::create_dir_all(&out).unwrap();
        Self { root, data, out }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
#[test]
fn destination_cannot_be_workspace_data_install_or_their_ancestor() {
    let f = Fixture::new();
    assert!(super::service::validate_destination(&f.data, &[f.data.clone()], true).is_err());
    assert!(super::service::validate_destination(&f.root, &[f.data.clone()], true).is_err());
    assert!(super::service::validate_destination(&f.out, &[f.data.clone()], true).is_ok());
}
#[test]
fn missing_or_relative_user_data_path_never_falls_back_to_source() {
    assert!(crate::db::resolve_data_directory(None).is_err());
    assert!(crate::db::resolve_data_directory(Some("relative".into())).is_err());
}

#[test]
fn git_private_cloud_folder_must_be_ignored_and_untracked() {
    let f = Fixture::new();
    let init = std::process::Command::new("git")
        .args(["init", "--quiet"])
        .arg(&f.root)
        .output()
        .unwrap();
    assert!(init.status.success());
    assert!(super::service::validate_destination(&f.out, &[], false).is_err());
    fs::write(f.out.join(".gitignore"), "*\n").unwrap();
    assert!(
        super::service::validate_destination(&f.out, &[], false).is_ok(),
        "A wholly ignored, untracked private cloud folder must be usable"
    );
    fs::write(f.out.join("synthetic.txt"), "synthetic").unwrap();
    let tracked = std::process::Command::new("git")
        .arg("-C")
        .arg(&f.root)
        .args(["add", "--force", "--", "cloud/synthetic.txt"])
        .output()
        .unwrap();
    assert!(tracked.status.success());
    assert!(
        super::service::validate_destination(&f.out, &[], false).is_err(),
        "Ignored rules must not hide already tracked data"
    );
    assert!(super::service::validate_destination(&f.out, &[f.out.clone()], false).is_err());
}

#[test]
fn unsafe_persisted_retention_is_rejected_instead_of_removing_all_backups() {
    let f = Fixture::new();
    super::restore::write_json(&f.data.join("backup-settings.json"),&serde_json::json!({"directory":f.out,"enabled":true,"interval_minutes":0,"keep_count":0,"owner":"fixture"})).unwrap();
    assert!(super::service::config(&f.data).is_err());
}

#[test]
fn private_cloud_preparation_preserves_existing_files_and_blocks_tracked_children() {
    let f = Fixture::new();
    let db = Database::open(&f.data.join(crate::db::DB_FILE_NAME)).unwrap();
    let init = std::process::Command::new("git")
        .args(["init", "--quiet"])
        .arg(&f.root)
        .output()
        .unwrap();
    assert!(init.status.success());
    let child = super::service::prepare_private_cloud_folder(&f.data, &f.out).unwrap();
    assert_eq!(child, f.out.join("MSLDesktop-private"));
    assert_eq!(fs::read(child.join(".gitignore")).unwrap(), b"*\n");
    assert!(super::service::validate_destination(&child, &[], false).is_ok());
    assert_eq!(
        super::service::prepare_private_cloud_folder(&f.data, &f.out).unwrap(),
        child
    );
    fs::write(child.join("synthetic.txt"), "preserve").unwrap();
    let tracked = std::process::Command::new("git")
        .arg("-C")
        .arg(&f.root)
        .args([
            "add",
            "--force",
            "--",
            "cloud/MSLDesktop-private/synthetic.txt",
        ])
        .output()
        .unwrap();
    assert!(tracked.status.success());
    assert!(super::service::prepare_private_cloud_folder(&f.data, &f.out).is_err());
    assert_eq!(fs::read(child.join("synthetic.txt")).unwrap(), b"preserve");
    assert!(super::service::prepare_private_cloud_folder(&f.data, &f.data).is_err());
    assert!(!f.data.join("MSLDesktop-private").exists());
    drop(db);
}
#[test]
fn corrupt_backup_cannot_overwrite_existing_data() {
    let f = Fixture::new();
    let source = f.root.join("bad.mslbackup");
    fs::write(&source, b"corrupt").unwrap();
    assert!(archive::unpack(&source, &f.data).is_err());
    assert!(f.data.is_dir());
}
#[test]
fn backup_roundtrip_includes_committed_wal_and_keeps_source_unchanged() {
    let f = Fixture::new();
    let db = Database::open(&f.data.join(crate::db::DB_FILE_NAME)).unwrap();
    AppSettingsRepo::new(db.conn())
        .set("synthetic_note", "Synthetic private record")
        .unwrap();
    let archive = archive::create_snapshot(&f.data, &f.out, "fixture-owner").unwrap();
    let target = f.root.join("restored");
    archive::unpack(&archive, &target).unwrap();
    let restored = Database::open(&target.join(crate::db::DB_FILE_NAME)).unwrap();
    assert_eq!(
        AppSettingsRepo::new(restored.conn())
            .get("synthetic_note")
            .unwrap()
            .as_deref(),
        Some("Synthetic private record")
    );
    assert_eq!(
        AppSettingsRepo::new(db.conn())
            .get("synthetic_note")
            .unwrap()
            .as_deref(),
        Some("Synthetic private record")
    );
}

#[test]
fn restore_requires_confirmation_and_preserves_pre_restore_records_for_rollback() {
    let f = Fixture::new();
    let sync = crate::sync::settings::SyncConfig {
        directory: f.out.to_string_lossy().into(),
        enabled: true,
        device_id: "previous-device".into(),
        dataset_id: "previous-dataset".into(),
        generation: "previous-generation".into(),
        ..Default::default()
    };
    super::restore::write_json(&f.data.join("sync-settings.json"), &sync).unwrap();
    let path = f.data.join(crate::db::DB_FILE_NAME);
    let db = Database::open(&path).unwrap();
    db.conn().execute_batch("INSERT INTO works(title,created_at,updated_at) VALUES('Synthetic project',0,0); INSERT INTO workspaces(name,root_path,created_at,updated_at) VALUES('Synthetic directory','C:/synthetic-workspace',0,0); INSERT INTO work_workspace_links(work_id,workspace_id,created_at) VALUES(1,1,0);").unwrap();
    AppSettingsRepo::new(db.conn())
        .set("main_workspace", "C:/synthetic-workspace")
        .unwrap();
    AppSettingsRepo::new(db.conn())
        .set("synthetic_note", "before")
        .unwrap();
    let package = archive::create_snapshot(&f.data, &f.out, "fixture-owner").unwrap();
    AppSettingsRepo::new(db.conn())
        .set("synthetic_note", "after")
        .unwrap();
    let token = super::restore::stage(&f.data, &package).unwrap();
    assert!(super::restore::confirm(&f.data, &token, "wrong").is_err());
    assert_eq!(
        AppSettingsRepo::new(db.conn())
            .get("synthetic_note")
            .unwrap()
            .as_deref(),
        Some("after")
    );
    super::restore::confirm(&f.data, &token, "恢复备份").unwrap();
    db.close().unwrap();
    super::restore::apply_pending(&f.data).unwrap();
    let sync = crate::sync::service::load_config(&f.data).unwrap();
    assert!(
        !sync.enabled,
        "Restored older data must not automatically publish into a live dataset"
    );
    assert!(sync.dataset_id.is_empty() && sync.generation.is_empty());
    assert_ne!(sync.device_id, "previous-device");
    let completed = f.data.join(format!(".restore-{token}"));
    assert!(
        !completed.join("incoming").exists(),
        "completed restore left a duplicate data tree"
    );
    assert!(
        !completed.join("source.mslbackup").exists(),
        "completed restore left a temporary archive"
    );
    let restored = Database::open(&path).unwrap();
    assert_eq!(
        restored
            .conn()
            .query_row("SELECT COUNT(*) FROM work_workspace_links", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert!(AppSettingsRepo::new(restored.conn())
        .get("main_workspace")
        .unwrap()
        .is_none());
    assert_eq!(
        AppSettingsRepo::new(restored.conn())
            .get("synthetic_note")
            .unwrap()
            .as_deref(),
        Some("before")
    );
    let old = Database::open(&f.data.join(format!(
        ".restore-{token}/previous/{}",
        crate::db::DB_FILE_NAME
    )))
    .unwrap();
    assert_eq!(
        AppSettingsRepo::new(old.conn())
            .get("synthetic_note")
            .unwrap()
            .as_deref(),
        Some("after")
    );
    assert_eq!(
        restored
            .conn()
            .query_row("SELECT enabled FROM analysis_schedule_state", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn attachment_snapshot_keeps_evidence_and_excludes_unregistered_files() {
    let f = Fixture::new();
    let db = Database::open(&f.data.join(crate::db::DB_FILE_NAME)).unwrap();
    db.conn().execute("INSERT INTO kol_experts(name,institution,created_at,updated_at) VALUES('Synthetic Expert','Example Institute',0,0)",[]).unwrap();
    let source = f.root.join("synthetic.txt");
    fs::write(&source, b"Synthetic evidence with no personal information.").unwrap();
    let material =
        crate::materials::import_one(&db, &f.data.join("attachments"), 1, &source).unwrap();
    crate::materials::index_local(
        &db,
        &f.data.join("attachments"),
        material["id"].as_i64().unwrap(),
    )
    .unwrap();
    fs::write(
        f.data.join("never-export-credentials.txt"),
        b"SYNTHETIC-NOT-A-KEY",
    )
    .unwrap();
    let package = archive::create_snapshot(&f.data, &f.out, "fixture").unwrap();
    let m = archive::inspect(&package).unwrap();
    assert_eq!(m.files.len(), 2);
    let restored = f.root.join("restored");
    archive::unpack(&package, &restored).unwrap();
    let attachment = m
        .files
        .iter()
        .find(|f| f.name.starts_with("attachments/"))
        .unwrap();
    assert_eq!(
        fs::read(restored.join(&attachment.name)).unwrap(),
        fs::read(&source).unwrap()
    );
    assert!(!restored.join("never-export-credentials.txt").exists());
    let restored_db = Database::open(&restored.join(crate::db::DB_FILE_NAME)).unwrap();
    assert_eq!(
        restored_db
            .conn()
            .query_row("SELECT count(*) FROM material_segments", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
}

fn mutate_package(
    source: &std::path::Path,
    dest: &std::path::Path,
    change: impl FnOnce(&mut archive::Manifest),
) {
    use std::io::{Read, Write};
    let mut m = archive::inspect(source).unwrap();
    change(&mut m);
    let mut z = zip::ZipArchive::new(fs::File::open(source).unwrap()).unwrap();
    let mut out = zip::ZipWriter::new(fs::File::create(dest).unwrap());
    let options = zip::write::SimpleFileOptions::default();
    out.start_file("manifest.json", options).unwrap();
    out.write_all(&serde_json::to_vec(&m).unwrap()).unwrap();
    for i in 0..z.len() {
        let mut f = z.by_index(i).unwrap();
        if f.name() == "manifest.json" {
            continue;
        }
        out.start_file(f.name(), options).unwrap();
        let mut b = Vec::new();
        f.read_to_end(&mut b).unwrap();
        out.write_all(&b).unwrap();
    }
    out.finish().unwrap();
}
#[test]
fn damaged_hash_future_schema_and_traversal_are_rejected_before_restore() {
    let f = Fixture::new();
    let _db = Database::open(&f.data.join(crate::db::DB_FILE_NAME)).unwrap();
    let original = archive::create_snapshot(&f.data, &f.out, "fixture").unwrap();
    for (name, change) in [("hash", 0), ("version", 1), ("path", 2)] {
        let bad = f.out.join(format!("{name}.mslbackup"));
        mutate_package(&original, &bad, |m| match change {
            0 => m.files[0].sha256 = "0".repeat(64),
            1 => m.schema = 999,
            _ => m.files[0].name = "../escape.db".into(),
        });
        let dest = f.root.join(name);
        assert!(archive::unpack(&bad, &dest).is_err());
        assert!(!dest.exists());
        assert!(!f.root.join("escape.db").exists());
    }
}
#[test]
fn interrupted_promotion_rolls_back_and_does_not_create_empty_database() {
    let f = Fixture::new();
    let db = Database::open(&f.data.join(crate::db::DB_FILE_NAME)).unwrap();
    AppSettingsRepo::new(db.conn())
        .set("synthetic_note", "retained")
        .unwrap();
    let package = archive::create_snapshot(&f.data, &f.out, "fixture").unwrap();
    let token = super::restore::stage(&f.data, &package).unwrap();
    db.close().unwrap();
    let staging = f.data.join(format!(".restore-{token}"));
    fs::create_dir(staging.join("previous")).unwrap();
    let j = super::restore::Journal {
        token,
        phase: "applying".into(),
        originals: vec![crate::db::DB_FILE_NAME.into()],
    };
    super::restore::write_json(&f.data.join("restore-pending.json"), &j).unwrap();
    fs::rename(
        f.data.join(crate::db::DB_FILE_NAME),
        staging.join("previous").join(crate::db::DB_FILE_NAME),
    )
    .unwrap();
    fs::write(
        f.data.join(crate::db::DB_FILE_NAME),
        b"incomplete promotion",
    )
    .unwrap();
    super::restore::apply_pending(&f.data).unwrap();
    super::restore::apply_pending(&f.data).unwrap();
    let recovered = Database::open(&f.data.join(crate::db::DB_FILE_NAME)).unwrap();
    assert_eq!(
        AppSettingsRepo::new(recovered.conn())
            .get("synthetic_note")
            .unwrap()
            .as_deref(),
        Some("retained")
    );
    assert!(!f.data.join("restore-pending.json").exists());
}
#[test]
fn retention_only_deletes_ledger_owned_verified_archives() {
    let f = Fixture::new();
    let _db = Database::open(&f.data.join(crate::db::DB_FILE_NAME)).unwrap();
    let first = archive::create_snapshot(&f.data, &f.out, "fixture").unwrap();
    let second = archive::create_snapshot(&f.data, &f.out, "fixture").unwrap();
    let stranger = archive::create_snapshot(&f.data, &f.out, "another-device").unwrap();
    let unlisted = archive::create_snapshot(&f.data, &f.out, "fixture").unwrap();
    let note = f.out.join("personal.txt");
    fs::write(&note, b"keep me").unwrap();
    let c = super::service::Config {
        directory: f.out.to_string_lossy().into(),
        keep_count: 1,
        owner: "fixture".into(),
        ..Default::default()
    };
    let mut s = super::service::State {
        files: vec![
            first.to_string_lossy().into(),
            second.to_string_lossy().into(),
            stranger.to_string_lossy().into(),
        ],
        ..Default::default()
    };
    super::service::prune(&f.data, &c, &mut s).unwrap();
    assert_ne!(first.exists(), second.exists());
    assert!(stranger.exists() && unlisted.exists() && note.exists());
}

#[test]
fn retention_preserves_latest_success_when_clock_timestamps_match() {
    let f = Fixture::new();
    let _db = Database::open(&f.data.join(crate::db::DB_FILE_NAME)).unwrap();
    let seed = archive::create_snapshot(&f.data, &f.out, "fixture").unwrap();
    let older = f.out.join("msl-backup-z.mslbackup");
    let latest = f.out.join("msl-backup-a.mslbackup");
    for p in [&older, &latest] {
        mutate_package(&seed, p, |m| m.created_at = 0);
    }
    let c = super::service::Config {
        directory: f.out.to_string_lossy().into(),
        owner: "fixture".into(),
        keep_count: 1,
        ..Default::default()
    };
    let mut s = super::service::State {
        last_file: latest.to_string_lossy().into(),
        files: vec![
            older.to_string_lossy().into(),
            latest.to_string_lossy().into(),
        ],
        ..Default::default()
    };
    super::service::prune(&f.data, &c, &mut s).unwrap();
    assert!(latest.exists());
    assert!(!older.exists());
}
#[test]
fn preview_cancel_cannot_delete_confirmed_or_rollback_data() {
    let f = Fixture::new();
    let _db = Database::open(&f.data.join(crate::db::DB_FILE_NAME)).unwrap();
    let p = archive::create_snapshot(&f.data, &f.out, "fixture").unwrap();
    let t = super::restore::stage(&f.data, &p).unwrap();
    super::restore::discard(&f.data, &t).unwrap();
    assert!(!f.data.join(format!(".restore-{t}")).exists());
    assert!(super::restore::discard(&f.data, "../escape").is_err());
    let t = super::restore::stage(&f.data, &p).unwrap();
    super::restore::confirm(&f.data, &t, "恢复备份").unwrap();
    assert!(super::restore::discard(&f.data, &t).is_err());
}
