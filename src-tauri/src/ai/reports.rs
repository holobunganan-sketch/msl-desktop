//! Model-generated weekly and monthly report snapshots and output contracts.

use chrono::{Datelike, Local, TimeZone};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::ai::analysis_snapshot::AnalysisSnapshot;
use crate::ai::provider::{AiMessage, AiTextRequest};
use crate::db::{Database, DbError, DbResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyReportEvidence {
    pub report_id: i64,
    pub period_start: i64,
    pub period_end: i64,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSnapshot {
    pub kind: String,
    pub period_start: i64,
    pub period_end: i64,
    pub analysis: AnalysisSnapshot,
    pub weekly_reports: Vec<WeeklyReportEvidence>,
    #[serde(default)]
    pub period_changes: Vec<serde_json::Value>,
    #[serde(default)]
    pub project_catalog: Vec<serde_json::Value>,
    pub source_counts: serde_json::Value,
    pub snapshot_hash: String,
}

pub fn default_weekly_period(now: i64) -> (i64, i64) {
    (now.saturating_sub(7 * 86400), now)
}

pub fn default_monthly_period(now: i64) -> Result<(i64, i64), String> {
    let local = Local
        .timestamp_opt(now, 0)
        .single()
        .ok_or_else(|| "报告时间无效".to_string())?;
    let this_month = Local
        .with_ymd_and_hms(local.year(), local.month(), 1, 0, 0, 0)
        .single()
        .ok_or_else(|| "无法计算本月起点".to_string())?;
    let (year, month) = if local.month() == 1 {
        (local.year() - 1, 12)
    } else {
        (local.year(), local.month() - 1)
    };
    let previous_month = Local
        .with_ymd_and_hms(year, month, 1, 0, 0, 0)
        .single()
        .ok_or_else(|| "无法计算上月起点".to_string())?;
    Ok((previous_month.timestamp(), this_month.timestamp()))
}

pub fn build_report_snapshot(
    db: &Database,
    kind: &str,
    period_start: i64,
    period_end: i64,
    locale: &str,
) -> DbResult<ReportSnapshot> {
    if !["weekly", "monthly"].contains(&kind) || period_end <= period_start {
        return Err(DbError::Migration("invalid report period or kind".into()));
    }
    let end = Local
        .timestamp_opt(period_end, 0)
        .single()
        .ok_or_else(|| DbError::Migration("invalid report end time".into()))?;
    let day_start = Local
        .with_ymd_and_hms(end.year(), end.month(), end.day(), 0, 0, 0)
        .single()
        .ok_or_else(|| DbError::Migration("invalid local report date".into()))?
        .timestamp();
    let task_kind = if kind == "weekly" {
        "weekly_report"
    } else {
        "monthly_report"
    };
    let mut analysis = crate::ai::analysis_snapshot::build(
        db,
        task_kind,
        &end.format("%Y-%m-%d").to_string(),
        period_start,
        period_end,
        day_start,
        day_start.saturating_add(86400),
        locale,
    )?;
    let project_catalog = crate::db::work::WorkRepo::new(db.conn())
        .list(None)?
        .into_iter()
        .map(|work| serde_json::json!({"id":work.id,"title":work.title,"status":work.status}))
        .collect::<Vec<_>>();
    let weekly_reports = if kind == "monthly" {
        crate::db::reports::ReportRepo::new(db.conn())
            .list_overlapping_weekly(period_start, period_end, 20)?
            .into_iter()
            .filter_map(|report| {
                report.content.map(|content| WeeklyReportEvidence {
                    report_id: report.id,
                    period_start: report.period_start,
                    period_end: report.period_end,
                    content: content.chars().take(20_000).collect(),
                })
            })
            .collect()
    } else {
        Vec::new()
    };
    // Preserve business history separately from current-state snapshots. Source files stay read-only.
    let reversed=crate::db::knowledge::rows(db.conn(),"SELECT entity_id FROM activity_events WHERE event_type IN ('task.completion_undone','waiting.completion_undone') AND entity_type='activity'",&[])?.into_iter().filter_map(|v|v["entity_id"].as_i64()).collect::<std::collections::BTreeSet<_>>();
    analysis
        .brief
        .activity
        .retain(|f| !f.entity_id.is_some_and(|id| reversed.contains(&id)));
    analysis.brief.source_counts.activity = analysis.brief.activity.len() as u32;
    analysis.source_refs.retain(|r| {
        r.source_type != "activity" || !r.entity_id.is_some_and(|id| reversed.contains(&id))
    });
    let mut stmt=db.conn().prepare("SELECT id,timestamp,event_type,work_id,entity_type,entity_id,display_text,metadata_json FROM activity_events e WHERE timestamp>=?1 AND timestamp<?2 AND NOT EXISTS(SELECT 1 FROM activity_events undo WHERE undo.event_type IN ('task.completion_undone','waiting.completion_undone') AND undo.entity_type='activity' AND undo.entity_id=e.id) AND (entity_type IN ('work','task','waiting','calendar','inbox','resume_point') OR event_type LIKE 'work.%' OR event_type LIKE 'task.%' OR event_type LIKE 'waiting.%' OR event_type LIKE 'calendar.%' OR event_type LIKE 'inbox.%' OR event_type LIKE 'resume%') ORDER BY timestamp DESC,id DESC LIMIT 1001")?;
    let rows=stmt.query_map(rusqlite::params![period_start,period_end],|r|Ok(serde_json::json!({"source_type":"activity","entity_id":r.get::<_,i64>(0)?,"timestamp":r.get::<_,i64>(1)?,"event_type":r.get::<_,String>(2)?,"work_id":r.get::<_,Option<i64>>(3)?,"target_type":r.get::<_,Option<String>>(4)?,"target_id":r.get::<_,Option<i64>>(5)?,"description":r.get::<_,String>(6)?,"change_details":r.get::<_,Option<String>>(7)?})))?;
    let mut period_changes = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    let omitted = period_changes.len().saturating_sub(1000);
    period_changes.truncate(1000);
    let mut stmt=db.conn().prepare("SELECT id,work_id,current_state,next_step,remember,created_at FROM resume_points WHERE created_at>=?1 AND created_at<?2 ORDER BY created_at DESC,id DESC LIMIT 501")?;
    let rows=stmt.query_map(rusqlite::params![period_start,period_end],|r|Ok(serde_json::json!({"source_type":"resume_point","entity_id":r.get::<_,i64>(0)?,"work_id":r.get::<_,i64>(1)?,"current_state":r.get::<_,String>(2)?,"next_step":r.get::<_,String>(3)?,"remember":r.get::<_,String>(4)?,"timestamp":r.get::<_,i64>(5)?})))?;
    let mut progress = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    let omitted = omitted + progress.len().saturating_sub(500);
    progress.truncate(500);
    period_changes.extend(progress);
    for row in &period_changes {
        analysis
            .source_refs
            .push(crate::ai::analysis_snapshot::AnalysisSourceRef {
                source_type: row["source_type"].as_str().unwrap().into(),
                entity_id: row["entity_id"].as_i64(),
                workspace_id: None,
                relative_path: None,
                content_hash: None,
                timestamp: row["timestamp"].as_i64(),
            });
    }
    let source_counts = serde_json::json!({
        "report_contract":"report-spec-v2",
        "period_changes":period_changes.len(),
        "period_changes_omitted":omitted,
        "analysis": analysis.source_counts,
        "weekly_reports": weekly_reports.len(),
    });
    let hash_input = serde_json::json!({
        "kind": kind,
        "period_start": period_start,
        "period_end": period_end,
        "analysis_hash": analysis.snapshot_hash,
        "weekly_reports": weekly_reports,
        "period_changes":period_changes,
        "project_catalog":project_catalog,
    });
    let mut hasher = Sha256::new();
    hasher.update(serde_json::to_vec(&hash_input).unwrap_or_default());
    Ok(ReportSnapshot {
        kind: kind.into(),
        period_start,
        period_end,
        analysis,
        weekly_reports,
        period_changes,
        project_catalog,
        source_counts,
        snapshot_hash: format!("{:x}", hasher.finalize()),
    })
}

pub fn build_report_system_prompt(kind: &str, locale: &str) -> Result<String, String> {
    let language = if locale == "en-US" {
        "English"
    } else {
        "Simplified Chinese"
    };
    if !["weekly", "monthly"].contains(&kind) {
        return Err("unknown report kind".into());
    }
    Ok(format!("Report kind: {kind}. Language: {language}. Treat each long-term Work as a project; preserve cross-project and historical change analysis. Use weekly reports for monthly synthesis. The application shows a numbered list. {}\n{}",crate::ai::efficiency::INPUT_CONTRACT,include_str!("report-spec.md")))
}

pub fn build_report_request(
    model_id: &str,
    snapshot: &ReportSnapshot,
) -> Result<AiTextRequest, String> {
    Ok(AiTextRequest {
        model_id: model_id.into(),
        system: Some(build_report_system_prompt(
            &snapshot.kind,
            &snapshot.analysis.locale,
        )?),
        messages: vec![AiMessage {
            role: "user".into(),
            content: crate::ai::efficiency::input_json(snapshot),
        }],
        temperature: Some(0.2),
        max_output_tokens: Some(if snapshot.kind == "monthly" {
            8000
        } else {
            4000
        }),
        output_format: crate::ai::output::OutputFormat::PromptJson,
        budget: Default::default(),
    })
}

pub fn normalize_weekly_numbering(content: &str) -> String {
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(30)
        .enumerate()
        .map(|(index, line)| {
            let stripped = line
                .trim_start_matches(|ch: char| {
                    ch.is_ascii_digit()
                        || ch.is_whitespace()
                        || matches!(ch, '.' | ')' | '、' | '-' | '•' | '*')
                })
                .trim();
            format!("{}. {}", index + 1, stripped)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn normalize_report_output(kind: &str, content: &str) -> Result<String, String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err("AI report response is empty".into());
    }
    let bounded = trimmed.chars().take(50_000).collect::<String>();
    Ok(if kind == "weekly" {
        normalize_weekly_numbering(&bounded)
    } else {
        bounded
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    #[test]
    fn default_periods_are_prior_seven_days_and_previous_calendar_month() {
        let now = chrono::DateTime::parse_from_rfc3339("2026-08-20T12:00:00+08:00").unwrap();
        let (weekly_start, weekly_end) = default_weekly_period(now.timestamp());
        assert_eq!(weekly_end - weekly_start, 7 * 86400);
        assert_eq!(weekly_end, now.timestamp());
        let (month_start, month_end) = default_monthly_period(now.timestamp()).unwrap();
        assert_eq!(
            Local
                .timestamp_opt(month_start, 0)
                .single()
                .unwrap()
                .format("%Y-%m-%d")
                .to_string(),
            "2026-07-01"
        );
        assert_eq!(
            Local
                .timestamp_opt(month_end, 0)
                .single()
                .unwrap()
                .format("%Y-%m-%d")
                .to_string(),
            "2026-08-01"
        );
    }

    #[test]
    fn monthly_snapshot_embeds_overlapping_weekly_reports_and_excludes_translation() {
        let db = Database::open_in_memory().unwrap();
        db.conn().execute_batch("INSERT INTO provider_settings(display_name,provider_type,base_url,model,enabled,created_at,updated_at) VALUES('Mock','openai_compatible','http://127.0.0.1','',1,1,1); INSERT INTO provider_models(provider_id,model_id,display_name,protocol,endpoint_path,source,created_at,updated_at) VALUES(1,'mock','Mock','chat_completions','/chat/completions','manual',1,1);").unwrap();
        let report_repo = crate::db::reports::ReportRepo::new(db.conn());
        let weekly = report_repo.create("weekly", 100, 200).unwrap();
        report_repo
            .complete(weekly.id, 1, "1. 周度推进", "h", "{}", "[]")
            .unwrap();
        let snapshot = build_report_snapshot(&db, "monthly", 50, 250, "zh-CN").unwrap();
        assert_eq!(snapshot.weekly_reports.len(), 1);
        assert_eq!(snapshot.weekly_reports[0].report_id, weekly.id);
        let serialized = serde_json::to_string(&snapshot).unwrap();
        assert!(!serialized.to_lowercase().contains("translation"));
    }

    #[test]
    fn weekly_numbering_and_prompts_enforce_deep_report_contracts() {
        assert_eq!(
            normalize_weekly_numbering("完成资料整理\n- 跟进专家反馈\n3) 安排下周会议"),
            "1. 完成资料整理\n2. 跟进专家反馈\n3. 安排下周会议"
        );
        let weekly = build_report_system_prompt("weekly", "zh-CN").unwrap();
        assert!(weekly.contains("numbered list"));
        assert!(weekly.contains("long-term Work"));
        let monthly = build_report_system_prompt("monthly", "zh-CN").unwrap();
        assert!(monthly.contains("cross-project"));
        assert!(monthly.contains("weekly reports"));
        assert!(monthly.contains("historical change"));
    }
}
