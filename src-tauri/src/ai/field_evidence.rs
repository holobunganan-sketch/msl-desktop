//! Evidence checks for newly generated fields. Historical drafts remain editable.
//! This verifies provenance and deterministic values; it never treats a source ID
//! alone as proof of a model's interpretation.
use super::{analysis_snapshot::AnalysisSnapshot, schema::ProposalContract};
use serde_json::Value;

pub const DATE_FIELDS: &[&str] = &[
    "due_at",
    "follow_up_at",
    "scheduled_start",
    "scheduled_end",
    "start_at",
    "end_at",
    "started_at",
];

fn needs_evidence(field: &str, value: &Value, proposal: &ProposalContract) -> bool {
    if DATE_FIELDS.contains(&field) {
        return true;
    }
    match field {
        "status" => {
            proposal.operation == "update"
                || !matches!(value.as_str(), Some("active" | "next" | "open"))
        }
        "priority" => value != "normal",
        "summary" | "objective" | "notes" | "current_state" | "next_step" | "remember"
        | "waiting_for" | "location" | "category" | "clinical_work_id" => true,
        "observation" | "implication" | "uncertainty" | "next_question" => true,
        _ => false,
    }
}

fn scalar_text(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn evidence_value<'a>(snapshot: &'a Value, path: &str) -> Result<&'a Value, String> {
    let allowed = [
        "/brief/",
        "/focused_inbox/",
        "/focused_work/",
        "/documents/",
        "/expert_context/",
        "/user_directions/",
        "/project_catalog/",
    ];
    if !allowed.iter().any(|root| path.starts_with(root)) || path.len() > 300 {
        return Err("字段依据必须指向本次提供的原话或工作记录".into());
    }
    // Counts, round tickets, IDs and model-derived indexes cannot establish facts.
    if path.starts_with("/brief/source_counts/")
        || path.contains("/source_ref")
        || path.contains("/round_history/")
        || path.contains("/previous_arrangement")
    {
        return Err("技术记录不能作为字段事实依据".into());
    }
    let value = snapshot
        .pointer(path)
        .ok_or("字段依据指向的内容不在本次资料中")?;
    if scalar_text(value).is_none() {
        return Err("字段依据必须指向具体文本或值".into());
    }
    Ok(value)
}

fn unnegated_quote(supplied: &str, quote: &str) -> bool {
    let Some((prefix, _)) = supplied.split_once(quote) else {
        return false;
    };
    let before = prefix
        .chars()
        .rev()
        .take(12)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>()
        .to_lowercase();
    !["不", "未", "没有", "取消", "暂缓", "not ", "don't", "非"]
        .iter()
        .any(|n| before.contains(n))
}

fn exact_value(field: &str, proposed: &Value, supplied: &Value, path: &str, quote: &str) -> bool {
    let leaf = path.rsplit('/').next().unwrap_or("");
    if DATE_FIELDS.contains(&field) {
        // A database timestamp (e.g. created_at) is not a deadline/appointment.
        if DATE_FIELDS.contains(&leaf) {
            return supplied == proposed;
        }
        if !matches!(leaf, "content" | "selected_text" | "notes" | "review_note") {
            return false;
        }
        return supplied
            .as_str()
            .is_some_and(|text| unnegated_quote(text, quote))
            && chrono::DateTime::parse_from_rfc3339(quote)
                .ok()
                .is_some_and(|time| Some(time.timestamp()) == proposed.as_i64());
    }
    if matches!(
        field,
        "status" | "priority" | "category" | "clinical_work_id"
    ) {
        if leaf == field {
            return supplied == proposed;
        }
        if field == "category"
            && matches!(leaf, "content" | "selected_text" | "notes" | "review_note")
            && supplied
                .as_str()
                .is_some_and(|text| unnegated_quote(text, quote))
        {
            return matches!(
                (proposed.as_str(), quote),
                (Some("clinical"), "临床研究" | "clinical")
                    | (Some("non_clinical"), "非临床研究" | "non_clinical")
            );
        }
        return false;
    }
    supplied == proposed
}

