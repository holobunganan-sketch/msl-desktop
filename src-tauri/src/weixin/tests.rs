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

// A restart must retain a current menu and message tombstones on disk.
#[test]
fn menu_state_and_deduplication_survive_database_reopen() {
    let root =
        std::env::temp_dir().join(format!("msl-weixin-menu-reopen-{}", uuid::Uuid::new_v4()));
    let path = root.join("menu.db");
    let (db, binding) = setup_database(Database::open(&path).unwrap());
    let opening = [text(1, "召唤秘书"), text(2, "普通文字直接记录")];
    ingest_batch(db.conn(), &binding, &opening, "menu-open", 1002).unwrap();
    db.close().unwrap();
    let db = Database::open(&path).unwrap();
    let current = super::store::binding(db.conn()).unwrap().unwrap();
    assert_eq!(current.cursor, "menu-open");
    assert!(
        ingest_batch(db.conn(), &current, &opening, "menu-open", 1003)
            .unwrap()
            .is_empty()
    );
    let choice = text(3, "一");
    let replies = ingest_batch(db.conn(), &current, &[choice.clone()], "selected", 1004).unwrap();
    assert_eq!(replies[0].job.as_ref().unwrap().command, "run_analysis_now");
    db.close().unwrap();
    let db = Database::open(&path).unwrap();
    let current = super::store::binding(db.conn()).unwrap().unwrap();
    assert!(
        ingest_batch(db.conn(), &current, &[choice], "selected", 1005)
            .unwrap()
            .is_empty()
    );
    let items = crate::db::inbox::InboxRepo::new(db.conn()).list().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].content, "普通文字直接记录");
    assert_eq!(crate::db::jobs::list(db.conn()).unwrap().len(), 1);
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

