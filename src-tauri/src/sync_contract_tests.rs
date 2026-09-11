use crate::sync::{
    merge::{merge_record, FieldValue, MergeRecord},
    protocol::{probe_directory, FolderProbe},
    rows::{publish_state, receive_states},
    service::{connect, load_config, ConnectMode},
    settings::SyncConfig,
};
pub(crate) static SERVICE_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn empty_directory_is_recognized_without_treating_foreign_files_as_empty() {
    let root = std::env::temp_dir().join(format!("msl-sync-probe-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    assert!(matches!(
        probe_directory(&root).unwrap(),
        FolderProbe::Empty
    ));

    std::fs::write(root.join("other.txt"), b"foreign").unwrap();
    assert!(matches!(
        probe_directory(&root).unwrap(),
        FolderProbe::Unrelated { .. }
    ));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn sync_defaults_to_three_hours_and_rejects_out_of_range_intervals() {
    let config = SyncConfig::default();
    assert_eq!(config.interval_minutes, 180);
    assert!(config.validate().is_ok());
    assert!(SyncConfig {
        interval_minutes: 4,
        ..config.clone()
    }
    .validate()
    .is_err());
    assert!(SyncConfig {
        interval_minutes: 10081,
        ..config
    }
    .validate()
    .is_err());
}

#[test]
fn independent_field_edits_merge_without_losing_either_change() {
    let base = MergeRecord::fixture([
        ("title", FieldValue::text("初稿", 10, "A")),
        ("due_at", FieldValue::null(10, "A")),
    ]);
    let local = base
        .clone()
        .with("title", FieldValue::text("修订稿", 20, "A"));
    let remote = base
        .clone()
        .with("due_at", FieldValue::number(2_000_000, 30, "B"));

    let outcome = merge_record(&local, &remote);
    assert_eq!(outcome.record.text("title"), Some("修订稿"));
    assert_eq!(outcome.record.number("due_at"), Some(2_000_000));
    assert!(outcome.conflicts.is_empty());
}

#[test]
fn close_concurrent_edits_are_kept_as_a_conflict() {
    let local = MergeRecord::fixture([("title", FieldValue::text("甲版本", 100, "A"))]);
    let remote = MergeRecord::fixture([("title", FieldValue::text("乙版本", 101, "B"))]);

    let outcome = merge_record(&local, &remote);
    assert_eq!(outcome.conflicts.len(), 1);
    assert_eq!(outcome.conflicts[0].field, "title");
}

#[test]
fn sync_schema_has_durable_identity_journal_and_conflicts() {
    let db = crate::db::Database::open_in_memory().unwrap();
    let version: i64 = db
        .conn()
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(version, 21);
    for table in [
        "sync_local_state",
        "sync_entities",
        "sync_row_versions",
        "sync_outbox",
        "sync_applied",
        "sync_conflicts",
        "sync_peer_rows",
        "sync_devices",
        "sync_checkpoints",
        "sync_workspace_bindings",
        "sync_feedback_events",
        "ai_readable_documents",
    ] {
        let found: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                [table],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(found, 1, "missing {table}");
    }
}

#[test]
fn two_databases_exchange_rows_and_merge_independent_fields() {
    let root = std::env::temp_dir().join(format!("msl-sync-pair-{}", uuid::Uuid::new_v4()));
    let shared = root.join("shared").join("MSLDesktop.sync");
    std::fs::create_dir_all(&shared).unwrap();
    let a = crate::db::Database::open(&root.join("a.db")).unwrap();
    let b = crate::db::Database::open(&root.join("b.db")).unwrap();

    a.conn().execute(
        "INSERT INTO works(title,status,summary,created_at,updated_at) VALUES('合成项目','active','初始',1,1)",
        [],
    ).unwrap();
    publish_state(a.conn(), &shared, "A", "dataset", "generation").unwrap();
    let first = receive_states(b.conn(), &shared, "B", "dataset", "generation").unwrap();
    assert!(first.applied > 0);

    b.conn()
        .execute(
            "UPDATE works SET title='B 标题',updated_at=2 WHERE id=1",
            [],
        )
        .unwrap();
    a.conn()
        .execute(
            "UPDATE works SET summary='A 摘要',updated_at=3 WHERE id=1",
            [],
        )
        .unwrap();
    publish_state(a.conn(), &shared, "A", "dataset", "generation").unwrap();
    publish_state(b.conn(), &shared, "B", "dataset", "generation").unwrap();
    receive_states(a.conn(), &shared, "A", "dataset", "generation").unwrap();
    receive_states(b.conn(), &shared, "B", "dataset", "generation").unwrap();

    for db in [&a, &b] {
        let row: (String, String) = db
            .conn()
            .query_row("SELECT title,summary FROM works WHERE id=1", [], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap();
        assert_eq!(row, ("B 标题".into(), "A 摘要".into()));
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn connecting_empty_folder_creates_identifiable_dataset_and_initial_state() {
    let _guard = SERVICE_TEST_LOCK.lock().unwrap();
    let root = std::path::PathBuf::from(std::env::var_os("USERPROFILE").unwrap())
        .join("AppData")
        .join("Local")
        .join("Temp")
        .join(".test-runtime")
        .join(format!("msl-sync-connect-{}", uuid::Uuid::new_v4()));
    let data = root.join("data");
    let folder = root.join("cloud");
    std::fs::create_dir_all(&data).unwrap();
    std::fs::create_dir_all(&folder).unwrap();
    let db = crate::db::Database::open(&data.join(crate::db::DB_FILE_NAME)).unwrap();
    db.conn().execute(
        "INSERT INTO works(title,status,summary,created_at,updated_at) VALUES('合成项目','active','',1,1)",
        [],
    ).unwrap();
    drop(db);

    let result = connect(&data, &folder, ConnectMode::InitializeFromLocal).unwrap();
    assert_eq!(result.phase, "connected");
    assert!(!result.dataset_id.is_empty());
    assert!(folder
        .join("MSLDesktop.sync")
        .join("msl-workspace.json")
        .is_file());
    let config = load_config(&data).unwrap();
    assert_eq!(config.interval_minutes, 180);
    assert_eq!(config.directory, folder.to_string_lossy());
    assert!(config.enabled);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn simultaneous_new_tasks_keep_distinct_identity_and_project_links() {
    let root = std::path::PathBuf::from(std::env::var_os("USERPROFILE").unwrap())
        .join("AppData")
        .join("Local")
        .join("Temp")
        .join(format!("msl-sync-collision-{}", uuid::Uuid::new_v4()));
    let shared = root.join("shared").join("MSLDesktop.sync");
    std::fs::create_dir_all(&shared).unwrap();
    let a = crate::db::Database::open(&root.join("a.db")).unwrap();
    let b = crate::db::Database::open(&root.join("b.db")).unwrap();
    a.conn().execute("INSERT INTO works(id,title,status,summary,created_at,updated_at) VALUES(1,'共同项目','active','',1,1)", []).unwrap();
    publish_state(a.conn(), &shared, "A", "dataset", "generation").unwrap();
    receive_states(b.conn(), &shared, "B", "dataset", "generation").unwrap();

    a.conn().execute("INSERT INTO tasks(id,work_id,title,status,priority,created_at,updated_at) VALUES(1,1,'A 新任务','next','normal',2,2)", []).unwrap();
    b.conn().execute("INSERT INTO tasks(id,work_id,title,status,priority,created_at,updated_at) VALUES(1,1,'B 新任务','next','normal',2,2)", []).unwrap();
    publish_state(a.conn(), &shared, "A", "dataset", "generation").unwrap();
    publish_state(b.conn(), &shared, "B", "dataset", "generation").unwrap();
    receive_states(a.conn(), &shared, "A", "dataset", "generation").unwrap();
    receive_states(b.conn(), &shared, "B", "dataset", "generation").unwrap();

    for db in [&a, &b] {
        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))
            .unwrap();
        let linked: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM tasks WHERE work_id=1", [], |row| {
                row.get(0)
            })
            .unwrap();
        let titles: String = db
            .conn()
            .query_row(
                "SELECT group_concat(title,'|') FROM (SELECT title FROM tasks ORDER BY title)",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);
        assert_eq!(linked, 2);
        assert_eq!(titles, "A 新任务|B 新任务");
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn reports_conversations_and_expert_insights_travel_between_devices() {
    let root = std::env::temp_dir().join(format!("msl-sync-knowledge-{}", uuid::Uuid::new_v4()));
    let shared = root.join("shared").join("MSLDesktop.sync");
    std::fs::create_dir_all(&shared).unwrap();
    let a = crate::db::Database::open(&root.join("a.db")).unwrap();
    let b = crate::db::Database::open(&root.join("b.db")).unwrap();
    a.conn().execute("INSERT INTO reports(kind,period_start,period_end,status,content,source_counts_json,source_report_ids_json,retention_state,generated_at,created_at,updated_at) VALUES('weekly',1,8,'completed','合成周报','{}','[]','kept',8,8,8)", []).unwrap();
    a.conn().execute("INSERT INTO qa_sessions(title,scope_json,created_at,updated_at) VALUES('合成会话','[]',8,8)", []).unwrap();
    a.conn().execute("INSERT INTO qa_turns(session_id,question,scope_json,status,answer_json,evidence_json,created_at,finished_at) VALUES(1,'项目有什么进展？','[]','completed','{\"claims\":[],\"gaps\":[\"合成资料不足\"]}','[]',8,8)", []).unwrap();
    a.conn().execute("INSERT INTO kol_experts(name,institution,department,specialty,summary,created_at,updated_at) VALUES('合成专家','合成机构','合成科室','','',8,8)", []).unwrap();
    a.conn().execute("INSERT INTO kol_notes(expert_id,content,occurred_at,created_at) VALUES(1,'合成交流记录',8,8)", []).unwrap();
    publish_state(a.conn(), &shared, "A", "dataset", "generation").unwrap();
    receive_states(b.conn(), &shared, "B", "dataset", "generation").unwrap();

    for table in [
        "reports",
        "qa_sessions",
        "qa_turns",
        "kol_experts",
        "kol_notes",
    ] {
        let count: i64 = b
            .conn()
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 1, "missing synced {table}");
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn expert_attachment_bytes_and_index_are_synchronized() {
    use sha2::{Digest, Sha256};
    let root = std::env::temp_dir().join(format!("msl-sync-blob-{}", uuid::Uuid::new_v4()));
    let shared = root.join("shared").join("MSLDesktop.sync");
    let data_a = root.join("data-a");
    let data_b = root.join("data-b");
    std::fs::create_dir_all(data_a.join("attachments/blobs")).unwrap();
    std::fs::create_dir_all(&data_b).unwrap();
    std::fs::create_dir_all(&shared).unwrap();
    let a = crate::db::Database::open(&data_a.join(crate::db::DB_FILE_NAME)).unwrap();
    let b = crate::db::Database::open(&data_b.join(crate::db::DB_FILE_NAME)).unwrap();
    let content = b"synthetic expert material";
    let digest = format!("{:x}", Sha256::digest(content));
    let filename = format!("{digest}.txt");
    std::fs::write(data_a.join("attachments/blobs").join(&filename), content).unwrap();
    a.conn().execute("INSERT INTO kol_experts(name,institution,department,specialty,summary,created_at,updated_at) VALUES('合成专家','合成机构','','','',1,1)", []).unwrap();
    a.conn().execute("INSERT INTO material_blobs(hash,relative_path,byte_size,media_type,created_at) VALUES(?1,?2,?3,'text/plain',1)", rusqlite::params![digest,filename,content.len()]).unwrap();
    a.conn().execute("INSERT INTO kol_materials(expert_id,blob_hash,filename,status,created_at) VALUES(1,?1,'合成.txt','ready',1)", [&digest]).unwrap();
    a.conn().execute("INSERT INTO material_segments(material_id,ordinal,locator,text,kind) VALUES(1,0,'全文','合成资料正文','extracted_text')", []).unwrap();

    crate::sync::blobs::publish(a.conn(), &data_a, &shared).unwrap();
    publish_state(a.conn(), &shared, "A", "dataset", "generation").unwrap();
    crate::sync::blobs::receive(&data_b, &shared).unwrap();
    receive_states(b.conn(), &shared, "B", "dataset", "generation").unwrap();

    assert_eq!(
        std::fs::read(data_b.join("attachments/blobs").join(filename)).unwrap(),
        content
    );
    let segment: String = b
        .conn()
        .query_row("SELECT text FROM material_segments", [], |row| row.get(0))
        .unwrap();
    assert_eq!(segment, "合成资料正文");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_visible_conflict_can_be_resolved_and_is_not_listed_again() {
    let db = crate::db::Database::open_in_memory().unwrap();
    db.conn().execute("INSERT INTO works(id,title,status,summary,created_at,updated_at) VALUES(1,'本机标题','active','',1,1)", []).unwrap();
    db.conn().execute("INSERT INTO sync_entities(table_name,local_key,entity_uid) VALUES('works','1','work-1')", []).unwrap();
    db.conn().execute("INSERT INTO sync_row_versions(table_name,row_key,row_json,field_versions_json,deleted,edited_at_ms,device_id,seq) VALUES('works','1','{\"id\":1,\"title\":\"本机标题\"}','{\"title\":{\"edited_at_ms\":1000,\"device_id\":\"A\",\"seq\":1}}',0,1000,'A',1)", []).unwrap();
    db.conn().execute("INSERT INTO sync_conflicts(id,table_name,row_key,field,local_value,remote_value,base_value,created_at) VALUES('conflict-1','works','1','title','\"本机标题\"','\"同步标题\"','\"旧标题\"',1)", []).unwrap();
    assert_eq!(
        crate::sync::rows::list_conflicts(db.conn()).unwrap().len(),
        1
    );
    crate::sync::rows::resolve_conflict(db.conn(), "conflict-1", "remote").unwrap();
    let title: String = db
        .conn()
        .query_row("SELECT title FROM works WHERE id=1", [], |row| row.get(0))
        .unwrap();
    assert_eq!(title, "同步标题");
    assert!(crate::sync::rows::list_conflicts(db.conn())
        .unwrap()
        .is_empty());
    assert!(crate::sync::rows::resolve_conflict(db.conn(), "conflict-1", "local").is_err());
}
