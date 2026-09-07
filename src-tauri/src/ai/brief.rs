//! Morning Brief：将工作台事实压缩为可追溯、有限大小的本地快照。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ai::provider::{AiError, AiMessage, AiTextRequest};
use crate::db::provider::{ProviderConnection, ProviderModel};
use crate::db::{Database, DbResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefFact {
    #[serde(rename = "type")]
    pub source_type: String,
    pub entity_id: Option<i64>,
    pub work_id: Option<i64>,
    pub title: Option<String>,
    pub display: Option<String>,
    pub timestamp: Option<i64>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub due_at: Option<i64>,
    pub start_at: Option<i64>,
    pub end_at: Option<i64>,
    pub summary: Option<String>,
    pub current_state: Option<String>,
    pub next_step: Option<String>,
    pub waiting_for: Option<String>,
    pub follow_up_at: Option<i64>,
    pub content: Option<String>,
}

impl BriefFact {
    fn new(source_type: &str) -> Self {
        Self {
            source_type: source_type.into(),
            entity_id: None,
            work_id: None,
            title: None,
            display: None,
            timestamp: None,
            status: None,
            priority: None,
            due_at: None,
            start_at: None,
            end_at: None,
            summary: None,
            current_state: None,
            next_step: None,
            waiting_for: None,
            follow_up_at: None,
            content: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceCounts {
    pub works: u32,
    pub resume_points: u32,
    pub tasks_open: u32,
    pub tasks_completed: u32,
    pub waiting: u32,
    pub calendar: u32,
    pub inbox: u32,
    pub file_changes: u32,
    pub activity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefSnapshot {
    pub date: String,
    pub period_start: i64,
    pub period_end: i64,
    pub today_start: i64,
    pub today_end: i64,
    pub locale: String,
    pub activity: Vec<BriefFact>,
    pub works: Vec<BriefFact>,
    pub resume_points: Vec<BriefFact>,
    pub tasks_open: Vec<BriefFact>,
    pub tasks_completed: Vec<BriefFact>,
    pub waiting: Vec<BriefFact>,
    pub calendar: Vec<BriefFact>,
    pub inbox: Vec<BriefFact>,
    pub file_changes: Vec<BriefFact>,
    pub source_counts: SourceCounts,
    pub truncated: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BriefResult {
    pub brief: crate::db::brief::DailyBrief,
    pub content: String,
    pub source_counts: SourceCounts,
    pub source_preview: Vec<BriefFact>,
    pub ai_used: bool,
    pub warning: Option<String>,
    pub period_start: i64,
    pub period_end: i64,
    pub locale: String,
}

fn cap<T>(
    mut values: Vec<T>,
    limit: usize,
    truncated: &mut BTreeMap<String, u32>,
    key: &str,
) -> Vec<T> {
    if values.len() > limit {
        let count = (values.len() - limit) as u32;
        truncated.insert(key.to_string(), count);
        values.truncate(limit);
    }
    values
}

/// 构造结构化快照。只读取管理事实和文件 metadata 活动，不读取工作文件正文。
pub fn build_snapshot(
    db: &Database,
    date: &str,
    period_start: i64,
    period_end: i64,
    today_start: i64,
    today_end: i64,
    locale: &str,
) -> DbResult<BriefSnapshot> {
    use crate::db::activity::ActivityRepo;
    use crate::db::calendar::CalendarRepo;
    use crate::db::inbox::InboxRepo;
    use crate::db::task::{TaskRepo, WaitingRepo};
    use crate::db::work::{ResumePointRepo, WorkRepo};

    let mut truncated = BTreeMap::new();
    let mut activity_rows = ActivityRepo::new(db.conn()).query(
        Some(period_start),
        Some(period_end),
        None,
        None,
        None,
        Some(201),
    )?;
    if activity_rows.len() > 200 {
        truncated.insert("activity".into(), 1);
        activity_rows.truncate(200);
    }
    let activity = activity_rows
        .into_iter()
        .map(|a| {
            let mut f = BriefFact::new("activity");
            f.entity_id = Some(a.id);
            f.work_id = a.work_id;
            f.display = Some(a.display_text);
            f.timestamp = Some(a.timestamp);
            f.status = Some(a.event_type);
            f
        })
        .collect::<Vec<_>>();

    let works_all = WorkRepo::new(db.conn()).list(None)?;
    let works = cap(
        works_all
            .iter()
            .filter(|w| w.status != "archived")
            .map(|w| {
                let mut f = BriefFact::new("work");
                f.entity_id = Some(w.id);
                f.title = Some(w.title.clone());
                f.status = Some(w.status.clone());
                f.summary = w.summary.clone();
                f.timestamp = Some(w.updated_at);
                f
            })
            .collect(),
        100,
        &mut truncated,
        "works",
    );

    let resume_repo = ResumePointRepo::new(db.conn());
    let resume_points = cap(
        works_all
            .iter()
            .filter(|w| w.status != "archived")
            .filter_map(|w| {
                resume_repo
                    .latest_for_work(w.id)
                    .ok()
                    .flatten()
                    .map(|rp| (w, rp))
            })
            .map(|(w, rp)| {
                let mut f = BriefFact::new("resume_point");
                f.entity_id = Some(rp.id);
                f.work_id = Some(w.id);
                f.title = Some(w.title.clone());
                f.current_state = Some(rp.current_state);
                f.next_step = Some(rp.next_step);
                f.summary = Some(rp.remember);
                f.timestamp = Some(rp.created_at);
                f
            })
            .collect(),
        100,
        &mut truncated,
        "resume_points",
    );

    let mut open_tasks = TaskRepo::new(db.conn())
        .list(None, None)?
        .into_iter()
        .filter(|t| {
            t.status != "done" && (t.due_at.is_none() || t.due_at.is_some_and(|d| d <= today_end))
        })
        .collect::<Vec<_>>();
    open_tasks.sort_by(|a, b| {
        let rank = |p: &str| match p {
            "high" => 0,
            "normal" => 1,
            _ => 2,
        };
        rank(&a.priority)
            .cmp(&rank(&b.priority))
            .then_with(|| b.updated_at.cmp(&a.updated_at))
    });
    let tasks_open = cap(
        open_tasks
            .into_iter()
            .map(|t| {
                let mut f = BriefFact::new("task_open");
                f.entity_id = Some(t.id);
                f.work_id = t.work_id;
                f.title = Some(t.title);
                f.status = Some(t.status);
                f.priority = Some(t.priority);
                f.due_at = t.due_at;
                f.summary = t.notes;
                f.timestamp = Some(t.updated_at);
                f
            })
            .collect(),
        100,
        &mut truncated,
        "tasks_open",
    );

    let tasks_completed = cap(
        TaskRepo::new(db.conn())
            .list(Some("done"), None)?
            .into_iter()
            .filter(|t| {
                t.completed_at
                    .is_some_and(|ts| ts >= period_start && ts <= period_end)
            })
            .map(|t| {
                let mut f = BriefFact::new("task_completed");
                f.entity_id = Some(t.id);
                f.work_id = t.work_id;
                f.title = Some(t.title);
                f.status = Some(t.status);
                f.priority = Some(t.priority);
                f.due_at = t.due_at;
                f.timestamp = t.completed_at;
                f
            })
            .collect(),
        100,
        &mut truncated,
        "tasks_completed",
    );

    let waiting = cap(
        WaitingRepo::new(db.conn())
            .list(Some("open"), None)?
            .into_iter()
            .map(|w| {
                let mut f = BriefFact::new("waiting");
                f.entity_id = Some(w.id);
                f.work_id = w.work_id;
                f.title = Some(w.title);
                f.status = Some(w.status);
                f.waiting_for = Some(w.waiting_for);
                f.follow_up_at = w.follow_up_at;
                f.summary = w.notes;
                f.timestamp = Some(w.updated_at);
                f
            })
            .collect(),
        100,
        &mut truncated,
        "waiting",
    );

    let calendar = cap(
        CalendarRepo::new(db.conn())
            .list_between(today_start, today_end)?
            .into_iter()
            .map(|e| {
                let mut f = BriefFact::new("calendar");
                f.entity_id = Some(e.id);
                f.work_id = e.work_id;
                f.title = Some(e.title);
                f.status = Some(e.kind);
                f.start_at = Some(e.start_at);
                f.end_at = e.end_at;
                f.summary = e.notes.or(e.location);
                f
            })
            .collect(),
        100,
        &mut truncated,
        "calendar",
    );

    let inbox = cap(
        InboxRepo::new(db.conn())
            .list_unprocessed(100)?
            .into_iter()
            .map(|item| {
                let mut f = BriefFact::new("inbox");
                f.entity_id = Some(item.id);
                f.content = Some(item.content);
                f.timestamp = Some(item.created_at);
                f
            })
            .collect(),
        100,
        &mut truncated,
        "inbox",
    );

    let file_changes = cap(
        ActivityRepo::new(db.conn())
            .list_file_changes(Some(period_start), Some(period_end), 200)?
            .into_iter()
            .map(|event| {
                let mut f = BriefFact::new("file_change");
                f.entity_id = Some(event.entity_id.unwrap_or(event.id));
                f.work_id = event.work_id;
                f.title = event.path;
                f.display = Some(event.display_text);
                f.status = Some(event.event_type);
                f.timestamp = Some(event.timestamp);
                f
            })
            .collect(),
        200,
        &mut truncated,
        "file_changes",
    );

    Ok(BriefSnapshot {
        date: date.to_string(),
        period_start,
        period_end,
        today_start,
        today_end,
        locale: locale.to_string(),
        source_counts: SourceCounts {
            works: works.len() as u32,
            resume_points: resume_points.len() as u32,
            tasks_open: tasks_open.len() as u32,
            tasks_completed: tasks_completed.len() as u32,
            waiting: waiting.len() as u32,
            calendar: calendar.len() as u32,
            inbox: inbox.len() as u32,
            file_changes: file_changes.len() as u32,
            activity: activity.len() as u32,
        },
        activity,
        works,
        resume_points,
        tasks_open,
        tasks_completed,
        waiting,
        calendar,
        inbox,
        file_changes,
        truncated,
    })
}

pub fn snapshot_hash(snapshot: &BriefSnapshot) -> String {
    use std::hash::{Hash, Hasher};
    let json = serde_json::to_string(snapshot).unwrap_or_default();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    json.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

pub fn build_messages(snapshot: &BriefSnapshot) -> Vec<AiMessage> {
    vec![AiMessage {
        role: "user".into(),
        content: serde_json::to_string_pretty(snapshot).unwrap_or_default(),
    }]
}

pub fn build_ai_request(model_id: &str, snapshot: &BriefSnapshot) -> AiTextRequest {
    AiTextRequest {
        model_id: model_id.to_string(),
        system: Some(crate::ai::prompts::build_system_prompt(
            crate::ai::prompts::SecretaryStage::DailyBrief,
            crate::ai::prompts::PromptOptions {
                locale: &snapshot.locale,
                ..Default::default()
            },
        )),
        messages: build_messages(snapshot),
        temperature: Some(0.3),
        // Reasoning models can spend a meaningful part of the budget before they
        // emit the visible brief. Leave enough room for the required bullet text.
        max_output_tokens: Some(2_000),
    }
}

/// Keep model and local summaries visually consistent even when a provider
/// ignores the requested bullet contract.
fn strip_internal_source_markers(line: &str) -> String {
    const MARKERS: [&str; 5] = [
        "source_type",
        "entity_id",
        "workspace_id",
        "source_id",
        "[source_",
    ];
    let mut cleaned = line.to_string();
    loop {
        let Some(marker_index) = MARKERS
            .iter()
            .filter_map(|marker| cleaned.find(marker))
            .min()
        else {
            break;
        };
        let before = &cleaned[..marker_index];
        let opening = [('(', ')'), ('（', '）'), ('[', ']')]
            .iter()
            .filter_map(|(open, close)| before.rfind(*open).map(|index| (index, *close)))
            .max_by_key(|(index, _)| *index);
        if let Some((start, close)) = opening {
            if let Some(relative_end) = cleaned[marker_index..].find(close) {
                let end = marker_index + relative_end + close.len_utf8();
                cleaned.replace_range(start..end, "");
                continue;
            }
        }
        for marker in MARKERS {
            cleaned = cleaned.replace(marker, "");
        }
    }
    cleaned
        .replace("  ", " ")
        .replace(" 。", "。")
        .replace(" ,", ",")
        .trim()
        .to_string()
}

pub fn normalize_bullet_output(content: &str, locale: &str) -> String {
    let bullets = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| {
            strip_internal_source_markers(
                line.trim_start_matches(|ch: char| {
                    matches!(ch, '•' | '-' | '*' | '·' | '▪' | '‣') || ch.is_whitespace()
                })
                .trim(),
            )
        })
        .filter(|line| !line.is_empty())
        .take(7)
        .map(|line| format!("• {line}"))
        .collect::<Vec<_>>();
    if bullets.is_empty() {
        if locale == "en-US" {
            "• No reliable summary was returned. Review the source items.".into()
        } else {
            "• 未返回可靠摘要，请检查来源事项。".into()
        }
    } else {
        bullets.join("\n")
    }
}

fn fact_title(fact: &BriefFact) -> String {
    fact.title
        .clone()
        .or_else(|| fact.content.clone())
        .or_else(|| fact.display.clone())
        .unwrap_or_else(|| "（未命名事项）".into())
}

/// 不依赖 AI 的确定性摘要。候选顺序固定：逾期高优任务 > 今日安排 > 到期 waiting > Resume next_step > Inbox。
pub fn render_local(snapshot: &BriefSnapshot) -> String {
    let english = snapshot.locale == "en-US";
    let no_records = if english {
        "• No records in the selected range. Check the workspace baseline or add items."
    } else {
        "• 当前范围没有记录。请检查工作目录基线或录入事项。"
    };
    let has_any = snapshot.source_counts.activity
        + snapshot.source_counts.works
        + snapshot.source_counts.resume_points
        + snapshot.source_counts.tasks_open
        + snapshot.source_counts.tasks_completed
        + snapshot.source_counts.waiting
        + snapshot.source_counts.calendar
        + snapshot.source_counts.inbox
        + snapshot.source_counts.file_changes
        > 0;
    if !has_any {
        return no_records.into();
    }

    let mut recommendations = Vec::new();
    for task in snapshot
        .tasks_open
        .iter()
        .filter(|task| task.priority.as_deref() == Some("high"))
    {
        if recommendations.len() < 3 {
            recommendations.push(if english {
                format!("Task: {}", fact_title(task))
            } else {
                format!("任务：{}", fact_title(task))
            });
        }
    }
    for event in &snapshot.calendar {
        if recommendations.len() < 3 {
            recommendations.push(if english {
                format!("Calendar: {}", fact_title(event))
            } else {
                format!("日程：{}", fact_title(event))
            });
        }
    }
    for waiting in snapshot
        .waiting
        .iter()
        .filter(|item| item.follow_up_at.is_some())
    {
        if recommendations.len() < 3 {
            recommendations.push(if english {
                format!("Follow up: {}", fact_title(waiting))
            } else {
                format!("跟进：{}", fact_title(waiting))
            });
        }
    }
    for resume in &snapshot.resume_points {
        if recommendations.len() < 3 {
            if let Some(next) = &resume.next_step {
                recommendations.push(if english {
                    format!("Resume {}: {}", fact_title(resume), next)
                } else {
                    format!("接续 {}：{}", fact_title(resume), next)
                });
            }
        }
    }
    for inbox in &snapshot.inbox {
        if recommendations.len() < 3 {
            recommendations.push(if english {
                format!("Inbox: {}", fact_title(inbox))
            } else {
                format!("收件箱：{}", fact_title(inbox))
            });
        }
    }
    let progress = snapshot
        .tasks_completed
        .iter()
        .take(3)
        .map(fact_title)
        .collect::<Vec<_>>()
        .join(if english { "; " } else { "；" });
    let current = snapshot
        .works
        .iter()
        .take(3)
        .map(fact_title)
        .collect::<Vec<_>>()
        .join(if english { "; " } else { "；" });
    let next_steps = snapshot
        .resume_points
        .iter()
        .filter_map(|fact| fact.next_step.clone())
        .take(3)
        .collect::<Vec<_>>()
        .join(if english { "; " } else { "；" });
    let hard = snapshot
        .calendar
        .iter()
        .take(3)
        .map(fact_title)
        .collect::<Vec<_>>()
        .join(if english { "; " } else { "；" });
    let follow = snapshot
        .waiting
        .iter()
        .take(3)
        .map(fact_title)
        .collect::<Vec<_>>()
        .join(if english { "; " } else { "；" });
    let tasks = snapshot
        .tasks_open
        .iter()
        .take(4)
        .map(|fact| {
            let title = fact_title(fact);
            if fact.due_at.is_some_and(|due| due < snapshot.today_start) {
                if english {
                    format!("Overdue: {title}")
                } else {
                    format!("逾期：{title}")
                }
            } else {
                title
            }
        })
        .collect::<Vec<_>>()
        .join(if english { "; " } else { "；" });
    let inbox = snapshot
        .inbox
        .iter()
        .take(3)
        .map(fact_title)
        .collect::<Vec<_>>()
        .join(if english { "; " } else { "；" });
    let files = snapshot
        .file_changes
        .iter()
        .take(3)
        .map(fact_title)
        .collect::<Vec<_>>()
        .join(if english { "; " } else { "；" });
    let recs = recommendations.join(if english { "; " } else { "；" });
    let value = |text: &String, empty: &str| {
        if text.is_empty() {
            empty.to_string()
        } else {
            text.clone()
        }
    };
    if english {
        [
            format!("• Past progress: {}", value(&progress, "None recorded")),
            format!("• Work in progress: {}", value(&current, "None recorded")),
            format!("• Today's calendar: {}", value(&hard, "No fixed schedule")),
            format!(
                "• Open and overdue tasks: {}",
                value(&tasks, "None recorded")
            ),
            format!(
                "• Waiting and blockers: {}",
                value(&follow, "None recorded")
            ),
            format!("• Inbox to triage: {}", value(&inbox, "None recorded")),
            format!("• Workspace changes: {}", value(&files, "None recorded")),
            format!(
                "• Recommended next actions: {}",
                if recs.is_empty() {
                    value(&next_steps, "Review the open items")
                } else {
                    recs
                }
            ),
        ]
        .join("\n")
    } else {
        [
            format!("• 过去进展：{}", value(&progress, "无记录")),
            format!("• 当前推进：{}", value(&current, "无记录")),
            format!("• 今日日程：{}", value(&hard, "无安排")),
            format!("• 待办与逾期：{}", value(&tasks, "无记录")),
            format!("• 等待与阻塞：{}", value(&follow, "无记录")),
            format!("• 收件箱待整理：{}", value(&inbox, "无记录")),
            format!("• 工作目录变化：{}", value(&files, "无记录")),
            format!(
                "• 建议推进：{}",
                if recs.is_empty() {
                    value(&next_steps, "优先检查未完成事项")
                } else {
                    recs
                }
            ),
        ]
        .join("\n")
    }
}

pub fn prepare(
    db: &Database,
    date: &str,
    period_start: i64,
    period_end: i64,
    today_start: i64,
    today_end: i64,
    locale: &str,
    force: bool,
) -> DbResult<(Option<crate::db::brief::DailyBrief>, BriefSnapshot, String)> {
    let snapshot = build_snapshot(
        db,
        date,
        period_start,
        period_end,
        today_start,
        today_end,
        locale,
    )?;
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

pub async fn call(
    connection: &ProviderConnection,
    model: &ProviderModel,
    api_key: &str,
    snapshot: &BriefSnapshot,
) -> Result<String, AiError> {
    let response = crate::ai::provider::complete_model(
        connection,
        model,
        api_key,
        &build_ai_request(&model.model_id, snapshot),
    )
    .await?;
    Ok(normalize_bullet_output(&response.content, &snapshot.locale))
}

pub fn save(
    db: &Database,
    date: &str,
    provider_name: &str,
    content: &str,
    hash: &str,
    snapshot: &BriefSnapshot,
) -> DbResult<crate::db::brief::DailyBrief> {
    save_with_meta(db, date, provider_name, content, hash, snapshot, true, None)
}

pub fn save_with_meta(
    db: &Database,
    date: &str,
    provider_name: &str,
    content: &str,
    hash: &str,
    snapshot: &BriefSnapshot,
    ai_used: bool,
    warning: Option<&str>,
) -> DbResult<crate::db::brief::DailyBrief> {
    crate::db::brief::BriefRepo::new(db.conn()).insert_with_snapshot(
        date,
        Some(provider_name),
        content,
        Some(hash),
        Some(snapshot.period_start),
        Some(snapshot.period_end),
        Some(&snapshot.locale),
        Some(&serde_json::to_string(snapshot).unwrap_or_default()),
        ai_used,
        warning,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::activity::ActivityRepo;
    use crate::db::calendar::CalendarRepo;
    use crate::db::inbox::InboxRepo;
    use crate::db::task::{TaskRepo, WaitingRepo};
    use crate::db::work::{ResumePointRepo, WorkRepo};
    use crate::db::workspace::WorkspaceRepo;

    #[test]
    fn snapshot_contains_all_sources_and_excludes_file_body() {
        let db = Database::open_in_memory().unwrap();
        let now = crate::db::now_unix();
        let work = WorkRepo::new(db.conn())
            .insert("方案工作", "active")
            .unwrap();
        ResumePointRepo::new(db.conn())
            .insert(work.id, "已完成访谈", "整理结论", "记得回访", "manual")
            .unwrap();
        TaskRepo::new(db.conn())
            .insert(Some(work.id), "无截止任务", "high", None, None)
            .unwrap();
        let done = TaskRepo::new(db.conn())
            .insert(Some(work.id), "已完成任务", "normal", Some(now), None)
            .unwrap();
        TaskRepo::new(db.conn()).complete(done.id).unwrap();
        WaitingRepo::new(db.conn())
            .insert(Some(work.id), "等待确认", "王老师", Some(now), None)
            .unwrap();
        CalendarRepo::new(db.conn())
            .insert(
                Some(work.id),
                "今天会议",
                now,
                Some(now + 3600),
                false,
                "meeting",
                None,
                None,
            )
            .unwrap();
        InboxRepo::new(db.conn()).insert("收件箱事项").unwrap();
        let root = std::env::temp_dir().join(format!("msl-brief-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("change.md"), "不得进入快照的文件正文").unwrap();
        let ws = WorkspaceRepo::new(db.conn())
            .insert("工作目录", root.to_string_lossy().as_ref())
            .unwrap();
        let event = ActivityRepo::new(db.conn())
            .insert(
                "file.modified",
                Some(ws.id),
                None,
                Some("file"),
                None,
                Some(root.join("change.md").to_string_lossy().as_ref()),
                "修改文件",
                None,
                None,
            )
            .unwrap();
        let snapshot = build_snapshot(
            &db,
            "2026-08-14",
            now - 86400,
            now + 86400,
            now - 60,
            now + 86400,
            "zh-CN",
        )
        .unwrap();
        assert!(snapshot.source_counts.works >= 1);
        assert!(snapshot.source_counts.resume_points >= 1);
        assert!(snapshot.source_counts.tasks_open >= 1);
        assert!(snapshot.source_counts.tasks_completed >= 1);
        assert!(snapshot.source_counts.waiting >= 1);
        assert!(snapshot.source_counts.calendar >= 1);
        assert!(snapshot.source_counts.inbox >= 1);
        assert!(snapshot.source_counts.file_changes >= 1);
        assert!(snapshot
            .activity
            .iter()
            .any(|f| f.entity_id == Some(event.id)));
        let serialized = serde_json::to_string(&snapshot).unwrap();
        assert!(!serialized.contains("不得进入快照的文件正文"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn local_renderer_is_non_empty_bilingual_and_deterministically_prioritized() {
        let db = Database::open_in_memory().unwrap();
        let now = crate::db::now_unix();
        let work = WorkRepo::new(db.conn()).insert("工作 A", "active").unwrap();
        TaskRepo::new(db.conn())
            .insert(Some(work.id), "高优任务", "high", Some(now), None)
            .unwrap();
        CalendarRepo::new(db.conn())
            .insert(
                Some(work.id),
                "硬安排",
                now + 60,
                None,
                true,
                "meeting",
                None,
                None,
            )
            .unwrap();
        let snapshot = build_snapshot(
            &db,
            "2026-08-14",
            now - 60,
            now + 3600,
            now,
            now + 3600,
            "zh-CN",
        )
        .unwrap();
        let text = render_local(&snapshot);
        assert!(!text.trim().is_empty());
        assert!(text.contains("高优任务"));
        let suggested = text.lines().find(|line| line.contains("建议推进")).unwrap();
        assert!(suggested.find("任务：").unwrap() < suggested.find("日程：").unwrap());
        assert!(text.lines().all(|line| line.starts_with("• ")));
        for section in [
            "过去进展",
            "当前推进",
            "今日日程",
            "待办与逾期",
            "等待与阻塞",
            "工作目录变化",
            "建议推进",
        ] {
            assert!(text.contains(section), "missing brief section: {section}");
        }
        let mut empty = snapshot.clone();
        empty.activity.clear();
        empty.works.clear();
        empty.resume_points.clear();
        empty.tasks_open.clear();
        empty.tasks_completed.clear();
        empty.waiting.clear();
        empty.calendar.clear();
        empty.inbox.clear();
        empty.file_changes.clear();
        empty.source_counts = SourceCounts {
            works: 0,
            resume_points: 0,
            tasks_open: 0,
            tasks_completed: 0,
            waiting: 0,
            calendar: 0,
            inbox: 0,
            file_changes: 0,
            activity: 0,
        };
        assert!(render_local(&empty).contains("当前范围没有记录"));
        empty.locale = "en-US".into();
        assert!(render_local(&empty).contains("No records"));
    }

    #[test]
    fn brief_ai_request_uses_the_routed_model_and_protocol_neutral_shape() {
        let db = Database::open_in_memory().unwrap();
        let snapshot = build_snapshot(&db, "2026-08-20", 0, 100, 0, 100, "zh-CN").unwrap();
        let request = build_ai_request("gpt-5.6-luna", &snapshot);
        assert_eq!(request.model_id, "gpt-5.6-luna");
        assert!(request
            .system
            .as_deref()
            .unwrap()
            .contains("management brief"));
        assert!(request
            .system
            .as_deref()
            .unwrap()
            .contains("Every non-empty line must start with the bullet character"));
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.messages[0].role, "user");
        assert!(request.max_output_tokens.unwrap_or_default() >= 1_600);
    }

    #[test]
    fn provider_summary_is_normalized_to_visible_bullets() {
        assert_eq!(
            normalize_bullet_output("进展完成\n- 等待反馈\n• 明日推进", "zh-CN"),
            "• 进展完成\n• 等待反馈\n• 明日推进"
        );
    }

    #[test]
    fn provider_summary_removes_internal_source_markers_and_limits_home_bullets() {
        let output = [
            "• 完成资料整理（source_type=activity，entity_id=629）。",
            "• 新增结构化文档 ([source_type=file_change], workspace_id=2)。",
            "• 推进项目 A",
            "• 推进项目 B",
            "• 推进项目 C",
            "• 推进项目 D",
            "• 推进项目 E",
            "• 第八项不会出现在首页",
        ]
        .join("\n");
        let normalized = normalize_bullet_output(&output, "zh-CN");
        assert_eq!(normalized.lines().count(), 7);
        for marker in ["source_type", "entity_id", "workspace_id", "[source_"] {
            assert!(!normalized.contains(marker), "leaked marker: {marker}");
        }
        assert!(normalized.contains("完成资料整理"));
        assert!(normalized.contains("新增结构化文档"));
    }
}
