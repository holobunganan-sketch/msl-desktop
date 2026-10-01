use super::store::*;
use crate::db::Database;
use serde_json::{json, Value};

fn setup() -> (Database, Binding) {
    setup_database(Database::open_in_memory().unwrap())
}

fn setup_database(db: Database) -> (Database, Binding) {
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

// A restart must retain record-once mode and message tombstones on disk.
#[test]
fn menu_record_state_and_deduplication_survive_database_reopen() {
    let root =
        std::env::temp_dir().join(format!("msl-weixin-menu-reopen-{}", uuid::Uuid::new_v4()));
    let path = root.join("menu.db");
    let (db, binding) = setup_database(Database::open(&path).unwrap());
    let opening = [text(1, "召唤秘书"), text(2, "一")];
    ingest_batch(db.conn(), &binding, &opening, "record-next", 1002).unwrap();
    db.close().unwrap();
    let db = Database::open(&path).unwrap();
    let current = super::store::binding(db.conn()).unwrap().unwrap();
    assert_eq!(current.cursor, "record-next");
    assert!(
        ingest_batch(db.conn(), &current, &opening, "record-next", 1003)
            .unwrap()
            .is_empty()
    );
    let literal = text(3, "二");
    ingest_batch(db.conn(), &current, &[literal.clone()], "recorded", 1004).unwrap();
    db.close().unwrap();
    let db = Database::open(&path).unwrap();
    let current = super::store::binding(db.conn()).unwrap().unwrap();
    assert!(
        ingest_batch(db.conn(), &current, &[literal], "recorded", 1005)
            .unwrap()
            .is_empty()
    );
    let items = crate::db::inbox::InboxRepo::new(db.conn()).list().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].content, "二");
    assert!(crate::db::jobs::list(db.conn()).unwrap().is_empty());
    db.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
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

// Retired wording must never enqueue a job; menu selection is the sole command route.
#[test]
fn retired_command_redirects_to_menu_without_starting_work() {
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
    let receipts = ingest_batch(
        db.conn(),
        &binding,
        &[text(2, "全局交给秘书整理一遍")],
        "b",
        1003,
    )
    .unwrap();
    let jobs = crate::db::jobs::list(db.conn()).unwrap();
    assert!(jobs.is_empty());
    assert!(receipts[0].ack.contains("召唤秘书"));
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
        let mut m = text(messages.len() as u64 + 1, "召唤秘书");
        m[field] = value;
        messages.push(m);
    }
    let mut media = text(20, "ignored");
    media["item_list"] = json!([{"type":3,"voice_item":{"text":"召唤秘书"}}]);
    messages.push(media);
    assert!(
        ingest_batch(db.conn(), &binding, &messages, "processed", 1002)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        db.conn()
            .query_row("SELECT menu_mode FROM weixin_binding", [], |row| row
                .get::<_, Option<String>>(0))
            .unwrap(),
        None
    );
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
    ingest_batch(db.conn(), &binding, &[text(100, "召唤秘书")], "open", 1003).unwrap();
    for (offset, choice) in [(1000, "二"), (2000, "三")] {
        let choices: Vec<Value> = messages
            .iter()
            .enumerate()
            .map(|(index, message)| {
                let mut message = message.clone();
                message["message_id"] = json!(offset + index);
                message["item_list"][0]["text_item"]["text"] = json!(choice);
                message
            })
            .collect();
        assert!(ingest_batch(db.conn(), &binding, &choices, "ignored", 1004)
            .unwrap()
            .is_empty());
    }
    assert!(crate::db::jobs::list(db.conn()).unwrap().is_empty());
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
    ingest_batch(db.conn(), &binding, &[text(1, "召唤秘书")], "menu", 1001).unwrap();
    let message = text(2, "二");
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
        &[text(1, "召唤秘书"), text(2, "二")],
        "a",
        1002,
    )
    .unwrap();
    let first = crate::db::jobs::list(db.conn()).unwrap();
    assert_eq!(first.len(), 1);
    crate::db::jobs::finish(db.conn(), first[0].id, Ok(json!(1))).unwrap();
    let receipts = ingest_batch(db.conn(), &binding, &[text(3, "二")], "b", 1003).unwrap();
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
    ingest_batch(db.conn(), &binding, &[text(1, "召唤秘书")], "menu", 1001).unwrap();
    db.conn().execute_batch("CREATE TRIGGER receipt_failure BEFORE INSERT ON weixin_receipts BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap();
    assert!(ingest_batch(db.conn(), &binding, &[text(2, "二")], "advanced", 1002).is_err());
    assert!(crate::db::jobs::list(db.conn()).unwrap().is_empty());
    let cursor: String = db
        .conn()
        .query_row("SELECT cursor FROM weixin_binding", [], |r| r.get(0))
        .unwrap();
    assert_eq!(cursor, "menu");
}

