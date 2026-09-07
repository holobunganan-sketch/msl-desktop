//! Three-hour/daily AI secretary scheduling. File watcher events never call this module directly.

use crate::db::ai::AnalysisSchedule;
use chrono::{DateTime, Datelike, Local, TimeZone, Timelike};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DueTrigger {
    Interval,
    Daily,
}

pub fn evaluate_due(schedule: &AnalysisSchedule, now: DateTime<Local>) -> Option<DueTrigger> {
    if !schedule.enabled {
        return None;
    }
    let interval_due = schedule
        .last_interval_run_at
        .map(|last| now.timestamp() - last >= schedule.interval_minutes * 60)
        .unwrap_or(true);
    let local_date = now.date_naive().to_string();
    let hour = i64::from(now.hour());
    let minute = i64::from(now.minute());
    let daily_due = schedule.daily_enabled
        && (hour > schedule.daily_hour
            || (hour == schedule.daily_hour && minute >= schedule.daily_minute))
        && schedule.last_daily_local_date.as_deref() != Some(local_date.as_str());
    if daily_due {
        Some(DueTrigger::Daily)
    } else if interval_due {
        Some(DueTrigger::Interval)
    } else {
        None
    }
}

pub fn mark_attempt(schedule: &mut AnalysisSchedule, trigger: DueTrigger, now: DateTime<Local>) {
    schedule.last_interval_run_at = Some(now.timestamp());
    if matches!(trigger, DueTrigger::Daily) {
        schedule.last_daily_local_date = Some(now.date_naive().to_string());
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportDue {
    pub kind: String,
    pub period_start: i64,
    pub period_end: i64,
    pub period_key: String,
}

pub fn evaluate_report_due(
    schedule: &crate::db::reports::ReportSchedule,
    now: DateTime<Local>,
) -> Option<ReportDue> {
    let reached = |hour: i64, minute: i64| {
        i64::from(now.hour()) > hour
            || (i64::from(now.hour()) == hour && i64::from(now.minute()) >= minute)
    };
    if schedule.monthly_enabled
        && i64::from(now.day()) == schedule.monthly_day
        && reached(schedule.monthly_hour, schedule.monthly_minute)
    {
        let scheduled = Local
            .with_ymd_and_hms(
                now.year(),
                now.month(),
                now.day(),
                schedule.monthly_hour as u32,
                schedule.monthly_minute as u32,
                0,
            )
            .single()?;
        let key = format!("monthly:{}", scheduled.format("%Y-%m"));
        if schedule.last_monthly_period_key.as_deref() != Some(key.as_str()) {
            let (period_start, period_end) =
                crate::ai::reports::default_monthly_period(scheduled.timestamp()).ok()?;
            return Some(ReportDue {
                kind: "monthly".into(),
                period_start,
                period_end,
                period_key: key,
            });
        }
    }
    if schedule.weekly_enabled
        && i64::from(now.weekday().num_days_from_monday()) == schedule.weekly_weekday
        && reached(schedule.weekly_hour, schedule.weekly_minute)
    {
        let scheduled = Local
            .with_ymd_and_hms(
                now.year(),
                now.month(),
                now.day(),
                schedule.weekly_hour as u32,
                schedule.weekly_minute as u32,
                0,
            )
            .single()?;
        let key = format!("weekly:{}", scheduled.format("%Y-%m-%dT%H:%M"));
        if schedule.last_weekly_period_key.as_deref() != Some(key.as_str()) {
            return Some(ReportDue {
                kind: "weekly".into(),
                period_start: scheduled.timestamp().saturating_sub(7 * 86400),
                period_end: scheduled.timestamp(),
                period_key: key,
            });
        }
    }
    None
}

pub fn mark_report_attempt(schedule: &mut crate::db::reports::ReportSchedule, due: &ReportDue) {
    if due.kind == "monthly" {
        schedule.last_monthly_period_key = Some(due.period_key.clone());
    } else {
        schedule.last_weekly_period_key = Some(due.period_key.clone());
    }
}

async fn poll_once(app: &tauri::AppHandle) {
    use tauri::Manager;

    let state = app.state::<crate::app_state::AppState>();
    let now = Local::now();
    let analysis_due = state.with_database(|db| {
        let repo = crate::db::ai::ScheduleRepo::new(db.conn());
        let mut schedule = repo.get().ok()?;
        let due = evaluate_due(&schedule, now)?;
        mark_attempt(&mut schedule, due, now);
        repo.save(&schedule).ok()?;
        Some(due)
    });
    if let Some(Some(trigger)) = analysis_due {
        let name = match trigger {
            DueTrigger::Daily => "daily",
            DueTrigger::Interval => "interval",
        };
        if crate::commands::jobs::start_ai_job(
            app.clone(),
            app.state(),
            crate::commands::jobs::JobRequest::RunAnalysisNow {
                trigger: Some(name.into()),
            },
        )
        .is_err()
        {
            eprintln!("[scheduler] scheduled analysis failed");
        }
    }

    let report_due = state.with_database(|db| {
        let repo = crate::db::reports::ReportScheduleRepo::new(db.conn());
        let mut schedule = repo.get().ok()?;
        let due = evaluate_report_due(&schedule, now)?;
        mark_report_attempt(&mut schedule, &due);
        repo.save(&schedule).ok()?;
        Some(due)
    });
    if let Some(Some(due)) = report_due {
        if crate::commands::jobs::start_ai_job(
            app.clone(),
            app.state(),
            crate::commands::jobs::JobRequest::GenerateReport {
                kind: due.kind,
                period_start: Some(due.period_start),
                period_end: Some(due.period_end),
            },
        )
        .is_err()
        {
            eprintln!("[scheduler] scheduled report failed");
        }
    }
}

pub fn spawn(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            poll_once(&app).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    #[test]
    fn interval_daily_coalesce_and_cross_day_are_deterministic() {
        let mut s = AnalysisSchedule {
            id: 1,
            enabled: true,
            interval_minutes: 180,
            daily_enabled: true,
            daily_hour: 6,
            daily_minute: 0,
            last_interval_run_at: Some(1_000),
            last_daily_local_date: Some("2026-08-14".into()),
            updated_at: 0,
        };
        let at = Local.with_ymd_and_hms(2026, 8, 15, 6, 5, 0).unwrap();
        assert_eq!(evaluate_due(&s, at), Some(DueTrigger::Daily));
        mark_attempt(&mut s, DueTrigger::Daily, at);
        assert_eq!(s.last_daily_local_date.as_deref(), Some("2026-08-15"));
        assert!(evaluate_due(&s, at).is_none());
    }
    #[test]
    fn disabled_and_interval_bounds() {
        let mut s = AnalysisSchedule {
            id: 1,
            enabled: false,
            interval_minutes: 180,
            daily_enabled: false,
            daily_hour: 6,
            daily_minute: 0,
            last_interval_run_at: None,
            last_daily_local_date: None,
            updated_at: 0,
        };
        let at = Local.with_ymd_and_hms(2026, 8, 15, 7, 0, 0).unwrap();
        assert_eq!(evaluate_due(&s, at), None);
        s.enabled = true;
        assert_eq!(evaluate_due(&s, at), Some(DueTrigger::Interval));
    }

    #[test]
    fn weekly_and_monthly_report_due_times_are_deduplicated_by_period() {
        let schedule = crate::db::reports::ReportSchedule {
            id: 1,
            weekly_enabled: true,
            weekly_weekday: 6,
            weekly_hour: 17,
            weekly_minute: 0,
            last_weekly_period_key: None,
            monthly_enabled: true,
            monthly_day: 1,
            monthly_hour: 9,
            monthly_minute: 0,
            last_monthly_period_key: None,
            updated_at: 0,
        };
        let sunday = Local.with_ymd_and_hms(2026, 8, 23, 17, 5, 0).unwrap();
        let due = evaluate_report_due(&schedule, sunday).unwrap();
        assert_eq!(due.kind, "weekly");
        assert_eq!(due.period_end - due.period_start, 7 * 86400);
        let mut marked = schedule.clone();
        mark_report_attempt(&mut marked, &due);
        assert!(evaluate_report_due(&marked, sunday).is_none());

        let month_start = Local.with_ymd_and_hms(2026, 9, 1, 9, 5, 0).unwrap();
        let monthly = evaluate_report_due(&marked, month_start).unwrap();
        assert_eq!(monthly.kind, "monthly");
        assert_eq!(
            Local
                .timestamp_opt(monthly.period_start, 0)
                .single()
                .unwrap()
                .format("%Y-%m-%d")
                .to_string(),
            "2026-08-01"
        );
        assert_eq!(
            Local
                .timestamp_opt(monthly.period_end, 0)
                .single()
                .unwrap()
                .format("%Y-%m-%d")
                .to_string(),
            "2026-09-01"
        );
    }

    #[test]
    fn configurable_weekly_schedule_waits_until_selected_time() {
        let schedule = crate::db::reports::ReportSchedule {
            id: 1,
            weekly_enabled: true,
            weekly_weekday: 4,
            weekly_hour: 16,
            weekly_minute: 30,
            last_weekly_period_key: None,
            monthly_enabled: false,
            monthly_day: 1,
            monthly_hour: 9,
            monthly_minute: 0,
            last_monthly_period_key: None,
            updated_at: 0,
        };
        assert!(evaluate_report_due(
            &schedule,
            Local.with_ymd_and_hms(2026, 8, 21, 16, 29, 0).unwrap()
        )
        .is_none());
        assert_eq!(
            evaluate_report_due(
                &schedule,
                Local.with_ymd_and_hms(2026, 8, 21, 16, 30, 0).unwrap()
            )
            .unwrap()
            .kind,
            "weekly"
        );
    }
}
