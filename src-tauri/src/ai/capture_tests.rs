use super::{analysis, analysis_snapshot, apply, receipts};
use crate::db::{ai::ProposalRepo, inbox::InboxRepo, kol, Database};
use serde_json::{json, Value};

fn setup() -> (Database, i64, i64) {
    let db = Database::open_in_memory().unwrap();
    let expert = kol::save_expert(
        &db,
        None,
        None,
        "陈医生",
        "合成医院",
        Some("肾内科"),
        "",
        &[],
        false,
    )
    .unwrap();
    let inbox = InboxRepo::new(db.conn())
        .insert("合成医院陈医生希望补充长期随访证据，请加入专家洞察")
        .unwrap();
    (db, expert["id"].as_i64().unwrap(), inbox.id)
}
fn snapshot(db: &Database) -> analysis_snapshot::AnalysisSnapshot {
    analysis_snapshot::build(
        db,
        "global_analysis",
        "2026-09-30",
        0,
        i64::MAX,
        0,
        i64::MAX,
        "zh-CN",
    )
    .unwrap()
}
fn output(db: &Database, expert: i64, inbox: i64) -> Value {
    let s = serde_json::to_value(snapshot(db)).unwrap();
    let index = s["brief"]["inbox"]
        .as_array()
        .unwrap()
        .iter()
        .position(|v| v["entity_id"] == inbox)
        .unwrap();
    json!({"proposals":[{"kind":"kol_insight","operation":"create","title":"长期证据需求","payload":{"expert_id":expert,"expert_revision":1,"categories":["evidence_need"],"observation":"希望补充长期随访证据","field_evidence":{"observation":{"snapshot_path":format!("/brief/inbox/{index}/content"),"quote":"陈医生希望补充长期随访证据","basis":"explicit"},"expert_id":{"snapshot_path":format!("/brief/inbox/{index}/content"),"quote":"合成医院陈医生希望补充长期随访证据","basis":"explicit"}}},"source_refs":[{"source_type":"inbox","entity_id":inbox}]}]})
}
fn queue(db: &Database, value: &Value) -> crate::db::ai::AiProposal {
    let s = snapshot(db);
    let run = analysis::create_run(db, "manual", 0, i64::MAX).unwrap();
    assert_eq!(
        analysis::apply_output(db, run, &s, &value.to_string()).unwrap(),
        1
    );
    ProposalRepo::new(db.conn())
        .list(Some("pending"), 100)
        .unwrap()
        .into_iter()
        .find(|p| p.analysis_run_id == Some(run))
        .unwrap()
}

#[test]
fn unsupported_expert_attribution_keeps_answer_and_queues_valid_peer_only() {
    let (db, expert, inbox) = setup();
    let mut value = output(&db, expert, inbox);
    value["proposals"][0]["payload"]["field_evidence"]["expert_id"]["snapshot_path"] =
        json!("/expert_catalog/0/name");
    value["proposals"].as_array_mut().unwrap().push(json!({"kind":"task","operation":"create","title":"核对长期随访资料","payload":{},"source_refs":[{"source_type":"inbox","entity_id":inbox}]}));
    let raw = value.to_string();
    let run = analysis::create_run(&db, "manual", 0, i64::MAX).unwrap();
    assert_eq!(
        analysis::apply_output(&db, run, &snapshot(&db), &raw).unwrap(),
        1
    );
    let stored: (String, String) = db
        .conn()
        .query_row(
            "SELECT raw_output,warnings_json FROM analysis_outputs WHERE run_id=?1",
            [run],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(stored.0, raw);
    assert!(stored
        .1
        .contains("专家归属必须引用本次提供的原话或交流记录"));
    assert!(stored.1.contains("长期证据需求"));
    let queue = ProposalRepo::new(db.conn())
        .list(Some("pending"), 100)
        .unwrap();
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].title, "核对长期随访资料");
    assert!(kol::insights(&db, Some(expert)).unwrap().is_empty());
    assert_eq!(
        db.conn()
            .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert!(InboxRepo::new(db.conn())
        .get(inbox)
        .unwrap()
        .unwrap()
        .processed_at
        .is_none());
}

