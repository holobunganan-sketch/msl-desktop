//! Morning Brief（指南 §7.1-E / §8.2 / §8.3 / §22）。
//!
//! - 只使用本地管理事实构造 snapshot，不把整个数据库/文件夹发给模型；
//! - 系统提示约束：只能根据 snapshot、不确定就说不确定、不发明完成状态、
//!   输出管理建议、不生成医学结论、不扩写；
//! - daily cache：同一天 + snapshot 无变化时复用已有 brief。

use serde::Serialize;

use crate::ai::provider::{AiError, AiMessage, complete};
use crate::db::provider::ProviderSetting;
use crate::db::{Database, DbResult};

/// 简化的今日日期（YYYY-MM-DD，由前端按本地时区计算传入）。
#[derive(Debug, Clone, Serialize)]
pub struct BriefSnapshot {
    pub date: String,
    pub yesterday_activity: Vec<String>,
    pub active_works: Vec<serde_json::Value>,
    pub latest_resume_points: Vec<serde_json::Value>,
    pub open_waiting: Vec<serde_json::Value>,
    pub today_tasks: Vec<serde_json::Value>,
    pub today_calendar: Vec<serde_json::Value>,
    pub recent_file_changes: Vec<String>,
}

/// 从数据库构造 snapshot（只取管理事实，字段克制）。
/// `day_start` / `day_end`：今天的本地时区起止；`yesterday_start/end`：昨天。
pub fn build_snapshot(
    db: &Database,
    date: &str,
    yesterday_start: i64,
    yesterday_end: i64,
    day_start: i64,
    day_end: i64,
) -> DbResult<BriefSnapshot> {
    use crate::db::activity::ActivityRepo;
    use crate::db::calendar::CalendarRepo;
    use crate::db::task::{TaskRepo, WaitingRepo};
    use crate::db::work::{ResumePointRepo, WorkRepo};

    let act = ActivityRepo::new(db.conn());
    let yesterday_activity: Vec<String> = act
        .query(Some(yesterday_start), Some(yesterday_end), None, None, None, Some(30))?
        .into_iter()
        .map(|a| a.display_text)
        .collect();

    let recent_file_changes: Vec<String> = act
        .query(None, None, None, None, None, Some(10))?
        .into_iter()
        .filter(|a| a.event_type.starts_with("file."))
        .map(|a| a.display_text)
        .collect();

    let work_repo = WorkRepo::new(db.conn());
    let resume_repo = ResumePointRepo::new(db.conn());
    let active_works: Vec<serde_json::Value> = work_repo
        .list(Some("active"))?
        .into_iter()
        .map(|w| {
            serde_json::json!({
                "title": w.title,
                "status": w.status,
                "updated_at": w.updated_at,
            })
        })
        .collect();

    let latest_resume_points: Vec<serde_json::Value> = work_repo
        .list(None)?
        .into_iter()
        .filter(|w| w.status != "done" && w.status != "archived")
        .filter_map(|w| resume_repo.latest_for_work(w.id).ok().flatten())
        .map(|rp| {
            serde_json::json!({
                "current_state": rp.current_state,
                "next_step": rp.next_step,
                "remember": rp.remember,
            })
        })
        .collect();

    let open_waiting: Vec<serde_json::Value> = WaitingRepo::new(db.conn())
        .list(Some("open"), None)?
        .into_iter()
        .map(|w| {
            serde_json::json!({
                "title": w.title,
                "waiting_for": w.waiting_for,
                "follow_up_at": w.follow_up_at,
            })
        })
        .collect();

    let today_tasks: Vec<serde_json::Value> = TaskRepo::new(db.conn())
        .list(None, None)?
        .into_iter()
        .filter(|t| t.status != "done" && t.due_at.is_some_and(|d| d <= day_end))
        .map(|t| {
            serde_json::json!({
                "title": t.title,
                "status": t.status,
                "priority": t.priority,
                "due_at": t.due_at,
            })
        })
        .collect();

    let today_calendar: Vec<serde_json::Value> = CalendarRepo::new(db.conn())
        .list_between(day_start, day_end)?
        .into_iter()
        .map(|e| {
            serde_json::json!({
                "title": e.title,
                "kind": e.kind,
                "start_at": e.start_at,
            })
        })
        .collect();

    Ok(BriefSnapshot {
        date: date.to_string(),
        yesterday_activity,
        active_works,
        latest_resume_points,
        open_waiting,
        today_tasks,
        today_calendar,
        recent_file_changes,
    })
}

/// snapshot 的确定性指纹（同一天输入无变化时复用缓存）。
pub fn snapshot_hash(snapshot: &BriefSnapshot) -> String {
    use std::hash::{Hash, Hasher};
    let json = serde_json::to_string(snapshot).unwrap_or_default();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    json.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// 构造系统提示（指南 §8.3 约束）。
pub fn build_messages(snapshot: &BriefSnapshot) -> Vec<AiMessage> {
    let system = "\
你是 MSL（医学联络官）的工作管理助手。你的任务是基于用户提供的工作事实快照，\
生成简短的管理建议。

严格规则：
1. 只能使用快照中已存在的事实；
2. 快照中没有的信息，明确说'不确定'，不要编造；
3. 不得发明任务的完成状态；
4. 输出中文，按以下结构，总长度不超过 300 字：
   - 昨天主要推进了什么（没有就写'无记录'）
   - 哪些工作停在哪里
   - 今天有什么硬安排
   - 有哪些 waiting 需要关注
   - 建议优先接续的 1-3 项工作
5. 只输出管理建议，不生成医学结论，不扩写成报告。";

    let snapshot_json = serde_json::to_string_pretty(snapshot).unwrap_or_default();
    vec![
        AiMessage {
            role: "system".into(),
            content: system.into(),
        },
        AiMessage {
            role: "user".into(),
            content: snapshot_json,
        },
    ]
}

/// 同步准备：构造 snapshot + 指纹，并检查 daily cache。
/// 返回 (缓存命中, snapshot, hash)；命中时无需调用模型。
pub fn prepare(
    db: &Database,
    date: &str,
    yesterday_start: i64,
    yesterday_end: i64,
    day_start: i64,
    day_end: i64,
    force: bool,
) -> DbResult<(
    Option<crate::db::brief::DailyBrief>,
    BriefSnapshot,
    String,
)> {
    let snapshot = build_snapshot(db, date, yesterday_start, yesterday_end, day_start, day_end)?;
    let hash = snapshot_hash(&snapshot);

    if !force {
        if let Some(existing) = crate::db::brief::BriefRepo::new(db.conn()).latest_for_date(date)? {
            if existing.source_snapshot_hash.as_deref() == Some(hash.as_str()) {
                return Ok((Some(existing), snapshot, hash));
            }
        }
    }
    Ok((None, snapshot, hash))
}

/// 异步调用模型（不接触数据库）。
pub async fn call(
    provider: &ProviderSetting,
    api_key: &str,
    snapshot: &BriefSnapshot,
) -> Result<String, AiError> {
    let messages = build_messages(snapshot);
    let response = complete(
        provider,
        api_key,
        &crate::ai::provider::AiRequest {
            model: provider.model.clone(),
            messages,
            temperature: Some(0.3),
            max_tokens: Some(800),
        },
    )
    .await?;
    Ok(response.content)
}

/// 同步保存结果。
pub fn save(
    db: &Database,
    date: &str,
    provider_name: &str,
    content: &str,
    hash: &str,
) -> DbResult<crate::db::brief::DailyBrief> {
    crate::db::brief::BriefRepo::new(db.conn())
        .insert(date, Some(provider_name), content, Some(hash))
}
