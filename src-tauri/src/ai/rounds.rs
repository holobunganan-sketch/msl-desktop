//! User-paced secretary rounds. File scanning never advances a round.
//! Reservations live in SQLite so concurrent command entry points share a gate.
use crate::db::{now_unix, Database, DbResult};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticket {
    pub scope: String,
    pub epoch: i64,
    pub fingerprint: String,
}
#[derive(Debug, Serialize)]
pub struct RoundStatus {
    pub scope: String,
    pub state: String,
    pub pending: i64,
    pub last_run: Option<i64>,
    pub epoch: i64,
}

fn id(scope: &str, prefix: &str) -> Option<i64> {
    scope.strip_prefix(prefix)?.parse().ok()
}
pub fn inbox_scope(conn: &Connection, inbox: i64) -> DbResult<String> {
    let work: Option<i64> = conn
        .query_row(
            "SELECT work_id FROM capture_context WHERE inbox_id=?1",
            [inbox],
            |r| r.get(0),
        )
        .optional()?
        .flatten();
    Ok(work
        .map(|w| format!("work:{w}"))
        .unwrap_or_else(|| format!("inbox:{inbox}")))
}
fn entity_scope(conn: &Connection, kind: &str, target: i64) -> DbResult<Option<String>> {
    if kind == "work" {
        return Ok(Some(format!("work:{target}")));
    }
    if kind == "inbox" {
        return Ok(Some(inbox_scope(conn, target)?));
    }
    let table = match kind {
        "task" | "task_open" | "task_completed" => "tasks",
        "waiting" => "waiting_items",
        "calendar" => "calendar_events",
        "resume_point" | "resume" => "resume_points",
        "kol_note" => "kol_notes",
        _ => return Ok(None),
    };
    let work: Option<i64> = conn
        .query_row(
            &format!("SELECT work_id FROM {table} WHERE id=?1"),
            [target],
            |r| r.get(0),
        )
        .optional()?
        .flatten();
    Ok(Some(
        work.map(|w| format!("work:{w}"))
            .unwrap_or_else(|| "loose".into()),
    ))
}
pub fn proposal_scopes(
    conn: &Connection,
    kind: &str,
    target: Option<i64>,
    work: Option<i64>,
    sources: &Value,
) -> DbResult<BTreeSet<String>> {
    let mut scopes = BTreeSet::new();
    if let Some(w) = work {
        scopes.insert(format!("work:{w}"));
    }
    if let Some(t) = target {
        if let Some(s) = entity_scope(conn, kind, t)? {
            scopes.insert(s);
        }
    }
    if let Some(refs) = sources.as_array() {
        for r in refs {
            if let (Some(kind), Some(target)) = (r["source_type"].as_str(), r["entity_id"].as_i64())
            {
                if let Some(s) = entity_scope(conn, kind, target)? {
                    scopes.insert(s);
                }
            }
        }
    }
    Ok(scopes)
}
// Adopt old and newly synced opinions without rewriting their contents/status.
fn adopt(conn: &Connection) -> DbResult<()> {
    let mut stmt=conn.prepare("SELECT id,kind,target_id,work_id,source_refs_json,workspace_id FROM ai_proposals WHERE id NOT IN (SELECT proposal_id FROM secretary_proposal_scopes)")?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<i64>>(2)?,
                r.get::<_, Option<i64>>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, Option<i64>>(5)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (p, kind, target, work, refs, workspace) in rows {
        let mut scopes = proposal_scopes(
            conn,
            &kind,
            target,
            work,
            &serde_json::from_str(&refs).unwrap_or_default(),
        )?;
        if scopes.is_empty() {
            if let Some(ws) = workspace {
                let mut s =
                    conn.prepare("SELECT work_id FROM work_workspace_links WHERE workspace_id=?1")?;
                for w in s.query_map([ws], |r| r.get::<_, i64>(0))? {
                    scopes.insert(format!("work:{}", w?));
                }
                if scopes.is_empty() {
                    scopes.insert(format!("workspace:{ws}"));
                }
            } else {
                scopes.insert("loose".into());
            }
        }
        for scope in scopes {
            conn.execute(
                "INSERT OR IGNORE INTO secretary_proposal_scopes VALUES (?1,?2)",
                params![p, scope],
            )?;
        }
    }
    Ok(())
}

// Hash business content, never wall-clock timestamps, scanner output, model
// summaries, last-access times or UI flags. Accepting drafts resets this baseline.
pub fn fingerprint(conn: &Connection, scope: &str) -> DbResult<String> {
    fingerprint_with_expert_feedback(conn, scope, true)
}