#[test]
fn capture_flow_insight_is_confirmed_only_once_and_undo_restores_inbox() {
    let (db, expert, inbox) = setup();
    let p = queue(&db, &output(&db, expert, inbox));
    assert!(kol::insights(&db, Some(expert)).unwrap().is_empty());
    assert!(InboxRepo::new(db.conn())
        .get(inbox)
        .unwrap()
        .unwrap()
        .processed_at
        .is_none());
    let applied = apply::confirm_proposal(&db, p.id, p.updated_at, None).unwrap();
    let insights = kol::insights(&db, Some(expert)).unwrap();
    assert_eq!(insights.len(), 1);
    assert_eq!(insights[0]["observation"], "希望补充长期随访证据");
    assert!(insights[0]["citations_json"]
        .as_str()
        .unwrap()
        .contains(&format!("inbox:{inbox}")));
    assert!(apply::confirm_proposal(&db, p.id, p.updated_at, None).is_err());
    receipts::undo(&db, &applied.receipt_id).unwrap();
    assert!(kol::insights(&db, Some(expert)).unwrap().is_empty());
    assert!(InboxRepo::new(db.conn())
        .get(inbox)
        .unwrap()
        .unwrap()
        .processed_at
        .is_none());
}

#[test]
fn capture_flow_insight_partial_update_retains_citations_and_rejects_same_second_change() {
    let (db, expert, inbox) = setup();
    let p = queue(&db, &output(&db, expert, inbox));
    let first = apply::confirm_proposal(&db, p.id, p.updated_at, None).unwrap();
    let later = InboxRepo::new(db.conn())
        .insert("合成医院陈医生补充：先关注两年随访证据，请调整长期证据需求洞察")
        .unwrap();
    let mut value = output(&db, expert, later.id);
    value["proposals"][0]["operation"] = json!("update");
    value["proposals"][0]["target_id"] = json!(first.target_id);
    value["proposals"][0]["payload"]["observation"] = json!("先关注两年随访证据");
    value["proposals"][0]["payload"]["field_evidence"]["observation"]["quote"] =
        json!("先关注两年随访证据");
    value["proposals"][0]["payload"]["field_evidence"]["expert_id"]["quote"] =
        json!("合成医院陈医生补充");
    let p = queue(&db, &value);
    let original = kol::insights(&db, Some(expert)).unwrap().remove(0);
    db.conn()
        .execute(
            "UPDATE kol_insights SET review_note='用户刚刚修正' WHERE id=?1",
            [first.target_id],
        )
        .unwrap();
    assert!(apply::confirm_proposal(&db, p.id, p.updated_at, None).is_err());
    db.conn()
        .execute(
            "UPDATE kol_insights SET review_note='' WHERE id=?1",
            [first.target_id],
        )
        .unwrap();
    let result = apply::confirm_proposal(&db, p.id, p.updated_at, None).unwrap();
    let changed = kol::insights(&db, Some(expert)).unwrap().remove(0);
    assert_eq!(changed["observation"], "先关注两年随访证据");
    assert_eq!(changed["implication"], original["implication"]);
    let citations = changed["citations_json"].as_str().unwrap();
    assert!(
        citations.contains(&format!("inbox:{inbox}"))
            && citations.contains(&format!("inbox:{}", later.id))
    );
    receipts::undo(&db, &result.receipt_id).unwrap();
    assert_eq!(
        kol::insights(&db, Some(expert)).unwrap()[0]["observation"],
        original["observation"]
    );
}

#[test]
fn capture_flow_ambiguous_identity_and_wrong_insight_owner_never_queue() {
    let (db, expert, inbox) = setup();
    let other = kol::save_expert(
        &db,
        None,
        None,
        "陈医生",
        "另一医院",
        Some("肾内科"),
        "",
        &[],
        false,
    )
    .unwrap();
    let mut value = output(&db, expert, inbox);
    value["proposals"][0]["payload"]["field_evidence"]["expert_id"]["quote"] = json!("陈医生");
    assert!(analysis::parse_output("global_analysis", &value.to_string(), &snapshot(&db)).is_err());
    let p = queue(&db, &output(&db, expert, inbox));
    let first = apply::confirm_proposal(&db, p.id, p.updated_at, None).unwrap();
    let next = InboxRepo::new(db.conn())
        .insert("合成医院陈医生希望补充长期随访证据，请加入专家洞察")
        .unwrap();
    let mut value = output(&db, expert, next.id);
    value["proposals"][0]["operation"] = json!("update");
    value["proposals"][0]["target_id"] = json!(first.target_id);
    value["proposals"][0]["payload"]["expert_id"] = other["id"].clone();
    assert!(analysis::parse_output("global_analysis", &value.to_string(), &snapshot(&db)).is_err());
}

#[test]
fn capture_flow_deleted_expert_and_missing_evidence_cannot_be_confirmed() {
    let (db, expert, inbox) = setup();
    let mut value = output(&db, expert, inbox);
    value["proposals"][0]["payload"]["implication"] = json!("没有资料支持的判断");
    assert!(analysis::parse_output("global_analysis", &value.to_string(), &snapshot(&db)).is_err());
    let p = queue(&db, &output(&db, expert, inbox));
    kol::delete_expert(&db, expert, "陈医生", 1).unwrap();
    assert!(apply::confirm_proposal(&db, p.id, p.updated_at, None).is_err());
    assert!(kol::insights(&db, None).unwrap().is_empty());
}

