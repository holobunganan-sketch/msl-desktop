//! Read-only, scope-enforced business evidence. No model-authored SQL.
use super::{Database, DbError, DbResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub id: String,
    pub kind: String,
    pub entity_id: i64,
    pub title: String,
    pub text: String,
    pub timestamp: i64,
    pub trust: String,
    pub hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<Value>,
    #[serde(default)]
    pub priority: u8,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvidencePack {
    pub scope_ids: Vec<i64>,
    pub as_of: i64,
    pub sources: Vec<Evidence>,
    pub counts: BTreeMap<String, usize>,
    pub omitted: usize,
    pub notes: Vec<String>,
}
pub(crate) fn rows(
    conn: &rusqlite::Connection,
    sql: &str,
    args: &[&dyn rusqlite::ToSql],
) -> DbResult<Vec<Value>> {
    let mut stmt = conn.prepare(sql)?;
    let names = stmt
        .column_names()
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let values = stmt.query_map(args, |r| {
        let mut obj = serde_json::Map::new();
        for (i, name) in names.iter().enumerate() {
            use rusqlite::types::ValueRef;
            let value = match r.get_ref(i)? {
                ValueRef::Null => Value::Null,
                ValueRef::Integer(n) => Value::from(n),
                ValueRef::Real(n) => Value::from(n),
                ValueRef::Text(s) => Value::from(String::from_utf8_lossy(s).to_string()),
                ValueRef::Blob(_) => Value::Null,
            };
            obj.insert(name.clone(), value);
        }
        Ok(Value::Object(obj))
    })?;
    Ok(values.collect::<rusqlite::Result<Vec<_>>>()?)
}
pub fn validate_scope(conn: &rusqlite::Connection, scope: &[i64]) -> DbResult<Vec<i64>> {
    let mut ids = scope.to_vec();
    ids.sort_unstable();
    ids.dedup();
    if ids.len() > 20 {
        return Err(DbError::Migration("最多同时聚焦 20 个项目".into()));
    }
    for id in &ids {
        if *id <= 0
            || !conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM works WHERE id=?1)",
                [id],
                |r| r.get::<_, bool>(0),
            )?
        {
            return Err(DbError::NotFound("聚焦项目已移除，请重新选择范围".into()));
        }
    }
    Ok(ids)
}
pub fn expert_scope(
    conn: &rusqlite::Connection,
    scope: &[i64],
    expert: Option<i64>,
) -> DbResult<(Vec<i64>, Option<String>)> {
    let mut ids = scope.to_vec();
    let mut label = None;
    if let Some(expert) = expert {
        label = Some(
            conn.query_row("SELECT name FROM kol_experts WHERE id=?1", [expert], |r| {
                r.get::<_, String>(0)
            })
            .map_err(|_| DbError::NotFound("专家范围已失效，请重新选择".into()))?,
        );
        ids.extend(
            rows(
                conn,
                "SELECT work_id FROM kol_projects WHERE expert_id=?1",
                &[&expert],
            )?
            .iter()
            .filter_map(|v| v["work_id"].as_i64()),
        );
    }
    Ok((validate_scope(conn, &ids)?, label))
}
pub fn collect_scoped(
    db: &Database,
    scope: &[i64],
    query: &str,
    expert: Option<i64>,
    expert_scoped: bool,
) -> DbResult<EvidencePack> {
    if expert_scoped && expert.is_none() {
        return Err(DbError::NotFound("专家范围已失效，不能改为全局".into()));
    }
    let Some(expert) = expert else {
        return collect(db, scope, query);
    };
    let (ids, _) = expert_scope(db.conn(), scope, Some(expert))?;
    let mut pack = if ids.is_empty() {
        EvidencePack {
            as_of: super::now_unix(),
            ..Default::default()
        }
    } else {
        collect(db, &ids, query)?
    };
    let expert_pack = super::kol::qa_evidence_pack(db, expert)?;
    pack.sources.extend(expert_pack.sources);
    pack.omitted += expert_pack.omitted;
    pack.notes.extend(expert_pack.notes);
    pack.scope_ids = ids;
    pack.counts.extend(expert_pack.counts);
    prioritize(&mut pack, query, 64, 75000);
    attach_locations(db, &mut pack)?;
    Ok(pack)
}
pub(crate) fn prioritize(pack: &mut EvidencePack, query: &str, limit: usize, max_chars: usize) {
    pack.sources.sort_by(|a, b| {
        b.priority
            .cmp(&a.priority)
            .then(
                crate::cognition::relevance(query, &b.text)
                    .cmp(&crate::cognition::relevance(query, &a.text)),
            )
            .then(b.timestamp.cmp(&a.timestamp))
    });
    let mut seen = std::collections::BTreeSet::new();
    let mut chars = 0usize;
    let mut count = 0usize;
    pack.sources.retain(|s| {
        if !seen.insert(s.id.clone()) {
            return false;
        }
        if count >= limit || chars + s.text.chars().count() > max_chars {
            pack.omitted += 1;
            return false;
        }
        chars += s.text.chars().count();
        count += 1;
        true
    });
    if pack.omitted > 0 {
        pack.notes.push(format!(
            "本次证据预算省略 {} 条来源；未覆盖内容不能视为不存在。",
            pack.omitted
        ));
    }
}
pub fn evidence(kind: &str, row: &Value, trust: &str, query: &str) -> Evidence {
    let id = row["id"].as_i64().unwrap_or(0);
    let full = serde_json::to_string(row).unwrap_or_default();
    let text = crate::cognition::task_excerpt(&full, query, 2200);
    Evidence {
        id: format!("{kind}:{id}"),
        kind: kind.into(),
        entity_id: id,
        title: row["title"]
            .as_str()
            .or(row["name"].as_str())
            .or(row["relative_path"].as_str())
            .map(str::to_string)
            .unwrap_or_else(|| format!("{kind} #{id}")),
        timestamp: [
            "updated_at",
            "occurred_at",
            "timestamp",
            "created_at",
            "generated_at",
        ]
        .iter()
        .find_map(|k| row[*k].as_i64())
        .unwrap_or(0),
        hash: crate::cognition::digest(&text),
        text,
        trust: trust.into(),
        location: None,
        priority: if trust == "user_decision" {
            3
        } else if kind == "expert"
            || kind == "work"
            || (matches!(kind, "task" | "waiting" | "kol_followup")
                && !matches!(row["status"].as_str(), Some("done" | "resolved")))
        {
            2
        } else {
            0
        },
    }
}
pub fn collect(db: &Database, scope: &[i64], query: &str) -> DbResult<EvidencePack> {
    let scope = validate_scope(db.conn(), scope)?;
    let scope_json = serde_json::to_string(&scope).unwrap();
    let in_scope = "(?1='[]' OR work_id IN (SELECT value FROM json_each(?1)))";
    let mut pack = EvidencePack {
        scope_ids: scope.clone(),
        as_of: super::now_unix(),
        ..Default::default()
    };
    // These are fixed application queries. Neither SQL nor identifiers come from AI.
    let queries=vec![
        ("work",format!("SELECT * FROM works WHERE ?1='[]' OR id IN (SELECT value FROM json_each(?1))"),"record"),
        ("task",format!("SELECT * FROM tasks WHERE {in_scope}"),"record"),
        ("waiting",format!("SELECT * FROM waiting_items WHERE {in_scope}"),"record"),
        ("calendar",format!("SELECT * FROM calendar_events WHERE {in_scope}"),"record"),
        ("resume_point",format!("SELECT * FROM resume_points WHERE {in_scope}"),"record"),
        ("activity_rollup",format!("SELECT * FROM daily_activity_rollups WHERE {in_scope}"),"compacted_event_counts"),
        ("cognition","SELECT rowid AS id,scope_key AS title,markdown,generated_at,version FROM cognition_entries WHERE ?1='[]' OR scope_key IN (SELECT 'work-'||value FROM json_each(?1))".into(),"generated_entry"),
        ("file_ref",format!("SELECT id,work_id,workspace_id,label AS title,path,created_at FROM work_file_refs WHERE {in_scope}"),"reference_only"),
        ("proposal",format!("SELECT id,work_id,kind,operation,target_id,title,payload_json,reason,source_refs_json,status,created_at,updated_at FROM ai_proposals WHERE {in_scope}"),"proposal_not_fact"),
        ("inbox","SELECT DISTINCT i.*,c.work_id FROM inbox_items i LEFT JOIN capture_context c ON c.inbox_id=i.id WHERE ?1='[]' OR c.work_id IN (SELECT value FROM json_each(?1)) OR (i.converted_to_type='work' AND i.converted_to_id IN (SELECT value FROM json_each(?1))) OR (i.converted_to_type='task' AND i.converted_to_id IN (SELECT id FROM tasks WHERE work_id IN (SELECT value FROM json_each(?1)))) OR (i.converted_to_type='waiting' AND i.converted_to_id IN (SELECT id FROM waiting_items WHERE work_id IN (SELECT value FROM json_each(?1)))) OR (i.converted_to_type='calendar' AND i.converted_to_id IN (SELECT id FROM calendar_events WHERE work_id IN (SELECT value FROM json_each(?1))))".into(),"user_record"),
        ("activity",format!("SELECT id,timestamp,event_type,work_id,workspace_id,entity_type,entity_id,path,display_text FROM activity_events WHERE {in_scope} OR (work_id IS NULL AND workspace_id IN (SELECT workspace_id FROM work_workspace_links WHERE work_id IN (SELECT value FROM json_each(?1))))"),"event"),
        ("decision","SELECT d.id,d.reason_code,d.note,d.created_at,p.title,p.work_id,p.status FROM review_decisions d JOIN ai_proposals p ON p.id=d.proposal_id WHERE ?1='[]' OR p.work_id IN (SELECT value FROM json_each(?1))".into(),"user_decision"),
        ("kol_note",format!("SELECT n.id,n.expert_id,e.name,e.institution,e.department,n.work_id,n.content,n.occurred_at,n.created_at FROM kol_notes n JOIN kol_experts e ON e.id=n.expert_id WHERE {in_scope}"),"user_record"),
        ("expert","SELECT id,name,institution,department,specialty,archived,updated_at FROM kol_experts WHERE ?1='[]' OR id IN (SELECT expert_id FROM kol_projects WHERE work_id IN (SELECT value FROM json_each(?1)))".into(),"record"),
    ];
    for (kind, sql, trust) in queries {
        let items = rows(db.conn(), &sql, &[&scope_json])?;
        pack.counts.insert(kind.into(), items.len());
        for item in items {
            if let Some(status) = item["status"].as_str() {
                *pack.counts.entry(format!("{kind}.{status}")).or_default() += 1;
            }
            pack.sources.push(evidence(kind, &item, trust, query));
        }
    }
    let global_queries=[
        ("brief","SELECT id,brief_date AS title,content,generated_at,period_start,period_end,ai_used FROM daily_briefs WHERE retention_state='kept'","generated_summary"),
        ("report","SELECT id,kind AS title,content,period_start,period_end,generated_at FROM reports WHERE status='completed' AND retention_state='kept' AND NOT EXISTS(SELECT 1 FROM json_each(reports.evidence_json,'$.sources') s WHERE json_extract(s.value,'$.trust')='deleted')","generated_summary"),
        ("analysis","SELECT id,trigger AS title,status,summary,period_start,period_end,created_at FROM analysis_runs","generated_summary"),
        ("kol_insight","SELECT * FROM kol_insights","reviewed_hypothesis"),
    ];
    let materials = crate::materials::evidence(db, None, &scope, query)?;
    let material_count = crate::materials::evidence_count(db, None, &scope)?;
    pack.omitted += material_count.saturating_sub(materials.len());
    pack.counts.insert("kol_material".into(), material_count);
    pack.sources.extend(materials);
    for (kind, sql, trust) in global_queries {
        let mut items = rows(db.conn(), sql, &[])?;
        if kind == "kol_insight" {
            items.retain(|r| super::source_lifecycle::usable_insight(db.conn(), r));
        }
        if !scope.is_empty() {
            if kind == "kol_insight" {
                let note_ids = pack
                    .sources
                    .iter()
                    .filter(|s| s.kind == "kol_note" || s.kind == "kol_material")
                    .map(|s| s.id.as_str())
                    .collect::<std::collections::HashSet<_>>();
                items.retain(|r| {
                    serde_json::from_str::<Vec<Value>>(r["citations_json"].as_str().unwrap_or("[]"))
                        .is_ok_and(|refs| {
                            !refs.is_empty()
                                && refs.iter().all(|c| {
                                    note_ids.contains(c["source_id"].as_str().unwrap_or(""))
                                })
                        })
                });
            } else {
                if !items.is_empty() {
                    pack.notes.push(format!("{kind}: 跨项目历史摘要未保存可靠的分段项目 ID；聚焦时不混入全文，可在全局范围查阅。"));
                }
                continue;
            }
        }
        pack.counts.insert(kind.into(), items.len());
        pack.sources
            .extend(items.iter().map(|r| evidence(kind, r, trust, query)));
    }
    add_documents(db, &scope_json, query, &mut pack)?;

    // Counts are from the complete scope before ranking, never from the sample.
    let stats = serde_json::json!({"id":0,"title":"范围统计 / Scope counts","as_of":pack.as_of,"scope_ids":scope,"counts":pack.counts});
    pack.sources.sort_by(|a, b| {
        b.priority.cmp(&a.priority).then(
            crate::cognition::relevance(query, &b.text)
                .cmp(&crate::cognition::relevance(query, &a.text))
                .then(b.timestamp.cmp(&a.timestamp))
                .then(a.id.cmp(&b.id)),
        )
    });
    let mut selected = Vec::new();
    let mut budget = 0;
    for source in std::mem::take(&mut pack.sources) {
        if selected.len() >= 64 || budget + source.text.chars().count() > 75_000 {
            pack.omitted += 1;
            continue;
        }
        budget += source.text.chars().count();
        selected.push(source);
    }
    selected.insert(0, evidence("coverage", &stats, "computed", query));
    pack.sources = selected;
    attach_locations(db, &mut pack)?;
    pack.notes.push("内容按问题相关性选取；未命中、遗漏或不可读不能证明工作未发生。生成摘要及已审阅洞察均须回到原始证据核实。".into());
    Ok(pack)
}

