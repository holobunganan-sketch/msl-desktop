use super::store::*;
use crate::db::Database;
use serde_json::{json, Value};

fn setup() -> (Database, Binding) {
    let db = Database::open_in_memory().unwrap();
    let binding = Binding {
        bot_id: "test-bot".into(),
        owner_id: "test-owner".into(),
        base_url: "https://ilinkai.weixin.qq.com".into(),
        credential_ref: "test-only-unused".into(),
        bound_at: 1000,
        enabled: true,
        cursor: String::new(),
    };
    if !db
        .conn()
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name='weixin_binding')",
            [],
            |r| r.get::<_, bool>(0),
        )
        .unwrap()
    {
        db.conn()
            .execute_batch(include_str!("../../migrations/0027_weixin_bridge.sql"))
            .unwrap();
    }
    db.conn().execute("INSERT INTO weixin_binding(id,bot_id,owner_id,base_url,credential_ref,bound_at,enabled) VALUES(1,'test-bot','test-owner','https://ilinkai.weixin.qq.com','test-only-unused',1000,1)",[]).unwrap();
    (db, binding)
}

#[test]
fn synchronized_capture_does_not_transfer_the_device_binding_or_receipt() {
    let (a, binding) = setup();
    let b = Database::open_in_memory().unwrap();
    let root = std::env::temp_dir().join(format!("msl-weixin-sync-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    ingest_batch(
        a.conn(),
        &binding,
        &[text(11, "Synthetic phone capture")],
        "private-cursor",
        1001,
    )
    .unwrap();
    crate::sync::rows::publish_state(a.conn(), &root, "A", "dataset", "generation").unwrap();
    crate::sync::rows::receive_states(b.conn(), &root, "B", "dataset", "generation").unwrap();
    assert_eq!(
        crate::db::inbox::InboxRepo::new(b.conn()).list().unwrap()[0].content,
        "Synthetic phone capture"
    );
    for table in ["weixin_binding", "weixin_receipts"] {
        assert_eq!(
            b.conn()
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            a.conn()
                .query_row(
                    "SELECT COUNT(*) FROM sync_entities WHERE table_name=?1",
                    [table],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
    }
    assert!(super::store::binding(a.conn()).unwrap().is_some());
    std::fs::remove_dir_all(root).unwrap();
}
fn text(id: u64, content: &str) -> Value {
    json!({"message_id":id,"from_user_id":"test-owner","to_user_id":"test-bot","create_time_ms":1_001_000,"message_type":1,"message_state":2,"context_token":"synthetic-context","item_list":[{"type":1,"text_item":{"text":content}}]})
}

// A missing inbox insert or text normalization must break this assertion.
#[test]
fn owner_text_is_captured_verbatim_once() {
    let (db, binding) = setup();
    let original = "  合成材料\n保留原话  ";
    let message = text(1, original);
    ingest_batch(db.conn(), &binding, &[message.clone()], "cursor-1", 1002).unwrap();
    ingest_batch(db.conn(), &binding, &[message], "cursor-1", 1003).unwrap();
    let rows = crate::db::inbox::InboxRepo::new(db.conn()).list().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].content, original);
}

// An inferred command would create an AI job from this ordinary capture.
#[test]
fn only_exact_command_enqueues_global_work() {
    let (db, binding) = setup();
    ingest_batch(
        db.conn(),
        &binding,
        &[text(1, "请全局交给秘书整理一遍，谢谢")],
        "a",
        1002,
    )
    .unwrap();
    assert_eq!(crate::db::jobs::list(db.conn()).unwrap().len(), 0);
    ingest_batch(
        db.conn(),
        &binding,
        &[text(2, "全局交给秘书整理一遍")],
        "b",
        1003,
    )
    .unwrap();
    let jobs = crate::db::jobs::list(db.conn()).unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].command, "run_analysis_now");
    assert_eq!(jobs[0].args, json!({"trigger":"manual"}));
    assert_eq!(
        crate::db::inbox::InboxRepo::new(db.conn())
            .list()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn untrusted_and_nontext_messages_have_no_effects() {
    let (db, binding) = setup();
    let mut messages = vec![];
    for (field, value) in [
        ("from_user_id", json!("stranger")),
        ("group_id", json!("group")),
        ("to_user_id", json!("other-bot")),
        ("message_type", json!(2)),
        ("message_state", json!(1)),
        ("create_time_ms", json!(999_000)),
        ("create_time_ms", json!(9_999_000)),
    ] {
        let mut m = text(messages.len() as u64 + 1, "全局交给秘书整理一遍");
        m[field] = value;
        messages.push(m);
    }
    let mut media = text(20, "ignored");
    media["item_list"] = json!([{"type":3,"voice_item":{"text":"全局交给秘书整理一遍"}}]);
    messages.push(media);
    ingest_batch(db.conn(), &binding, &messages, "processed", 1002).unwrap();
    assert!(crate::db::jobs::list(db.conn()).unwrap().is_empty());
    assert!(crate::db::inbox::InboxRepo::new(db.conn())
        .list()
        .unwrap()
        .is_empty());
    let cursor: String = db
        .conn()
        .query_row("SELECT cursor FROM weixin_binding", [], |r| r.get(0))
        .unwrap();
    assert_eq!(cursor, "processed");
}

#[test]
fn capture_and_cursor_roll_back_together_on_receipt_failure() {
    let (db, binding) = setup();
    db.conn().execute_batch("CREATE TRIGGER receipt_failure BEFORE INSERT ON weixin_receipts BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap();
    assert!(ingest_batch(
        db.conn(),
        &binding,
        &[text(1, "synthetic")],
        "advanced",
        1002
    )
    .is_err());
    assert!(crate::db::inbox::InboxRepo::new(db.conn())
        .list()
        .unwrap()
        .is_empty());
    let cursor: String = db
        .conn()
        .query_row("SELECT cursor FROM weixin_binding", [], |r| r.get(0))
        .unwrap();
    assert_eq!(cursor, "");
}

#[test]
fn replay_after_job_recovery_cannot_start_global_again() {
    let (db, binding) = setup();
    let message = text(1, "全局交给秘书整理一遍");
    assert_eq!(
        ingest_batch(db.conn(), &binding, &[message.clone()], "a", 1002)
            .unwrap()
            .len(),
        1
    );
    crate::db::jobs::recover(db.conn()).unwrap();
    assert!(ingest_batch(db.conn(), &binding, &[message], "a", 1100)
        .unwrap()
        .is_empty());
    let jobs = crate::db::jobs::list(db.conn()).unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].status, "interrupted");
}