fn text_at(id: u64, content: &str, created_ms: i64) -> Value {
    let mut message = text(id, content);
    message["create_time_ms"] = json!(created_ms);
    message
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

// Every ordinary text, including the old command, must create a literal capture.
#[test]
fn ordinary_text_and_old_command_are_captured_directly() {
    for open_menu in [false, true] {
        let (db, binding) = setup();
        if open_menu {
            ingest_batch(db.conn(), &binding, &[text(1, "召唤秘书")], "menu", 1001).unwrap();
        }
        for (index, content) in [
            "给张医生发学术资料",
            " 全局交给秘书整理一遍\n",
            "全局交给秘书整理一遍",
            "请全局交给秘书整理一遍，谢谢",
            "记一句话",
            "请召唤秘书",
            "召唤秘书。",
            "秘书",
            "6",
            "六",
            "６",
            "1 帮我整理",
            "一、",
            "  原话\n下一行  ",
        ]
        .iter()
        .enumerate()
        {
            let message = text(index as u64 + 2, content);
            let replies =
                ingest_batch(db.conn(), &binding, &[message.clone()], "capture", 1002).unwrap();
            assert_eq!(replies[0].ack, "已记入收件箱。", "content={content}");
            assert!(replies[0].job.is_none());
            let items = crate::db::inbox::InboxRepo::new(db.conn()).list().unwrap();
            assert_eq!(
                items.len(),
                index + 1,
                "content={content}, open_menu={open_menu}"
            );
            assert_eq!(items[0].content, *content);
            assert!(
                ingest_batch(db.conn(), &binding, &[message], "capture", 1003)
                    .unwrap()
                    .is_empty()
            );
        }
        assert_eq!(
            crate::db::inbox::InboxRepo::new(db.conn())
                .list()
                .unwrap()
                .len(),
            14
        );
        assert!(crate::db::jobs::list(db.conn()).unwrap().is_empty());
        assert_eq!(db.conn().query_row("SELECT COUNT(*) FROM weixin_receipts WHERE action IN ('record_next','retired_command')", [], |r| r.get::<_, i64>(0)).unwrap(), 0);
    }
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
    for (offset, choice) in [(1000, "一"), (2000, "二")] {
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
    let message = text(2, "一");
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
        &[text(1, "召唤秘书"), text(2, "一")],
        "a",
        1002,
    )
    .unwrap();
    let first = crate::db::jobs::list(db.conn()).unwrap();
    assert_eq!(first.len(), 1);
    crate::db::jobs::finish(db.conn(), first[0].id, Ok(json!(1))).unwrap();
    let receipts = ingest_batch(db.conn(), &binding, &[text(3, "一")], "b", 1003).unwrap();
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
    assert!(ingest_batch(db.conn(), &binding, &[text(2, "一")], "advanced", 1002).is_err());
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
    for digit in ["1", "一", "１", "  一  "] {
        let (db, binding) = setup();
        let replies =
            ingest_batch(db.conn(), &binding, &[text(1, " 召唤秘书 ")], "open", 1002).unwrap();
        assert!(replies[0].ack.contains(
            "一、全局整理\n二、生成每日简报\n三、查看整理进度\n四、查看待确认数量\n五、退出菜单"
        ));
        assert!(!replies[0].ack.contains("记一句话"));
        assert!(!replies[0].ack.contains("六、"));
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

// Every supported numeral must select the same advertised action.
#[test]
fn all_five_menu_actions_accept_arabic_chinese_and_fullwidth_numerals() {
    for (forms, action, command) in [
        (["1", "一", "１"], "global", Some("run_analysis_now")),
        (["2", "二", "２"], "brief", Some("generate_brief")),
        (["3", "三", "３"], "job_status", None),
        (["4", "四", "４"], "pending_status", None),
        (["5", "五", "５"], "exit_menu", None),
    ] {
        for content in forms {
            let (db, binding) = setup();
            let replies = ingest_batch(
                db.conn(),
                &binding,
                &[text(1, "召唤秘书"), text(2, content)],
                "choice",
                1002,
            )
            .unwrap();
            assert_eq!(
                replies[1].job.as_ref().map(|job| job.command.as_str()),
                command,
                "choice={content}"
            );
            assert_eq!(
                db.conn()
                    .query_row(
                        "SELECT action FROM weixin_receipts WHERE message_id='2'",
                        [],
                        |r| r.get::<_, String>(0)
                    )
                    .unwrap(),
                action
            );
            assert!(crate::db::inbox::InboxRepo::new(db.conn())
                .list()
                .unwrap()
                .is_empty());
        }
    }
}

// Missing or expired authorization must save all numeral forms without running any action.
#[test]
fn all_numerals_outside_an_active_menu_are_captured() {
    for state in ["closed", "exited", "expired"] {
        let (db, binding) = setup();
        if state != "closed" {
            ingest_batch(db.conn(), &binding, &[text(1, "召唤秘书")], "open", 1001).unwrap();
        }
        if state == "exited" {
            ingest_batch(db.conn(), &binding, &[text(2, "五")], "exit", 1002).unwrap();
        }
        let now = if state == "expired" { 1601 } else { 1003 };
        for (index, content) in [
            "1", "一", "１", "2", "二", "２", "3", "三", "３", "4", "四", "４", "5", "五", "５",
            "6", "六", "６",
        ]
        .iter()
        .enumerate()
        {
            let replies = ingest_batch(
                db.conn(),
                &binding,
                &[text(index as u64 + 3, content)],
                "capture",
                now,
            )
            .unwrap();
            assert!(replies[0].job.is_none());
            let items = crate::db::inbox::InboxRepo::new(db.conn()).list().unwrap();
            assert_eq!(items.len(), index + 1, "content={content}, state={state}");
            assert_eq!(items[0].content, *content);
        }
        assert_eq!(
            crate::db::inbox::InboxRepo::new(db.conn())
                .list()
                .unwrap()
                .len(),
            18
        );
        assert!(crate::db::jobs::list(db.conn()).unwrap().is_empty());
    }
}

// Upgrading a v29 session must revoke old numbering before any delayed selection is processed.
#[test]
fn upgrade_revokes_old_menu_and_record_sessions_before_processing_choices() {
    for mode in ["menu", "record"] {
        let (mut db, binding) = setup_database(crate::db::test_support::v29_database(
            rusqlite::Connection::open_in_memory().unwrap(),
        ));
        db.conn().execute("UPDATE weixin_binding SET menu_mode=?1,menu_expires_at=1601,menu_after_ms=1001000,menu_after_id='1'", [mode]).unwrap();
        db.conn().execute("INSERT INTO weixin_receipts(bot_id,message_id,received_at,action) VALUES('test-bot','1',1001,'menu')", []).unwrap();
        db.migrate().unwrap();
        assert!(
            ingest_batch(db.conn(), &binding, &[text(1, "召唤秘书")], "replay", 1002)
                .unwrap()
                .is_empty()
        );
        for (index, content) in ["1", "2", "3", "4", "5", "6"].iter().enumerate() {
            ingest_batch(
                db.conn(),
                &binding,
                &[text(index as u64 + 2, content)],
                "after-upgrade",
                1003,
            )
            .unwrap();
            let items = crate::db::inbox::InboxRepo::new(db.conn()).list().unwrap();
            assert_eq!(items.len(), index + 1, "content={content}, mode={mode}");
            assert_eq!(items[0].content, *content);
        }
        assert!(crate::db::jobs::list(db.conn()).unwrap().is_empty());
        let after_switch = db
            .conn()
            .query_row(
                "SELECT menu_protocol_after_ms / 1000 + 1 FROM weixin_binding",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap();
        let replies = ingest_batch(
            db.conn(),
            &binding,
            &[
                text_at(8, "召唤秘书", after_switch * 1000),
                text_at(9, "一", after_switch * 1000),
            ],
            "new-menu",
            after_switch,
        )
        .unwrap();
        assert_eq!(replies[1].job.as_ref().unwrap().command, "run_analysis_now");
    }
}

// A queued old summon must not authorize renumbered actions, even at the accepted clock-skew limit.
#[test]
fn upgrade_blocks_unpolled_summons_including_allowed_positive_clock_skew() {
    for mode in [None, Some("menu"), Some("record")] {
        for offset_ms in [0, 300_999] {
            let (mut db, binding) = setup_database(crate::db::test_support::v29_database(
                rusqlite::Connection::open_in_memory().unwrap(),
            ));
            let sent_at = crate::db::now_unix();
            db.conn().execute("UPDATE weixin_binding SET menu_mode=?1,menu_expires_at=?2,menu_after_ms=?3,menu_after_id='1'", rusqlite::params![mode, sent_at + 600, (sent_at - 120) * 1000]).unwrap();
            db.migrate().unwrap();
            let messages = [
                text_at(2, "召唤秘书", sent_at * 1000 + offset_ms),
                text_at(3, "2", sent_at * 1000 + offset_ms),
                text_at(4, "升级期间的普通文字", sent_at * 1000 + offset_ms),
            ];
            let replies =
                ingest_batch(db.conn(), &binding, &messages, "old-backlog", sent_at + 1).unwrap();
            assert!(
                crate::db::jobs::list(db.conn()).unwrap().is_empty(),
                "mode={mode:?}, offset_ms={offset_ms}"
            );
            assert!(replies[0].ack.contains("升级后的菜单正在切换，请约"));
            assert!(replies[0]
                .ack
                .contains("秒后重新发送“召唤秘书”；普通文字仍会记录"));
            assert!(!replies[0].ack.contains("一、全局整理"));
            let items = crate::db::inbox::InboxRepo::new(db.conn()).list().unwrap();
            assert_eq!(items.len(), 2);
            assert!(items.iter().any(|item| item.content == "2"));
            assert!(items
                .iter()
                .any(|item| item.content == "升级期间的普通文字"));
            assert!(
                ingest_batch(db.conn(), &binding, &messages, "replay", sent_at + 2)
                    .unwrap()
                    .is_empty()
            );
        }
    }
}

// The source-time boundary survives its waiting period; only a later summon opens the new menu.
#[test]
fn upgrade_requires_elapsed_local_wait_even_with_a_fast_source_clock() {
    let (mut db, binding) = setup_database(crate::db::test_support::v29_database(
        rusqlite::Connection::open_in_memory().unwrap(),
    ));
    db.migrate().unwrap();
    let cutoff = db
        .conn()
        .query_row(
            "SELECT menu_protocol_after_ms FROM weixin_binding",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap();
    let after_switch = cutoff / 1000 + 1;
    // A fast source clock must not open the menu while local waiting time remains.
    let replies = ingest_batch(
        db.conn(),
        &binding,
        &[text_at(1, "召唤秘书", cutoff + 1)],
        "wait",
        after_switch - 2,
    )
    .unwrap();
    assert_eq!(
        replies[0].ack,
        "升级后的菜单正在切换，请约2秒后重新发送“召唤秘书”；普通文字仍会记录。"
    );
    assert_eq!(
        db.conn()
            .query_row("SELECT menu_mode FROM weixin_binding", [], |row| row
                .get::<_, Option<String>>(0))
            .unwrap(),
        None
    );
}

#[test]
fn upgrade_requires_source_time_after_boundary_even_after_wait_ends() {
    let (mut db, binding) = setup_database(crate::db::test_support::v29_database(
        rusqlite::Connection::open_in_memory().unwrap(),
    ));
    db.migrate().unwrap();
    let cutoff = db
        .conn()
        .query_row(
            "SELECT menu_protocol_after_ms FROM weixin_binding",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap();
    let after_switch = cutoff / 1000 + 1;
    // Higher IDs at the same millisecond do not turn a cutoff-equal summon into authorization.
    let replies = ingest_batch(
        db.conn(),
        &binding,
        &[
            text_at(2, "召唤秘书", cutoff - 1),
            text_at(3, "召唤秘书", cutoff),
            text_at(4, "2", cutoff),
        ],
        "late",
        after_switch,
    )
    .unwrap();
    assert!(replies.iter().all(|reply| reply.job.is_none()));
    assert!(replies[0].ack.contains("重新发送“召唤秘书”"));
    assert!(!replies[0].ack.contains("约0秒"));
    assert_eq!(
        crate::db::inbox::InboxRepo::new(db.conn()).list().unwrap()[0].content,
        "2"
    );
    let replies = ingest_batch(
        db.conn(),
        &binding,
        &[
            text_at(5, "召唤秘书", cutoff + 1),
            text_at(6, "一", cutoff + 1),
        ],
        "new",
        after_switch,
    )
    .unwrap();
    assert!(replies[0].ack.contains("一、全局整理"));
    assert_eq!(replies[1].job.as_ref().unwrap().command, "run_analysis_now");
}

// Reopening must neither remove nor extend the one-time upgrade boundary.
#[test]
fn upgrade_boundary_survives_database_reopen_without_restarting_wait() {
    let root = std::env::temp_dir().join(format!("msl-weixin-upgrade-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("upgrade.db");
    let (mut db, binding) = setup_database(crate::db::test_support::v29_database(
        rusqlite::Connection::open(&path).unwrap(),
    ));
    db.migrate().unwrap();
    let cutoff = db
        .conn()
        .query_row(
            "SELECT menu_protocol_after_ms FROM weixin_binding",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap();
    db.close().unwrap();
    let db = Database::open(&path).unwrap();
    assert_eq!(
        db.conn()
            .query_row(
                "SELECT menu_protocol_after_ms FROM weixin_binding",
                [],
                |row| row.get::<_, i64>(0)
            )
            .unwrap(),
        cutoff
    );
    let replies = ingest_batch(
        db.conn(),
        &binding,
        &[text_at(1, "召唤秘书", cutoff), text_at(2, "2", cutoff)],
        "late",
        cutoff / 1000 + 1,
    )
    .unwrap();
    assert!(replies.iter().all(|reply| reply.job.is_none()));
    assert_eq!(
        crate::db::inbox::InboxRepo::new(db.conn()).list().unwrap()[0].content,
        "2"
    );
    db.close().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

// Binding revocation/replacement must retain the upgrade cutoff while clearing menu authorization.
#[test]
fn upgrade_boundary_survives_disabled_and_rebound_binding() {
    for change in [
        "UPDATE weixin_binding SET enabled=0; UPDATE weixin_binding SET enabled=1;",
        "UPDATE weixin_binding SET owner_id='replacement'; UPDATE weixin_binding SET owner_id='test-owner';",
        "UPDATE weixin_binding SET bound_at=1001;",
    ] {
        let (mut db, _) = setup_database(crate::db::test_support::v29_database(rusqlite::Connection::open_in_memory().unwrap()));
        db.migrate().unwrap();
        let cutoff = db.conn().query_row("SELECT menu_protocol_after_ms FROM weixin_binding", [], |row| row.get::<_, i64>(0)).unwrap();
        db.conn().execute_batch(change).unwrap();
        assert_eq!(db.conn().query_row("SELECT menu_protocol_after_ms FROM weixin_binding", [], |row| row.get::<_, i64>(0)).unwrap(), cutoff);
        let current = super::store::binding(db.conn()).unwrap().unwrap();
        let replies = ingest_batch(db.conn(), &current, &[text_at(1, "召唤秘书", cutoff), text_at(2, "2", cutoff)], "rebound", cutoff / 1000).unwrap();
        assert!(replies[0].ack.contains("约1秒"));
        assert!(crate::db::jobs::list(db.conn()).unwrap().is_empty());
        assert_eq!(crate::db::inbox::InboxRepo::new(db.conn()).list().unwrap()[0].content, "2");
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
    ingest_batch(db.conn(), &binding, &[text(6, "一")], "job", 1003).unwrap();
    assert_eq!(crate::db::jobs::list(db.conn()).unwrap().len(), 1);
}

// Expired/closed state must stop treating bare numbers as actions.
#[test]
fn expired_or_exited_menu_returns_to_verbatim_capture() {
    for exit in [false, true] {
        let (db, binding) = setup();
        ingest_batch(db.conn(), &binding, &[text(1, "召唤秘书")], "open", 1002).unwrap();
        if exit {
            ingest_batch(db.conn(), &binding, &[text(2, "５")], "exit", 1003).unwrap();
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
        &[text(1, "召唤秘书"), text(2, "二")],
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
        &[text(3, "二"), text(4, "一")],
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
        &[text(1, "召唤秘书"), text(2, "三"), text(3, "四")],
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
    let mut stale = text(9, "一");
    stale["create_time_ms"] = json!(1_001_999);
    let mut choice = text(6, "一");
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