pub fn source_location(conn: &rusqlite::Connection, kind: &str, id: i64) -> DbResult<Value> {
    let kind = match kind {
        "task_open" | "task_completed" => "task",
        "weekly_report" => "report",
        "kol_expert" => "expert",
        "resume" => "resume_point",
        _ => kind,
    };
    let table = match kind {
        "work" => "works",
        "task" => "tasks",
        "waiting" => "waiting_items",
        "calendar" => "calendar_events",
        "inbox" => "inbox_items",
        "resume_point" => "resume_points",
        "expert" => "kol_experts",
        "kol_note" => "kol_notes",
        "kol_insight" => "kol_insights",
        "kol_followup" => "kol_actions",
        "kol_material" => "material_segments",
        "report" => "reports",
        "proposal" => "ai_proposals",
        "decision" => "review_decisions",
        "document" => "document_index",
        "workspace" => "workspaces",
        "activity" | "file_change" => "activity_events",
        _ => return Ok(serde_json::json!({"entity_kind":kind,"entity_id":id,"available":false})),
    };
    let row = rows(conn, &format!("SELECT * FROM {table} WHERE id=?1"), &[&id])?
        .into_iter()
        .next();
    let Some(row) = row else {
        return Ok(serde_json::json!({"entity_kind":kind,"entity_id":id,"available":false}));
    };
    if kind == "decision" {
        return source_location(conn, "proposal", row["proposal_id"].as_i64().unwrap_or(0));
    }
    if kind == "kol_followup" {
        return source_location(
            conn,
            row["entity_kind"].as_str().unwrap_or(""),
            row["entity_id"].as_i64().unwrap_or(0),
        );
    }
    if matches!(kind, "activity" | "file_change") {
        if let (Some(k), Some(i)) = (row["entity_type"].as_str(), row["entity_id"].as_i64()) {
            if k != kind && !matches!(k, "activity" | "file_change") {
                return source_location(conn, k, i);
            }
        }
    }
    Ok(
        serde_json::json!({"entity_kind":kind,"entity_id":id,"work_id":if kind=="work" {Value::from(id)}else{row["work_id"].clone()},"expert_id":if kind=="expert"{Value::from(id)}else{row["expert_id"].clone()},"workspace_id":if kind=="workspace"{Value::from(id)}else{row["workspace_id"].clone()},"relative_path":row["relative_path"],"available":true}),
    )
}
pub fn attach_locations(db: &Database, pack: &mut EvidencePack) -> DbResult<()> {
    for source in &mut pack.sources {
        if source
            .location
            .as_ref()
            .is_some_and(|v| v["remote_only"] == true)
        {
            continue;
        }
        let mut location = source_location(db.conn(), &source.kind, source.entity_id)?;
        if source.trust == "deleted" {
            location["available"] = Value::Bool(false);
        }
        source.location = Some(location);
    }
    Ok(())
}

