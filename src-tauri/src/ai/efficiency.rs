//! Request compaction and conservative reuse. No model text or credentials stored.
use crate::ai::analysis_snapshot::AnalysisSnapshot;
use serde_json::Value;

pub const INPUT_CONTRACT: &str = "Input uses compact JSON. An omitted optional field means null/unknown, including work_id=null for independent items. All non-null values are retained. This rule applies to INPUT facts only; preserve explicit null in OUTPUT patches that intentionally clear a field.";
pub fn compact_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, v)| (k.clone(), compact_json(v)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(compact_json).collect()),
        _ => value.clone(),
    }
}
pub fn input_json<T: serde::Serialize>(value: &T) -> String {
    compact_json(&serde_json::to_value(value).expect("serializable AI evidence")).to_string()
}
/// Never interpret a bounded/partial input as proof that everything is unchanged.
pub fn complete_coverage(snapshot: &AnalysisSnapshot) -> bool {
    snapshot.truncated.is_empty()
        && snapshot.brief.truncated.is_empty()
        && !snapshot
            .project_cognition
            .iter()
            .any(|entry| entry["entry_truncated"].as_bool().unwrap_or(false))
}
pub fn reuse_fingerprint(snapshot: &AnalysisSnapshot, route: &Value, now: i64) -> String {
    let mut value = serde_json::to_value(snapshot).expect("snapshot serialization");
    for key in ["snapshot_hash", "period_start", "period_end"] {
        value.as_object_mut().unwrap().remove(key);
    }
    for key in ["period_start", "period_end"] {
        value["brief"].as_object_mut().unwrap().remove(key);
    }
    fn phases(value: &Value, now: i64, out: &mut Vec<Value>) {
        match value {
            Value::Object(map) => {
                for (key, v) in map {
                    if matches!(
                        key.as_str(),
                        "due_at"
                            | "follow_up_at"
                            | "start_at"
                            | "end_at"
                            | "scheduled_start"
                            | "scheduled_end"
                    ) {
                        if let Some(at) = v.as_i64().filter(|at| *at > 0) {
                            out.push(serde_json::json!([
                                key,
                                at,
                                now >= at,
                                now >= at.saturating_sub(3600)
                            ]));
                        }
                    }
                    phases(v, now, out);
                }
            }
            Value::Array(items) => {
                for v in items {
                    phases(v, now, out);
                }
            }
            _ => (),
        }
    }
    let mut time_phases = Vec::new();
    phases(&value, now, &mut time_phases);
    crate::cognition::digest(&serde_json::json!({"evidence":value,"route":route,"time_phases":time_phases,"prompt":crate::ai::prompts::PROMPT_VERSION,"spec":include_str!("secretary-spec.md"),"input_contract":INPUT_CONTRACT}).to_string())
}