fn target_record<'a>(input: &'a Value, kind: &str, id: i64) -> Option<&'a Value> {
    if kind == "work" {
        if input["focused_work"]["work"]["id"] == id {
            return Some(&input["focused_work"]["work"]);
        }
        return input["project_catalog"]
            .as_array()?
            .iter()
            .find(|v| v["id"] == id);
    }
    let (focused, brief) = match kind {
        "task" => ("tasks", vec!["tasks_open", "tasks_completed"]),
        "waiting" => ("waiting", vec!["waiting"]),
        "calendar" => ("calendar", vec!["calendar"]),
        "resume_point" => ("resume_history", vec!["resume_points"]),
        "inbox" => ("inbox", vec!["inbox"]),
        _ => return None,
    };
    if let Some(found) = input["focused_work"][focused]
        .as_array()
        .and_then(|rows| rows.iter().find(|v| v["id"] == id))
    {
        return Some(found);
    }
    if kind == "inbox" && input["focused_inbox"]["id"] == id {
        return Some(&input["focused_inbox"]);
    }
    brief.iter().find_map(|key| {
        input["brief"][key]
            .as_array()
            .and_then(|rows| rows.iter().find(|v| v["entity_id"] == id))
    })
}

fn user_statement_path(path: &str) -> bool {
    (path == "/focused_inbox/content"
        || path.starts_with("/brief/inbox/")
        || path.starts_with("/user_directions/"))
        && path.ends_with("/content")
}

