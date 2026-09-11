//! Tombstones make a previously captured model context unusable after deletion.
use super::{
    knowledge::{self, EvidencePack},
    DbError, DbResult,
};
use rusqlite::{params, Connection};
use serde_json::Value;

pub fn ensure_live(conn: &Connection, pack: &EvidencePack) -> DbResult<()> {
    for source in &pack.sources {
        if source.trust == "deleted"
            || conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM knowledge_deleted_sources WHERE source_id=?1)",
                [&source.id],
                |r| r.get::<_, bool>(0),
            )?
        {
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
    for id in ids {
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
                if ids.contains(&source.id)
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