// Opening a menu must not create a capture or job, and only an exact menu digit starts work.
#[test]
fn menu_opens_and_selects_existing_global_job_with_all_digit_forms() {
    for digit in ["2", "二", "２", "  二  "] {
        let (db, binding) = setup();
        let replies =
            ingest_batch(db.conn(), &binding, &[text(1, " 召唤秘书 ")], "open", 1002).unwrap();
        assert!(replies[0].ack.contains("一"));
        assert!(replies[0].ack.contains("六"));
        assert!(crate::db::inbox::InboxRepo::new(db.conn())
            .list()
            .unwrap()
            .is_empty());
        let replies = ingest_batch(db.conn(), &binding, &[text(2, digit)], "run", 1003).unwrap();
        let job = replies[0].job.as_ref().expect("menu choice starts a job");
        assert_eq!(job.command, "run_analysis_now");
        assert_eq!(job.args, json!({"trigger":"manual"}));
    }
}

// Losing the record-once state would interpret the next literal as a menu action.
#[test]
fn menu_record_once_preserves_command_words_and_digits_as_literal_text() {
    for content in [" ２ ", "全局交给秘书整理一遍", "召唤秘书", "六"] {
        let (db, binding) = setup();
        ingest_batch(
            db.conn(),
            &binding,
            &[text(1, "召唤秘书"), text(2, "一")],
            "record",
            1002,
        )
        .unwrap();
        let reopened_binding = super::store::binding(db.conn()).unwrap().unwrap();
        ingest_batch(
            db.conn(),
            &reopened_binding,
            &[text(3, content)],
            "recorded",
            1003,
        )
        .unwrap();
        let items = crate::db::inbox::InboxRepo::new(db.conn()).list().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].content, content);
        assert!(crate::db::jobs::list(db.conn()).unwrap().is_empty());
        ingest_batch(db.conn(), &binding, &[text(4, "二")], "after-record", 1004).unwrap();
        assert_eq!(crate::db::jobs::list(db.conn()).unwrap().len(), 1);
    }
}

// Free text must keep its original bytes, including near matches and out-of-menu numbers.
#[test]
fn ordinary_text_and_non_exact_choices_are_captured_without_losing_menu() {
    let (db, binding) = setup();
    for (id, content) in [
        (1, "2"),
        (2, "请召唤秘书"),
        (3, "召唤秘书"),
        (4, " 二、 "),
        (5, "  原话\n下一行  "),
    ] {
        ingest_batch(db.conn(), &binding, &[text(id, content)], "capture", 1002).unwrap();
    }
    let mut contents: Vec<String> = crate::db::inbox::InboxRepo::new(db.conn())
        .list()
        .unwrap()
        .into_iter()
        .map(|item| item.content)
        .collect();
    contents.sort();
    let mut expected = vec!["2", "请召唤秘书", " 二、 ", "  原话\n下一行  "];
    expected.sort();
    assert_eq!(contents, expected);
    assert!(crate::db::jobs::list(db.conn()).unwrap().is_empty());
    ingest_batch(db.conn(), &binding, &[text(6, "二")], "job", 1003).unwrap();
    assert_eq!(crate::db::jobs::list(db.conn()).unwrap().len(), 1);
}

