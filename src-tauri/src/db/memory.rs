//! Local, explainable classification memory built from proposal-review decisions.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::ai::AiProposal;
use super::{now_unix, DbResult};

pub const MAX_MEMORY_EXAMPLES: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassificationFeedback {
    Accepted,
    Corrected,
    Rejected,
}

impl ClassificationFeedback {
    fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Corrected => "corrected",
            Self::Rejected => "rejected",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClassificationMemoryExample {
    pub cue: String,
    pub suggested_kind: String,
    pub preferred_kind: Option<String>,
    pub suggested_scope: String,
    pub preferred_scope: Option<String>,
    pub source_types: Vec<String>,
    pub payload_keys: Vec<String>,
    pub accepted: i64,
    pub corrected: i64,
    pub rejected: i64,
    pub last_feedback: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClassificationMemoryContext {
    pub strategy: String,
    pub feedback_count: i64,
    pub examples: Vec<ClassificationMemoryExample>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClassificationMemoryStats {
    pub pattern_count: i64,
    pub feedback_count: i64,
    pub accepted_count: i64,
    pub corrected_count: i64,
    pub rejected_count: i64,
    pub updated_at: Option<i64>,
}

fn bounded_text(value: &str, limit: usize) -> String {
    value.trim().chars().take(limit).collect()
}

fn normalized_cue(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .take(160)
        .collect()
}

fn source_types(source_refs_json: &str) -> Vec<String> {
    let mut values = serde_json::from_str::<serde_json::Value>(source_refs_json)
        .ok()
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|value| {
            value
                .get("source_type")
                .and_then(serde_json::Value::as_str)
                .map(|value| bounded_text(value, 40))
        })
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values.truncate(12);
    values
}

fn payload_keys(payload: &serde_json::Value) -> Vec<String> {
    let mut values: Vec<String> = payload
        .as_object()
        .map(|object| object.keys().map(|key| bounded_text(key, 40)).collect())
        .unwrap_or_default();
    values.sort();
    values.dedup();
    values.truncate(20);
    values
}

fn fingerprint(proposal: &AiProposal, sources: &[String]) -> String {
    let input = format!(
        "{}|{}|{}",
        normalized_cue(&proposal.title),
        proposal.suggested_kind,
        sources.join(",")
    );
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn record_feedback(
    conn: &Connection,
    proposal: &AiProposal,
    final_kind: Option<&str>,
    final_work_id: Option<i64>,
    payload: &serde_json::Value,
    feedback: ClassificationFeedback,
) -> DbResult<()> {
    let sources = source_types(&proposal.source_refs_json);
    let keys = payload_keys(payload);
    let fingerprint = fingerprint(proposal, &sources);
    let now = now_unix();
    let (positive, negative, corrected) = match feedback {
        ClassificationFeedback::Accepted => (1, 0, 0),
        ClassificationFeedback::Corrected => (0, 0, 1),
        ClassificationFeedback::Rejected => (0, 1, 0),
    };
    conn.execute(
        "INSERT INTO classification_memories (
           fingerprint,cue_text,suggested_kind,preferred_kind,suggested_work_id,
           preferred_work_id,source_types_json,payload_keys_json,positive_count,
           negative_count,correction_count,last_feedback,last_proposal_id,created_at,updated_at
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?14)
         ON CONFLICT(fingerprint) DO UPDATE SET
           cue_text=excluded.cue_text,
           preferred_kind=COALESCE(excluded.preferred_kind,classification_memories.preferred_kind),
           preferred_work_id=CASE
             WHEN excluded.preferred_kind IS NULL THEN classification_memories.preferred_work_id
             ELSE excluded.preferred_work_id
           END,
           source_types_json=excluded.source_types_json,
           payload_keys_json=excluded.payload_keys_json,
           positive_count=classification_memories.positive_count+excluded.positive_count,
           negative_count=classification_memories.negative_count+excluded.negative_count,
           correction_count=classification_memories.correction_count+excluded.correction_count,
           last_feedback=excluded.last_feedback,
           last_proposal_id=excluded.last_proposal_id,
           updated_at=excluded.updated_at",
        params![
            fingerprint,
            bounded_text(&proposal.title, 200),
            proposal.suggested_kind,
            final_kind,
            proposal.suggested_work_id,
            final_work_id,
            serde_json::to_string(&sources).unwrap_or_else(|_| "[]".into()),
            serde_json::to_string(&keys).unwrap_or_else(|_| "[]".into()),
            positive,
            negative,
            corrected,
            feedback.as_str(),
            proposal.id,
            now,
        ],
    )?;
    Ok(())
}

pub fn stats(conn: &Connection) -> DbResult<ClassificationMemoryStats> {
    conn.query_row(
        "SELECT COUNT(*),
                COALESCE(SUM(positive_count+negative_count+correction_count),0),
                COALESCE(SUM(positive_count),0),
                COALESCE(SUM(correction_count),0),
                COALESCE(SUM(negative_count),0),
                MAX(updated_at)
         FROM classification_memories",
        [],
        |row| {
            Ok(ClassificationMemoryStats {
                pattern_count: row.get(0)?,
                feedback_count: row.get(1)?,
                accepted_count: row.get(2)?,
                corrected_count: row.get(3)?,
                rejected_count: row.get(4)?,
                updated_at: row.get(5)?,
            })
        },
    )
    .map_err(Into::into)
}

pub fn context(conn: &Connection, limit: usize) -> DbResult<ClassificationMemoryContext> {
    let memory_stats = stats(conn)?;
    let mut stmt = conn.prepare(
        "SELECT cue_text,suggested_kind,preferred_kind,suggested_work_id,preferred_work_id,
                source_types_json,payload_keys_json,positive_count,correction_count,
                negative_count,last_feedback
         FROM classification_memories
         ORDER BY ((positive_count+correction_count)*3+negative_count*2) DESC,updated_at DESC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map([limit.clamp(1, MAX_MEMORY_EXAMPLES) as i64], |row| {
        let suggested_work_id: Option<i64> = row.get(3)?;
        let preferred_work_id: Option<i64> = row.get(4)?;
        let source_types_json: String = row.get(5)?;
        let payload_keys_json: String = row.get(6)?;
        Ok(ClassificationMemoryExample {
            cue: row.get(0)?,
            suggested_kind: row.get(1)?,
            preferred_kind: row.get(2)?,
            suggested_scope: suggested_work_id
                .map(|id| format!("work:{id}"))
                .unwrap_or_else(|| "temporary".into()),
            preferred_scope: preferred_work_id.map(|id| format!("work:{id}")),
            source_types: serde_json::from_str(&source_types_json).unwrap_or_default(),
            payload_keys: serde_json::from_str(&payload_keys_json).unwrap_or_default(),
            accepted: row.get(7)?,
            corrected: row.get(8)?,
            rejected: row.get(9)?,
            last_feedback: row.get(10)?,
        })
    })?;
    Ok(ClassificationMemoryContext {
        strategy: "local_feedback_memory_v1".into(),
        feedback_count: memory_stats.feedback_count,
        examples: rows.collect::<rusqlite::Result<Vec<_>>>()?,
    })
}

pub fn last_feedback_for_proposal(conn: &Connection, proposal_id: i64) -> DbResult<Option<String>> {
    conn.query_row(
        "SELECT last_feedback FROM classification_memories WHERE last_proposal_id=?1",
        [proposal_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(Into::into)
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryCard {
    pub id: i64,
    pub cue: String,
    pub suggested_kind: String,
    pub preferred_kind: Option<String>,
    pub preferred_work_id: Option<i64>,
    pub accepted: i64,
    pub corrected: i64,
    pub rejected: i64,
    pub updated_at: i64,
}
pub fn list_cards(conn: &Connection) -> DbResult<Vec<MemoryCard>> {
    let mut stmt=conn.prepare("SELECT id,cue_text,suggested_kind,preferred_kind,preferred_work_id,positive_count,correction_count,negative_count,updated_at FROM classification_memories ORDER BY updated_at DESC,id DESC LIMIT 200")?;
    let rows = stmt.query_map([], |r| {
        Ok(MemoryCard {
            id: r.get(0)?,
            cue: r.get(1)?,
            suggested_kind: r.get(2)?,
            preferred_kind: r.get(3)?,
            preferred_work_id: r.get(4)?,
            accepted: r.get(5)?,
            corrected: r.get(6)?,
            rejected: r.get(7)?,
            updated_at: r.get(8)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}
pub fn edit_card(
    conn: &Connection,
    id: i64,
    expected: i64,
    kind: Option<&str>,
    work_id: Option<i64>,
) -> DbResult<()> {
    if kind.is_some_and(|kind| !crate::ai::schema::PROPOSAL_KINDS.contains(&kind)) {
        return Err(super::DbError::Migration("记忆类别无效".into()));
    }
    let changed = if let Some(kind) = kind {
        conn.execute("UPDATE classification_memories SET preferred_kind=?1,preferred_work_id=?2,correction_count=correction_count+1,negative_count=0,last_feedback='corrected',updated_at=MAX(updated_at+1,?3) WHERE id=?4 AND updated_at=?5",params![kind,work_id,now_unix(),id,expected])?
    } else {
        conn.execute(
            "DELETE FROM classification_memories WHERE id=?1 AND updated_at=?2",
            params![id, expected],
        )?
    };
    if changed != 1 {
        return Err(super::DbError::Migration(
            "记忆已发生变化，请刷新后操作".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ai::{AnalysisRunRepo, ProposalRepo};
    use crate::db::Database;

    #[test]
    fn repeated_feedback_is_consolidated_and_context_is_bounded() {
        let db = Database::open_in_memory().unwrap();
        let run = AnalysisRunRepo::new(db.conn())
            .create("manual", None, None)
            .unwrap();
        let proposal = ProposalRepo::new(db.conn())
            .upsert_pending(
                run.id,
                "task",
                "create",
                None,
                None,
                None,
                "memory-unit",
                "等待回复",
                r#"{"waiting_for":"医学部"}"#,
                "需要外部反馈",
                r#"[{"source_type":"inbox","entity_id":1}]"#,
                Some(0.8),
            )
            .unwrap()
            .unwrap();
        let payload = serde_json::json!({"waiting_for":"医学部"});
        record_feedback(
            db.conn(),
            &proposal,
            Some("waiting"),
            None,
            &payload,
            ClassificationFeedback::Corrected,
        )
        .unwrap();
        record_feedback(
            db.conn(),
            &proposal,
            Some("waiting"),
            None,
            &payload,
            ClassificationFeedback::Corrected,
        )
        .unwrap();

        let memory_stats = stats(db.conn()).unwrap();
        assert_eq!(memory_stats.pattern_count, 1);
        assert_eq!(memory_stats.feedback_count, 2);
        assert_eq!(memory_stats.corrected_count, 2);
        let memory = context(db.conn(), 24).unwrap();
        assert_eq!(memory.examples.len(), 1);
        assert_eq!(
            memory.examples[0].preferred_kind.as_deref(),
            Some("waiting")
        );
        assert_eq!(memory.examples[0].source_types, vec!["inbox"]);
    }
}
