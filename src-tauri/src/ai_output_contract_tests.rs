use crate::ai::output::{
    apply_output_format, parse_document, parse_translation, AttemptKind, Basis, OutputFormat,
    RequestBudget,
};

fn open_document(section_count: usize) -> String {
    let sections: Vec<_> = (0..section_count)
        .map(|index| {
            serde_json::json!({
                "title": format!("开放主题 {index}"),
                "blocks": [{
                    "type": "bullets",
                    "items": [{
                        "text": format!("合成工作内容 {index}"),
                        "basis": "suggestion",
                        "citations": []
                    }]
                }]
            })
        })
        .collect();
    serde_json::json!({
        "schema_version": "msl.readable.v1",
        "title": "合成工作观察",
        "sections": sections
    })
    .to_string()
}

#[test]
fn structured_format_uses_each_protocols_supported_request_shape() {
    let schema = serde_json::json!({
        "type":"object",
        "properties":{"value":{"type":"string"}},
        "required":["value"],
        "additionalProperties":false
    });
    let format = OutputFormat::JsonSchema {
        name: "msl_test".into(),
        schema,
    };
    let mut chat = serde_json::json!({"model":"synthetic"});
    apply_output_format("chat_completions", &mut chat, &format).unwrap();
    assert_eq!(chat["response_format"]["type"], "json_schema");
    let mut responses = serde_json::json!({"model":"synthetic"});
    apply_output_format("responses", &mut responses, &format).unwrap();
    assert_eq!(responses["text"]["format"]["type"], "json_schema");
    let mut anthropic = serde_json::json!({"model":"synthetic"});
    apply_output_format("anthropic_messages", &mut anthropic, &format).unwrap();
    assert_eq!(anthropic["output_config"]["format"]["type"], "json_schema");
}

#[test]
fn open_work_topics_are_preserved_without_fixed_categories() {
    let document = parse_document(&open_document(40)).unwrap();
    assert_eq!(document.sections.len(), 40);
    assert_eq!(document.sections[39].title, "开放主题 39");
    assert_eq!(
        document.sections[0].blocks[0].statements()[0].basis,
        Basis::Suggestion
    );
}

#[test]
fn factual_statements_require_sources_but_suggestions_can_be_open() {
    let fact = open_document(1).replace("suggestion", "fact");
    assert!(parse_document(&fact).is_err());
    assert!(parse_document(&open_document(1)).is_ok());
}

#[test]
fn translation_contract_returns_only_the_translated_text() {
    let translation = parse_translation(
        r#"{"schema_version":"msl.translation.v1","translated_text":"Hello","source_language":"zh","target_language":"en"}"#,
    )
    .unwrap();
    assert_eq!(translation.translated_text, "Hello");
    assert!(parse_translation(
        r#"{"schema_version":"msl.translation.v1","translated_text":"你好，我是 MSL 助手","source_language":"en","target_language":"zh","explanation":"extra"}"#
    )
    .is_err());
}

#[test]
fn format_repair_and_transport_share_one_request_budget() {
    let mut budget = RequestBudget::new(3, 1);
    budget.claim(AttemptKind::Initial).unwrap();
    budget.claim(AttemptKind::FormatFallback).unwrap();
    budget.claim(AttemptKind::FormatRepair).unwrap();
    assert!(budget.claim(AttemptKind::TransportRetry).is_err());
    assert!(budget.claim(AttemptKind::FormatRepair).is_err());
    assert_eq!(budget.used(), 3);
}