#[derive(Debug, serde::Serialize)]
pub struct EfficiencyStats {
    pub reused_checks: i64,
    pub input_chars_saved: i64,
    pub last_checked_at: Option<i64>,
}
pub fn stats(db: &crate::db::Database) -> crate::db::DbResult<EfficiencyStats> {
    db.conn().query_row("SELECT reused_checks,input_chars_saved,last_checked_at FROM ai_efficiency_state WHERE id=1",[],|r|Ok(EfficiencyStats{reused_checks:r.get(0)?,input_chars_saved:r.get(1)?,last_checked_at:r.get(2)?})).map_err(Into::into)
}
pub fn reusable_run(
    db: &crate::db::Database,
    fingerprint: &str,
    trigger: &str,
) -> crate::db::DbResult<Option<i64>> {
    use rusqlite::OptionalExtension;
    if !matches!(trigger, "interval" | "daily") {
        return Ok(None);
    }
    db.conn().query_row("SELECT s.run_id FROM ai_efficiency_state s JOIN analysis_runs r ON r.id=s.run_id WHERE s.id=1 AND s.fingerprint=?1 AND r.status='completed'",[fingerprint],|r|r.get(0)).optional().map_err(Into::into)
}
pub fn remember_success(
    db: &crate::db::Database,
    fingerprint: &str,
    run_id: i64,
) -> crate::db::DbResult<()> {
    db.conn().execute("UPDATE ai_efficiency_state SET fingerprint=?1,run_id=?2 WHERE id=1 AND EXISTS(SELECT 1 FROM analysis_runs WHERE id=?2 AND status='completed')",rusqlite::params![fingerprint,run_id])?;
    Ok(())
}
pub fn record_check(
    db: &crate::db::Database,
    reused: bool,
    saved: usize,
) -> crate::db::DbResult<()> {
    db.conn().execute("UPDATE ai_efficiency_state SET reused_checks=reused_checks+?1,input_chars_saved=input_chars_saved+?2,last_checked_at=?3 WHERE id=1",rusqlite::params![i64::from(reused),saved as i64,crate::db::now_unix()])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn incomplete_input_never_allows_a_skip() {
        let db = Database::open_in_memory().unwrap();
        let mut s = crate::ai::analysis_snapshot::build(
            &db,
            "global_analysis",
            "2026-09-06",
            0,
            100,
            0,
            86400,
            "zh-CN",
        )
        .unwrap();
        assert!(complete_coverage(&s));
        s.truncated.insert("tasks_open_global".into(), 1);
        assert!(!complete_coverage(&s));
        s.truncated.clear();
        s.project_cognition[0]["entry_truncated"] = serde_json::json!(true);
        assert!(!complete_coverage(&s));
    }
    #[test]
    fn compaction_preserves_meaning_and_all_text_while_omitting_optional_null_fields() {
        let value = serde_json::json!({"unknown":null,"zero":0,"off":false,"text":"所有记录都保留","empty":"","rows":[null,{"work_id":null,"title":"原话","due_at":200} ]});
        assert_eq!(
            compact_json(&value),
            serde_json::json!({"zero":0,"off":false,"text":"所有记录都保留","empty":"","rows":[null,{"title":"原话","due_at":200}]})
        );
    }
    #[test]
    fn reuse_ignores_clock_drift_but_not_day_deadline_or_fact_changes() {
        let db = Database::open_in_memory().unwrap();
        crate::db::task::TaskRepo::new(db.conn())
            .insert(None, "follow up", "normal", Some(1500), None)
            .unwrap();
        let a = crate::ai::analysis_snapshot::build(
            &db,
            "global_analysis",
            "2026-09-06",
            0,
            1000,
            0,
            86400,
            "zh-CN",
        )
        .unwrap();
        let route = serde_json::json!({"model_id":1,"prompt":"v1"});
        let mut b = a.clone();
        b.period_end = 1200;
        b.brief.period_end = 1200;
        b.snapshot_hash = "different clock hash".into();
        assert_eq!(
            reuse_fingerprint(&a, &route, 1000),
            reuse_fingerprint(&b, &route, 1200)
        );
        assert_ne!(
            reuse_fingerprint(&a, &route, 1000),
            reuse_fingerprint(&a, &route, 1500)
        );
        b = a.clone();
        b.brief.tasks_open[0].status = Some("done".into());
        assert_ne!(
            reuse_fingerprint(&a, &route, 1000),
            reuse_fingerprint(&b, &route, 1000)
        );
        b = a.clone();
        b.brief.date = "2026-09-07".into();
        assert_ne!(
            reuse_fingerprint(&a, &route, 1000),
            reuse_fingerprint(&b, &route, 1000)
        );
        assert_ne!(
            reuse_fingerprint(&a, &route, 1000),
            reuse_fingerprint(&a, &serde_json::json!({"model_id":2,"prompt":"v1"}), 1000)
        );
    }
    #[test]
    fn only_successful_matching_periodic_checks_reuse_and_keep_original_proposals() {
        let db = Database::open_in_memory().unwrap();
        let run = crate::ai::analysis::create_run(&db, "manual", 0, 1).unwrap();
        remember_success(&db, "facts-a", run).unwrap();
        assert_eq!(reusable_run(&db, "facts-a", "interval").unwrap(), None);
        crate::ai::analysis::finish_run(&db, run, "completed", Some("summary"), None).unwrap();
        remember_success(&db, "facts-a", run).unwrap();
        assert_eq!(reusable_run(&db, "facts-a", "interval").unwrap(), Some(run));
        assert_eq!(reusable_run(&db, "facts-a", "daily").unwrap(), Some(run));
        for trigger in ["manual", "retry", "inbox"] {
            assert_eq!(reusable_run(&db, "facts-a", trigger).unwrap(), None);
        }
        assert_eq!(reusable_run(&db, "facts-b", "interval").unwrap(), None);
        record_check(&db, true, 0).unwrap();
        record_check(&db, false, 220).unwrap();
        assert_eq!(stats(&db).unwrap().reused_checks, 1);
        assert_eq!(stats(&db).unwrap().input_chars_saved, 220);
        assert_eq!(
            db.conn()
                .query_row("SELECT COUNT(*) FROM ai_efficiency_state", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        crate::ai::analysis::fail_run(&db, run, "interrupted", "test").unwrap();
        assert_eq!(reusable_run(&db, "facts-a", "interval").unwrap(), None);
    }
    #[test]
    fn efficiency_state_is_a_migrated_fixed_size_local_record() {
        let db = Database::open_in_memory().unwrap();
        let count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name='ai_efficiency_state'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}
