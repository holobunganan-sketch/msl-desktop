//! Tombstones make a previously captured model context unusable after deletion.
use super::{
    knowledge::{self, EvidencePack},
    DbError, DbResult,
};
use rusqlite::{params, Connection};
use serde_json::Value;

pub(crate) fn canonical_source_id(id: &str) -> String {
    let Some((kind, id)) = id.split_once(':') else {
        return id.to_string();
    };
    let kind = match kind {
        "task_open" | "task_completed" => "task",
        "resume_point" | "progress" => "resume",
        "weekly_report" => "report",
        "kol_expert" => "expert",
        other => other,
    };
    format!("{kind}:{id}")
}
pub(crate) fn is_retired(conn: &Connection, id: &str) -> DbResult<bool> {
    Ok(conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM knowledge_deleted_sources WHERE source_id=?1 OR source_id=?2)",
        params![id, canonical_source_id(id)],
        |r| r.get(0),
    )?)
}

pub fn ensure_live(conn: &Connection, pack: &EvidencePack) -> DbResult<()> {
    for source in &pack.sources {
        if source.trust == "deleted" || is_retired(conn, &source.id)? {
            return Err(DbError::Migration(
                "分析依据已被删除，请根据当前资料重新分析".into(),
            ));
        }
    }
    Ok(())
}

/// Caller owns the write transaction. Historical answers stay readable but their
/// deleted snapshots are stripped and pending/running work is invalidated.
pub fn retire(conn: &Connection, ids: &[String]) -> DbResult<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let ids = ids
        .iter()
        .map(|id| canonical_source_id(id))
        .collect::<Vec<_>>();
    for id in &ids {
        conn.execute(
            "INSERT OR IGNORE INTO knowledge_deleted_sources VALUES(?1,?2)",
            params![id, super::now_unix()],
        )?;
    }
    for table in ["qa_turns", "kol_drafts"] {
        for row in knowledge::rows(
            conn,
            &format!("SELECT id,evidence_json,status FROM {table} WHERE evidence_json IS NOT NULL"),
            &[],
        )? {
            let Ok(mut pack) =
                serde_json::from_str::<EvidencePack>(row["evidence_json"].as_str().unwrap_or(""))
            else {
                continue;
            };
            let mut changed = false;
            for source in &mut pack.sources {
                if ids.contains(&canonical_source_id(&source.id))
                    && (source.trust != "deleted" || !source.text.is_empty())
                {
                    source.trust = "deleted".into();
                    source.text.clear();
                    changed = true;
                }
            }
            if !changed {
                continue;
            }
            pack.notes
                .push("部分来源已删除，历史内容仅供回顾，不能继续作为分析依据。".into());
            conn.execute(
                &format!("UPDATE {table} SET evidence_json=?1 WHERE id=?2"),
                params![serde_json::to_string(&pack).unwrap(), row["id"].as_i64()],
            )?;
            if table == "qa_turns" {
                conn.execute("UPDATE qa_turns SET status='failed',error='分析来源已删除，请重新提问',finished_at=?1 WHERE id=?2 AND status IN ('pending','running')",params![super::now_unix(),row["id"].as_i64()])?;
            } else {
                conn.execute("UPDATE kol_drafts SET status='rejected',revision=revision+1,decided_at=?1 WHERE id=?2 AND status='pending'",params![super::now_unix(),row["id"].as_i64()])?;
            }
        }
    }
    let mut retired_reports = Vec::new();
    for row in knowledge::rows(
        conn,
        "SELECT id,evidence_json FROM reports WHERE evidence_json IS NOT NULL",
        &[],
    )? {
        let Ok(mut evidence) =
            serde_json::from_str::<Value>(row["evidence_json"].as_str().unwrap_or(""))
        else {
            continue;
        };
        let mut changed = false;
        if let Some(sources) = evidence["sources"].as_array_mut() {
            for source in sources {
                let key = canonical_source_id(&format!(
                    "{}:{}",
                    source["source_type"].as_str().unwrap_or(""),
                    source["entity_id"].as_i64().unwrap_or(0)
                ));
                if ids.contains(&key) && source["trust"] != "deleted" {
                    source["trust"] = Value::String("deleted".into());
                    source["location"]["available"] = Value::Bool(false);
                    changed = true;
                }
            }
        }
        if changed {
            retired_reports.push(format!("report:{}", row["id"].as_i64().unwrap_or(0)));
            conn.execute(
                "UPDATE reports SET evidence_json=?1 WHERE id=?2",
                params![evidence.to_string(), row["id"].as_i64()],
            )?;
        }
    }
    if !retired_reports.is_empty() {
        retire(conn, &retired_reports)?;
    }
    Ok(())
}