// Expired/closed state must stop treating bare numbers as actions.
#[test]
fn expired_or_exited_menu_returns_to_verbatim_capture() {
    for exit in [false, true] {
        let (db, binding) = setup();
        ingest_batch(db.conn(), &binding, &[text(1, "召唤秘书")], "open", 1002).unwrap();
        if exit {
            ingest_batch(db.conn(), &binding, &[text(2, "６")], "exit", 1003).unwrap();
        }
        ingest_batch(
            db.conn(),
            &binding,
            &[text(3, "二")],
            "after",
            if exit { 1004 } else { 1602 },
        )
        .unwrap();
        let items = crate::db::inbox::InboxRepo::new(db.conn()).list().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].content, "二");
        assert!(crate::db::jobs::list(db.conn()).unwrap().is_empty());
    }
}

// Receiving a backlog must not grant another ten minutes to hours-old authorization.
#[test]
fn offline_backlog_cannot_reopen_expired_menu_or_start_work() {
    let (db, binding) = setup();
    let replies = ingest_batch(
        db.conn(),
        &binding,
        &[
            text(1, "召唤秘书"),
            text(2, "二"),
            text(3, "离线期间的原话"),
        ],
        "backlog",
        9000,
    )
    .unwrap();
    assert!(crate::db::jobs::list(db.conn()).unwrap().is_empty());
    assert!(replies[0].ack.contains("过期"));
    let items = crate::db::inbox::InboxRepo::new(db.conn()).list().unwrap();
    assert_eq!(items.len(), 2);
    assert!(items.iter().any(|item| item.content == "离线期间的原话"));
}

// A late-arriving summon expires ten minutes after its source time, not its receipt time.
#[test]
fn delayed_summon_does_not_extend_the_menu_lifetime() {
    let (db, binding) = setup();
    ingest_batch(db.conn(), &binding, &[text(1, "召唤秘书")], "delayed", 1500).unwrap();
    let mut selection = text(2, "二");
    selection["create_time_ms"] = json!(1_601_000);
    ingest_batch(db.conn(), &binding, &[selection], "expired", 1601).unwrap();
    assert!(crate::db::jobs::list(db.conn()).unwrap().is_empty());
    assert_eq!(
        crate::db::inbox::InboxRepo::new(db.conn()).list().unwrap()[0].content,
        "二"
    );
}

// Daily brief selection must enqueue the existing brief command with complete, valid local-day inputs.
#[test]
fn brief_choice_enqueues_brief_and_uses_its_own_cooldown() {
    let (db, binding) = setup();
    let receipts = ingest_batch(
        db.conn(),
        &binding,
        &[text(1, "召唤秘书"), text(2, "三")],
        "brief",
        1002,
    )
    .unwrap();
    let job = receipts[1].job.as_ref().expect("brief job");
    assert_eq!(job.command, "generate_brief");
    let request: crate::commands::jobs::JobRequest =
        serde_json::from_value(json!({"command":job.command,"args":job.args})).unwrap();
    assert!(matches!(
        request,
        crate::commands::jobs::JobRequest::GenerateBrief {
            force: Some(false),
            ..
        }
    ));
    assert_eq!(job.args["locale"], "zh-CN");
    assert_eq!(job.args["periodEnd"], job.args["todayStart"]);
    assert!(job.args["periodStart"].as_i64().unwrap() < job.args["periodEnd"].as_i64().unwrap());
    assert!(job.args["todayStart"].as_i64().unwrap() < job.args["todayEnd"].as_i64().unwrap());
    crate::db::jobs::finish(db.conn(), job.id, Ok(json!({"content":"private-content"}))).unwrap();
    let replies = ingest_batch(
        db.conn(),
        &binding,
        &[text(3, "三"), text(4, "二")],
        "cooldown",
        1003,
    )
    .unwrap();
    assert!(replies[0].job.is_none());
    assert_eq!(replies[1].job.as_ref().unwrap().command, "run_analysis_now");
    assert_eq!(crate::db::jobs::list(db.conn()).unwrap().len(), 2);
}

