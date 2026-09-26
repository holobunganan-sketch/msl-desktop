//! Typed identity slots only. Free text, quotations and file bodies are never rewritten.
use super::{SyncError, SyncResult};
use serde_json::Value;

#[derive(Clone)]
pub(super) struct Slot {
    pub column: String,
    pub pointer: String,
    pub table: &'static str,
    pub prefix: String,
    pub key: String,
}
pub(super) fn kind_table(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "work" | "project" => "works",
        "task" | "task_open" | "task_completed" => "tasks",
        "waiting" => "waiting_items",
        "calendar" => "calendar_events",
        "inbox" => "inbox_items",
        "resume" | "resume_point" | "progress" => "resume_points",
        "activity" | "file_change" => "activity_events",
        "proposal" => "ai_proposals",
        "decision" => "review_decisions",
        "report" | "weekly_report" => "reports",
        "expert" | "kol_expert" => "kol_experts",
        "kol_note" => "kol_notes",
        "kol_draft" => "kol_drafts",
        "kol_insight" => "kol_insights",
        "kol_material" => "material_segments",
        "qa_turn" => "qa_turns",
        "qa_session" => "qa_sessions",
        _ => return None,
    })
}
fn add(
    slots: &mut Vec<Slot>,
    column: &str,
    pointer: &str,
    table: &'static str,
    prefix: &str,
    value: &Value,
) {
    let key = match value {
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        _ => return,
    };
    if key.parse::<i64>().is_ok_and(|n| n > 0) {
        slots.push(Slot {
            column: column.into(),
            pointer: pointer.into(),
            table,
            prefix: prefix.into(),
            key,
        });
    }
}
fn escape(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}
fn walk(slots: &mut Vec<Slot>, column: &str, pointer: &str, value: &Value) {
    match value {
        Value::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                walk(slots, column, &format!("{pointer}/{i}"), item)
            }
        }
        Value::Object(object) => {
            for (key, item) in object {
                let p = format!("{pointer}/{}", escape(key));
                let table = match key.as_str() {
                    "work_id" | "project_id" | "suggested_work_id" | "preferred_work_id" => {
                        Some("works")
                    }
                    "expert_id" => Some("kol_experts"),
                    "inbox_id" => Some("inbox_items"),
                    "material_id" => Some("kol_materials"),
                    _ => None,
                };
                if let Some(table) = table {
                    add(slots, column, &p, table, "", item);
                }
                if matches!(key.as_str(), "scope_ids" | "source_report_ids") {
                    if let Some(ids) = item.as_array() {
                        for (i, id) in ids.iter().enumerate() {
                            add(
                                slots,
                                column,
                                &format!("{p}/{i}"),
                                if key == "scope_ids" {
                                    "works"
                                } else {
                                    "reports"
                                },
                                "",
                                id,
                            );
                        }
                    }
                }
                if matches!(key.as_str(), "source_id" | "id") {
                    if let Some((kind, id)) = item.as_str().and_then(|s| s.split_once(':')) {
                        if let Some(table) = kind_table(kind) {
                            add(
                                slots,
                                column,
                                &p,
                                table,
                                &format!("{kind}:"),
                                &Value::String(id.into()),
                            );
                        }
                    }
                }
                walk(slots, column, &p, item);
            }
            for (kind, id) in [
                ("source_type", "entity_id"),
                ("entity_kind", "entity_id"),
                ("entity_type", "entity_id"),
                ("kind", "entity_id"),
                ("kind", "target_id"),
                ("owner_kind", "owner_id"),
                ("converted_to_type", "converted_to_id"),
            ] {
                if let Some(table) = object
                    .get(kind)
                    .and_then(Value::as_str)
                    .and_then(kind_table)
                {
                    if let Some(value) = object.get(id) {
                        add(slots, column, &format!("{pointer}/{id}"), table, "", value);
                    }
                }
            }
        }
        _ => {}
    }
}
pub(super) fn slots(table: &str, row: &Value) -> SyncResult<Vec<Slot>> {
    let mut out = Vec::new();
    // Top-level identity fields handled by the same typed pairing rules.
    walk(&mut out, "", "", row);
    for column in [
        "scope_json",
        "linked_projects_json",
        "source_report_ids_json",
        "payload_json",
        "source_refs_json",
        "answer_json",
        "evidence_json",
        "structured_json",
        "citations_json",
        "document_json",
    ] {
        let Some(raw) = row.get(column).and_then(Value::as_str) else {
            continue;
        };
        if raw.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(raw).map_err(|_| {
            SyncError::new(
                "SYNC_REFERENCE_INVALID",
                format!("{table} 的结构化字段无法验证"),
            )
        })?;
        if matches!(
            column,
            "scope_json" | "source_report_ids_json" | "linked_projects_json"
        ) {
            if let Some(ids) = value.as_array() {
                for (i, id) in ids.iter().enumerate() {
                    add(
                        &mut out,
                        column,
                        &format!("/{i}"),
                        if column != "source_report_ids_json" {
                            "works"
                        } else {
                            "reports"
                        },
                        "",
                        id,
                    );
                }
            }
        } else {
            walk(&mut out, column, "", &value);
        }
    }
    out.sort_by(|a, b| (&a.column, &a.pointer).cmp(&(&b.column, &b.pointer)));
    out.dedup_by(|a, b| a.column == b.column && a.pointer == b.pointer);
    Ok(out)
}
pub(super) fn tag(slot: &Slot) -> String {
    format!("typed|{}|{}", slot.column, slot.pointer)
}
pub(super) fn replace(row: &mut Value, slot: &Slot, key: &str) -> SyncResult<()> {
    let mut value = if slot.column.is_empty() {
        row.clone()
    } else {
        serde_json::from_str(row[&slot.column].as_str().unwrap_or("null"))
            .map_err(|_| SyncError::new("SYNC_REFERENCE_INVALID", "关系字段无法读取"))?
    };
    let target = value
        .pointer_mut(&slot.pointer)
        .ok_or_else(|| SyncError::new("SYNC_REFERENCE_INVALID", "关系位置不存在"))?;
    *target = if !slot.prefix.is_empty() {
        Value::String(format!("{}{key}", slot.prefix))
    } else if target.is_string() {
        Value::String(key.into())
    } else {
        Value::from(
            key.parse::<i64>()
                .map_err(|_| SyncError::new("SYNC_REFERENCE_INVALID", "关系编号无效"))?,
        )
    };
    if slot.column.is_empty() {
        *row = value;
    } else {
        row[&slot.column] = Value::String(value.to_string());
    }
    Ok(())
}