fn fingerprint_with_expert_feedback(
    conn: &Connection,
    scope: &str,
    include_expert_feedback: bool,
) -> DbResult<String> {
    let work = id(scope, "work:");
    let mut data = Vec::<Value>::new();
    let tables = [
        "works",
        "tasks",
        "waiting_items",
        "calendar_events",
        "resume_points",
        "inbox_items",
    ];
    for table in tables {
        let condition = if let Some(w) = work {
            match table {
                "works" => format!("id={w}"),
                "inbox_items" => {
                    format!("id IN (SELECT inbox_id FROM capture_context WHERE work_id={w})")
                }
                _ => format!("work_id={w}"),
            }
        } else if let Some(inbox) = id(scope, "inbox:") {
            if table != "inbox_items" {
                continue;
            }
            format!("id={inbox}")
        } else if scope == "loose" {
            if !matches!(table, "tasks" | "waiting_items" | "calendar_events") {
                continue;
            }
            "work_id IS NULL".into()
        } else {
            continue;
        };
        let mut stmt = conn.prepare(&format!(
            "SELECT * FROM {table} WHERE {condition} ORDER BY id"
        ))?;
        let cols = stmt
            .column_names()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        let rows = stmt.query_map([], |r| {
            let mut obj = serde_json::Map::new();
            for (i, c) in cols.iter().enumerate() {
                if matches!(
                    c.as_str(),
                    "created_at"
                        | "updated_at"
                        | "revision"
                        | "processed_at"
                        | "converted_to_id"
                        | "converted_to_type"
                ) {
                    continue;
                }
                let v: rusqlite::types::Value = r.get(i)?;
                let value = match v {
                    rusqlite::types::Value::Null => Value::Null,
                    rusqlite::types::Value::Integer(n) => json!(n),
                    rusqlite::types::Value::Real(n) => json!(n),
                    rusqlite::types::Value::Text(s) => json!(s),
                    rusqlite::types::Value::Blob(_) => Value::Null,
                };
                obj.insert(c.clone(), value);
            }
            Ok(Value::Object(obj))
        })?;
        data.push(json!({"table":table,"rows":rows.collect::<rusqlite::Result<Vec<_>>>()?}));
    }
    if include_expert_feedback && (work.is_some() || scope == "loose") {
        let condition = if let Some(w) = work {
            format!("n.work_id={w}")
        } else {
            "n.work_id IS NULL".into()
        };
        let mut stmt = conn.prepare(&format!(
            "SELECT n.id,n.expert_id,n.work_id,n.content,n.occurred_at \
             FROM kol_notes n JOIN kol_experts e ON e.id=n.expert_id \
             WHERE e.archived=0 AND {condition} ORDER BY n.id"
        ))?;
        let notes = stmt
            .query_map([], |r| {
                Ok(
                    json!({"id":r.get::<_,i64>(0)?,"expert_id":r.get::<_,i64>(1)?,
                "work_id":r.get::<_,Option<i64>>(2)?,"content":r.get::<_,String>(3)?,
                "occurred_at":r.get::<_,i64>(4)?}),
                )
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        data.push(json!({"table":"kol_notes","rows":notes}));
        let project_condition = if let Some(w) = work {
            format!("k.work_id={w}")
        } else {
            "k.work_id IS NULL".into()
        };
        let mut stmt = conn.prepare(&format!(
            "SELECT i.id,i.status,i.review_note,k.work_id FROM kol_insights i \
             LEFT JOIN kol_projects k ON k.expert_id=i.expert_id \
             WHERE i.status IN ('reviewed','revised','dismissed') \
             AND {project_condition} ORDER BY i.id"
        ))?;
        let reviews = stmt
            .query_map([], |r| {
                Ok(
                    json!({"id":r.get::<_,i64>(0)?,"status":r.get::<_,String>(1)?,
                "review_note":r.get::<_,String>(2)?,"work_id":r.get::<_,Option<i64>>(3)?}),
                )
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        data.push(json!({"table":"expert_user_reviews","rows":reviews}));
    }
    let mut closed = conn.prepare("SELECT p.id,p.status FROM ai_proposals p WHERE p.status IN ('resolved','deleted') AND (p.id IN (SELECT proposal_id FROM secretary_proposal_scopes WHERE scope=?1) OR (?2 IS NOT NULL AND p.work_id=?2)) ORDER BY p.id")?;
    let decisions = closed
        .query_map(params![scope, work], |r| {
            Ok(json!({"id":r.get::<_,i64>(0)?,"status":r.get::<_,String>(1)?}))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    data.push(json!({"closed_decisions":decisions}));
    Ok(crate::cognition::digest(&json!(data).to_string()))
}
fn ensure(conn: &Connection, scope: &str) -> DbResult<()> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM secretary_rounds WHERE scope=?1)",
        [scope],
        |r| r.get(0),
    )?;
    if !exists {
        let last:Option<i64>=conn.query_row("SELECT MAX(p.analysis_run_id) FROM ai_proposals p JOIN secretary_proposal_scopes s ON s.proposal_id=p.id WHERE s.scope=?1",[scope],|r|r.get(0))?;
        let seen: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM secretary_proposal_scopes WHERE scope=?1)",
            [scope],
            |r| r.get(0),
        )?;
        conn.execute(
            "INSERT INTO secretary_rounds(scope,baseline,last_run) VALUES (?1,?2,?3)",
            params![
                scope,
                if seen {
                    fingerprint(conn, scope)?
                } else {
                    String::new()
                },
                last
            ],
        )?;
    } else {
        // Existing 0.3.10 baselines used the same business tables without
        // expert notes. Upgrade a truly unchanged scope in place so installing
        // this version does not look like user progress and launch new advice.
        let baseline: String = conn.query_row(
            "SELECT baseline FROM secretary_rounds WHERE scope=?1",
            [scope],
            |row| row.get(0),
        )?;
        if !baseline.is_empty() && baseline == fingerprint_with_expert_feedback(conn, scope, false)?
        {
            conn.execute(
                "UPDATE secretary_rounds SET baseline=?1 WHERE scope=?2",
                params![fingerprint(conn, scope)?, scope],
            )?;
        }
    }
    Ok(())
}
fn status_inner(conn: &Connection, scope: &str) -> DbResult<RoundStatus> {
    ensure(conn, scope)?;
    let (baseline, last, active, epoch): (String, Option<i64>, Option<i64>, i64) = conn.query_row(
        "SELECT baseline,last_run,active_run,epoch FROM secretary_rounds WHERE scope=?1",
        [scope],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )?;
    let running = if let Some(run) = active {
        conn.query_row("SELECT EXISTS(SELECT 1 FROM analysis_runs WHERE id=?1 AND status='running' AND started_at>?2)",params![run,now_unix()-1800],|r|r.get::<_,bool>(0))?
    } else {
        false
    };
    let pending:i64=conn.query_row("SELECT COUNT(*) FROM ai_proposals p JOIN secretary_proposal_scopes s ON s.proposal_id=p.id WHERE s.scope=?1 AND p.status='pending'",[scope],|r|r.get(0))?;
    let state = if running {
        "running"
    } else if pending > 0 {
        "waiting_review"
    } else if baseline == fingerprint(conn, scope)? {
        "waiting_progress"
    } else {
        "ready"
    };
    Ok(RoundStatus {
        scope: scope.into(),
        state: state.into(),
        pending,
        last_run: last,
        epoch,
    })
}
pub fn status(db: &Database, scope: &str) -> DbResult<RoundStatus> {
    let tx = crate::db::write_transaction(db.conn())?;
    adopt(&tx)?;
    let s = status_inner(&tx, scope)?;
    tx.commit()?;
    Ok(s)
}
pub fn scopes_for_proposal(db: &Database, proposal: i64) -> DbResult<Vec<String>> {
    adopt(db.conn())?;
    let mut stmt = db.conn().prepare(
        "SELECT scope FROM secretary_proposal_scopes WHERE proposal_id=?1 ORDER BY scope",
    )?;
    let result = stmt
        .query_map([proposal], |r| r.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(result)
}
pub fn acknowledge_result(conn: &Connection, result: &super::apply::ApplyResult) -> DbResult<()> {
    adopt(conn)?;
    if let Some(scope) = entity_scope(conn, &result.kind, result.target_id)? {
        conn.execute(
            "INSERT OR IGNORE INTO secretary_proposal_scopes VALUES (?1,?2)",
            params![result.proposal_id, scope],
        )?;
    }
    acknowledge(conn, result.proposal_id)
}
pub fn release_new_association(
    conn: &Connection,
    result: &super::apply::ApplyResult,
) -> DbResult<()> {
    // Explicitly placing external advice in a different project adds new
    // material there. Ordinary acceptance within its original project waits.
    let (current, suggested): (Option<i64>, Option<i64>) = conn.query_row(
        "SELECT work_id,suggested_work_id FROM ai_proposals WHERE id=?1",
        [result.proposal_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if let Some(work) = current.filter(|w| Some(*w) != suggested) {
        let scope = format!("work:{work}");
        ensure(conn, &scope)?;
        conn.execute(
            "UPDATE secretary_rounds SET baseline='',epoch=epoch+1,active_run=NULL WHERE scope=?1",
            [&scope],
        )?;
    }
    Ok(())
}
pub fn candidates(
    conn: &Connection,
    inbox: Option<i64>,
    work: Option<i64>,
    workspace: Option<i64>,
) -> DbResult<Vec<String>> {
    if let Some(w) = work {
        return Ok(vec![format!("work:{w}")]);
    }
    if let Some(i) = inbox {
        return Ok(vec![inbox_scope(conn, i)?]);
    }
    if let Some(ws) = workspace {
        let mut s = conn.prepare(
            "SELECT work_id FROM work_workspace_links WHERE workspace_id=?1 ORDER BY work_id",
        )?;
        let linked = s
            .query_map([ws], |r| r.get::<_, i64>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        return Ok(if linked.is_empty() {
            vec![format!("workspace:{ws}")]
        } else {
            linked.iter().map(|w| format!("work:{w}")).collect()
        });
    }
    let mut s=conn.prepare("SELECT 'work:'||id FROM works WHERE status<>'archived' UNION SELECT 'inbox:'||id FROM inbox_items WHERE processed_at IS NULL AND id NOT IN (SELECT inbox_id FROM capture_context WHERE work_id IS NOT NULL) UNION SELECT 'loose' WHERE EXISTS(SELECT 1 FROM tasks WHERE work_id IS NULL AND status<>'done') OR EXISTS(SELECT 1 FROM waiting_items WHERE work_id IS NULL AND status='open') OR EXISTS(SELECT 1 FROM calendar_events WHERE work_id IS NULL) OR EXISTS(SELECT 1 FROM kol_notes WHERE work_id IS NULL)")?;
    let scopes = s
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(scopes)
}
pub fn reserve(db: &Database, run: i64, scopes: &[String]) -> DbResult<Vec<Ticket>> {
    let tx = crate::db::write_transaction(db.conn())?;
    adopt(&tx)?;
    let mut tickets = Vec::new();
    for scope in scopes {
        let s = status_inner(&tx, scope)?;
        if s.state != "ready" {
            continue;
        }
        tx.execute(
            "UPDATE secretary_rounds SET active_run=?1 WHERE scope=?2",
            params![run, scope],
        )?;
        tickets.push(Ticket {
            scope: scope.clone(),
            epoch: s.epoch,
            fingerprint: fingerprint(&tx, scope)?,
        });
    }
    if tickets.is_empty() {
        super::analysis::finish_run(
            db,
            run,
            "reused",
            Some("秘书正在等待您处理已有建议或推进事项；本次未调用模型。"),
            None,
        )?;
    }
    tx.commit()?;
    Ok(tickets)
}
pub fn valid(conn: &Connection, run: i64, tickets: &[Ticket]) -> DbResult<bool> {
    adopt(conn)?;
    for t in tickets {
        let ok:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM secretary_rounds WHERE scope=?1 AND epoch=?2 AND active_run=?3)",params![t.scope,t.epoch,run],|r|r.get(0))?;
        if !ok || fingerprint(conn, &t.scope)? != t.fingerprint {
            return Ok(false);
        }
        let pending:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM ai_proposals p JOIN secretary_proposal_scopes s ON s.proposal_id=p.id WHERE s.scope=?1 AND p.status='pending' AND p.analysis_run_id IS NOT ?2)",params![t.scope,run],|r|r.get(0))?;
        if pending {
            return Ok(false);
        }
    }
    Ok(true)
}
pub fn remember(conn: &Connection, run: i64, tickets: &[Ticket]) -> DbResult<()> {
    for t in tickets {
        conn.execute(
            "UPDATE secretary_rounds SET baseline=?1,last_run=?2,active_run=NULL WHERE scope=?3",
            params![fingerprint(conn, &t.scope)?, run, t.scope],
        )?;
    }
    Ok(())
}
pub fn acknowledge(conn: &Connection, proposal: i64) -> DbResult<()> {
    adopt(conn)?;
    let mut s = conn.prepare("SELECT scope FROM secretary_proposal_scopes WHERE proposal_id=?1")?;
    let scopes = s
        .query_map([proposal], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for scope in scopes {
        ensure(conn, &scope)?;
        conn.execute(
            "UPDATE secretary_rounds SET baseline=?1,epoch=epoch+1,active_run=NULL WHERE scope=?2",
            params![fingerprint(conn, &scope)?, scope],
        )?;
    }
    Ok(())
}
/// Explicitly finishing a round retains opinions as history and changes no
/// tasks, files or appointments. Actual task completion remains a separate act.
pub fn complete(db: &Database, scope: &str, expected_epoch: i64) -> DbResult<()> {
    let tx = crate::db::write_transaction(db.conn())?;
    adopt(&tx)?;
    let s = status_inner(&tx, scope)?;
    if s.epoch != expected_epoch
        || !matches!(s.state.as_str(), "waiting_review" | "waiting_progress")
    {
        return Err(crate::db::DbError::Migration(
            "项目状态已变化，请刷新后再完成本轮".into(),
        ));
    }
    tx.execute("UPDATE ai_proposals SET status='completed',decided_at=?1,updated_at=MAX(updated_at+1,?1) WHERE status='pending' AND id IN (SELECT proposal_id FROM secretary_proposal_scopes WHERE scope=?2)",params![now_unix(),scope])?;
    tx.execute("UPDATE secretary_rounds SET baseline='',epoch=epoch+1,active_run=NULL,completed_at=?1 WHERE scope=?2",params![now_unix(),scope])?;
    tx.commit()?;
    Ok(())
}

/// Exclude held scopes from the model's input, including mixed/global cognition.
pub fn restrict(
    db: &Database,
    mut snapshot: super::analysis_snapshot::AnalysisSnapshot,
    tickets: Vec<Ticket>,
) -> DbResult<super::analysis_snapshot::AnalysisSnapshot> {
    let allowed = tickets
        .iter()
        .map(|t| t.scope.clone())
        .collect::<BTreeSet<_>>();
    let workspace_allowed = |ws: i64| -> DbResult<bool> {
        if allowed.contains(&format!("workspace:{ws}")) {
            return Ok(true);
        }
        let mut s = db
            .conn()
            .prepare("SELECT work_id FROM work_workspace_links WHERE workspace_id=?1")?;
        let works = s
            .query_map([ws], |r| r.get::<_, i64>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(!works.is_empty() && works.iter().all(|w| allowed.contains(&format!("work:{w}"))))
    };
    let allowed_workspaces = crate::db::workspace::WorkspaceRepo::new(db.conn())
        .list()?
        .into_iter()
        .filter_map(|w| match workspace_allowed(w.id) {
            Ok(true) => Some(Ok(w.id)),
            Ok(false) => None,
            Err(e) => Some(Err(e)),
        })
        .collect::<DbResult<BTreeSet<_>>>()?;
    let fact_allowed = |f: &super::brief::BriefFact| {
        if let Some(w) = f.work_id {
            return allowed.contains(&format!("work:{w}"));
        }
        if let Some(i) = f.entity_id {
            if let Ok(Some(s)) = entity_scope(db.conn(), &f.source_type, i) {
                return allowed.contains(&s);
            }
        }
        false
    };
    let b = &mut snapshot.brief;
    for facts in [
        &mut b.works,
        &mut b.tasks_open,
        &mut b.tasks_completed,
        &mut b.waiting,
        &mut b.calendar,
        &mut b.resume_points,
        &mut b.inbox,
        &mut b.activity,
        &mut b.file_changes,
    ] {
        facts.retain(fact_allowed);
    }
    snapshot
        .documents
        .retain(|d| allowed_workspaces.contains(&d.workspace_id));
    snapshot.source_refs.retain(|r| {
        if let Some(ws) = r.workspace_id {
            return allowed_workspaces.contains(&ws);
        }
        r.entity_id
            .and_then(|i| entity_scope(db.conn(), &r.source_type, i).ok().flatten())
            .is_some_and(|s| allowed.contains(&s))
    });
    snapshot.project_cognition.retain(|v| {
        v["scope"]
            .as_str()
            .is_some_and(|s| allowed.contains(&s.replacen('-', ":", 1)))
    });
    snapshot.capture_contexts.retain(|v| {
        v["inbox_id"]
            .as_i64()
            .and_then(|i| inbox_scope(db.conn(), i).ok())
            .is_some_and(|s| allowed.contains(&s))
    });
    snapshot.user_directions.retain(|v| {
        if let Some(w) = v["work_id"].as_i64() {
            return allowed.contains(&format!("work:{w}"));
        }
        if let Some(inbox) = v["inbox_id"].as_i64() {
            return inbox_scope(db.conn(), inbox).is_ok_and(|scope| allowed.contains(&scope));
        }
        allowed.contains("loose")
    });
    // Keep expert evidence explicitly attached to eligible projects.
    snapshot.expert_context.retain(|v| {
        v["work_id"].as_i64().map_or_else(
            || allowed.contains("loose"),
            |w| allowed.contains(&format!("work:{w}")),
        )
    });
    snapshot.source_counts = json!({"works":b.works.len(),"tasks":b.tasks_open.len()+b.tasks_completed.len(),"waiting":b.waiting.len(),"calendar":b.calendar.len(),"inbox":b.inbox.len(),"documents":snapshot.documents.len(),"eligible_scopes":allowed});
    for t in &tickets {
        if let Some(work) = id(&t.scope, "work:") {
            let key = format!("work-{work}");
            if !snapshot.project_cognition.iter().any(|v| v["scope"] == key) {
                let entry = crate::cognition::preview(db, "work", Some(work))?;
                snapshot.project_cognition.push(json!({"scope":key,"version":entry.version,"entry":crate::cognition::bounded(&entry.markdown,4000),"entry_truncated":entry.markdown.chars().count()>4000}));
            }
        }
        let mut stmt=db.conn().prepare("SELECT p.id,p.kind,p.title,p.status,p.payload_json FROM ai_proposals p JOIN secretary_proposal_scopes s ON s.proposal_id=p.id WHERE s.scope=?1 AND p.status IN ('pending','confirmed') ORDER BY p.id DESC LIMIT 30")?;
        let items=stmt.query_map([&t.scope],|r|Ok(json!({"id":r.get::<_,i64>(0)?,"kind":r.get::<_,String>(1)?,"title":r.get::<_,String>(2)?,"status":r.get::<_,String>(3)?,"previous_arrangement":crate::cognition::bounded(&r.get::<_,String>(4)?,2000)})))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let mut stmt=db.conn().prepare("SELECT p.id,p.title,p.status,o.kind,o.target_id FROM ai_proposals p JOIN secretary_proposal_scopes s ON s.proposal_id=p.id LEFT JOIN ai_proposal_outcomes o ON o.proposal_id=p.id WHERE s.scope=?1 AND p.status IN ('resolved','deleted') ORDER BY p.id DESC LIMIT 200")?;
        let closed=stmt.query_map([&t.scope],|r|Ok(json!({"id":r.get::<_,i64>(0)?,"title":r.get::<_,String>(1)?,"status":r.get::<_,String>(2)?,"kind":r.get::<_,Option<String>>(3)?,"target_id":r.get::<_,Option<i64>>(4)?})))?.collect::<rusqlite::Result<Vec<_>>>()?;
        snapshot
            .round_history
            .push(json!({"scope":t.scope,"opinions":items,"closed_opinions":closed}));
    }
    b.source_counts.works = b.works.len() as u32;
    b.source_counts.tasks_open = b.tasks_open.len() as u32;
    b.source_counts.tasks_completed = b.tasks_completed.len() as u32;
    b.source_counts.waiting = b.waiting.len() as u32;
    b.source_counts.calendar = b.calendar.len() as u32;
    b.source_counts.inbox = b.inbox.len() as u32;
    b.source_counts.resume_points = b.resume_points.len() as u32;
    b.source_counts.activity = b.activity.len() as u32;
    b.source_counts.file_changes = b.file_changes.len() as u32;
    super::lifecycle::filter_active_context(db, &mut snapshot)?;
    snapshot.round_tickets = tickets;
    snapshot.snapshot_hash =
        crate::cognition::digest(&serde_json::to_string(&snapshot).unwrap_or_default());
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn project(db: &Database) -> i64 {
        crate::db::work::WorkRepo::new(db.conn())
            .insert("Synthetic project", "active")
            .unwrap()
            .id
    }
    fn run(db: &Database) -> i64 {
        super::super::analysis::create_run(db, "manual", 0, 1).unwrap()
    }
    fn opinion(db: &Database, w: i64, r: i64) -> crate::db::ai::AiProposal {
        crate::db::ai::ProposalRepo::new(db.conn())
            .upsert_pending(
                r,
                "task",
                "create",
                None,
                Some(w),
                None,
                &format!("new:{r}"),
                "Synthetic action",
                "{}",
                "reason",
                "[]",
                None,
            )
            .unwrap()
            .unwrap()
    }
    #[test]
    fn legacy_pending_blocks_all_entry_points_and_new_wording() {
        let db = Database::open_in_memory().unwrap();
        let w = project(&db);
        opinion(&db, w, run(&db));
        let i = crate::db::inbox::InboxRepo::new(db.conn())
            .insert("Synthetic note")
            .unwrap();
        db.conn()
            .execute(
                "INSERT INTO capture_context(inbox_id,work_id) VALUES (?1,?2)",
                params![i.id, w],
            )
            .unwrap();
        let scopes = candidates(db.conn(), Some(i.id), None, None).unwrap();
        assert_eq!(scopes, vec![format!("work:{w}")]);
        assert!(reserve(&db, run(&db), &scopes).unwrap().is_empty());
        assert_eq!(status(&db, &scopes[0]).unwrap().state, "waiting_review");
    }
    #[test]
    fn confirmation_waits_for_actual_task_progress() {
        let db = Database::open_in_memory().unwrap();
        let w = project(&db);
        let scope = format!("work:{w}");
        let p = opinion(&db, w, run(&db));
        status(&db, &scope).unwrap();
        let result = crate::ai::apply::confirm_proposal(&db, p.id, p.updated_at, None).unwrap();
        assert_eq!(status(&db, &scope).unwrap().state, "waiting_progress");
        assert!(reserve(&db, run(&db), &[scope.clone()]).unwrap().is_empty());
        db.conn()
            .execute(
                "UPDATE tasks SET status='done',completed_at=123 WHERE id=?1",
                [result.target_id],
            )
            .unwrap();
        assert_eq!(status(&db, &scope).unwrap().state, "ready");
    }
    #[test]
    fn a_new_expert_note_releases_only_its_project_round() {
        let db = Database::open_in_memory().unwrap();
        let work = project(&db);
        let scope = format!("work:{work}");
        let first = run(&db);
        let tickets = reserve(&db, first, &[scope.clone()]).unwrap();
        remember(db.conn(), first, &tickets).unwrap();
        assert_eq!(status(&db, &scope).unwrap().state, "waiting_progress");
        db.conn().execute("INSERT INTO kol_experts(name,institution,created_at,updated_at) VALUES('Expert','Clinic',1,1)", []).unwrap();
        db.conn().execute("INSERT INTO kol_notes(expert_id,work_id,content,occurred_at,created_at) VALUES(1,?1,'New evidence need',1,1)", [work]).unwrap();
        assert_eq!(status(&db, &scope).unwrap().state, "ready");
    }
    #[test]
    fn upgrade_preserves_unchanged_legacy_round_baseline() {
        let db = Database::open_in_memory().unwrap();
        let work = project(&db);
        let scope = format!("work:{work}");
        db.conn().execute("INSERT INTO kol_experts(name,institution,created_at,updated_at) VALUES('Expert','Clinic',1,1)", []).unwrap();
        db.conn().execute("INSERT INTO kol_notes(expert_id,work_id,content,occurred_at,created_at) VALUES(1,?1,'Existing note',1,1)", [work]).unwrap();
        let old = fingerprint_with_expert_feedback(db.conn(), &scope, false).unwrap();
        db.conn()
            .execute(
                "INSERT INTO secretary_rounds(scope,baseline) VALUES (?1,?2)",
                params![scope, old],
            )
            .unwrap();
        assert_eq!(status(&db, &scope).unwrap().state, "waiting_progress");
        let upgraded: String = db
            .conn()
            .query_row(
                "SELECT baseline FROM secretary_rounds WHERE scope=?1",
                [&scope],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(upgraded, fingerprint(db.conn(), &scope).unwrap());
        db.conn().execute("INSERT INTO kol_notes(expert_id,work_id,content,occurred_at,created_at) VALUES(1,?1,'New note',2,2)", [work]).unwrap();
        assert_eq!(status(&db, &scope).unwrap().state, "ready");
    }
    #[test]
    fn scoped_round_does_not_read_other_project_directions() {
        let db = Database::open_in_memory().unwrap();
        let allowed = project(&db);
        let held = project(&db);
        for (work, content) in [(allowed, "Allowed direction"), (held, "Held direction")] {
            crate::db::flow::capture(db.conn(), content, Some(work), Some("work"), Some(work))
                .unwrap();
        }
        let tickets = reserve(&db, run(&db), &[format!("work:{allowed}")]).unwrap();
        let snapshot = super::super::analysis_snapshot::build(
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
        let scoped = restrict(&db, snapshot, tickets).unwrap();
        let serialized = serde_json::to_string(&scoped).unwrap();
        assert!(serialized.contains("Allowed direction"));
        assert!(!serialized.contains("Held direction"));
        assert!(scoped.brief.user_directions.is_empty());
    }
    #[test]
    fn completed_adopted_task_closes_its_insight() {
        let db = Database::open_in_memory().unwrap();
        let w = project(&db);
        let p = opinion(&db, w, run(&db));
        let result = crate::ai::apply::confirm_proposal(&db, p.id, p.updated_at, None).unwrap();
        crate::db::task::TaskRepo::new(db.conn())
            .complete(result.target_id)
            .unwrap();
        let saved = crate::db::ai::ProposalRepo::new(db.conn())
            .get(p.id)
            .unwrap()
            .unwrap();
        assert_eq!(
            saved.status, "resolved",
            "Completing the adopted task must close its insight"
        );
    }
    #[test]
    fn assigning_external_secretary_content_releases_destination_project() {
        let db = Database::open_in_memory().unwrap();
        let origin = project(&db);
        let destination = project(&db);
        let scope = format!("work:{destination}");
        let first = run(&db);
        let tickets = reserve(&db, first, &[scope.clone()]).unwrap();
        remember(db.conn(), first, &tickets).unwrap();
        db.conn()
            .execute(
                "UPDATE analysis_runs SET status='completed' WHERE id=?1",
                [first],
            )
            .unwrap();
        let p = opinion(&db, origin, run(&db));
        let p = crate::db::ai::ProposalRepo::new(db.conn())
            .update_classification(
                p.id,
                p.updated_at,
                "task",
                Some(destination),
                &p.title,
                &p.payload_json,
            )
            .unwrap();
        crate::ai::apply::confirm_proposal(&db, p.id, p.updated_at, None).unwrap();
        assert_eq!(
            status(&db, &scope).unwrap().state,
            "ready",
            "New project material must be analysed once"
        );
    }
    #[test]
    fn regression_batch_association_is_order_independent() {
        for external_first in [true, false] {
            let db = Database::open_in_memory().unwrap();
            let origin = project(&db);
            let destination = project(&db);
            let external = opinion(&db, origin, run(&db));
            let own = opinion(&db, destination, run(&db));
            let external = crate::db::ai::ProposalRepo::new(db.conn())
                .update_classification(
                    external.id,
                    external.updated_at,
                    "task",
                    Some(destination),
                    "External material",
                    "{}",
                )
                .unwrap();
            let a = (external.id, external.updated_at);
            let b = (own.id, own.updated_at);
            crate::ai::receipts::confirm_batch(&db, &if external_first { [a, b] } else { [b, a] })
                .unwrap();
            assert_eq!(
                status(&db, &format!("work:{destination}")).unwrap().state,
                "ready"
            );
        }
    }
    #[test]
    fn explicit_completion_unlocks_once_and_retains_original_opinions() {
        let db = Database::open_in_memory().unwrap();
        let w = project(&db);
        let scope = format!("work:{w}");
        let p = opinion(&db, w, run(&db));
        let s = status(&db, &scope).unwrap();
        complete(&db, &scope, s.epoch).unwrap();
        assert!(complete(&db, &scope, s.epoch).is_err());
        assert_eq!(
            crate::db::ai::ProposalRepo::new(db.conn())
                .get(p.id)
                .unwrap()
                .unwrap()
                .payload_json,
            p.payload_json
        );
        let r = run(&db);
        let tickets = reserve(&db, r, &[scope.clone()]).unwrap();
        assert_eq!(tickets.len(), 1);
        remember(db.conn(), r, &tickets).unwrap();
        assert!(reserve(&db, run(&db), &[scope]).unwrap().is_empty());
    }
    #[test]
    fn parallel_project_and_inbox_runs_share_one_reservation() {
        let db = Database::open_in_memory().unwrap();
        let w = project(&db);
        let scope = format!("work:{w}");
        let a = run(&db);
        assert_eq!(reserve(&db, a, &[scope.clone()]).unwrap().len(), 1);
        assert!(reserve(&db, run(&db), &[scope.clone()]).unwrap().is_empty());
        super::super::analysis::fail_run(&db, a, "test", "test").unwrap();
        assert_eq!(reserve(&db, run(&db), &[scope]).unwrap().len(), 1);
    }
    #[test]
    fn late_response_rejected_after_progress_or_confirmation() {
        let db = Database::open_in_memory().unwrap();
        let w = project(&db);
        let scope = format!("work:{w}");
        let r = run(&db);
        let t = reserve(&db, r, &[scope]).unwrap();
        assert!(valid(db.conn(), r, &t).unwrap());
        crate::db::work::ResumePointRepo::new(db.conn())
            .insert(w, "Moved forward", "Next", "", "manual")
            .unwrap();
        assert!(!valid(db.conn(), r, &t).unwrap());
    }
    #[test]
    fn projects_independent_and_blocked_content_absent() {
        let db = Database::open_in_memory().unwrap();
        let a = project(&db);
        let b = project(&db);
        opinion(&db, a, run(&db));
        let r = run(&db);
        let t = reserve(&db, r, &candidates(db.conn(), None, None, None).unwrap()).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].scope, format!("work:{b}"));
        let snapshot = super::super::analysis_snapshot::build(
            &db,
            "global_analysis",
            "2026-09-17",
            0,
            i64::MAX,
            0,
            i64::MAX,
            "zh-CN",
        )
        .unwrap();
        let s = restrict(&db, snapshot, t).unwrap();
        assert!(s.brief.works.iter().all(|w| w.entity_id != Some(a)));
        assert!(s
            .source_refs
            .iter()
            .all(|w| w.source_type != "work" || w.entity_id != Some(a)));
    }
    #[test]
    fn timestamps_and_scans_do_not_release_waiting_round() {
        let db = Database::open_in_memory().unwrap();
        let w = project(&db);
        let scope = format!("work:{w}");
        let r = run(&db);
        let t = reserve(&db, r, &[scope.clone()]).unwrap();
        remember(db.conn(), r, &t).unwrap();
        db.conn()
            .execute("UPDATE works SET updated_at=999999999 WHERE id=?1", [w])
            .unwrap();
        assert_eq!(status(&db, &scope).unwrap().state, "waiting_progress");
    }
    #[test]
    fn stale_output_writes_neither_summary_nor_new_opinions() {
        let db = Database::open_in_memory().unwrap();
        let w = project(&db);
        let r = run(&db);
        let t = reserve(&db, r, &[format!("work:{w}")]).unwrap();
        let s = super::super::analysis_snapshot::build(
            &db,
            "global_analysis",
            "2026-09-17",
            0,
            1,
            0,
            1,
            "zh-CN",
        )
        .unwrap();
        let s = restrict(&db, s, t).unwrap();
        db.conn()
            .execute(
                "UPDATE works SET summary='New user progress' WHERE id=?1",
                [w],
            )
            .unwrap();
        let output=json!({"summary":"Old interpretation","proposals":[{"kind":"task","operation":"create","work_id":w,"title":"Old opinion","payload":{}}]}).to_string();
        assert_eq!(
            super::super::analysis::apply_output(&db, r, &s, &output).unwrap(),
            0
        );
        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM ai_proposals", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
    #[test]
    fn rejection_without_progress_does_not_trigger_another_opinion() {
        let db = Database::open_in_memory().unwrap();
        let w = project(&db);
        let p = opinion(&db, w, run(&db));
        let scope = format!("work:{w}");
        status(&db, &scope).unwrap();
        super::super::apply::reject_proposal(&db, p.id, p.updated_at, Some("duplicate")).unwrap();
        assert_eq!(status(&db, &scope).unwrap().state, "waiting_progress");
    }
}