fn definite_statement(text: &str) -> bool {
    let lower = text.to_lowercase();
    ![
        "不", "未", "没有", "取消", "暂缓", "可能", "如果", "假如", "预计", "打算", "计划", "应该",
        "？", "?", "not ", "don't", "might ", "maybe ", "plan to ",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn explicit_association(input: &Value, path: &str, quote: &str, proposed: &Value) -> bool {
    if !user_statement_path(path) || !definite_statement(quote) {
        return false;
    }
    if ![
        "关联到",
        "关联至",
        "挂到",
        "归入",
        "关联",
        "link to ",
        "attach to ",
    ]
    .iter()
    .any(|verb| quote.contains(verb))
    {
        return false;
    }
    let Some(catalog) = input["project_catalog"].as_array() else {
        return false;
    };
    let named = catalog
        .iter()
        .filter(|work| {
            work["category"] == "clinical"
                && work["status"] != "archived"
                && work["title"]
                    .as_str()
                    .is_some_and(|name| !name.is_empty() && quote.contains(name))
        })
        .collect::<Vec<_>>();
    named.len() == 1 && named[0]["id"] == *proposed
}

fn explicit_status(
    input: &Value,
    path: &str,
    quote: &str,
    proposal: &ProposalContract,
    proposed: &Value,
) -> bool {
    if !user_statement_path(path) || !definite_statement(quote) {
        return false;
    }
    let Some(target) = proposal.target_id else {
        return false;
    };
    let Some(record) = target_record(input, &proposal.kind, target) else {
        return false;
    };
    let owner_path = path.rsplit_once('/').map(|(owner, _)| owner).unwrap_or("");
    let owner = input.pointer(owner_path).unwrap_or(&Value::Null);
    let context = if path == "/focused_inbox/content" {
        &owner["capture_context"]
    } else {
        owner
    };
    let selected =
        context["entity_kind"] == proposal.kind && context["entity_id"].as_i64() == Some(target);
    if !selected
        && !record["title"]
            .as_str()
            .is_some_and(|title| title.chars().count() > 1 && quote.contains(title))
    {
        return false;
    }
    let phrases: &[&str] = match proposed.as_str() {
        Some("done") => &[
            "已经完成",
            "已完成",
            "完成了",
            "已经做完",
            "已做完",
            "已结束",
            "completed",
        ],
        Some("resolved") => &[
            "已经解决",
            "已解决",
            "已收到回复",
            "已经收到回复",
            "依赖已满足",
            "resolved",
        ],
        Some("paused") => &["暂停", "paused"],
        Some("archived") => &["归档", "archived"],
        _ => &[],
    };
    phrases.iter().any(|phrase| quote.contains(phrase))
}

/// Unknowns are omissions, not destructive null patches. User editing remains a
/// separate explicit path and can deliberately clear fields after review.
pub fn validate_and_normalize(
    proposal: &mut ProposalContract,
    snapshot: &AnalysisSnapshot,
) -> Result<(), String> {
    if proposal.payload.get("time_confirmation").is_some() {
        return Err("模型不能代替用户确认时间".into());
    }
    proposal
        .payload
        .as_object_mut()
        .ok_or("建议字段格式无效")?
        .retain(|_, value| !value.is_null());
    let input = serde_json::to_value(snapshot).map_err(|_| "读取字段依据失败")?;
    if proposal.operation == "update" {
        if proposal.kind == "work"
            && proposal
                .target_id
                .and_then(|id| target_record(&input, "work", id))
                .is_none()
        {
            return Err("要修改的项目未出现在本次资料中，请先核对项目，不可猜测编号".into());
        }
        if let Some(record) = proposal
            .target_id
            .and_then(|id| target_record(&input, &proposal.kind, id))
        {
            if proposal.work_id.is_none() && !matches!(proposal.kind.as_str(), "work" | "inbox") {
                proposal.work_id = record["work_id"].as_i64();
            }
            if proposal.kind == "work" {
                proposal.payload["project_revision"] = record["revision"].clone();
            }
            if proposal.payload.get("clinical_work_id").is_some() {
                let revision = record["clinical_relation_revision"]
                    .as_i64()
                    .or_else(|| {
                        snapshot
                            .project_catalog
                            .iter()
                            .filter_map(|w| w["clinical_links"].as_array())
                            .flatten()
                            .find(|link| {
                                link["entity_kind"] == proposal.kind
                                    && link["entity_id"].as_i64() == proposal.target_id
                            })
                            .and_then(|link| link["revision"].as_i64())
                    })
                    .unwrap_or(0);
                proposal.payload["clinical_relation_revision"] = Value::from(revision);
            }
        }
    }
    if let Some(unknowns) = proposal.payload.get("unknowns") {
        if !unknowns.as_array().is_some_and(|items| {
            items.len() <= 20
                && items.iter().all(|v| {
                    v.as_str()
                        .is_some_and(|s| !s.trim().is_empty() && s.chars().count() <= 500)
                })
        }) {
            return Err("待确认信息应为简短文本数组".into());
        }
    }
    let evidence = proposal.payload.get("field_evidence");
    if evidence.is_some_and(|v| !v.is_object()) {
        return Err("字段依据应为对象".into());
    }
    for (field, proposed) in proposal.payload.as_object().unwrap() {
        if !needs_evidence(field, proposed, proposal) {
            continue;
        }
        let entry = evidence.and_then(|v| v.get(field)).ok_or_else(|| {
            format!("字段 {field} 缺少可核验依据；请补充 field_evidence 或省略该字段")
        })?;
        let path = entry["snapshot_path"]
            .as_str()
            .ok_or("字段依据缺少 snapshot_path")?;
        let supplied = evidence_value(&input, path)?;
        let quote = entry["quote"]
            .as_str()
            .filter(|s| !s.trim().is_empty() && s.chars().count() <= 1200)
            .ok_or("字段依据缺少简短原文")?;
        if !scalar_text(supplied).unwrap().contains(quote) {
            return Err(format!("字段 {field} 引用的原文与资料不一致"));
        }
        let basis = entry["basis"]
            .as_str()
            .ok_or("字段依据缺少 explicit/suggestion 标记")?;
        if !matches!(basis, "explicit" | "suggestion") {
            return Err("字段依据类型无效".into());
        }
        if DATE_FIELDS.contains(&field.as_str()) && proposal.payload["time_basis"] == "inferred" {
            if basis != "suggestion" {
                return Err("推算时间须标记为建议并等待单独确认".into());
            }
            continue;
        }
        if DATE_FIELDS.contains(&field.as_str())
            || matches!(
                field.as_str(),
                "status" | "priority" | "category" | "clinical_work_id"
            )
        {
            let owner_path = path
                .rsplit_once('/')
                .map(|(parent, _)| parent)
                .unwrap_or("");
            let definite_source = supplied.as_str().is_some_and(definite_statement);
            let text_status = field == "status"
                && definite_source
                && explicit_status(&input, path, quote, proposal, proposed);
            let text_association = field == "clinical_work_id"
                && definite_source
                && explicit_association(&input, path, quote, proposed);
            if field == "status" && proposal.operation == "update" && !text_status {
                let owner = input.pointer(owner_path).unwrap_or(&Value::Null);
                let owner_id = owner["id"].as_i64().or(owner["entity_id"].as_i64());
                let source_kind = if path.starts_with("/focused_work/work/")
                    || path.starts_with("/project_catalog/")
                    || path.starts_with("/brief/works/")
                {
                    "work"
                } else if path.starts_with("/focused_work/tasks/")
                    || path.starts_with("/brief/tasks_")
                {
                    "task"
                } else if path.starts_with("/focused_work/waiting/")
                    || path.starts_with("/brief/waiting/")
                {
                    "waiting"
                } else {
                    ""
                };
                if owner_id != proposal.target_id || source_kind != proposal.kind {
                    return Err("状态依据属于另一条事项，不能用来改变当前事项".into());
                }
            }
            if basis != "explicit"
                || !(exact_value(field, proposed, supplied, path, quote)
                    || text_status
                    || text_association)
            {
                return Err(format!(
                    "字段 {field} 的值缺少明确依据；请保留未知或提出待确认问题"
                ));
            }
        }
        if matches!(field.as_str(), "waiting_for" | "location") && basis == "explicit" {
            let Some(value) = proposed.as_str() else {
                return Err("对象或地点应为文本".into());
            };
            if !quote.contains(value) {
                return Err(format!("字段 {field} 不能添加原文中没有的对象或地点"));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn snapshot() -> AnalysisSnapshot {
        super::super::analysis_snapshot::build(
            &crate::db::Database::open_in_memory().unwrap(),
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
    fn proposal(payload: Value) -> ProposalContract {
        serde_json::from_value(json!({"kind":"task","operation":"create","title":"Synthetic action","payload":payload})).unwrap()
    }
    #[test]
    fn accuracy_accepts_exact_supplied_datetime_but_never_fills_a_vague_deadline() {
        let mut s = snapshot();
        s.focused_inbox = Some(json!({"id":1,"content":"Meeting: 2026-10-01T15:00:00+08:00"}));
        let time = chrono::DateTime::parse_from_rfc3339("2026-10-01T15:00:00+08:00")
            .unwrap()
            .timestamp();
        let mut p = proposal(
            json!({"scheduled_start":time,"time_basis":"explicit","field_evidence":{"scheduled_start":{"snapshot_path":"/focused_inbox/content","quote":"2026-10-01T15:00:00+08:00","basis":"explicit"}}}),
        );
        assert!(validate_and_normalize(&mut p, &s).is_ok());
        s.focused_inbox = Some(json!({"id":1,"content":"尽快联系专家"}));
        p.payload["field_evidence"]["scheduled_start"]["quote"] = json!("尽快联系专家");
        assert!(validate_and_normalize(&mut p, &s).is_err());
    }
    #[test]
    fn accuracy_status_of_another_task_cannot_complete_this_task() {
        let mut s = snapshot();
        s.focused_work = Some(
            json!({"work":{"id":1},"tasks":[{"id":10,"status":"done"},{"id":11,"status":"next"}]}),
        );
        let mut p = proposal(
            json!({"status":"done","field_evidence":{"status":{"snapshot_path":"/focused_work/tasks/0/status","quote":"done","basis":"explicit"}}}),
        );
        p.operation = "update".into();
        p.target_id = Some(11);
        assert!(validate_and_normalize(&mut p, &s).is_err());
        s.focused_work.as_mut().unwrap()["work"] = json!({"id":11,"status":"done"});
        p.payload["field_evidence"]["status"]["snapshot_path"] = json!("/focused_work/work/status");
        assert!(
            validate_and_normalize(&mut p, &s).is_err(),
            "A project with the same numeric ID cannot complete a task"
        );
    }
    #[test]
    fn accuracy_raw_user_classification_is_usable_without_guessing_by_project_name() {
        let mut s = snapshot();
        s.focused_inbox = Some(json!({"id":1,"content":"项目类别：临床研究"}));
        let mut p = proposal(
            json!({"category":"clinical","field_evidence":{"category":{"snapshot_path":"/focused_inbox/content","quote":"临床研究","basis":"explicit"}}}),
        );
        p.kind = "work".into();
        assert!(validate_and_normalize(&mut p, &s).is_ok());
        s.focused_inbox = Some(json!({"id":1,"content":"这项工作不是临床研究"}));
        assert!(validate_and_normalize(&mut p, &s).is_err());
    }
    #[test]
    fn accuracy_model_cannot_inject_a_user_time_confirmation() {
        let mut p = proposal(json!({"time_confirmation":"user_confirmed"}));
        assert!(validate_and_normalize(&mut p, &snapshot()).is_err());
    }
    #[test]
    fn accuracy_explicit_new_clinical_association_requires_unique_named_study_and_no_negation() {
        let mut s = snapshot();
        s.project_catalog =
            vec![json!({"id":7,"title":"研究甲","category":"clinical","status":"active"})];
        s.focused_inbox = Some(json!({"id":1,"content":"这次交流关联到研究甲"}));
        let mut p = proposal(
            json!({"clinical_work_id":7,"field_evidence":{"clinical_work_id":{"snapshot_path":"/focused_inbox/content","quote":"这次交流关联到研究甲","basis":"explicit"}}}),
        );
        assert!(validate_and_normalize(&mut p, &s).is_ok());
        for content in [
            "不要把这次交流关联到研究甲",
            "可能把这次交流关联到研究甲",
            "这次交流提到了研究甲",
        ] {
            s.focused_inbox.as_mut().unwrap()["content"] = json!(content);
            p.payload["field_evidence"]["clinical_work_id"]["quote"] = json!(content);
            assert!(validate_and_normalize(&mut p, &s).is_err(), "{content}");
        }
        s.focused_inbox.as_mut().unwrap()["content"] = json!("不要这样做：这次交流关联到研究甲");
        p.payload["field_evidence"]["clinical_work_id"]["quote"] = json!("这次交流关联到研究甲");
        assert!(
            validate_and_normalize(&mut p, &s).is_err(),
            "A cropped quote must not erase the user's negation"
        );
        s.focused_inbox.as_mut().unwrap()["content"] = json!("这次交流关联到研究甲");
        p.payload["field_evidence"]["clinical_work_id"]["quote"] = json!("这次交流关联到研究甲");
        s.project_catalog
            .push(json!({"id":8,"title":"研究甲","category":"clinical","status":"active"}));
        assert!(validate_and_normalize(&mut p, &s).is_err());
    }
    #[test]
    fn accuracy_named_completed_observation_can_close_only_its_task() {
        let mut s = snapshot();
        s.focused_work = Some(
            json!({"work":{"id":1},"tasks":[{"id":10,"title":"核对研究方案","status":"next","work_id":1}]}),
        );
        s.focused_inbox = Some(json!({"id":1,"content":"我已经完成核对研究方案"}));
        let mut p = proposal(
            json!({"status":"done","field_evidence":{"status":{"snapshot_path":"/focused_inbox/content","quote":"我已经完成核对研究方案","basis":"explicit"}}}),
        );
        p.operation = "update".into();
        p.target_id = Some(10);
        assert!(validate_and_normalize(&mut p, &s).is_ok());
        for text in [
            "我还没有完成核对研究方案",
            "我打算完成核对研究方案",
            "核对研究方案可能已经完成",
            "我已经完成另一项工作",
        ] {
            s.focused_inbox.as_mut().unwrap()["content"] = json!(text);
            p.payload["field_evidence"]["status"]["quote"] = json!(text);
            assert!(validate_and_normalize(&mut p, &s).is_err(), "{text}");
        }
        s.focused_inbox.as_mut().unwrap()["content"] =
            json!("如果我已经完成核对研究方案，再安排复核");
        p.payload["field_evidence"]["status"]["quote"] = json!("我已经完成核对研究方案");
        assert!(
            validate_and_normalize(&mut p, &s).is_err(),
            "Do not turn a conditional clause into completed work"
        );
    }
}
