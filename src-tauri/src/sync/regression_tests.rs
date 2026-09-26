use super::{
    rows::{publish_state, receive_states},
    service::{connect, ConnectMode},
};
use crate::db::Database;
use std::path::{Path, PathBuf};

struct Pair {
    root: PathBuf,
    a: Database,
    b: Database,
    left: PathBuf,
    right: PathBuf,
}
impl Pair {
    fn new() -> Self {
        let root = std::env::temp_dir()
            .join(".test-runtime")
            .join(format!("sync-regression-{}", uuid::Uuid::new_v4()));
        let a = Database::open(&root.join("a/data/msl-desktop.db")).unwrap();
        let b = Database::open(&root.join("b/data/msl-desktop.db")).unwrap();
        let left = root.join("cloud-a/MSLDesktop.sync");
        let right = root.join("cloud-b/MSLDesktop.sync");
        std::fs::create_dir_all(&left).unwrap();
        std::fs::create_dir_all(&right).unwrap();
        Self {
            root,
            a,
            b,
            left,
            right,
        }
    }
    fn seed(&self) {
        self.a.conn().execute("INSERT INTO works(title,status,summary,created_at,updated_at) VALUES('Shared','active','Baseline',1,1)", []).unwrap();
        self.send_a();
    }
    fn send_a(&self) {
        publish_state(self.a.conn(), &self.left, "A", "dataset", "generation").unwrap();
        mirror(&self.left, &self.right);
        receive_states(self.b.conn(), &self.right, "B", "dataset", "generation").unwrap();
    }
    fn send_b(&self) {
        publish_state(self.b.conn(), &self.right, "B", "dataset", "generation").unwrap();
        mirror(&self.right, &self.left);
        receive_states(self.a.conn(), &self.left, "A", "dataset", "generation").unwrap();
    }
}
fn mirror(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            mirror(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn report_and_expert_scope_sync_maps_business_ids_but_never_local_documents() {
    let p = Pair::new();
    p.b.conn().execute_batch("INSERT INTO tasks(id,title,created_at,updated_at) VALUES(1,'Receiver unrelated',1,1); INSERT INTO kol_experts(id,name,institution,created_at,updated_at) VALUES(1,'Receiver expert','B',1,1);").unwrap();
    p.a.conn().execute_batch("INSERT INTO tasks(id,title,created_at,updated_at) VALUES(1,'Sender task',1,1); INSERT INTO kol_experts(id,name,institution,created_at,updated_at) VALUES(1,'Sender expert','A',1,1);").unwrap();
    let session = crate::db::qa::create_scoped(&p.a, "Scoped", &[], Some(1)).unwrap();
    let report = crate::db::reports::ReportRepo::new(p.a.conn())
        .create("weekly", 1, 2)
        .unwrap();
    let structured=serde_json::json!({"items":[{"evidence_refs":[{"source_type":"task_completed","entity_id":1}]}]}).to_string();
    let evidence=serde_json::json!({"sources":[{"source_type":"task_completed","entity_id":1},{"source_type":"document","entity_id":1,"location":{"entity_kind":"document","entity_id":1,"workspace_id":1,"relative_path":"synthetic.txt","available":true}}]}).to_string();
    p.a.conn()
        .execute(
            "UPDATE reports SET structured_json=?1,evidence_json=?2 WHERE id=?3",
            rusqlite::params![structured, evidence, report.id],
        )
        .unwrap();
    p.send_a();
    let expert: i64 =
        p.b.conn()
            .query_row(
                "SELECT id FROM kol_experts WHERE name='Sender expert'",
                [],
                |r| r.get(0),
            )
            .unwrap();
    assert_ne!(expert, 1);
    let sessions = crate::db::qa::sessions(&p.b).unwrap();
    assert_eq!(sessions[0]["expert_id"], expert);
    assert_eq!(sessions[0]["expert_scoped"], 1);
    let task: i64 =
        p.b.conn()
            .query_row("SELECT id FROM tasks WHERE title='Sender task'", [], |r| {
                r.get(0)
            })
            .unwrap();
    let report = crate::db::reports::ReportRepo::new(p.b.conn())
        .list(None, 10)
        .unwrap()
        .remove(0);
    let structured: serde_json::Value =
        serde_json::from_str(&report.structured_json.unwrap()).unwrap();
    assert_eq!(
        structured["items"][0]["evidence_refs"][0]["entity_id"],
        task
    );
    let evidence: serde_json::Value = serde_json::from_str(&report.evidence_json.unwrap()).unwrap();
    assert_eq!(evidence["sources"][1]["location"]["available"], false);
    assert_eq!(evidence["sources"][1]["location"]["remote_only"], true);
    p.a.conn()
        .execute("DELETE FROM kol_experts WHERE id=1", [])
        .unwrap();
    p.send_a();
    let sessions = crate::db::qa::sessions(&p.b).unwrap();
    assert!(sessions[0]["expert_id"].is_null());
    assert_eq!(sessions[0]["expert_scoped"], 1);
    assert!(
        crate::db::qa::queue_scoped(&p.b, sessions[0]["id"].as_i64().unwrap(), "Q", &[], None)
            .is_err()
    );
    assert_eq!(session["expert_scoped"], 1);
}

#[test]
fn incoming_change_preserves_unpublished_local_edit() {
    let p = Pair::new();
    p.seed();
    p.send_b();
    p.a.conn()
        .execute("UPDATE works SET summary='Local pending' WHERE id=1", [])
        .unwrap();
    p.b.conn()
        .execute("UPDATE works SET title='Remote title' WHERE id=1", [])
        .unwrap();
    p.send_b();
    let result: (String, String) =
        p.a.conn()
            .query_row("SELECT title,summary FROM works WHERE id=1", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
    assert_eq!(result, ("Remote title".into(), "Local pending".into()));
}

#[test]
fn conflict_write_failure_rolls_back_the_entire_received_batch() {
    for table in ["sync_conflicts", "sync_peer_rows"] {
        let p = Pair::new();
        p.seed();
        p.send_b();
        p.a.conn()
            .execute("UPDATE works SET summary='Local protected' WHERE id=1", [])
            .unwrap();
        p.b.conn()
            .execute(
                "UPDATE works SET summary='Remote concurrent' WHERE id=1",
                [],
            )
            .unwrap();
        p.a.conn().execute_batch(&format!("CREATE TRIGGER synthetic_conflict_failure BEFORE INSERT ON {table} BEGIN SELECT RAISE(ABORT,'synthetic write failure'); END;")).unwrap();
        publish_state(p.b.conn(), &p.right, "B", "dataset", "generation").unwrap();
        mirror(&p.right, &p.left);
        assert!(
            receive_states(p.a.conn(), &p.left, "A", "dataset", "generation").is_err(),
            "Failed conflict persistence must never be reported as a completed merge"
        );
        assert_eq!(
            p.a.conn()
                .query_row("SELECT summary FROM works WHERE id=1", [], |r| r
                    .get::<_, String>(0))
                .unwrap(),
            "Local protected"
        );
    }
}

#[test]
fn question_scope_maps_project_identity_not_integer_id() {
    let p = Pair::new();
    p.b.conn().execute("INSERT INTO works(title,status,created_at,updated_at) VALUES('Unrelated','active',1,1)",[]).unwrap();
    p.a.conn()
        .execute(
            "INSERT INTO works(title,status,created_at,updated_at) VALUES('Shared','active',1,1)",
            [],
        )
        .unwrap();
    p.a.conn().execute("INSERT INTO qa_sessions(title,scope_json,created_at,updated_at) VALUES('Scoped','[1]',1,1)",[]).unwrap();
    p.send_a();
    let scope: String =
        p.b.conn()
            .query_row("SELECT scope_json FROM qa_sessions", [], |r| r.get(0))
            .unwrap();
    assert_eq!(scope, "[2]");
}

#[test]
fn expert_project_links_sync_and_unlink_without_touching_projects() {
    let p = Pair::new();
    p.b.conn()
        .execute(
            "INSERT INTO works(title,created_at,updated_at) VALUES('Unrelated',1,1)",
            [],
        )
        .unwrap();
    p.a.conn()
        .execute(
            "INSERT INTO works(title,created_at,updated_at) VALUES('Shared',1,1)",
            [],
        )
        .unwrap();
    p.a.conn().execute("INSERT INTO kol_experts(name,institution,created_at,updated_at) VALUES('Synthetic expert','Synthetic institution',1,1)",[]).unwrap();
    p.a.conn()
        .execute(
            "INSERT INTO kol_projects(expert_id,work_id) VALUES(1,1)",
            [],
        )
        .unwrap();
    p.send_a();
    assert_eq!(
        p.b.conn()
            .query_row("SELECT work_id FROM kol_projects", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        2
    );
    p.a.conn().execute("DELETE FROM kol_projects", []).unwrap();
    p.send_a();
    assert_eq!(
        p.b.conn()
            .query_row("SELECT count(*) FROM kol_projects", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        p.b.conn()
            .query_row("SELECT count(*) FROM works", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
}

#[test]
fn evidence_and_converted_item_references_follow_global_identity() {
    let p = Pair::new();
    p.b.conn()
        .execute(
            "INSERT INTO works(title,created_at,updated_at) VALUES('Unrelated',1,1)",
            [],
        )
        .unwrap();
    p.a.conn()
        .execute(
            "INSERT INTO works(title,created_at,updated_at) VALUES('Shared',1,1)",
            [],
        )
        .unwrap();
    p.a.conn().execute("INSERT INTO inbox_items(content,converted_to_type,converted_to_id,created_at) VALUES('Captured','work',1,1)",[]).unwrap();
    p.a.conn().execute("INSERT INTO qa_sessions(title,scope_json,created_at,updated_at) VALUES('Scoped','[1]',1,1)",[]).unwrap();
    p.a.conn().execute("INSERT INTO qa_turns(session_id,question,scope_json,status,answer_json,evidence_json,created_at) VALUES(1,'Question','[1]','completed',?1,?2,1)",rusqlite::params![
        serde_json::json!({"claims":[{"text":"Shared","basis":"fact","citations":[{"source_id":"work:1","quote":"Shared"}]}],"gaps":[]}).to_string(),
        serde_json::json!({"scope_ids":[1],"sources":[{"id":"work:1","kind":"work","entity_id":1,"text":"Shared"}]}).to_string()
    ]).unwrap();
    p.send_a();
    assert_eq!(
        p.b.conn()
            .query_row("SELECT converted_to_id FROM inbox_items", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        2
    );
    let (answer, evidence): (String, String) =
        p.b.conn()
            .query_row("SELECT answer_json,evidence_json FROM qa_turns", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
    let answer: serde_json::Value = serde_json::from_str(&answer).unwrap();
    let evidence: serde_json::Value = serde_json::from_str(&evidence).unwrap();
    assert_eq!(answer["claims"][0]["citations"][0]["source_id"], "work:2");
    assert_eq!(evidence["scope_ids"][0], 2);
    assert_eq!(evidence["sources"][0]["id"], "work:2");
    assert_eq!(evidence["sources"][0]["entity_id"], 2);
}

#[test]
fn corrupted_state_is_rejected_before_business_write() {
    let p = Pair::new();
    p.a.conn()
        .execute(
            "INSERT INTO works(title,status,created_at,updated_at) VALUES('Original','active',1,1)",
            [],
        )
        .unwrap();
    let file = publish_state(p.a.conn(), &p.left, "A", "dataset", "generation").unwrap();
    let original = std::fs::read_to_string(&file).unwrap();
    std::fs::write(&file, original.replace("Original", "Modified")).unwrap();
    mirror(&p.left, &p.right);
    assert!(receive_states(p.b.conn(), &p.right, "B", "dataset", "generation").is_err());
    assert_eq!(
        p.b.conn()
            .query_row("SELECT count(*) FROM works", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn deleted_remote_evidence_cannot_reenter_analysis_through_a_late_snapshot() {
    use crate::db::knowledge::{self, EvidencePack};
    let p = Pair::new();
    p.a.conn().execute("INSERT INTO kol_experts(name,institution,created_at,updated_at) VALUES('Synthetic','Synthetic',1,1)",[]).unwrap();
    p.a.conn().execute("INSERT INTO kol_notes(expert_id,content,occurred_at,created_at) VALUES(1,'Synthetic evidence',1,1)",[]).unwrap();
    p.a.conn().execute("INSERT INTO qa_sessions(title,scope_json,created_at,updated_at) VALUES('Synthetic','[]',1,1)",[]).unwrap();
    let pack = EvidencePack {
        sources: vec![knowledge::evidence(
            "kol_note",
            &serde_json::json!({"id":1,"content":"Synthetic evidence"}),
            "record",
            "",
        )],
        ..Default::default()
    };
    p.a.conn().execute("INSERT INTO qa_turns(session_id,question,scope_json,status,evidence_json,created_at) VALUES(1,'Question','[]','completed',?1,1)",[serde_json::to_string(&pack).unwrap()]).unwrap();
    p.send_a();
    p.a.conn()
        .execute("DELETE FROM kol_notes WHERE id=1", [])
        .unwrap();
    p.send_a();
    // A stale historical snapshot arrives after the source deletion.
    p.a.conn()
        .execute(
            "UPDATE qa_turns SET question='Updated historical title',evidence_json=?1",
            [serde_json::to_string(&pack).unwrap()],
        )
        .unwrap();
    p.send_a();
    assert!(crate::db::source_lifecycle::ensure_live(p.b.conn(), &pack).is_err());
    let stored: String =
        p.b.conn()
            .query_row("SELECT evidence_json FROM qa_turns", [], |r| r.get(0))
            .unwrap();
    let stored: EvidencePack = serde_json::from_str(&stored).unwrap();
    assert_eq!(stored.sources[0].trust, "deleted");
    assert!(stored.sources[0].text.is_empty());
}

#[test]
fn unchanged_publication_reuses_existing_snapshot() {
    let p = Pair::new();
    p.seed();
    let first = publish_state(p.a.conn(), &p.left, "A", "dataset", "generation").unwrap();
    let second = publish_state(p.a.conn(), &p.left, "A", "dataset", "generation").unwrap();
    assert_eq!(first, second);
}

#[test]
fn reused_device_sequence_with_different_content_is_rejected() {
    use sha2::{Digest, Sha256};
    let p = Pair::new();
    p.seed();
    let state = publish_state(p.a.conn(), &p.left, "A", "dataset", "generation").unwrap();
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&state).unwrap()).unwrap();
    value["records"][0]["row"]["title"] = "Conflicting device clone".into();
    let bytes = serde_json::to_vec(&value).unwrap();
    let name = format!(
        "state-{:020}-{:x}.json",
        value["sequence"].as_u64().unwrap(),
        Sha256::digest(&bytes)
    );
    std::fs::write(state.parent().unwrap().join(name), bytes).unwrap();
    mirror(&p.left, &p.right);
    let error = receive_states(p.b.conn(), &p.right, "B", "dataset", "generation").unwrap_err();
    assert_eq!(error.code, "SYNC_DEVICE_SEQUENCE_CONFLICT");
    assert_eq!(
        p.b.conn()
            .query_row("SELECT title FROM works", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "Shared"
    );
}

#[test]
fn old_owned_checkpoints_are_bounded_and_latest_still_bootstraps_offline_peer() {
    let p = Pair::new();
    p.seed();
    for i in 0..12 {
        p.a.conn()
            .execute(
                "UPDATE works SET title=?1 WHERE id=1",
                [format!("Revision {i}")],
            )
            .unwrap();
        publish_state(p.a.conn(), &p.left, "A", "dataset", "generation").unwrap();
    }
    let states = p.left.join("generations/generation/devices/A/states");
    let count = std::fs::read_dir(&states)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().starts_with("state-"))
        .count();
    assert!(
        count <= 8,
        "History must not grow for every scheduled change: {count}"
    );
    p.send_a();
    assert_eq!(
        p.b.conn()
            .query_row("SELECT title FROM works WHERE id=1", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        "Revision 11"
    );
}

#[test]
fn folder_direction_replaces_active_baseline_and_preserves_recovery() {
    let _guard = crate::sync_contract_tests::SERVICE_TEST_LOCK
        .lock()
        .unwrap();
    let p = Pair::new();
    p.a.conn()
        .execute(
            "INSERT INTO works(title,status,created_at,updated_at) VALUES('Cloud','active',1,1)",
            [],
        )
        .unwrap();
    p.b.conn()
        .execute(
            "INSERT INTO works(title,status,created_at,updated_at) VALUES('Local','active',1,1)",
            [],
        )
        .unwrap();
    let data_a = p.root.join("a/data");
    let data_b = p.root.join("b/data");
    let cloud_a = p.root.join("initial-a");
    let cloud_b = p.root.join("initial-b");
    std::fs::create_dir_all(&cloud_a).unwrap();
    std::fs::create_dir_all(&cloud_b).unwrap();
    connect(&data_a, &cloud_a, ConnectMode::InitializeFromLocal).unwrap();
    mirror(&cloud_a, &cloud_b);
    connect(&data_b, &cloud_b, ConnectMode::UseFolder).unwrap();
    let titles: Vec<String> =
        p.b.conn()
            .prepare("SELECT title FROM works ORDER BY title")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
    assert_eq!(titles, vec!["Cloud"]);
    assert!(data_b.join("sync-recovery").is_dir());
}

#[test]
fn remote_update_does_not_resurrect_deleted_project() {
    let p = Pair::new();
    p.seed();
    p.send_b();
    p.a.conn()
        .execute("DELETE FROM works WHERE id=1", [])
        .unwrap();
    p.b.conn()
        .execute("UPDATE works SET title='Late edit' WHERE id=1", [])
        .unwrap();
    p.send_b();
    assert_eq!(
        p.a.conn()
            .query_row("SELECT count(*) FROM works", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert!(!super::rows::list_conflicts(p.a.conn()).unwrap().is_empty());
}

#[test]
fn explicitly_restored_deletion_propagates_to_the_other_device() {
    let p = Pair::new();
    p.seed();
    p.send_b();
    p.a.conn()
        .execute(
            "UPDATE sync_local_state SET device_id='A',dataset_id='dataset'",
            [],
        )
        .unwrap();
    p.b.conn()
        .execute(
            "UPDATE sync_local_state SET device_id='B',dataset_id='dataset'",
            [],
        )
        .unwrap();
    p.a.conn()
        .execute("DELETE FROM works WHERE id=1", [])
        .unwrap();
    p.b.conn()
        .execute("UPDATE works SET title='Keep this edit' WHERE id=1", [])
        .unwrap();
    p.send_b();
    p.send_a();
    let conflicts = super::rows::list_conflicts(p.a.conn()).unwrap();
    let deletion = conflicts.iter().find(|c| c.field == "__deleted").unwrap();
    super::rows::resolve_conflict(p.a.conn(), &deletion.id, "remote").unwrap();
    p.send_a();
    p.send_b();
    assert_eq!(
        p.a.conn()
            .query_row("SELECT count(*) FROM works", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        p.b.conn()
            .query_row("SELECT count(*) FROM works", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn project_deletion_preserves_remote_actionable_children() {
    let p = Pair::new();
    p.seed();
    p.a.conn()
        .execute(
            "INSERT INTO tasks(work_id,title,created_at,updated_at) VALUES(1,'Shared child',1,1)",
            [],
        )
        .unwrap();
    p.send_a();
    p.b.conn()
        .execute(
            "INSERT INTO tasks(work_id,title,created_at,updated_at) VALUES(1,'Offline child',1,1)",
            [],
        )
        .unwrap();
    crate::db::work::WorkRepo::new(p.a.conn())
        .delete(1)
        .unwrap();
    p.send_a();
    assert_eq!(
        p.b.conn()
            .query_row("SELECT count(*) FROM works", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        p.b.conn()
            .query_row(
                "SELECT count(*) FROM tasks WHERE work_id IS NULL",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        2
    );
}

#[test]
fn committed_edit_is_journaled_at_write_time_and_rollback_leaves_no_event() {
    let p = Pair::new();
    p.seed();
    p.a.conn()
        .execute(
            "UPDATE sync_local_state SET dataset_id='dataset',device_id='A' WHERE id=1",
            [],
        )
        .unwrap();
    p.a.conn()
        .execute_batch(
            "BEGIN IMMEDIATE; UPDATE works SET title='Rolled back' WHERE id=1; ROLLBACK;",
        )
        .unwrap();
    assert_eq!(
        p.a.conn()
            .query_row("SELECT count(*) FROM sync_pending_edits", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    p.a.conn()
        .execute("UPDATE works SET title='Committed' WHERE id=1", [])
        .unwrap();
    assert!(
        p.a.conn()
            .query_row("SELECT count(*) FROM sync_pending_edits", [], |r| r
                .get::<_, i64>(0))
            .unwrap()
            > 0
    );
    // A fixed fixture time makes it impossible for publication time to pass this assertion.
    p.a.conn()
        .execute("UPDATE sync_pending_edits SET occurred_at_ms=123456789", [])
        .unwrap();
    publish_state(p.a.conn(), &p.left, "A", "dataset", "generation").unwrap();
    let fields:String=p.a.conn().query_row("SELECT field_versions_json FROM sync_row_versions WHERE table_name='works' AND row_key='1'",[],|r|r.get(0)).unwrap();
    let fields: serde_json::Value = serde_json::from_str(&fields).unwrap();
    assert_eq!(fields["title"]["edited_at_ms"], 123456789);
    assert_eq!(
        p.a.conn()
            .query_row("SELECT count(*) FROM sync_pending_edits", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn large_dataset_survives_chunked_transfer_without_truncation() {
    let p = Pair::new();
    let tx = crate::db::write_transaction(p.a.conn()).unwrap();
    let summary = "Synthetic detail. ".repeat(1600);
    for n in 0..400 {
        tx.execute("INSERT INTO works(title,status,summary,created_at,updated_at) VALUES(?1,'active',?2,1,1)",rusqlite::params![format!("Project {n}"),summary]).unwrap();
    }
    tx.commit().unwrap();
    p.send_a();
    assert_eq!(
        p.b.conn()
            .query_row("SELECT count(*) FROM works", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        400
    );
    let last: String =
        p.b.conn()
            .query_row(
                "SELECT summary FROM works WHERE title='Project 399'",
                [],
                |r| r.get(0),
            )
            .unwrap();
    assert_eq!(last, summary);
}

#[test]
fn manifest_cannot_escape_its_generation_directory() {
    let p = Pair::new();
    let manifest = serde_json::json!({"application":"MSLDesktop","format_version":1,"dataset_id":"dataset","generation":"../../outside","created_at":0,"ai_primary_device":"A"});
    std::fs::write(p.left.join("msl-workspace.json"), manifest.to_string()).unwrap();
    assert!(super::protocol::probe_directory(p.left.parent().unwrap()).is_err());
}

#[test]
fn failed_initial_upload_does_not_publish_an_active_manifest() {
    let _guard = crate::sync_contract_tests::SERVICE_TEST_LOCK
        .lock()
        .unwrap();
    let p = Pair::new();
    let hash = "a".repeat(64);
    p.a.conn().execute("INSERT INTO kol_experts(name,institution,created_at,updated_at) VALUES('Synthetic','Synthetic',1,1)",[]).unwrap();
    p.a.conn().execute("INSERT INTO material_blobs(hash,relative_path,byte_size,media_type,created_at) VALUES(?1,?2,10,'text/plain',1)",rusqlite::params![hash,format!("{hash}.txt")]).unwrap();
    p.a.conn().execute("INSERT INTO kol_materials(expert_id,blob_hash,filename,created_at) VALUES(1,?1,'missing-synthetic.txt',1)",[hash]).unwrap();
    let folder = p.root.join("failed-initial");
    std::fs::create_dir_all(&folder).unwrap();
    assert!(connect(
        &p.root.join("a/data"),
        &folder,
        ConnectMode::InitializeFromLocal
    )
    .is_err());
    assert!(!folder.join("MSLDesktop.sync/msl-workspace.json").exists());
}

#[test]
fn incomplete_baseline_switch_stops_before_overwriting_database_identity() {
    let _guard = crate::sync_contract_tests::SERVICE_TEST_LOCK
        .lock()
        .unwrap();
    let p = Pair::new();
    let folder = p.root.join("baseline-safety");
    std::fs::create_dir_all(&folder).unwrap();
    let data = p.root.join("a/data");
    connect(&data, &folder, ConnectMode::InitializeFromLocal).unwrap();
    // Simulate a process interruption after committing a new DB baseline but before its config.
    p.a.conn()
        .execute(
            "UPDATE sync_local_state SET generation='uncommitted-config-generation' WHERE id=1",
            [],
        )
        .unwrap();
    let error = super::service::run(&data).unwrap_err();
    assert_eq!(error.code, "SYNC_LOCAL_BASELINE_CHANGED");
    assert_eq!(
        p.a.conn()
            .query_row(
                "SELECT generation FROM sync_local_state WHERE id=1",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "uncommitted-config-generation"
    );
}