#[test]
fn global_commands_are_rate_limited_without_extra_jobs() {
    let (db, binding) = setup();
    ingest_batch(
        db.conn(),
        &binding,
        &[text(1, "全局交给秘书整理一遍")],
        "a",
        1002,
    )
    .unwrap();
    let first = crate::db::jobs::list(db.conn()).unwrap();
    assert_eq!(first.len(), 1);
    crate::db::jobs::finish(db.conn(), first[0].id, Ok(json!(1))).unwrap();
    let receipts = ingest_batch(
        db.conn(),
        &binding,
        &[text(2, "全局交给秘书整理一遍")],
        "b",
        1003,
    )
    .unwrap();
    assert_eq!(receipts.len(), 1);
    assert!(receipts[0].job.is_none());
    assert_eq!(crate::db::jobs::list(db.conn()).unwrap().len(), 1);
}

#[test]
fn disabled_binding_cannot_capture_or_advance_cursor() {
    let (db, binding) = setup();
    db.conn()
        .execute("UPDATE weixin_binding SET enabled=0", [])
        .unwrap();
    assert!(
        ingest_batch(db.conn(), &binding, &[text(1, "synthetic")], "next", 1002)
            .unwrap()
            .is_empty()
    );
    assert!(crate::db::inbox::InboxRepo::new(db.conn())
        .list()
        .unwrap()
        .is_empty());
    let cursor: String = db
        .conn()
        .query_row("SELECT cursor FROM weixin_binding", [], |r| r.get(0))
        .unwrap();
    assert_eq!(cursor, "");
}

#[test]
fn global_job_and_cursor_roll_back_if_receipt_cannot_be_saved() {
    let (db, binding) = setup();
    db.conn().execute_batch("CREATE TRIGGER receipt_failure BEFORE INSERT ON weixin_receipts BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap();
    assert!(ingest_batch(
        db.conn(),
        &binding,
        &[text(1, "全局交给秘书整理一遍")],
        "advanced",
        1002
    )
    .is_err());
    assert!(crate::db::jobs::list(db.conn()).unwrap().is_empty());
    let cursor: String = db
        .conn()
        .query_row("SELECT cursor FROM weixin_binding", [], |r| r.get(0))
        .unwrap();
    assert_eq!(cursor, "");
}

#[test]
fn uint64_string_id_and_optional_recipient_are_supported_without_losing_deduplication() {
    let (db, binding) = setup();
    let mut message = text(1, "合成原话");
    message["message_id"] = json!("18446744073709551615");
    message.as_object_mut().unwrap().remove("to_user_id");
    ingest_batch(db.conn(), &binding, &[message.clone()], "a", 1002).unwrap();
    message["message_id"] = json!(u64::MAX);
    ingest_batch(db.conn(), &binding, &[message], "b", 1003).unwrap();
    let items = crate::db::inbox::InboxRepo::new(db.conn()).list().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].content, "合成原话");
}

#[test]
fn processed_weixin_capture_keeps_original_user_direction() {
    let (db, binding) = setup();
    ingest_batch(
        db.conn(),
        &binding,
        &[text(1, "我决定等待总部回复")],
        "a",
        1002,
    )
    .unwrap();
    let items = crate::db::inbox::InboxRepo::new(db.conn()).list().unwrap();
    assert_eq!(items.len(), 1);
    db.conn()
        .execute(
            "UPDATE inbox_items SET processed_at=1003 WHERE id=?1",
            [items[0].id],
        )
        .unwrap();
    let (directions, _) = crate::ai::analysis_snapshot::scoped_user_directions(&db, None).unwrap();
    assert!(directions
        .iter()
        .any(|v| v["content"] == "我决定等待总部回复"));
}
