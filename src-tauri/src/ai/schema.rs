//! Strict JSON contract emitted by AI secretary prompts.

use serde::{Deserialize, Serialize};

pub const TASK_KINDS: &[&str] = &[
    "workspace_analysis",
    "work_draft",
    "global_analysis",
    "daily_brief",
    "weekly_report",
    "monthly_report",
    "translation",
    "general",
];
pub const PROPOSAL_KINDS: &[&str] = &[
    "work",
    "task",
    "waiting",
    "calendar",
    "inbox",
    "resume_point",
];
pub const SAFE_OPERATIONS: &[&str] = &["create", "update"];

/// Shared by generation, draft editing and confirmation so invalid states are
/// rejected while they are still editable, before any entity is written.
pub fn validate_payload_fields(kind: &str, payload: &serde_json::Value) -> Result<(), String> {
    if !payload.is_object() {
        return Err("建议内容必须是对象".into());
    }
    if let Some(basis) = payload.get("time_basis").filter(|v| !v.is_null()) {
        if !basis
            .as_str()
            .is_some_and(|s| matches!(s, "explicit" | "inferred"))
        {
            return Err("时间依据应为指定或推算，请调整安排".into());
        }
        if basis == "inferred"
            && !payload["time_reason"]
                .as_str()
                .is_some_and(|s| !s.trim().is_empty() && s.chars().count() <= 500)
        {
            return Err("AI 推算时间需要说明依据，请调整后确认".into());
        }
    }
    if let (Some(start), Some(end)) = (payload["start_at"].as_i64(), payload["end_at"].as_i64()) {
        if end < start {
            return Err("结束时间不能早于开始时间".into());
        }
    }
    for field in [
        "scheduled_start",
        "scheduled_end",
        "due_at",
        "follow_up_at",
        "start_at",
        "end_at",
    ] {
        if let Some(value) = payload.get(field).filter(|v| !v.is_null()) {
            if !value.as_i64().is_some_and(|v| v > 0) {
                return Err("任务安排时间无效，请在调整中确认日期".into());
            }
        }
    }
    if let (Some(start), Some(end)) = (
        payload["scheduled_start"].as_i64(),
        payload["scheduled_end"].as_i64(),
    ) {
        if end < start {
            return Err("结束时间不能早于开始时间".into());
        }
    }
    let statuses: &[&str] = match kind {
        "work" => &["active", "paused", "waiting", "done", "archived"],
        "task" => &["next", "doing", "scheduled", "waiting", "paused", "done"],
        "waiting" => &["open", "resolved"],
        _ => &[],
    };
    for (field, allowed) in [
        ("status", statuses),
        ("priority", &["low", "normal", "high"][..]),
    ] {
        if let Some(value) = payload.get(field).filter(|value| !value.is_null()) {
            if !allowed.is_empty() && !value.as_str().is_some_and(|v| allowed.contains(&v)) {
                return Err(format!(
                    "{}的{}无效，请在审阅详情中选择有效值（{}）",
                    match kind {
                        "work" => "长期项目",
                        "task" => "任务",
                        "waiting" => "等待事项",
                        _ => "建议",
                    },
                    if field == "status" {
                        "状态"
                    } else {
                        "优先级"
                    },
                    allowed.join(" / ")
                ));
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiOutputContract {
    #[serde(default)]
    pub summary: Option<String>,
    pub proposals: Vec<ProposalContract>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalContract {
    #[serde(default)]
    pub related_proposal_id: Option<i64>,
    pub kind: String,
    pub operation: String,
    #[serde(default)]
    pub target_id: Option<i64>,
    #[serde(default)]
    pub work_id: Option<i64>,
    #[serde(default)]
    pub workspace_id: Option<i64>,
    pub title: String,
    #[serde(default)]
    pub payload: serde_json::Value,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub source_refs: Vec<serde_json::Value>,
    #[serde(default)]
    pub confidence: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct ValidatedOutput {
    pub summary: Option<String>,
    pub proposals: Vec<ProposalContract>,
}

pub(crate) fn source_ref_matches(
    candidate: &serde_json::Value,
    allowed: &serde_json::Value,
) -> bool {
    if candidate == allowed {
        return true;
    }
    let (Some(candidate), Some(allowed)) = (candidate.as_object(), allowed.as_object()) else {
        return false;
    };
    if candidate.get("source_type") != allowed.get("source_type") {
        return false;
    }
    let identity_keys = ["entity_id", "workspace_id", "relative_path", "content_hash"];
    let mut has_identity = false;
    for key in identity_keys {
        if let Some(value) = candidate.get(key).filter(|value| !value.is_null()) {
            has_identity = true;
            if allowed.get(key) != Some(value) {
                return false;
            }
        }
    }
    has_identity
}

pub fn strip_single_code_fence(input: &str) -> String {
    let trimmed = input.trim();
    if let Some(rest) = trimmed.strip_prefix("```") {
        if let Some((_, body)) = rest.split_once('\n') {
            if let Some(body) = body.strip_suffix("```") {
                return body.trim().to_string();
            }
        }
    }
    trimmed.to_string()
}

pub fn validate(
    task_kind: &str,
    input: &str,
    allowed_sources: &[serde_json::Value],
) -> Result<ValidatedOutput, String> {
    if !TASK_KINDS.contains(&task_kind) {
        return Err("未知 task kind".into());
    }
    let clean = strip_single_code_fence(input);
    // Accept a single complete JSON object wrapped in explanatory text. Never
    // repair punctuation, truncate arrays, select nested proposals or merge objects.
    let candidate = match (clean.find('{'), clean.rfind('}')) {
        (Some(start), Some(end)) if end >= start => &clean[start..=end],
        _ => return Err("AI 输出缺少完整 JSON 对象，可能已截断；未写入任何建议".into()),
    };
    let value: serde_json::Value = serde_json::from_str(candidate)
        .map_err(|_| "AI JSON 语法无效或已截断；未写入任何建议".to_string())?;
    let contract: AiOutputContract = serde_json::from_value(value)
        .map_err(|_| "AI JSON 字段结构无效：summary 应为文本，proposals 应为建议数组，建议须有 kind、operation、title、payload".to_string())?;
    if contract.proposals.len() > 50 {
        return Err("AI 建议数量超过上限".into());
    }
    for proposal in &contract.proposals {
        if !PROPOSAL_KINDS.contains(&proposal.kind.as_str()) {
            return Err("AI 建议 kind 不受支持".into());
        }
        if !SAFE_OPERATIONS.contains(&proposal.operation.as_str()) {
            return Err("AI 建议 operation 不受支持".into());
        }
        if proposal.title.trim().is_empty() || proposal.title.chars().count() > 200 {
            return Err("AI 建议标题无效".into());
        }
        if !proposal.payload.is_object() {
            return Err("AI 建议 payload 必须是对象".into());
        }
        validate_payload_fields(&proposal.kind, &proposal.payload)?;
        if proposal.operation == "update" && proposal.target_id.is_none() {
            return Err("update 建议缺少 target_id".into());
        }
        if let Some(confidence) = proposal.confidence {
            if !(0.0..=1.0).contains(&confidence) {
                return Err("confidence 超出范围".into());
            }
        }
        if proposal.reason.chars().count() > 1000 {
            return Err("AI reason 超长".into());
        }
        for source in &proposal.source_refs {
            if !allowed_sources
                .iter()
                .any(|allowed| source_ref_matches(source, allowed))
            {
                return Err("AI 建议引用了不存在的 source_ref".into());
            }
        }
    }
    Ok(ValidatedOutput {
        summary: contract.summary.map(|s| s.chars().take(2000).collect()),
        proposals: contract.proposals,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn inferred_time_needs_reason_and_every_date_field_must_be_valid() {
        for kind in ["task", "waiting", "calendar"] {
            assert!(super::validate_payload_fields(
                kind,
                &serde_json::json!({"time_basis":"inferred","scheduled_start":200})
            )
            .is_err());
            assert!(super::validate_payload_fields(
                kind,
                &serde_json::json!({"time_basis":"mystery"})
            )
            .is_err());
        }
        for field in ["due_at", "follow_up_at", "start_at", "end_at"] {
            assert!(
                super::validate_payload_fields("task", &serde_json::json!({field:"tomorrow"}))
                    .is_err()
            );
        }
        assert!(super::validate_payload_fields("task",&serde_json::json!({"time_basis":"inferred","time_reason":"留出准备时间","scheduled_start":200,"scheduled_end":300})).is_ok());
        assert!(super::validate_payload_fields(
            "calendar",
            &serde_json::json!({"start_at":300,"end_at":200})
        )
        .is_err());
    }

    use super::*;
    #[test]
    fn invalid_entity_status_is_rejected_before_queueing() {
        let output = r#"{"proposals":[{"kind":"work","operation":"create","title":"Synthetic","payload":{"status":"next"}}]}"#;
        assert!(validate("global_analysis", output, &[]).is_err());
    }
    #[test]
    fn accepts_one_wrapped_contract_but_never_guesses_broken_json() {
        assert!(validate(
            "global_analysis",
            "分析结果如下：\n```json\n{\"summary\":\"• 进展\",\"proposals\":[]}\n```\n请审阅。",
            &[]
        )
        .is_ok());
        assert!(validate("global_analysis", "{\"summary\":\"未完成", &[])
            .unwrap_err()
            .contains("截断"));
        assert!(
            validate("global_analysis", "{\"summary\":[],\"proposals\":[]}", &[])
                .unwrap_err()
                .contains("字段")
        );
        assert!(validate("global_analysis", "{\"message\":\"hello\"}", &[]).is_err());
        assert!(validate(
            "global_analysis",
            "{\"proposals\":[]} {\"proposals\":[]}",
            &[]
        )
        .is_err());
    }
    fn source() -> serde_json::Value {
        serde_json::json!({"source_type":"task","entity_id":1})
    }

    #[test]
    fn accepts_fenced_json_and_rejects_unsafe_or_fake_sources() {
        let allowed = vec![source()];
        let text = format!("```json\n{{\"proposals\":[{{\"kind\":\"task\",\"operation\":\"create\",\"title\":\"推进\",\"payload\":{{}},\"source_refs\":[{}]}}]}}\n```", source());
        assert_eq!(
            validate("global_analysis", &text, &allowed)
                .unwrap()
                .proposals
                .len(),
            1
        );
        let unsafe_json = "{\"proposals\":[{\"kind\":\"task\",\"operation\":\"delete\",\"title\":\"x\",\"payload\":{}}]}";
        assert!(validate("global_analysis", unsafe_json, &[]).is_err());
        let fake = "{\"proposals\":[{\"kind\":\"task\",\"operation\":\"create\",\"title\":\"x\",\"payload\":{},\"source_refs\":[{\"source_type\":\"fake\"}]}]}";
        assert!(validate("global_analysis", fake, &[]).is_err());
    }

    #[test]
    fn compact_source_reference_matches_the_bounded_snapshot_reference() {
        let allowed = vec![serde_json::json!({
            "source_type":"task",
            "entity_id":7,
            "workspace_id":null,
            "relative_path":null,
            "content_hash":null,
            "timestamp":123
        })];
        let output = r#"{"proposals":[{"kind":"task","operation":"create","title":"跟进","payload":{},"source_refs":[{"source_type":"task","entity_id":7}]}]}"#;
        assert!(validate("global_analysis", output, &allowed).is_ok());
    }
}