/// Machine-local indices have no portable identity. Preserve the cited text but
/// never resolve their numbers against this receiver's unrelated local files.
pub(super) fn retire_local_locations(row: &mut Value) -> SyncResult<()> {
    fn walk(value: &mut Value) {
        match value {
            Value::Array(items) => {
                for item in items {
                    walk(item)
                }
            }
            Value::Object(object) => {
                let local = ["source_type", "entity_kind", "kind"].iter().any(|k| {
                    object
                        .get(*k)
                        .and_then(Value::as_str)
                        .is_some_and(|v| matches!(v, "document" | "workspace" | "file_ref"))
                });
                for item in object.values_mut() {
                    walk(item);
                }
                if object.contains_key("workspace_id") {
                    object.insert("workspace_id".into(), Value::Null);
                }
                if local {
                    object.insert("entity_id".into(), Value::from(0));
                    object.insert("available".into(), Value::Bool(false));
                    object.insert("remote_only".into(), Value::Bool(true));
                    if object.contains_key("source_type") || object.contains_key("kind") {
                        object.insert("location".into(),serde_json::json!({"entity_kind":object.get("source_type").or(object.get("kind")).cloned().unwrap_or(Value::Null),"entity_id":0,"workspace_id":null,"available":false,"remote_only":true}));
                    }
                }
            }
            _ => {}
        }
    }
    for column in [
        "evidence_json",
        "structured_json",
        "source_refs_json",
        "citations_json",
        "answer_json",
        "document_json",
    ] {
        if let Some(raw) = row[column].as_str() {
            if let Ok(mut value) = serde_json::from_str::<Value>(raw) {
                walk(&mut value);
                row[column] = Value::String(value.to_string());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn material_evidence_identity_is_the_reading_segment_not_the_file() {
        assert_eq!(super::kind_table("kol_material"), Some("material_segments"));
    }
}
