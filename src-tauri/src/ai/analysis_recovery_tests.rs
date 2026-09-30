//! Synthetic loopback coverage for bounded secretary output recovery.
use super::*;
use crate::db::provider::{ProviderConnection, ProviderModel};
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

struct MockServer {
    base: String,
    requests: Arc<Mutex<Vec<Value>>>,
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

fn read_request(stream: &mut TcpStream) -> Value {
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 8192];
    let (start, length) = loop {
        let count = stream.read(&mut chunk).unwrap();
        assert!(count > 0, "request ended before its headers");
        bytes.extend_from_slice(&chunk[..count]);
        if let Some(at) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&bytes[..at]).to_lowercase();
            let length = headers
                .lines()
                .find_map(|line| line.strip_prefix("content-length:"))
                .unwrap()
                .trim()
                .parse::<usize>()
                .unwrap();
            break (at + 4, length);
        }
    };
    while bytes.len() < start + length {
        let count = stream.read(&mut chunk).unwrap();
        assert!(count > 0, "request body was not fully transmitted");
        bytes.extend_from_slice(&chunk[..count]);
    }
    serde_json::from_slice(&bytes[start..start + length]).unwrap()
}

impl MockServer {
    fn new(responses: Vec<(u16, Value)>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&requests);
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = Arc::clone(&stop);
        let handle = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(20);
            while !stopped.load(Ordering::SeqCst) && Instant::now() < deadline {
                let Ok((mut stream, _)) = listener.accept() else {
                    thread::sleep(Duration::from_millis(5));
                    continue;
                };
                let request = read_request(&mut stream);
                let index = {
                    let mut values = captured.lock().unwrap();
                    let index = values.len();
                    values.push(request);
                    index
                };
                let (status, value) = responses
                    .get(index)
                    .cloned()
                    .unwrap_or((400, json!({"error":"unexpected extra generation"})));
                let body = value.to_string();
                write!(stream, "HTTP/1.1 {status} Synthetic\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
        });
        Self {
            base,
            requests,
            stop,
            handle: Some(handle),
        }
    }

    fn finish(mut self) -> Vec<Value> {
        self.stop.store(true, Ordering::SeqCst);
        self.handle.take().unwrap().join().unwrap();
        self.requests.lock().unwrap().clone()
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn connection(base: &str, template: &str) -> ProviderConnection {
    ProviderConnection {
        id: 1,
        display_name: "Synthetic loopback".into(),
        provider_type: "custom".into(),
        base_url: base.into(),
        legacy_model: String::new(),
        enabled: true,
        credential_ref: "unused-synthetic".into(),
        template_kind: template.into(),
        auth_mode: "bearer".into(),
        models_endpoint: None,
        last_models_refresh_at: None,
        created_at: 0,
        updated_at: 0,
    }
}

fn model(protocol: &str, id: &str, capabilities: Value) -> ProviderModel {
    ProviderModel {
        id: 1,
        provider_id: 1,
        model_id: id.into(),
        display_name: "Synthetic".into(),
        protocol: protocol.into(),
        endpoint_path: match protocol {
            "responses" => "/responses",
            "anthropic_messages" => "/messages",
            _ => "/chat/completions",
        }
        .into(),
        capabilities_json: capabilities.to_string(),
        source: "manual".into(),
        enabled: true,
        available: true,
        created_at: 0,
        updated_at: 0,
    }
}

fn fixture(kind: &str) -> (Database, AnalysisSnapshot, String) {
    let db = Database::open_in_memory().unwrap();
    let inbox = crate::db::inbox::InboxRepo::new(db.conn())
        .insert("合成资料：整理长期随访证据，保留各独立工作细项。")
        .unwrap();
    let snapshot = crate::ai::analysis_snapshot::build(
        &db,
        kind,
        "2026-09-30",
        0,
        i64::MAX,
        0,
        i64::MAX,
        "zh-CN",
    )
    .unwrap();
    let output = json!({"summary":"合成整理建议", "proposals":[{
        "kind":"task", "operation":"create", "title":"整理长期随访证据", "payload":{},
        "source_refs":[{"source_type":"inbox", "entity_id":inbox.id}]
    }]})
    .to_string();
    parse_output(kind, &output, &snapshot).unwrap();
    (db, snapshot, output)
}

fn request_for(
    kind: &str,
    id: &str,
    snapshot: &AnalysisSnapshot,
) -> crate::ai::provider::AiTextRequest {
    if kind == "work_draft" {
        build_workspace_intake_request(id, snapshot, None)
    } else {
        build_request(id, snapshot)
    }
}

fn response(protocol: &str, text: &str, limited: bool) -> Value {
    match protocol {
        "responses" => {
            json!({"id":"synthetic", "status":if limited {"incomplete"} else {"completed"},
            "incomplete_details":if limited {json!({"reason":"max_output_tokens"})} else {Value::Null},
            "output":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":text}]}]})
        }
        "anthropic_messages" => {
            json!({"id":"synthetic", "stop_reason":if limited {"max_tokens"} else {"end_turn"},
            "content":[{"type":"text","text":text}]})
        }
        _ => json!({"id":"synthetic", "choices":[{"message":{"role":"assistant","content":text},
            "finish_reason":if limited {"length"} else {"stop"}}]}),
    }
}

fn limit(body: &Value) -> u64 {
    body.get("max_output_tokens")
        .or_else(|| body.get("max_tokens"))
        .unwrap()
        .as_u64()
        .unwrap()
}

fn without_limit(mut body: Value) -> Value {
    body.as_object_mut().unwrap().remove("max_tokens");
    body.as_object_mut().unwrap().remove("max_output_tokens");
    body
}

fn pending_count(db: &Database) -> i64 {
    db.conn()
        .query_row("SELECT COUNT(*) FROM ai_proposals", [], |row| row.get(0))
        .unwrap()
}

#[test]
fn recovery_three_protocols_preserve_full_evidence_and_queue_only_valid_complete_output() {
    for protocol in ["chat_completions", "responses", "anthropic_messages"] {
        for kind in ["global_analysis", "work_draft"] {
            let (db, snapshot, output) = fixture(kind);
            // A syntactically complete object still cannot be accepted when the provider reports truncation.
            let partial = r#"{"proposals":[{"kind":"task","operation":"create","title":"不可保存的截断建议","payload":{}}]}"#;
            let server = MockServer::new(vec![
                (200, response(protocol, partial, true)),
                (200, response(protocol, &output, false)),
            ]);
            let request = request_for(kind, "synthetic", &snapshot);
            let result = tauri::async_runtime::block_on(complete_validated(
                &connection(&server.base, "custom"),
                &model(protocol, "synthetic", json!({})),
                "synthetic-test-only",
                &request,
                &snapshot,
            ));
            let calls = server.finish();
            let result = result.unwrap_or_else(|error| panic!("{protocol}/{kind}: {error}"));
            assert_eq!(calls.len(), 2);
            assert_eq!((limit(&calls[0]), limit(&calls[1])), (8000, 16000));
            assert_eq!(
                without_limit(calls[0].clone()),
                without_limit(calls[1].clone()),
                "recovery changed evidence or granularity"
            );
            assert_eq!(result.content, output);
            assert_eq!(pending_count(&db), 0);
            let run = create_run(&db, "synthetic", 0, i64::MAX).unwrap();
            assert_eq!(
                apply_output(&db, run, &snapshot, &result.content).unwrap(),
                1
            );
            assert_eq!(pending_count(&db), 1);
            let title: String = db
                .conn()
                .query_row("SELECT title FROM ai_proposals", [], |row| row.get(0))
                .unwrap();
            assert_eq!(title, "整理长期随访证据");
        }
    }
}

#[test]
fn recovery_stops_after_two_truncated_generations_without_partial_proposals() {
    let (db, snapshot, output) = fixture("global_analysis");
    let server = MockServer::new(vec![
        (200, response("chat_completions", &output, true)),
        (200, response("chat_completions", &output, true)),
    ]);
    let result = tauri::async_runtime::block_on(complete_validated(
        &connection(&server.base, "custom"),
        &model("chat_completions", "synthetic", json!({})),
        "synthetic-test-only",
        &build_request("synthetic", &snapshot),
        &snapshot,
    ));
    let calls = server.finish();
    assert!(result.is_err());
    assert_eq!(calls.len(), 2, "one bounded recovery is required");
    assert_eq!(pending_count(&db), 0);
}

#[test]
fn recovery_invalid_or_untrusted_second_output_never_triggers_a_third_generation() {
    for second in [
        "not-json",
        r#"{"proposals":[{"kind":"task","operation":"create","title":"伪造来源","payload":{},"source_refs":[{"source_type":"inbox","entity_id":999999}]}]}"#,
    ] {
        let (db, snapshot, output) = fixture("global_analysis");
        let server = MockServer::new(vec![
            (200, response("chat_completions", &output, true)),
            (200, response("chat_completions", second, false)),
        ]);
        let result = tauri::async_runtime::block_on(complete_validated(
            &connection(&server.base, "custom"),
            &model("chat_completions", "synthetic", json!({})),
            "synthetic-test-only",
            &build_request("synthetic", &snapshot),
            &snapshot,
        ));
        let calls = server.finish();
        assert!(result.is_err());
        assert_eq!(calls.len(), 2);
        assert_eq!(pending_count(&db), 0);
    }
}

#[test]
fn recovery_deepseek_uses_reasoning_budget_and_respects_declared_model_cap() {
    for (template, id, capabilities, expected_first, expected_second) in [
        ("deepseek", "deepseek-v4-flash", json!({}), 65536, 131072),
        ("deepseek", "deepseek-flash", json!({}), 65536, 131072),
        ("custom", "deepseek-v4-flash", json!({}), 8000, 16000),
        (
            "deepseek",
            "deepseek-v4-pro",
            json!({"max_output_tokens":96000,"limit":{"output":70000}}),
            65536,
            70000,
        ),
        (
            "deepseek",
            "deepseek-v4-flash",
            json!({"limit":{"output":32000}}),
            32000,
            32000,
        ),
        (
            "custom",
            "synthetic",
            json!({"max_output_tokens":10000}),
            8000,
            10000,
        ),
    ] {
        let (_db, snapshot, output) = fixture("global_analysis");
        let server = MockServer::new(vec![
            (200, response("chat_completions", "{", true)),
            (200, response("chat_completions", &output, false)),
        ]);
        let result = tauri::async_runtime::block_on(complete_validated(
            &connection(&server.base, template),
            &model("chat_completions", id, capabilities),
            "synthetic-test-only",
            &build_request(id, &snapshot),
            &snapshot,
        ));
        let calls = server.finish();
        assert_eq!(limit(&calls[0]), expected_first, "{template}/{id}");
        if expected_second == expected_first {
            assert!(
                result.is_err(),
                "an unchanged cap cannot recover a truncated answer"
            );
            assert_eq!(calls.len(), 1);
        } else {
            assert!(result.is_ok(), "{template}/{id}: {result:?}");
            assert_eq!(calls.len(), 2);
            assert_eq!(limit(&calls[1]), expected_second);
        }
    }
}

#[test]
fn recovery_terminal_and_unknown_incomplete_states_fail_without_regeneration() {
    let cases = [
        (
            "chat_completions",
            json!({"choices":[{"message":{"content":"PLACEHOLDER"},"finish_reason":"aborted"}]}),
        ),
        (
            "chat_completions",
            json!({"choices":[{"message":{"content":"PLACEHOLDER"},"finish_reason":"insufficient_system_resource"}]}),
        ),
        (
            "chat_completions",
            json!({"choices":[{"message":{"content":"PLACEHOLDER"},"finish_reason":"tool_calls"}]}),
        ),
        (
            "chat_completions",
            json!({"choices":[{"message":{"content":"PLACEHOLDER"},"finish_reason":"content_filter"}]}),
        ),
        (
            "responses",
            json!({"status":"incomplete","incomplete_details":{"reason":"content_filter"},"output_text":"PLACEHOLDER"}),
        ),
        (
            "responses",
            json!({"status":"incomplete","output_text":"PLACEHOLDER"}),
        ),
        (
            "responses",
            json!({"status":"cancelled","output_text":"PLACEHOLDER"}),
        ),
        (
            "anthropic_messages",
            json!({"stop_reason":"model_context_window_exceeded","content":[{"type":"text","text":"PLACEHOLDER"}]}),
        ),
    ];
    for (protocol, mut body) in cases {
        let (db, snapshot, output) = fixture("global_analysis");
        match protocol {
            "responses" => body["output_text"] = json!(output),
            "anthropic_messages" => body["content"][0]["text"] = json!(output),
            _ => body["choices"][0]["message"]["content"] = json!(output),
        }
        let server = MockServer::new(vec![(200, body)]);
        let result = tauri::async_runtime::block_on(complete_validated(
            &connection(&server.base, "custom"),
            &model(protocol, "synthetic", json!({})),
            "synthetic-test-only",
            &build_request("synthetic", &snapshot),
            &snapshot,
        ));
        let calls = server.finish();
        assert!(
            result.is_err(),
            "{protocol} terminal/incomplete state was accepted"
        );
        assert_eq!(calls.len(), 1);
        assert_eq!(pending_count(&db), 0);
    }
}

#[test]
fn recovery_and_transport_retries_share_three_http_attempts() {
    let (db, snapshot, output) = fixture("global_analysis");
    let server = MockServer::new(vec![
        (500, json!({"error":"synthetic transient"})),
        (200, response("chat_completions", "{", true)),
        (500, json!({"error":"synthetic transient"})),
        (200, response("chat_completions", &output, false)),
    ]);
    let request = build_request("synthetic", &snapshot);
    let result = tauri::async_runtime::block_on(complete_validated(
        &connection(&server.base, "custom"),
        &model("chat_completions", "synthetic", json!({})),
        "synthetic-test-only",
        &request,
        &snapshot,
    ));
    let calls = server.finish();
    assert!(result.is_err());
    assert_eq!(
        calls.len(),
        3,
        "shared HTTP budget must prevent a fourth transmission"
    );
    assert_eq!(request.budget.lock().unwrap().used(), 3);
    assert_eq!(pending_count(&db), 0);
}

#[test]
fn recovery_existing_format_repair_remains_available_for_complete_invalid_json() {
    let (_db, snapshot, output) = fixture("global_analysis");
    let server = MockServer::new(vec![
        (200, response("chat_completions", "not-json", false)),
        (200, response("chat_completions", &output, false)),
    ]);
    let result = tauri::async_runtime::block_on(complete_validated(
        &connection(&server.base, "custom"),
        &model("chat_completions", "synthetic", json!({})),
        "synthetic-test-only",
        &build_request("synthetic", &snapshot),
        &snapshot,
    ));
    let calls = server.finish();
    assert_eq!(result.unwrap().content, output);
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0]["messages"][0], calls[1]["messages"][0]);
    assert_eq!(calls[0]["messages"][1], calls[1]["messages"][1]);
}