fn add_documents(
    db: &Database,
    scope_json: &str,
    query: &str,
    pack: &mut EvidencePack,
) -> DbResult<()> {
    let folders=rows(db.conn(),"SELECT DISTINCT w.id,w.root_path FROM workspaces w JOIN work_workspace_links l ON l.workspace_id=w.id WHERE w.enabled=1 AND (?1='[]' OR l.work_id IN (SELECT value FROM json_each(?1)))",&[&scope_json])?;
    let mut candidates = Vec::new();
    for folder in folders {
        let id = folder["id"].as_i64().unwrap();
        let root = std::path::PathBuf::from(folder["root_path"].as_str().unwrap_or(""));
        let root = root.canonicalize().ok();
        for doc in super::documents::DocumentIndexRepo::new(db.conn()).list(id)? {
            *pack.counts.entry("document".into()).or_default() += 1;
            let path = std::path::PathBuf::from(&doc.path);
            let current = path
                .canonicalize()
                .ok()
                .zip(root.as_ref())
                .is_some_and(|(p, r)| p.starts_with(r));
            if !current || doc.extract_status != "ready" {
                *pack
                    .counts
                    .entry("document.unavailable".into())
                    .or_default() += 1;
                continue;
            }
            let row = serde_json::json!({"id":doc.id,"relative_path":doc.relative_path,"summary":doc.summary,"modified_at":doc.modified_at,"workspace_id":id});
            let rank = crate::cognition::relevance(query, &row.to_string());
            candidates.push((rank, doc, row));
        }
    }
    candidates.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.modified_at.cmp(&a.1.modified_at)));
    let count = candidates.len();
    for (_, doc, mut row) in candidates.into_iter().take(24) {
        // Metadata validation prevents stale cache text being called current.
        let meta = std::fs::metadata(&doc.path).ok();
        let fresh = meta.as_ref().is_some_and(|m| {
            m.len() as i64 == doc.size
                && m.modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .is_some_and(|t| t.as_secs() as i64 == doc.modified_at)
        });
        let cached = if fresh {
            doc.cache_rel_path
                .as_deref()
                .and_then(|s| crate::storage::paths::existing_cache_path(s).ok())
                .and_then(|p| std::fs::read_to_string(p).ok())
        } else {
            None
        };
        let text = match cached {
            Some(text) => Some(text),
            None => {
                let extracted = crate::documents::extract_path(std::path::Path::new(&doc.path));
                if extracted.status == crate::documents::ExtractStatus::Ready {
                    Some(extracted.text)
                } else {
                    None
                }
            }
        };
        if let Some(text) = text {
            row["excerpt"] = Value::String(crate::cognition::task_excerpt(&text, query, 6000));
            row["read_at"] = Value::from(pack.as_of);
            let mut source = evidence("document", &row, "document_excerpt", query);
            source.text = serde_json::to_string(&row).unwrap();
            source.hash = crate::cognition::digest(&source.text);
            pack.sources.push(source);
        } else {
            *pack
                .counts
                .entry("document.unavailable".into())
                .or_default() += 1;
        }
    }
    if count > 24 {
        pack.omitted += count - 24;
        pack.notes.push(format!("已定位 {count} 个可读索引，本次按文件名/摘要相关性选择 24 个文件片段；可进一步指定文件名或关键词。"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn user_decision_survives_pressure_from_recent_matching_tasks() {
        let db = Database::open_in_memory().unwrap();
        let w = super::super::work::WorkRepo::new(db.conn())
            .insert("Synthetic", "active")
            .unwrap();
        let run = crate::ai::analysis::create_run(&db, "manual", 0, 1).unwrap();
        let p = super::super::ai::ProposalRepo::new(db.conn())
            .upsert_pending(
                run,
                "task",
                "create",
                None,
                Some(w.id),
                None,
                "x",
                "Original objective",
                "{}",
                "",
                "[]",
                None,
            )
            .unwrap()
            .unwrap();
        db.conn().execute("INSERT INTO review_decisions(proposal_id,reason_code,note,created_at) VALUES(?1,'misunderstood','Preserve objective',1)",[p.id]).unwrap();
        for _ in 0..90 {
            super::super::task::TaskRepo::new(db.conn())
                .insert(Some(w.id), "Recent keyword keyword", "normal", None, None)
                .unwrap();
        }
        let pack = collect(&db, &[w.id], "keyword").unwrap();
        assert!(pack.sources.iter().any(|s| s.kind == "decision"));
        assert!(pack.omitted > 0);
    }
    #[test]
    fn knowledge_scope_includes_compacted_progress_and_its_cognition_entry() {
        let db = Database::open_in_memory().unwrap();
        db.conn().execute_batch("INSERT INTO works(id,title,created_at,updated_at) VALUES(1,'Alpha',1,1),(2,'Beta',1,1); INSERT INTO daily_activity_rollups(work_id,rollup_date,event_type,event_count,first_at,last_at,sample_text,created_at) VALUES(1,'2026-09-01','work.updated',8,1,2,'Alpha history',1),(2,'2026-09-01','work.updated',9,1,2,'Beta historical',1); INSERT INTO cognition_entries(scope_key,fingerprint,version,markdown,generated_at) VALUES('work-1','a',1,'Alpha entry',1),('work-2','b',1,'Beta entry',1);").unwrap();
        let pack = collect(&db, &[1], "history entry").unwrap();
        assert!(pack.sources.iter().any(|s| s.kind == "activity_rollup"));
        assert!(pack.sources.iter().any(|s| s.kind == "cognition"));
        assert!(!serde_json::to_string(&pack)
            .unwrap()
            .contains("Beta historical"));
        assert!(!serde_json::to_string(&pack).unwrap().contains("Beta entry"));
    }
    #[test]
    fn knowledge_scope_excludes_other_projects_and_unscoped_notes() {
        let db = Database::open_in_memory().unwrap();
        db.conn().execute_batch("INSERT INTO works(id,title,status,created_at,updated_at) VALUES(1,'Alpha','active',1,1),(2,'Beta','archived',1,1); INSERT INTO tasks(id,work_id,title,status,priority,created_at,updated_at) VALUES(1,1,'Alpha deadline','next','normal',1,1),(2,2,'Beta secret','next','normal',1,1),(3,NULL,'Independent','next','normal',1,1); INSERT INTO inbox_items(id,content,created_at) VALUES(1,'Alpha original',1),(2,'Other original',1); INSERT INTO capture_context(inbox_id,work_id) VALUES(1,1);").unwrap();
        let pack = collect(&db, &[1], "deadline").unwrap();
        assert!(pack.sources.iter().any(|r| r.id == "task:1"));
        assert!(pack.sources.iter().any(|r| r.id == "inbox:1"));
        let text = serde_json::to_string(&pack).unwrap();
        assert!(
            !text.contains("Beta secret")
                && !text.contains("Independent")
                && !text.contains("Other original")
        );
        let all = collect(&db, &[], "").unwrap();
        assert_eq!(all.counts.get("task"), Some(&3));
    }
    #[test]
    fn knowledge_deleted_scope_cannot_fall_back_to_global() {
        let db = Database::open_in_memory().unwrap();
        assert!(collect(&db, &[888], "anything").is_err());
    }
    #[test]
    fn knowledge_never_exposes_provider_settings_or_conversation_as_evidence() {
        let db = Database::open_in_memory().unwrap();
        db.conn().execute_batch("INSERT INTO app_settings(key,value,updated_at) VALUES('unsafe-secret','DO_NOT_EXPOSE',1); INSERT INTO qa_sessions(title,created_at,updated_at) VALUES('invented history',1,1); INSERT INTO qa_turns(session_id,question,scope_json,status,answer_json,created_at) VALUES(1,'?', '[]','completed','{\"text\":\"invented answer\"}',1);").unwrap();
        let pack = collect(&db, &[], "secret").unwrap();
        assert!(pack.sources.iter().any(|s| s.kind == "coverage"));
        let text = serde_json::to_string(&pack).unwrap();
        assert!(!text.contains("DO_NOT_EXPOSE") && !text.contains("invented answer"));
    }
}