pub fn usable_insight(conn: &Connection, row: &Value) -> bool {
    let Ok(citations) =
        serde_json::from_str::<Vec<Value>>(row["citations_json"].as_str().unwrap_or("[]"))
    else {
        return false;
    };
    citations.iter().all(|c| {
        conn.query_row(
            "SELECT NOT EXISTS(SELECT 1 FROM knowledge_deleted_sources WHERE source_id=?1)",
            [c["source_id"].as_str().unwrap_or("")],
            |r| r.get::<_, bool>(0),
        )
        .unwrap_or(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn project_deleted_resume_retires_report_and_dependent_monthly_sources() {
        let db = super::super::Database::open_in_memory().unwrap();
        let w = super::super::work::WorkRepo::new(db.conn())
            .insert("Synthetic", "active")
            .unwrap();
        db.conn().execute("INSERT INTO resume_points(work_id,current_state,next_step,remember,source,created_at) VALUES(?1,'Progress','','','manual',1)",[w.id]).unwrap();
        let id = db.conn().last_insert_rowid();
        for (report_id, kind, source_type, source_id) in [
            (1, "weekly", "resume_point", id),
            (2, "monthly", "weekly_report", 1),
        ] {
            let evidence =
                serde_json::json!({"sources":[{"source_type":source_type,"entity_id":source_id}]})
                    .to_string();
            db.conn().execute("INSERT INTO reports(id,kind,period_start,period_end,status,content,retention_state,created_at,updated_at,evidence_json) VALUES(?1,?2,1,2,'completed','Historical','kept',1,1,?3)",params![report_id,kind,evidence]).unwrap();
        }
        super::super::work::WorkRepo::new(db.conn())
            .delete(w.id)
            .unwrap();
        assert_eq!(
            super::super::reports::ReportRepo::new(db.conn())
                .get(1)
                .unwrap()
                .unwrap()
                .content
                .as_deref(),
            Some("Historical")
        );
        assert!(!knowledge::collect(&db, &[], "")
            .unwrap()
            .sources
            .iter()
            .any(|s| s.kind == "report"));
        assert!(super::super::reports::ReportRepo::new(db.conn())
            .list_overlapping_weekly(0, 3, 10)
            .unwrap()
            .is_empty());
        assert!(is_retired(db.conn(), "report:2").unwrap());
    }
    #[test]
    fn deleted_report_evidence_remains_readable_but_not_reusable_as_current_evidence() {
        let db = super::super::Database::open_in_memory().unwrap();
        let evidence =
            serde_json::json!({"sources":[{"source_type":"task_completed","entity_id":42}]})
                .to_string();
        db.conn().execute("INSERT INTO reports(kind,period_start,period_end,status,content,retention_state,created_at,updated_at,evidence_json) VALUES('weekly',1,2,'completed','Historical','kept',1,1,?1)",[evidence]).unwrap();
        retire(db.conn(), &["task:42".into()]).unwrap();
        let row = super::super::reports::ReportRepo::new(db.conn())
            .get(1)
            .unwrap()
            .unwrap();
        assert_eq!(row.content.as_deref(), Some("Historical"));
        let evidence: Value = serde_json::from_str(&row.evidence_json.unwrap()).unwrap();
        assert_eq!(evidence["sources"][0]["trust"], "deleted");
        assert!(!knowledge::collect(&db, &[], "")
            .unwrap()
            .sources
            .iter()
            .any(|s| s.kind == "report"));
    }
    #[test]
    fn deleted_evidence_preserves_history_but_blocks_followup_and_late_answers() {
        let db = super::super::Database::open_in_memory().unwrap();
        let source = knowledge::evidence(
            "kol_material",
            &serde_json::json!({"id":42,"title":"synthetic","text":"test evidence"}),
            "model_reading",
            "",
        );
        let pack = EvidencePack {
            sources: vec![source],
            ..Default::default()
        };
        let session = super::super::qa::create(&db, "synthetic", &[]).unwrap()["id"]
            .as_i64()
            .unwrap();
        let first = super::super::qa::queue(&db, session, "first", &[]).unwrap()["id"]
            .as_i64()
            .unwrap();
        super::super::qa::claim(&db, first).unwrap();
        super::super::qa::save_evidence(&db, first, &pack).unwrap();
        let answer = crate::ai::knowledge_contract::Answer {
            document: None,
            claims: vec![],
            gaps: vec!["synthetic".into()],
        };
        super::super::qa::finish(&db, first, Ok(&answer)).unwrap();
        let late = super::super::qa::queue(&db, session, "second", &[]).unwrap()["id"]
            .as_i64()
            .unwrap();
        super::super::qa::claim(&db, late).unwrap();
        super::super::qa::save_evidence(&db, late, &pack).unwrap();
        let tx = super::super::write_transaction(db.conn()).unwrap();
        retire(&tx, &["kol_material:42".into()]).unwrap();
        tx.commit().unwrap();
        super::super::qa::finish(&db, late, Ok(&answer)).unwrap();
        assert_eq!(
            super::super::qa::get(&db, late).unwrap()["status"],
            "failed"
        );
        assert_eq!(
            super::super::qa::get(&db, first).unwrap()["status"],
            "completed"
        );
        let historic: EvidencePack = serde_json::from_str(
            super::super::qa::get(&db, first).unwrap()["evidence_json"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(historic.sources[0].trust, "deleted");
        assert!(historic.sources[0].text.is_empty());
        assert!(
            super::super::qa::history(&db, &super::super::qa::get(&db, late).unwrap())
                .unwrap()
                .is_empty()
        );
        assert!(ensure_live(db.conn(), &pack).is_err());
    }
}