#[test]
fn capture_flow_update_keeps_old_evidence_resolvable_and_blocks_unanswered_reanalysis() {
    let (db, expert, inbox) = setup();
    let p = queue(&db, &output(&db, expert, inbox));
    let second = analysis::create_run(&db, "manual", 0, i64::MAX).unwrap();
    assert_eq!(
        analysis::apply_output(
            &db,
            second,
            &snapshot(&db),
            &output(&db, expert, inbox).to_string()
        )
        .unwrap(),
        0
    );
    let result = apply::confirm_proposal(&db, p.id, p.updated_at, None).unwrap();
    let note = InboxRepo::new(db.conn())
        .insert("合成医院陈医生补充了长期证据需求：先关注两年随访证据")
        .unwrap();
    let mut value = output(&db, expert, note.id);
    value["proposals"][0]["operation"] = json!("update");
    value["proposals"][0]["target_id"] = json!(result.target_id);
    value["proposals"][0]["payload"]["observation"] = json!("先关注两年随访证据");
    value["proposals"][0]["payload"]["field_evidence"]["observation"]["quote"] =
        json!("先关注两年随访证据");
    value["proposals"][0]["payload"]["field_evidence"]["expert_id"]["quote"] =
        json!("合成医院陈医生");
    let next = queue(&db, &value);
    apply::confirm_proposal(&db, next.id, next.updated_at, None).unwrap();
    let insight = kol::insights(&db, Some(expert)).unwrap().remove(0);
    let evidence: String = db
        .conn()
        .query_row(
            "SELECT evidence_json FROM kol_drafts WHERE id=?1",
            [insight["draft_id"].as_i64().unwrap()],
            |r| r.get(0),
        )
        .unwrap();
    let pack: Value = serde_json::from_str(&evidence).unwrap();
    assert!(
        pack["sources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["id"] == format!("inbox:{inbox}")),
        "old citations need their evidence pack after a partial update"
    );
}

#[test]
fn capture_flow_resolved_insight_leaves_the_active_catalog() {
    let (db, expert, inbox) = setup();
    let p = queue(&db, &output(&db, expert, inbox));
    apply::confirm_proposal(&db, p.id, p.updated_at, None).unwrap();
    let confirmed = ProposalRepo::new(db.conn()).get(p.id).unwrap().unwrap();
    super::lifecycle::resolve(&db, p.id, confirmed.updated_at).unwrap();
    assert!(snapshot(&db).expert_catalog[0]["insights"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn capture_flow_confirmed_insight_and_undo_survive_database_reopen() {
    let root = std::env::temp_dir()
        .join(".test-runtime")
        .join(format!("capture-persistence-{}", uuid::Uuid::new_v4()));
    let path = root.join("data/msl-desktop.db");
    let db = Database::open(&path).unwrap();
    let expert = kol::save_expert(
        &db,
        None,
        None,
        "陈医生",
        "合成医院",
        Some("肾内科"),
        "",
        &[],
        false,
    )
    .unwrap()["id"]
        .as_i64()
        .unwrap();
    let inbox = InboxRepo::new(db.conn())
        .insert("合成医院陈医生希望补充长期随访证据，请加入专家洞察")
        .unwrap();
    let p = queue(&db, &output(&db, expert, inbox.id));
    let receipt = apply::confirm_proposal(&db, p.id, p.updated_at, None).unwrap();
    drop(db);
    let reopened = Database::open(&path).unwrap();
    let insight = kol::insights(&reopened, Some(expert)).unwrap().remove(0);
    assert_eq!(insight["observation"], "希望补充长期随访证据");
    assert!(insight["insight_revision"].as_str().is_some());
    receipts::undo(&reopened, &receipt.receipt_id).unwrap();
    assert!(kol::insights(&reopened, Some(expert)).unwrap().is_empty());
    assert!(InboxRepo::new(reopened.conn())
        .get(inbox.id)
        .unwrap()
        .unwrap()
        .processed_at
        .is_none());
}

#[test]
fn capture_flow_project_update_preserves_fields_and_rejects_intervening_user_edit() {
    let db = Database::open_in_memory().unwrap();
    let repo = crate::db::work::WorkRepo::new(db.conn());
    let work = repo.insert("合成研究项目", "active").unwrap();
    repo.update(work.id, &work.title, "active", Some("原有项目目标"))
        .unwrap();
    let inbox = InboxRepo::new(db.conn())
        .insert("合成研究项目的目标调整为核对两年随访资料")
        .unwrap();
    let s = serde_json::to_value(snapshot(&db)).unwrap();
    let index = s["brief"]["inbox"]
        .as_array()
        .unwrap()
        .iter()
        .position(|v| v["entity_id"] == inbox.id)
        .unwrap();
    let value = json!({"proposals":[{"kind":"work","operation":"update","target_id":work.id,"title":work.title,"payload":{"summary":"核对两年随访资料","field_evidence":{"summary":{"snapshot_path":format!("/brief/inbox/{index}/content"),"quote":"目标调整为核对两年随访资料","basis":"explicit"}}},"source_refs":[{"source_type":"inbox","entity_id":inbox.id}]}]});
    let mut invented = value.clone();
    invented["proposals"][0]["target_id"] = json!(work.id + 999);
    assert!(
        analysis::parse_output("global_analysis", &invented.to_string(), &snapshot(&db)).is_err(),
        "generated updates require a target present in the snapshot"
    );
    let p = queue(&db, &value);
    let payload: Value = serde_json::from_str(&p.payload_json).unwrap();
    assert_eq!(
        payload["project_revision"],
        repo.get(work.id).unwrap().unwrap().revision
    );
    repo.update(work.id, &work.title, "active", Some("用户新修正的目标"))
        .unwrap();
    assert!(apply::confirm_proposal(&db, p.id, p.updated_at, None).is_err());
    assert_eq!(
        repo.get(work.id).unwrap().unwrap().summary.as_deref(),
        Some("用户新修正的目标")
    );
}

#[test]
fn capture_flow_mock_provider_to_review_queue_across_three_protocols() {
    use crate::db::provider::{ProviderConnection, ProviderModel};
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
        time::{Duration, Instant},
    };
    for (protocol, endpoint) in [
        ("chat_completions", "/chat/completions"),
        ("responses", "/responses"),
        ("anthropic_messages", "/messages"),
    ] {
        let (db, expert, inbox) = setup();
        let expected = output(&db, expert, inbox).to_string();
        let response=match protocol {
            "responses"=>json!({"status":"completed","output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":expected}]}]}),
            "anthropic_messages"=>json!({"content":[{"type":"text","text":expected}],"stop_reason":"end_turn"}),
            _=>json!({"choices":[{"message":{"content":expected},"finish_reason":"stop"}]})
        }.to_string();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let handle = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(15);
            let mut stream = loop {
                if let Ok((stream, _)) = listener.accept() {
                    break stream;
                }
                assert!(Instant::now() < deadline, "mock was not called");
                thread::sleep(Duration::from_millis(10));
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut chunk = [0u8; 8192];
            let (start, length) = loop {
                let n = stream.read(&mut chunk).unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&chunk[..n]);
                if let Some(at) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..at]).to_lowercase();
                    assert!(headers.starts_with(&format!("post {endpoint} ")));
                    let length: usize = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length:"))
                        .unwrap()
                        .trim()
                        .parse()
                        .unwrap();
                    break (at + 4, length);
                }
            };
            while bytes.len() < start + length {
                let n = stream.read(&mut chunk).unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&chunk[..n]);
            }
            let request: Value = serde_json::from_slice(&bytes[start..start + length]).unwrap();
            let text = request.to_string();
            assert!(
                text.contains("expert_catalog")
                    && text.contains("capture_routing")
                    && text.contains("陈医生")
            );
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",response.len(),response).unwrap();
        });
        let connection = ProviderConnection {
            id: 1,
            display_name: "Synthetic loopback".into(),
            provider_type: "custom".into(),
            base_url: base,
            legacy_model: String::new(),
            enabled: true,
            credential_ref: "unused".into(),
            template_kind: "custom".into(),
            auth_mode: "bearer".into(),
            models_endpoint: None,
            last_models_refresh_at: None,
            created_at: 0,
            updated_at: 0,
        };
        let model = ProviderModel {
            id: 1,
            provider_id: 1,
            model_id: "synthetic".into(),
            display_name: "Synthetic".into(),
            protocol: protocol.into(),
            endpoint_path: endpoint.into(),
            capabilities_json: "{}".into(),
            source: "manual".into(),
            enabled: true,
            available: true,
            created_at: 0,
            updated_at: 0,
        };
        let s = snapshot(&db);
        let request = analysis::build_request("synthetic", &s);
        let response = tauri::async_runtime::block_on(analysis::complete_validated(
            &connection,
            &model,
            "synthetic-test-only",
            &request,
            &s,
        ))
        .unwrap();
        handle.join().unwrap();
        let p = queue(&db, &serde_json::from_str(&response.content).unwrap());
        assert!(kol::insights(&db, Some(expert)).unwrap().is_empty());
        apply::confirm_proposal(&db, p.id, p.updated_at, None).unwrap();
        assert_eq!(kol::insights(&db, Some(expert)).unwrap().len(), 1);
    }
}
