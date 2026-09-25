//! Bounded, traceable input assembled for AI secretary runs.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::ai::brief::{build_snapshot, BriefFact, BriefSnapshot};
use crate::db::documents::{DocumentIndex, DocumentIndexRepo};
use crate::db::{Database, DbResult};

pub const MAX_FILES: usize = 8;
pub const MAX_DOCUMENTS: usize = 100;
pub const MAX_FILE_CHARS: usize = 12_000;
pub const MAX_TOTAL_CHARS: usize = 40_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisSourceRef {
    pub source_type: String,
    pub entity_id: Option<i64>,
    pub workspace_id: Option<i64>,
    pub relative_path: Option<String>,
    pub content_hash: Option<String>,
    pub timestamp: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentEvidence {
    pub workspace_id: i64,
    pub document_id: i64,
    pub relative_path: String,
    pub extension: String,
    pub content_hash: Option<String>,
    pub summary: Option<String>,
    pub selected_text: Option<String>,
    pub selected_chars: usize,
    pub truncated: bool,
    pub source_ref: AnalysisSourceRef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisSnapshot {
    #[serde(default)]
    pub round_tickets: Vec<super::rounds::Ticket>,
    #[serde(default)]
    pub round_history: Vec<serde_json::Value>,
    #[serde(default)]
    pub expert_context: Vec<serde_json::Value>,
    /// Explicit user statements and corrections; old processed captures are
    /// guidance, not a request to recreate completed actions.
    #[serde(default)]
    pub user_directions: Vec<serde_json::Value>,
    /// Lightweight catalog for classifying loose information. Held projects
    /// remain ineligible for new proposals in the current secretary round.
    #[serde(default)]
    pub project_catalog: Vec<serde_json::Value>,
    #[serde(default)]
    pub capture_contexts: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focused_inbox: Option<serde_json::Value>,
    #[serde(default)]
    pub project_cognition: Vec<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focused_work: Option<serde_json::Value>,
    pub task_kind: String,
    pub locale: String,
    pub period_start: i64,
    pub period_end: i64,
    pub brief: BriefSnapshot,
    pub documents: Vec<DocumentEvidence>,
    pub source_refs: Vec<AnalysisSourceRef>,
    pub source_counts: serde_json::Value,
    pub classification_memory: crate::db::memory::ClassificationMemoryContext,
    pub truncated: std::collections::BTreeMap<String, u32>,
    pub snapshot_hash: String,
}

fn hash_snapshot(value: &serde_json::Value) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn select_documents(mut docs: Vec<DocumentIndex>) -> (Vec<DocumentIndex>, u32) {
    docs.retain(|doc| doc.extract_status == "ready");
    docs.sort_by(|left, right| {
        right
            .summary
            .is_some()
            .cmp(&left.summary.is_some())
            .then_with(|| {
                right
                    .last_extracted_at
                    .unwrap_or(0)
                    .cmp(&left.last_extracted_at.unwrap_or(0))
            })
            .then_with(|| {
                right
                    .last_accessed_at
                    .unwrap_or(0)
                    .cmp(&left.last_accessed_at.unwrap_or(0))
            })
            .then_with(|| left.relative_path.cmp(&right.relative_path))
    });
    let omitted = docs.len().saturating_sub(MAX_DOCUMENTS) as u32;
    docs.truncate(MAX_DOCUMENTS);
    (docs, omitted)
}

fn expand_global_workbench_sources(db: &Database, brief: &mut BriefSnapshot) -> DbResult<()> {
    let mut tasks = crate::db::task::TaskRepo::new(db.conn())
        .list(None, None)?
        .into_iter()
        .filter(|task| task.status != "done")
        .collect::<Vec<_>>();
    tasks.sort_by_key(|task| (task.due_at.unwrap_or(i64::MAX), task.id));
    if tasks.len() > 200 {
        brief
            .truncated
            .insert("tasks_open_global".into(), (tasks.len() - 200) as u32);
        tasks.truncate(200);
    }
    brief.tasks_open = tasks
        .into_iter()
        .map(|task| BriefFact {
            source_type: "task_open".into(),
            entity_id: Some(task.id),
            work_id: task.work_id,
            title: Some(task.title),
            display: None,
            timestamp: Some(task.updated_at),
            status: Some(task.status),
            priority: Some(task.priority),
            due_at: task.due_at,
            start_at: task.scheduled_start,
            end_at: task.scheduled_end,
            summary: task.notes,
            current_state: None,
            next_step: None,
            waiting_for: None,
            follow_up_at: None,
            content: None,
        })
        .collect();

    let mut calendar =
        crate::db::calendar::CalendarRepo::new(db.conn()).list_between(i64::MIN, i64::MAX)?;
    calendar.sort_by_key(|event| {
        (
            event.start_at < brief.today_start,
            event.start_at.abs_diff(brief.today_start),
        )
    });
    if calendar.len() > 200 {
        brief
            .truncated
            .insert("calendar_global".into(), (calendar.len() - 200) as u32);
        calendar.truncate(200);
    }
    brief.calendar = calendar
        .into_iter()
        .map(|event| BriefFact {
            source_type: "calendar".into(),
            entity_id: Some(event.id),
            work_id: event.work_id,
            title: Some(event.title),
            display: None,
            timestamp: Some(event.updated_at),
            status: Some(event.kind),
            priority: None,
            due_at: None,
            start_at: Some(event.start_at),
            end_at: event.end_at,
            summary: event.notes.or(event.location),
            current_state: None,
            next_step: None,
            waiting_for: None,
            follow_up_at: None,
            content: None,
        })
        .collect();
    brief.source_counts.tasks_open = brief.tasks_open.len() as u32;
    brief.source_counts.calendar = brief.calendar.len() as u32;
    Ok(())
}

pub(crate) fn user_directions(db: &Database) -> DbResult<Vec<serde_json::Value>> {
    let mut result = Vec::new();
    let mut captures = db.conn().prepare(
        "SELECT i.id,c.work_id,c.entity_kind,c.entity_id,i.content,i.created_at,i.processed_at \
         FROM capture_context c JOIN inbox_items i ON i.id=c.inbox_id \
         ORDER BY i.id DESC LIMIT 120",
    )?;
    result.extend(
        captures
            .query_map([], |row| {
                let content: String = row.get(4)?;
                Ok(serde_json::json!({
                    "type":"user_capture", "inbox_id":row.get::<_,i64>(0)?,
                    "work_id":row.get::<_,Option<i64>>(1)?,
                    "entity_kind":row.get::<_,Option<String>>(2)?,
                    "entity_id":row.get::<_,Option<i64>>(3)?,
                    "content":crate::cognition::bounded(&content,1200),
                    "created_at":row.get::<_,i64>(5)?,
                    "processed":row.get::<_,Option<i64>>(6)?.is_some()
                }))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?,
    );
    let mut decisions = db.conn().prepare(
        "SELECT d.id,p.work_id,p.title,d.reason_code,d.note,d.created_at \
         FROM review_decisions d JOIN ai_proposals p ON p.id=d.proposal_id \
         WHERE length(trim(d.note))>0 ORDER BY d.id DESC LIMIT 80",
    )?;
    result.extend(
        decisions
            .query_map([], |row| {
                let note: String = row.get(4)?;
                Ok(serde_json::json!({
                    "type":"review_correction", "decision_id":row.get::<_,i64>(0)?,
                    "work_id":row.get::<_,Option<i64>>(1)?,
                    "proposal_title":row.get::<_,String>(2)?,
                    "reason_code":row.get::<_,String>(3)?,
                    "content":crate::cognition::bounded(&note,1200),
                    "created_at":row.get::<_,i64>(5)?
                }))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?,
    );
    let mut insights = db.conn().prepare(
        "SELECT i.id,k.work_id,i.expert_id,i.title,i.status,i.review_note,i.updated_at \
         FROM kol_insights i LEFT JOIN kol_projects k ON k.expert_id=i.expert_id \
         WHERE i.status IN ('reviewed','revised','dismissed') \
         ORDER BY i.updated_at DESC,i.id DESC LIMIT 80",
    )?;
    result.extend(
        insights
            .query_map([], |row| {
                let note: String = row.get(5)?;
                Ok(serde_json::json!({
                    "type":"expert_insight_review", "insight_id":row.get::<_,i64>(0)?,
                    "work_id":row.get::<_,Option<i64>>(1)?,
                    "expert_id":row.get::<_,Option<i64>>(2)?,
                    "insight_title":row.get::<_,String>(3)?,
                    "status":row.get::<_,String>(4)?,
                    "content":crate::cognition::bounded(&note,1200),
                    "created_at":row.get::<_,i64>(6)?
                }))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?,
    );
    Ok(result)
}

fn project_catalog(db: &Database) -> DbResult<Vec<serde_json::Value>> {
    let mut works = crate::db::work::WorkRepo::new(db.conn())
        .list(None)?
        .into_iter()
        .filter(|work| work.status != "archived")
        .collect::<Vec<_>>();
    works.sort_by(|a, b| {
        b.updated_at
            .cmp(&a.updated_at)
            .then_with(|| b.id.cmp(&a.id))
    });
    let mut catalog = Vec::new();
    for work in works.into_iter().take(100) {
        let mut related = Vec::new();
        for query in [
            "SELECT title FROM tasks WHERE work_id=?1 AND status!='done' ORDER BY updated_at DESC LIMIT 3",
            "SELECT title FROM waiting_items WHERE work_id=?1 AND status='open' ORDER BY updated_at DESC LIMIT 2",
            "SELECT title FROM calendar_events WHERE work_id=?1 ORDER BY start_at DESC LIMIT 2",
            "SELECT e.name FROM kol_projects p JOIN kol_experts e ON e.id=p.expert_id WHERE p.work_id=?1 AND e.archived=0 ORDER BY e.updated_at DESC LIMIT 2",
        ] {
            let mut stmt = db.conn().prepare(query)?;
            related.extend(
                stmt.query_map([work.id], |row| row.get::<_, String>(0))?
                    .collect::<rusqlite::Result<Vec<_>>>()?
                    .into_iter()
                    .map(|title| crate::cognition::bounded(&title, 100)),
            );
        }
        catalog.push(serde_json::json!({
            "id":work.id, "title":work.title, "status":work.status,
            "objective":work.summary.as_deref().map(|text|crate::cognition::bounded(text,300)),
            "related_terms":related
        }));
    }
    Ok(catalog)
}

/// Build a bounded snapshot. It contains structured workbench facts plus selected,
/// cached document text; cache absolute paths are never serialized.
pub fn build(
    db: &Database,
    task_kind: &str,
    date: &str,
    period_start: i64,
    period_end: i64,
    today_start: i64,
    today_end: i64,
    locale: &str,
) -> DbResult<AnalysisSnapshot> {
    build_scoped(
        db,
        task_kind,
        date,
        period_start,
        period_end,
        today_start,
        today_end,
        locale,
        None,
    )
}

pub fn build_scoped(
    db: &Database,
    task_kind: &str,
    date: &str,
    period_start: i64,
    period_end: i64,
    today_start: i64,
    today_end: i64,
    locale: &str,
    workspace_filter: Option<&[i64]>,
) -> DbResult<AnalysisSnapshot> {
    let mut brief = build_snapshot(
        db,
        date,
        period_start,
        period_end,
        today_start,
        today_end,
        locale,
    )?;
    // Global rounds carry these in separately scoped fields below. Leaving
    // unscoped copies inside brief would expose held projects to the model.
    brief.user_directions.clear();
    brief.expert_context.clear();
    if matches!(
        task_kind,
        "global_analysis" | "weekly_report" | "monthly_report" | "work_draft"
    ) {
        expand_global_workbench_sources(db, &mut brief)?;
    }
    let mut documents = Vec::new();
    let mut source_refs = Vec::new();
    let mut total_chars = 0usize;
    let mut selected_files = 0usize;
    let mut truncated = brief.truncated.clone();
    for fact in brief
        .activity
        .iter()
        .chain(brief.works.iter())
        .chain(brief.resume_points.iter())
        .chain(brief.tasks_open.iter())
        .chain(brief.tasks_completed.iter())
        .chain(brief.waiting.iter())
        .chain(brief.calendar.iter())
        .chain(brief.inbox.iter())
        .chain(brief.file_changes.iter())
    {
        source_refs.push(AnalysisSourceRef {
            source_type: fact.source_type.clone(),
            entity_id: fact.entity_id,
            workspace_id: None,
            relative_path: None,
            content_hash: None,
            timestamp: fact.timestamp,
        });
    }
    let workspace_ids = crate::db::workspace::WorkspaceRepo::new(db.conn()).list()?;
    for workspace in workspace_ids {
        if !workspace.enabled {
            continue;
        }
        if workspace_filter.is_some_and(|ids| !ids.contains(&workspace.id)) {
            continue;
        }
        source_refs.push(AnalysisSourceRef {
            source_type: "workspace".into(),
            entity_id: Some(workspace.id),
            workspace_id: Some(workspace.id),
            relative_path: None,
            content_hash: None,
            timestamp: None,
        });
        let (workspace_documents, omitted) =
            select_documents(DocumentIndexRepo::new(db.conn()).list(workspace.id)?);
        if omitted > 0 {
            *truncated.entry("document_metadata".into()).or_insert(0) += omitted;
        }
        for doc in workspace_documents {
            let remaining = MAX_TOTAL_CHARS.saturating_sub(total_chars);
            let budget = remaining.min(MAX_FILE_CHARS);
            let mut selected_text = None;
            let mut selected_chars = 0;
            let mut was_truncated = false;
            let detailed = matches!(
                task_kind,
                "work_draft" | "workspace_analysis" | "monthly_report" | "weekly_report"
            );
            let changed_in_period =
                doc.modified_at >= period_start && doc.modified_at <= period_end;
            let file_limit = if detailed { MAX_FILES } else { 4 };
            let budget = if detailed {
                budget
            } else {
                budget
                    .min(4_000)
                    .min(12_000usize.saturating_sub(total_chars))
            };
            if (detailed || changed_in_period) && selected_files < file_limit && budget > 0 {
                if let Some(rel) = doc.cache_rel_path.as_deref() {
                    if let Ok(path) = crate::storage::paths::existing_cache_path(rel) {
                        if let Ok(text) = std::fs::read_to_string(path) {
                            let chars: Vec<char> = text.chars().collect();
                            let take = chars.len().min(budget);
                            was_truncated = take < chars.len();
                            let value: String = chars.into_iter().take(take).collect();
                            selected_chars = value.chars().count();
                            total_chars += selected_chars;
                            selected_files += 1;
                            selected_text = Some(value);
                            if was_truncated {
                                *truncated.entry("document_text".into()).or_insert(0) += 1;
                            }
                        }
                    }
                }
            }
            let source_ref = AnalysisSourceRef {
                source_type: "document".into(),
                entity_id: Some(doc.id),
                workspace_id: Some(doc.workspace_id),
                relative_path: Some(doc.relative_path.clone()),
                content_hash: doc.content_hash.clone(),
                timestamp: doc.last_extracted_at,
            };
            source_refs.push(source_ref.clone());
            documents.push(DocumentEvidence {
                workspace_id: doc.workspace_id,
                document_id: doc.id,
                relative_path: doc.relative_path,
                extension: doc.extension,
                content_hash: doc.content_hash.clone(),
                summary: if doc.summary_hash == doc.content_hash {
                    doc.summary.map(|s| crate::cognition::bounded(&s, 360))
                } else {
                    None
                },
                selected_text,
                selected_chars,
                truncated: was_truncated,
                source_ref,
            });
        }
    }
    let source_counts = serde_json::json!({
        "brief": brief.source_counts.clone(),
        "documents": documents.len(),
        "document_chars": total_chars,
    });
    let classification_memory = if matches!(
        task_kind,
        "global_analysis" | "workspace_analysis" | "work_draft"
    ) {
        crate::db::memory::context(db.conn(), crate::db::memory::MAX_MEMORY_EXAMPLES)?
    } else {
        crate::db::memory::ClassificationMemoryContext::default()
    };
    let mut project_cognition = Vec::new();
    // The deep entry remains local. Each request receives a bounded introduction.
    if let Some(ids) = workspace_filter {
        for id in ids.iter().take(4) {
            let entry = crate::cognition::preview(db, "workspace", Some(*id))?;
            project_cognition.push(serde_json::json!({"scope":entry.scope_key,"version":entry.version,"fingerprint":entry.fingerprint,"entry":crate::cognition::bounded(&entry.markdown,3000),"entry_truncated":entry.markdown.chars().count()>3000,"documents":entry.document_count,"readable":entry.ready_count,"unavailable_folders":entry.unavailable_folders}));
        }
        if ids.len() > 4 {
            truncated.insert("cognition_scopes".into(), (ids.len() - 4) as u32);
        }
    } else {
        let entry = crate::cognition::preview(db, "global", None)?;
        project_cognition.push(serde_json::json!({"scope":entry.scope_key,"version":entry.version,"fingerprint":entry.fingerprint,"entry":crate::cognition::bounded(&entry.markdown,6000),"entry_truncated":entry.markdown.chars().count()>6000,"documents":entry.document_count,"readable":entry.ready_count,"unavailable_folders":entry.unavailable_folders}));
    }
    let mut expert_context = crate::db::kol::secretary_context(db, workspace_filter)?;
    if expert_context.len() > 80 {
        expert_context.truncate(80);
        truncated.insert("expert_context_at_least".into(), 1);
    }
    for record in &mut expert_context {
        if let Some(content) = record["content"].as_str() {
            if content.chars().count() > 2000 {
                truncated.insert("expert_note_text".into(), 1);
            }
            record["content"] = serde_json::json!(crate::cognition::bounded(content, 2000));
        }
        if record["source_type"] == "kol_insight" {
            continue;
        }
        source_refs.push(AnalysisSourceRef {
            source_type: "kol_note".into(),
            entity_id: record["id"].as_i64(),
            workspace_id: None,
            relative_path: None,
            content_hash: None,
            timestamp: record["occurred_at"].as_i64(),
        });
    }
    let mut source_counts = source_counts;
    source_counts["expert_notes"] = serde_json::json!(expert_context
        .iter()
        .filter(|row| row["source_type"] != "kol_insight")
        .count());
    source_counts["expert_insights"] = serde_json::json!(expert_context
        .iter()
        .filter(|row| row["source_type"] == "kol_insight")
        .count());
    let mut snapshot = AnalysisSnapshot {
        round_tickets: Vec::new(),
        round_history: Vec::new(),
        expert_context,
        user_directions: user_directions(db)?,
        project_catalog: project_catalog(db)?,
        capture_contexts: {
            let mut stmt=db.conn().prepare("SELECT c.inbox_id,c.work_id,c.entity_kind,c.entity_id FROM capture_context c JOIN inbox_items i ON i.id=c.inbox_id ORDER BY i.created_at DESC,i.id DESC LIMIT 200")?;
            let rows=stmt.query_map([],|r|Ok(serde_json::json!({"inbox_id":r.get::<_,i64>(0)?,"work_id":r.get::<_,Option<i64>>(1)?,"entity_kind":r.get::<_,Option<String>>(2)?,"entity_id":r.get::<_,Option<i64>>(3)?})))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        },
        focused_inbox: None,
        project_cognition,
        focused_work: None,
        task_kind: task_kind.to_string(),
        locale: locale.to_string(),
        period_start,
        period_end,
        brief,
        documents,
        source_refs,
        source_counts,
        classification_memory,
        truncated,
        snapshot_hash: String::new(),
    };
    let value = serde_json::to_value(&snapshot).unwrap_or_else(|_| serde_json::json!({}));
    snapshot.snapshot_hash = hash_snapshot(&value);
    Ok(snapshot)
}

/// Include every category of project subitem, including future and completed work.
/// Bound each category and disclose omissions instead of silently claiming coverage.
pub fn focus_work(
    db: &Database,
    mut snapshot: AnalysisSnapshot,
    id: i64,
) -> DbResult<AnalysisSnapshot> {
    let work = crate::db::work::WorkRepo::new(db.conn())
        .get(id)?
        .ok_or_else(|| crate::db::DbError::NotFound("work".into()))?;
    let mut value = serde_json::json!({
        "work":work,
        "tasks":crate::db::task::TaskRepo::new(db.conn()).list(None, Some(id))?,
        "waiting":crate::db::task::WaitingRepo::new(db.conn()).list(None, Some(id))?,
        "calendar":crate::db::calendar::CalendarRepo::new(db.conn()).list_by_work(id)?,
        "resume_history":crate::db::work::ResumePointRepo::new(db.conn()).list_by_work(id)?,
    });
    for (key, source_type) in [
        ("tasks", "task"),
        ("waiting", "waiting"),
        ("calendar", "calendar"),
        ("resume_history", "resume_point"),
    ] {
        if let Some(items) = value[key].as_array_mut() {
            if items.len() > 300 {
                snapshot
                    .truncated
                    .insert(format!("focused_{key}"), (items.len() - 300) as u32);
                items.truncate(300);
            }
            for item in items {
                snapshot.source_refs.push(AnalysisSourceRef {
                    source_type: source_type.into(),
                    entity_id: item["id"].as_i64(),
                    workspace_id: None,
                    relative_path: None,
                    content_hash: None,
                    timestamp: item["updated_at"].as_i64().or(item["created_at"].as_i64()),
                });
            }
        }
    }
    snapshot.source_refs.push(AnalysisSourceRef {
        source_type: "work".into(),
        entity_id: Some(id),
        workspace_id: None,
        relative_path: None,
        content_hash: None,
        timestamp: Some(work.updated_at),
    });
    snapshot.focused_work = Some(value);
    let query = format!(
        "{} {} {}",
        work.title,
        work.summary.as_deref().unwrap_or(""),
        snapshot
            .focused_work
            .as_ref()
            .and_then(|v| v["tasks"].as_array())
            .map(|items| items
                .iter()
                .filter_map(|v| v["title"].as_str())
                .take(20)
                .collect::<Vec<_>>()
                .join(" "))
            .unwrap_or_default()
    );
    snapshot.documents.sort_by(|a, b| {
        crate::cognition::relevance(
            &query,
            &format!("{} {}", b.relative_path, b.summary.as_deref().unwrap_or("")),
        )
        .cmp(&crate::cognition::relevance(
            &query,
            &format!("{} {}", a.relative_path, a.summary.as_deref().unwrap_or("")),
        ))
    });
    let mut total = 0usize;
    for (position, doc) in snapshot.documents.iter_mut().enumerate() {
        doc.selected_text = None;
        doc.selected_chars = 0;
        doc.truncated = false;
        if position >= MAX_FILES || total >= MAX_TOTAL_CHARS {
            continue;
        }
        let rel: Option<String> = db.conn().query_row(
            "SELECT cache_rel_path FROM document_index WHERE id=?1 AND workspace_id=?2",
            rusqlite::params![doc.document_id, doc.workspace_id],
            |r| r.get(0),
        )?;
        if let Some(text) = rel
            .as_deref()
            .and_then(|rel| crate::storage::paths::existing_cache_path(rel).ok())
            .and_then(|path| std::fs::read_to_string(path).ok())
        {
            let budget = MAX_FILE_CHARS.min(MAX_TOTAL_CHARS - total);
            doc.truncated = text.chars().count() > budget;
            let excerpt = crate::cognition::task_excerpt(&text, &query, budget);
            doc.selected_chars = excerpt.chars().count();
            total += doc.selected_chars;
            doc.selected_text = Some(excerpt);
        }
    }
    snapshot.source_counts["document_chars"] = serde_json::json!(total);
    let entry = crate::cognition::preview(db, "work", Some(id))?;
    snapshot.project_cognition.insert(0,serde_json::json!({"scope":entry.scope_key,"version":entry.version,"fingerprint":entry.fingerprint,"entry":crate::cognition::bounded(&entry.markdown,4000),"entry_truncated":entry.markdown.chars().count()>4000,"documents":entry.document_count,"readable":entry.ready_count,"unavailable_folders":entry.unavailable_folders}));
    snapshot.snapshot_hash.clear();
    snapshot.snapshot_hash = hash_snapshot(&serde_json::to_value(&snapshot).unwrap_or_default());
    Ok(snapshot)
}

pub fn focus_workspace(mut snapshot: AnalysisSnapshot, workspace_id: i64) -> AnalysisSnapshot {
    snapshot
        .documents
        .retain(|document| document.workspace_id == workspace_id);
    snapshot.source_refs.retain(|source| {
        !matches!(source.source_type.as_str(), "workspace" | "document")
            || source.workspace_id == Some(workspace_id)
    });
    let selected_chars = snapshot
        .documents
        .iter()
        .map(|document| document.selected_chars)
        .sum::<usize>();
    snapshot.source_counts["documents"] = serde_json::json!(snapshot.documents.len());
    snapshot.source_counts["document_chars"] = serde_json::json!(selected_chars);
    snapshot.snapshot_hash.clear();
    let value = serde_json::to_value(&snapshot).unwrap_or_else(|_| serde_json::json!({}));
    snapshot.snapshot_hash = hash_snapshot(&value);
    snapshot
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::calendar::CalendarRepo;
    use crate::db::task::TaskRepo;
    use crate::db::workspace::WorkspaceRepo;

    #[test]
    fn retired_directories_do_not_supply_fresh_ai_document_context() {
        let db = Database::open_in_memory().unwrap();
        let repo = WorkspaceRepo::new(db.conn());
        let ws = repo.insert("retired", "C:/synthetic-retired").unwrap();
        DocumentIndexRepo::new(db.conn())
            .upsert(
                ws.id,
                "C:/synthetic-retired/a.txt",
                "a.txt",
                "txt",
                3,
                1,
                Some("old"),
                "ready",
                3,
                None,
                None,
                None,
            )
            .unwrap();
        repo.delete(ws.id).unwrap();
        let snapshot = build(
            &db,
            "global_analysis",
            "2026-09-06",
            0,
            200,
            0,
            200,
            "zh-CN",
        )
        .unwrap();
        assert!(snapshot.documents.is_empty());
        assert!(!snapshot
            .source_refs
            .iter()
            .any(|s| s.source_type == "workspace" && s.workspace_id == Some(ws.id)));
    }

    #[test]
    fn cognition_first_global_run_does_not_reread_unchanged_document_bodies() {
        let root =
            std::env::temp_dir().join(format!("msl-snapshot-cognition-{}", uuid::Uuid::new_v4()));
        let _guard = crate::storage::paths::LocalAppDataTestGuard::set(&root);
        let db = Database::open_in_memory().unwrap();
        let ws = WorkspaceRepo::new(db.conn())
            .insert("synthetic", "C:/synthetic")
            .unwrap();
        crate::storage::paths::write_cache_atomically("extracted/old.txt", b"old unchanged body")
            .unwrap();
        DocumentIndexRepo::new(db.conn())
            .upsert(
                ws.id,
                "C:/synthetic/old.txt",
                "old.txt",
                "txt",
                18,
                1,
                Some("old"),
                "ready",
                18,
                Some("extracted/old.txt"),
                None,
                None,
            )
            .unwrap();
        let snapshot = build(
            &db,
            "global_analysis",
            "2026-09-05",
            100,
            200,
            100,
            200,
            "zh-CN",
        )
        .unwrap();
        assert!(
            snapshot
                .documents
                .iter()
                .all(|doc| doc.selected_text.is_none()),
            "unchanged body was sent again"
        );
        let value = serde_json::to_value(&snapshot).unwrap();
        assert!(value["project_cognition"].is_array());
        assert!(!value["project_cognition"].as_array().unwrap().is_empty());
        let deep = build_scoped(
            &db,
            "work_draft",
            "2026-09-05",
            100,
            200,
            100,
            200,
            "zh-CN",
            Some(&[ws.id]),
        )
        .unwrap();
        assert_eq!(
            deep.documents[0].selected_text.as_deref(),
            Some("old unchanged body")
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn snapshot_is_bounded_and_does_not_include_absolute_cache_path() {
        let db = Database::open_in_memory().unwrap();
        let ws = WorkspaceRepo::new(db.conn())
            .insert("synthetic", "C:/synthetic")
            .unwrap();
        crate::db::documents::DocumentIndexRepo::new(db.conn())
            .upsert(
                ws.id,
                "C:/synthetic/a.txt",
                "a.txt",
                "txt",
                1,
                0,
                Some("hash"),
                "ready",
                3,
                None,
                None,
                None,
            )
            .unwrap();
        let snapshot = build(
            &db,
            "global_analysis",
            "2026-08-15",
            0,
            100,
            0,
            100,
            "zh-CN",
        )
        .unwrap();
        assert_eq!(snapshot.documents.len(), 1);
        assert!(snapshot.snapshot_hash.len() >= 32);
        let serialized = serde_json::to_string(&snapshot).unwrap();
        assert!(!serialized.contains("C:/synthetic"));
        assert!(snapshot.documents[0].selected_text.is_none());
        assert!(snapshot
            .source_refs
            .iter()
            .any(|source| source.source_type == "workspace" && source.entity_id == Some(ws.id)));
    }

    #[test]
    fn global_snapshot_includes_future_open_tasks_and_calendar_commitments() {
        let db = Database::open_in_memory().unwrap();
        TaskRepo::new(db.conn())
            .insert(None, "未来待办", "normal", Some(1_000), None)
            .unwrap();
        CalendarRepo::new(db.conn())
            .insert(None, "未来日程", 1_000, None, false, "meeting", None, None)
            .unwrap();
        let snapshot = build(
            &db,
            "global_analysis",
            "2026-08-20",
            0,
            100,
            0,
            100,
            "zh-CN",
        )
        .unwrap();
        assert!(snapshot
            .brief
            .tasks_open
            .iter()
            .any(|fact| fact.title.as_deref() == Some("未来待办")));
        assert!(snapshot
            .brief
            .calendar
            .iter()
            .any(|fact| fact.title.as_deref() == Some("未来日程")));
    }

    #[test]
    fn global_snapshot_recalls_confirmed_classification_feedback() {
        let db = Database::open_in_memory().unwrap();
        let run = crate::db::ai::AnalysisRunRepo::new(db.conn())
            .create("manual", None, None)
            .unwrap();
        let repo = crate::db::ai::ProposalRepo::new(db.conn());
        let proposal = repo
            .upsert_pending(
                run.id,
                "task",
                "create",
                None,
                None,
                None,
                "snapshot-memory",
                "等待医学部回复",
                "{}",
                "等待外部答复",
                "[]",
                Some(0.7),
            )
            .unwrap()
            .unwrap();
        let corrected = repo
            .update_classification(
                proposal.id,
                proposal.updated_at,
                "waiting",
                None,
                "等待医学部回复",
                r#"{"waiting_for":"医学部"}"#,
            )
            .unwrap();
        crate::ai::apply::confirm_proposal(&db, corrected.id, corrected.updated_at, None).unwrap();
        db.conn().execute("INSERT INTO review_decisions(proposal_id,reason_code,note,created_at) VALUES (?1,'misunderstood','The user clarified this is a waiting item',1)", [corrected.id]).unwrap();

        let snapshot = build(
            &db,
            "global_analysis",
            "2026-08-20",
            0,
            100,
            0,
            100,
            "zh-CN",
        )
        .unwrap();
        assert_eq!(snapshot.classification_memory.feedback_count, 1);
        assert_eq!(
            snapshot.classification_memory.examples[0]
                .preferred_kind
                .as_deref(),
            Some("waiting")
        );
        assert!(snapshot
            .user_directions
            .iter()
            .any(|row| row["type"] == "review_correction"
                && row["content"] == "The user clarified this is a waiting item"));
    }

    #[test]
    fn secretary_keeps_user_corrections_after_capture_is_processed() {
        let db = Database::open_in_memory().unwrap();
        let work = crate::db::work::WorkRepo::new(db.conn())
            .insert("Synthetic project", "active")
            .unwrap();
        crate::db::task::TaskRepo::new(db.conn())
            .insert(
                Some(work.id),
                "Connected planning item",
                "normal",
                None,
                None,
            )
            .unwrap();
        let note = crate::db::flow::capture(
            db.conn(),
            "User changed the project direction",
            Some(work.id),
            Some("work"),
            Some(work.id),
        )
        .unwrap();
        db.conn()
            .execute(
                "UPDATE inbox_items SET processed_at=1 WHERE id=?1",
                [note.id],
            )
            .unwrap();
        let snapshot = build(
            &db,
            "global_analysis",
            "2026-09-25",
            0,
            100,
            0,
            100,
            "zh-CN",
        )
        .unwrap();
        assert!(snapshot.user_directions.iter().any(|row| {
            row["inbox_id"] == note.id
                && row["work_id"] == work.id
                && row["content"] == "User changed the project direction"
        }));
        assert!(snapshot
            .project_catalog
            .iter()
            .any(|row| row["id"] == work.id
                && row["related_terms"].as_array().is_some_and(|terms| terms
                    .iter()
                    .any(|term| term == "Connected planning item"))));
    }

    #[test]
    fn secretary_receives_expert_original_notes_without_prior_ai_insight() {
        let db = Database::open_in_memory().unwrap();
        let work = crate::db::work::WorkRepo::new(db.conn())
            .insert("Synthetic project", "active")
            .unwrap();
        db.conn().execute("INSERT INTO kol_experts(name,institution,created_at,updated_at) VALUES('Expert','Clinic',1,1)", []).unwrap();
        db.conn().execute("INSERT INTO kol_notes(expert_id,work_id,content,occurred_at,created_at) VALUES(1,?1,'Expert raised an evidence gap',1,1)", [work.id]).unwrap();
        let snapshot = build(
            &db,
            "global_analysis",
            "2026-09-25",
            0,
            100,
            0,
            100,
            "zh-CN",
        )
        .unwrap();
        assert!(snapshot
            .expert_context
            .iter()
            .any(|row| row["content"] == "Expert raised an evidence gap"));
    }

    #[test]
    fn workspace_intake_snapshot_keeps_only_the_selected_workspace_documents() {
        let db = Database::open_in_memory().unwrap();
        let first = WorkspaceRepo::new(db.conn())
            .insert("first", "C:/first")
            .unwrap();
        let second = WorkspaceRepo::new(db.conn())
            .insert("second", "C:/second")
            .unwrap();
        for (workspace_id, root, name) in [
            (first.id, "C:/first", "first.txt"),
            (second.id, "C:/second", "second.txt"),
        ] {
            DocumentIndexRepo::new(db.conn())
                .upsert(
                    workspace_id,
                    &format!("{root}/{name}"),
                    name,
                    "txt",
                    10,
                    1,
                    Some(name),
                    "ready",
                    10,
                    None,
                    None,
                    None,
                )
                .unwrap();
        }
        let snapshot = build(&db, "work_draft", "2026-09-04", 0, 10, 0, 10, "zh-CN").unwrap();
        let focused = focus_workspace(snapshot, first.id);
        assert_eq!(focused.documents.len(), 1);
        assert_eq!(focused.documents[0].relative_path, "first.txt");
        assert!(focused.source_refs.iter().any(|source| {
            source.source_type == "workspace" && source.workspace_id == Some(first.id)
        }));
        assert!(!focused.source_refs.iter().any(|source| {
            matches!(source.source_type.as_str(), "workspace" | "document")
                && source.workspace_id == Some(second.id)
        }));
    }
}