// Status queries may expose only aggregate counts; job args, errors and proposal text stay local.
#[test]
fn status_and_pending_choices_return_counts_without_work_content() {
    let (db, binding) = setup();
    let (job, _) = crate::db::jobs::start(
        db.conn(),
        "synthetic",
        &json!({"input":"private-work-content"}),
    )
    .unwrap();
    crate::db::jobs::finish(db.conn(), job.id, Err("private-work-content".into())).unwrap();
    crate::db::jobs::start(
        db.conn(),
        "synthetic",
        &json!({"input":"private-work-content"}),
    )
    .unwrap();
    db.conn().execute("INSERT INTO ai_proposals(kind,operation,dedupe_key,title,payload_json,status,created_at,updated_at) VALUES('task','create','test-count','private-work-content','{}','pending',1000,1000)", []).unwrap();
    let replies = ingest_batch(
        db.conn(),
        &binding,
        &[text(1, "召唤秘书"), text(2, "四"), text(3, "五")],
        "counts",
        1002,
    )
    .unwrap();
    assert!(replies[1].ack.contains("进行中 1"));
    assert!(replies[1].ack.contains("失败 1"));
    assert!(replies[2].ack.contains("待确认 1"));
    for receipt in replies {
        assert!(!receipt.ack.contains("private-work-content"));
        assert!(receipt.job.is_none());
    }
    assert_eq!(crate::db::jobs::list(db.conn()).unwrap().len(), 2);
    assert!(crate::db::inbox::InboxRepo::new(db.conn())
        .list()
        .unwrap()
        .is_empty());
}

// A stale digit or command cannot consume/reset a menu created by a newer source message.
#[test]
fn stale_messages_do_not_consume_newer_menu_even_in_one_batch() {
    let (db, binding) = setup();
    let mut open = text(5, "召唤秘书");
    open["create_time_ms"] = json!(1_002_000);
    let mut stale = text(9, "二");
    stale["create_time_ms"] = json!(1_001_999);
    let mut choice = text(6, "二");
    choice["create_time_ms"] = json!(1_002_000);
    let replies = ingest_batch(
        db.conn(),
        &binding,
        &[open, stale, text(4, "一"), choice],
        "ordered",
        1003,
    )
    .unwrap();
    assert!(replies[1].job.is_none());
    assert!(replies[2].job.is_none());
    assert_eq!(replies[3].job.as_ref().unwrap().command, "run_analysis_now");
    let items = crate::db::inbox::InboxRepo::new(db.conn()).list().unwrap();
    assert_eq!(items.len(), 2);
}

// A failed receipt must roll back both a newly opened menu and its cursor.
#[test]
fn menu_open_rolls_back_with_receipt_failure() {
    let (db, binding) = setup();
    db.conn().execute_batch("CREATE TRIGGER receipt_failure BEFORE INSERT ON weixin_receipts BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap();
    assert!(ingest_batch(
        db.conn(),
        &binding,
        &[text(1, "召唤秘书")],
        "advanced",
        1002
    )
    .is_err());
    db.conn()
        .execute_batch("DROP TRIGGER receipt_failure;")
        .unwrap();
    ingest_batch(db.conn(), &binding, &[text(2, "二")], "after", 1003).unwrap();
    assert!(crate::db::jobs::list(db.conn()).unwrap().is_empty());
    assert_eq!(
        crate::db::inbox::InboxRepo::new(db.conn()).list().unwrap()[0].content,
        "二"
    );
}

// Disable/re-enable and binding replacement must revoke prior menu authorization.
#[test]
fn disabled_or_rebound_menu_cannot_execute_old_selection() {
    for change in ["UPDATE weixin_binding SET enabled=0; UPDATE weixin_binding SET enabled=1;", "UPDATE weixin_binding SET owner_id='replacement'; UPDATE weixin_binding SET owner_id='test-owner';", "UPDATE weixin_binding SET bound_at=1001;"] {
        let (db, binding) = setup();
        ingest_batch(db.conn(), &binding, &[text(1, "召唤秘书")], "open", 1002).unwrap();
        db.conn().execute_batch(change).unwrap();
        let current = super::store::binding(db.conn()).unwrap().unwrap();
        ingest_batch(db.conn(), &current, &[text(2, "二")], "after", 1003).unwrap();
        assert!(crate::db::jobs::list(db.conn()).unwrap().is_empty());
        assert_eq!(crate::db::inbox::InboxRepo::new(db.conn()).list().unwrap().len(), 1);
    }
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
