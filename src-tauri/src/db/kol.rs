use super::{
    knowledge::{self, EvidencePack},
    Database, DbError, DbResult,
};
use crate::ai::knowledge_contract::KolOutput;
use rusqlite::params;
use serde_json::{json, Value};

pub fn experts(db: &Database) -> DbResult<Vec<Value>> {
    knowledge::rows(db.conn(),"SELECT e.*,(SELECT COUNT(*) FROM kol_notes n WHERE n.expert_id=e.id) AS note_count,(SELECT MAX(occurred_at) FROM kol_notes n WHERE n.expert_id=e.id) AS last_contact,(SELECT json_group_array(work_id) FROM kol_projects p WHERE p.expert_id=e.id) AS project_ids_json FROM kol_experts e ORDER BY archived,name,id",&[])
}
pub fn save_expert(
    db: &Database,
    id: Option<i64>,
    revision: Option<i64>,
    name: &str,
    institution: &str,
    department: Option<&str>,
    specialty: &str,
    projects: &[i64],
    archived: bool,
) -> DbResult<Value> {
    if name.trim().is_empty()
        || institution.trim().is_empty()
        || name.chars().count() > 100
        || institution.chars().count() > 200
        || department.is_some_and(|value| value.chars().count() > 200)
        || specialty.chars().count() > 400
    {
        return Err(DbError::Migration(
            "请填写专家姓名和机构；科室可留空，各字段请勿超过长度限制".into(),
        ));
    }
    let tx = super::write_transaction(db.conn())?;
    let projects = knowledge::validate_scope(&tx, projects)?;
    let now = super::now_unix();
    let id = if let Some(id) = id {
        if tx.execute("UPDATE kol_experts SET name=?1,institution=?2,specialty=?3,archived=?4,updated_at=?5,revision=revision+1,department=COALESCE(?8,department) WHERE id=?6 AND revision=?7",params![name.trim(),institution.trim(),specialty.trim(),archived,now,id,revision,department.map(str::trim)])?!=1{return Err(DbError::Migration("专家资料已更新，请刷新后再保存".into()));}
        id
    } else {
        tx.execute("INSERT INTO kol_experts(name,institution,specialty,created_at,updated_at,department) VALUES(?1,?2,?3,?4,?4,?5)",params![name.trim(),institution.trim(),specialty.trim(),now,department.unwrap_or("").trim()])?;
        tx.last_insert_rowid()
    };
    tx.execute("DELETE FROM kol_projects WHERE expert_id=?1", [id])?;
    for work in projects {
        tx.execute(
            "INSERT INTO kol_projects(expert_id,work_id) VALUES(?1,?2)",
            params![id, work],
        )?;
    }
    tx.commit()?;
    experts(db)?
        .into_iter()
        .find(|e| e["id"] == id)
        .ok_or_else(|| DbError::NotFound("专家".into()))
}
pub fn notes(db: &Database, expert_id: Option<i64>) -> DbResult<Vec<Value>> {
    knowledge::rows(db.conn(),"SELECT n.*,e.name,e.institution,e.department,w.title AS project_title FROM kol_notes n JOIN kol_experts e ON e.id=n.expert_id LEFT JOIN works w ON w.id=n.work_id WHERE ?1 IS NULL OR n.expert_id=?1 ORDER BY occurred_at DESC,n.id DESC",&[&expert_id])
}
pub fn capture(
    db: &Database,
    expert_id: i64,
    work_id: Option<i64>,
    inbox_id: Option<i64>,
    content: &str,
    at: i64,
) -> DbResult<Value> {
    if content.trim().is_empty()
        || content.chars().count() > 20_000
        || !(946684800..4102444800).contains(&at)
    {
        return Err(DbError::Migration(
            "请检查交流内容（最多两万字）和日期".into(),
        ));
    }
    let tx = super::write_transaction(db.conn())?;
    super::work::validate_project_scope(&tx, "task", work_id)?;
    if !tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM kol_experts WHERE id=?1 AND archived=0)",
        [expert_id],
        |r| r.get::<_, bool>(0),
    )? {
        return Err(DbError::NotFound("专家已归档或不存在".into()));
    }
    let original = if let Some(inbox) = inbox_id {
        let prior = knowledge::rows(&tx, "SELECT * FROM kol_notes WHERE inbox_id=?1", &[&inbox])?;
        if let Some(note) = prior.into_iter().next() {
            if note["expert_id"] != expert_id {
                return Err(DbError::Migration(
                    "该收件箱记录已关联其他专家，请先核对身份".into(),
                ));
            }
            return Ok(note);
        }
        tx.query_row(
            "SELECT content FROM inbox_items WHERE id=?1",
            [inbox],
            |r| r.get::<_, String>(0),
        )?
    } else {
        content.trim().into()
    };
    tx.execute("INSERT INTO kol_notes(expert_id,work_id,inbox_id,content,occurred_at,created_at) VALUES(?1,?2,?3,?4,?5,?6)",params![expert_id,work_id,inbox_id,original,at,super::now_unix()])?;
    let id = tx.last_insert_rowid();
    if let Some(work) = work_id {
        tx.execute(
            "INSERT OR IGNORE INTO kol_projects(expert_id,work_id) VALUES(?1,?2)",
            params![expert_id, work],
        )?;
    }
    super::activity::ActivityRepo::new(&tx).insert(
        "kol.note_recorded",
        None,
        work_id,
        Some("kol_note"),
        Some(id),
        None,
        "保存专家交流原话",
        None,
        None,
    )?;
    tx.commit()?;
    Ok(knowledge::rows(db.conn(), "SELECT * FROM kol_notes WHERE id=?1", &[&id])?.remove(0))
}
pub fn drafts(db: &Database, expert_id: Option<i64>) -> DbResult<Vec<Value>> {
    knowledge::rows(db.conn(),"SELECT d.*,e.name FROM kol_drafts d LEFT JOIN kol_experts e ON e.id=d.expert_id WHERE ?1 IS NULL OR d.expert_id=?1 ORDER BY (d.status='pending') DESC,d.id DESC",&[&expert_id])
}
pub fn insights(db: &Database, expert_id: Option<i64>) -> DbResult<Vec<Value>> {
    knowledge::rows(db.conn(),"SELECT i.*,e.name FROM kol_insights i LEFT JOIN kol_experts e ON e.id=i.expert_id WHERE ?1 IS NULL OR i.expert_id=?1 ORDER BY i.updated_at DESC,i.id DESC",&[&expert_id])
}
pub fn secretary_context(db: &Database, workspace_filter: Option<&[i64]>) -> DbResult<Vec<Value>> {
    let folders = workspace_filter.map(|ids| serde_json::to_string(ids).unwrap());
    knowledge::rows(db.conn(),"SELECT DISTINCT n.id,n.expert_id,e.name,e.institution,e.department,n.work_id,n.content,n.occurred_at FROM kol_notes n JOIN kol_experts e ON e.id=n.expert_id WHERE EXISTS(SELECT 1 FROM kol_insights i,json_each(i.citations_json) c WHERE json_extract(c.value,'$.source_id')='kol_note:'||n.id AND i.status!='dismissed') AND (?1 IS NULL OR n.work_id IN(SELECT work_id FROM work_workspace_links WHERE workspace_id IN(SELECT value FROM json_each(?1)))) ORDER BY n.occurred_at DESC,n.id DESC LIMIT 81",&[&folders])
}
pub fn set_insight_status(db: &Database, id: i64, status: &str, note: &str) -> DbResult<()> {
    if !["hypothesis", "reviewed", "revised", "dismissed"].contains(&status)
        || note.chars().count() > 2000
    {
        return Err(DbError::Migration("洞察审阅状态无效".into()));
    }
    if db.conn().execute(
        "UPDATE kol_insights SET status=?1,review_note=?2,updated_at=?3 WHERE id=?4",
        params![status, note, super::now_unix(), id],
    )? != 1
    {
        return Err(DbError::NotFound("洞察".into()));
    }
    Ok(())
}
pub fn followups(db: &Database, expert_id: Option<i64>) -> DbResult<Vec<Value>> {
    knowledge::rows(db.conn(),"SELECT a.id,a.expert_id,COALESCE(e.name,'跨专家跟进') AS name,a.entity_kind,a.entity_id,t.title,t.status,t.work_id,t.scheduled_start AS at,t.notes FROM kol_actions a LEFT JOIN kol_experts e ON e.id=a.expert_id JOIN tasks t ON a.entity_kind='task' AND a.entity_id=t.id WHERE (?1 IS NULL OR a.expert_id=?1) AND t.status!='done' UNION ALL SELECT a.id,a.expert_id,COALESCE(e.name,'跨专家跟进'),a.entity_kind,a.entity_id,w.title,w.status,w.work_id,w.follow_up_at AS at,w.notes FROM kol_actions a LEFT JOIN kol_experts e ON e.id=a.expert_id JOIN waiting_items w ON a.entity_kind='waiting' AND a.entity_id=w.id WHERE (?1 IS NULL OR a.expert_id=?1) AND w.status='open' UNION ALL SELECT a.id,a.expert_id,COALESCE(e.name,'跨专家跟进'),a.entity_kind,a.entity_id,c.title,'scheduled',c.work_id,c.start_at AS at,c.notes FROM kol_actions a LEFT JOIN kol_experts e ON e.id=a.expert_id JOIN calendar_events c ON a.entity_kind='calendar' AND a.entity_id=c.id WHERE (?1 IS NULL OR a.expert_id=?1) AND COALESCE(c.end_at,c.start_at)>=?2 UNION ALL SELECT a.id,a.expert_id,COALESCE(e.name,'跨专家跟进'),a.entity_kind,a.entity_id,i.content,'inbox',NULL,NULL,i.content FROM kol_actions a LEFT JOIN kol_experts e ON e.id=a.expert_id JOIN inbox_items i ON a.entity_kind='inbox' AND a.entity_id=i.id WHERE (?1 IS NULL OR a.expert_id=?1) AND i.processed_at IS NULL ORDER BY at",&[&expert_id,&super::now_unix()])
}
pub fn evidence_pack(db: &Database, expert_id: Option<i64>) -> DbResult<EvidencePack> {
    let all_notes = notes(db, expert_id)?;
    let expert_records = experts(db)?
        .into_iter()
        .filter(|e| expert_id.is_none_or(|id| e["id"] == id))
        .collect::<Vec<_>>();
    if expert_records.is_empty() {
        return Err(DbError::NotFound("请先建立专家档案".into()));
    }
    let actions = followups(db, expert_id)?;
    let insights = insights(db, expert_id)?;
    let mut pack = EvidencePack {
        as_of: super::now_unix(),
        ..Default::default()
    };
    pack.counts.insert("kol_note".into(), all_notes.len());
    pack.counts.insert("expert".into(), expert_records.len());
    for (kind, items, trust) in [
        ("expert", expert_records, "record"),
        ("kol_note", all_notes, "user_record"),
        ("kol_followup", actions, "record"),
        ("kol_insight", insights, "reviewed_hypothesis"),
    ] {
        let count = items.len();
        pack.sources.extend(
            items
                .iter()
                .take(160)
                .map(|r| knowledge::evidence(kind, r, trust, "")),
        );
        pack.omitted += count.saturating_sub(160);
    }
    if !pack.sources.iter().any(|s| s.kind == "kol_note") {
        return Err(DbError::Migration(
            "请先记下一次交流，AI 将根据原话整理".into(),
        ));
    }
    // Every action references a real project catalog entry, not a guessed ID.
    let works = super::work::WorkRepo::new(db.conn()).list(None)?;
    pack.sources.extend(works.iter().map(|w| {
        knowledge::evidence(
            "work",
            &json!({"id":w.id,"title":w.title,"status":w.status}),
            "record",
            "",
        )
    }));
    let mut chars = 0usize;
    pack.sources.retain(|s| {
        let len = s.text.chars().count();
        if chars + len > 75000 {
            pack.omitted += 1;
            false
        } else {
            chars += len;
            true
        }
    });
    pack.notes
        .push("专家观点是交流记录，不代表已证实的医学结论。请按原始交流来源去重。".into());
    Ok(pack)
}
pub fn insert_draft(
    db: &Database,
    expert_id: Option<i64>,
    purpose: &str,
    output: &KolOutput,
    pack: &EvidencePack,
) -> DbResult<i64> {
    if !["organize", "prepare", "synthesize"].contains(&purpose) {
        return Err(DbError::Migration("整理方式无效".into()));
    }
    db.conn().execute("INSERT INTO kol_drafts(expert_id,purpose,payload_json,evidence_json,created_at) VALUES(?1,?2,?3,?4,?5)",params![expert_id,purpose,serde_json::to_string(output).unwrap(),serde_json::to_string(pack).unwrap(),super::now_unix()])?;
    Ok(db.conn().last_insert_rowid())
}
pub fn review(db: &Database, id: i64, revision: i64, decision: &str, raw: &str) -> DbResult<Value> {
    let tx = super::write_transaction(db.conn())?;
    let draft = knowledge::rows(&tx, "SELECT * FROM kol_drafts WHERE id=?1", &[&id])?
        .into_iter()
        .next()
        .ok_or_else(|| DbError::NotFound("专家整理草稿".into()))?;
    if draft["status"] == "confirmed" && decision == "confirm" {
        return Ok(draft);
    }
    if draft["status"] != "pending" || draft["revision"] != revision {
        return Err(DbError::Migration("草稿已更新或处理，请刷新".into()));
    }
    if !["save", "confirm", "reject"].contains(&decision) {
        return Err(DbError::Migration("审阅动作无效".into()));
    }
    let now = super::now_unix();
    if decision == "reject" {
        tx.execute(
            "UPDATE kol_drafts SET status='rejected',decided_at=?1,revision=revision+1 WHERE id=?2",
            params![now, id],
        )?;
    } else {
        let pack: EvidencePack =
            serde_json::from_str(draft["evidence_json"].as_str().unwrap_or(""))
                .map_err(|_| DbError::Migration("草稿来源无法读取".into()))?;
        let output =
            crate::ai::knowledge_contract::parse_kol(raw, &pack).map_err(DbError::Migration)?;
        if draft["purpose"] == "prepare" && !output.actions.is_empty() {
            return Err(DbError::Migration(
                "会前准备不创建承诺；请先移除动作或通过交流整理发起跟进".into(),
            ));
        }
        let expert = draft["expert_id"].as_i64();
        if decision == "confirm" {
            for action in output.actions.iter().filter(|a| a.enabled) {
                super::work::validate_project_scope(&tx, "task", action.work_id)?;
            }
            for insight in &output.insights {
                tx.execute("INSERT INTO kol_insights(draft_id,expert_id,title,categories_json,observation,implication,uncertainty,next_question,citations_json,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?10)",params![id,expert,insight.title,serde_json::to_string(&insight.categories).unwrap(),insight.observation,insight.implication,insight.uncertainty,insight.next_question,serde_json::to_string(&insight.citations).unwrap(),now])?;
            }
            for action in output.actions.iter().filter(|a| a.enabled) {
                let expert = expert.or_else(|| {
                    let ids = action
                        .citations
                        .iter()
                        .filter_map(|c| {
                            pack.sources
                                .iter()
                                .find(|s| s.id == c.source_id && s.kind == "kol_note")
                        })
                        .filter_map(|s| serde_json::from_str::<Value>(&s.text).ok())
                        .filter_map(|v| v["expert_id"].as_i64())
                        .collect::<std::collections::BTreeSet<_>>();
                    if ids.len() == 1 {
                        ids.first().copied()
                    } else {
                        None
                    }
                });
                let details = format!(
                    "{}\n{} {}",
                    action.notes, action.time_basis, action.time_reason
                );
                let entity_id = match action.kind.as_str() {
                    "task" => {
                        let task = super::task::TaskRepo::new(&tx).insert(
                            action.work_id,
                            &action.title,
                            "normal",
                            None,
                            Some(&details),
                        )?;
                        tx.execute(
                            "UPDATE tasks SET scheduled_start=?1 WHERE id=?2",
                            params![action.at, task.id],
                        )?;
                        task.id
                    }
                    "waiting" => {
                        super::task::WaitingRepo::new(&tx)
                            .insert(
                                action.work_id,
                                &action.title,
                                &action.waiting_for,
                                action.at,
                                Some(&details),
                            )?
                            .id
                    }
                    "calendar" => {
                        super::calendar::CalendarRepo::new(&tx)
                            .insert(
                                action.work_id,
                                &action.title,
                                action
                                    .at
                                    .ok_or_else(|| DbError::Migration("日历时间缺失".into()))?,
                                None,
                                false,
                                "kol_visit",
                                None,
                                Some(&details),
                            )?
                            .id
                    }
                    "inbox" => {
                        super::inbox::InboxRepo::new(&tx)
                            .insert(&format!("{}\n{}", action.title, details))?
                            .id
                    }
                    _ => return Err(DbError::Migration("动作类型无效".into())),
                };
                tx.execute("INSERT INTO kol_actions(draft_id,expert_id,entity_kind,entity_id,created_at) VALUES(?1,?2,?3,?4,?5)",params![id,expert,action.kind,entity_id,now])?;
                if let (Some(expert), Some(work)) = (expert, action.work_id) {
                    tx.execute(
                        "INSERT OR IGNORE INTO kol_projects(expert_id,work_id) VALUES(?1,?2)",
                        params![expert, work],
                    )?;
                }
                super::activity::ActivityRepo::new(&tx).insert(
                    "kol.action_confirmed",
                    None,
                    action.work_id,
                    Some(&action.kind),
                    Some(entity_id),
                    None,
                    &format!("专家交流后续：{}", action.title),
                    None,
                    None,
                )?;
            }
            if draft["purpose"] == "organize" {
                tx.execute("UPDATE kol_experts SET summary=?1,revision=revision+1,updated_at=?2 WHERE id=?3",params![output.summary,now,expert])?;
            }
        }
        tx.execute("UPDATE kol_drafts SET payload_json=?1,status=?2,revision=revision+1,decided_at=?3 WHERE id=?4",params![serde_json::to_string(&output).unwrap(),if decision=="confirm"{"confirmed"}else{"pending"},if decision=="confirm"{Some(now)}else{None},id])?;
    }
    tx.commit()?;
    Ok(knowledge::rows(db.conn(), "SELECT * FROM kol_drafts WHERE id=?1", &[&id])?.remove(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn kol_department_migration_preserves_legacy_profile_and_notes() {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY,name TEXT NOT NULL,applied_at INTEGER NOT NULL);").unwrap();
        for migration in super::super::migrations::MIGRATIONS
            .iter()
            .filter(|m| m.version <= 14)
        {
            conn.execute_batch(migration.sql).unwrap();
            conn.execute(
                "INSERT INTO schema_migrations VALUES(?1,?2,1)",
                params![migration.version, migration.name],
            )
            .unwrap();
        }
        conn.execute_batch("INSERT INTO kol_experts(id,name,institution,specialty,created_at,updated_at) VALUES(41,'合成专家','合成医院 / 肾内科','随访证据',1,1); INSERT INTO kol_notes(expert_id,content,occurred_at,created_at) VALUES(41,'合成交流记录',1788888888,1);").unwrap();
        super::super::migrations::run(&mut conn).unwrap();
        let columns: Vec<String> = conn
            .prepare("PRAGMA table_info(kol_experts)")
            .unwrap()
            .query_map([], |r| r.get(1))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(
            columns.iter().any(|name| name == "department"),
            "Expert departments need their own persisted field"
        );
        let saved: (String, String, String) = conn
            .query_row(
                "SELECT institution,department,specialty FROM kol_experts WHERE id=41",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(
            saved,
            ("合成医院 / 肾内科".into(), "".into(), "随访证据".into())
        );
        super::super::migrations::run(&mut conn).unwrap();
        assert_eq!(
            conn.query_row(
                "SELECT content FROM kol_notes WHERE expert_id=41",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "合成交流记录"
        );
    }
    fn db() -> Database {
        let db = Database::open_in_memory().unwrap();
        db.conn().execute_batch("INSERT INTO works(id,title,created_at,updated_at) VALUES(1,'研究项目',1,1); INSERT INTO kol_experts(id,name,institution,created_at,updated_at) VALUES(1,'合成专家','合成机构',1,1);").unwrap();
        db
    }
    #[test]
    fn kol_department_roundtrips_without_overwriting_institution_and_reaches_evidence() {
        let db = db();
        let saved = save_expert(
            &db,
            None,
            None,
            "新增合成专家",
            " 合成教学医院 ",
            Some(" 肾内科 "),
            "临床研究",
            &[1],
            false,
        )
        .unwrap();
        let id = saved["id"].as_i64().unwrap();
        assert_eq!(saved["institution"], "合成教学医院");
        assert_eq!(saved["department"], "肾内科");
        assert_eq!(saved["specialty"], "临床研究");
        capture(&db, id, Some(1), None, "合成证据需求", 1788888888).unwrap();
        assert_eq!(notes(&db, Some(id)).unwrap()[0]["department"], "肾内科");
        for pack in [
            evidence_pack(&db, Some(id)).unwrap(),
            knowledge::collect(&db, &[1], "肾内科").unwrap(),
        ] {
            for kind in ["expert", "kol_note"] {
                assert!(pack.sources.iter().filter(|s| s.kind == kind).any(|s| {
                    let v: Value = serde_json::from_str(&s.text).unwrap();
                    v["department"] == "肾内科" && v["institution"] == "合成教学医院"
                }));
            }
        }
        let revision = saved["revision"].as_i64();
        let updated = save_expert(
            &db,
            Some(id),
            revision,
            "新增合成专家",
            "合成教学医院分院",
            None,
            "临床研究",
            &[1],
            false,
        )
        .unwrap();
        assert_eq!(
            updated["department"], "肾内科",
            "Omitted field from an older caller must preserve department"
        );
        let revision = updated["revision"].as_i64();
        assert!(save_expert(
            &db,
            Some(id),
            revision,
            "新增合成专家",
            "机构",
            Some(&"科".repeat(201)),
            "",
            &[],
            false
        )
        .is_err());
        let cleared = save_expert(
            &db,
            Some(id),
            revision,
            "新增合成专家",
            "合成教学医院分院",
            Some(""),
            "临床研究",
            &[1],
            false,
        )
        .unwrap();
        assert_eq!(cleared["department"], "");
        assert_eq!(cleared["institution"], "合成教学医院分院");
    }
    fn draft(db: &Database) -> (i64, String) {
        let note = capture(db, 1, Some(1), None, "请提供长期随访证据", 1_788_000_000).unwrap();
        let source = knowledge::evidence("kol_note", &note, "user_record", "");
        let pack = EvidencePack {
            sources: vec![source.clone()],
            ..Default::default()
        };
        let citations = json!([{"source_id":source.id,"quote":"请提供长期随访证据"}]);
        let raw=json!({"summary":"补充证据","citations":citations,"insights":[{"title":"证据需求","categories":["evidence_need"],"observation":"请提供长期随访证据","implication":"","uncertainty":"尚未明确结局","next_question":"最关心什么结局？","citations":citations}],"actions":[{"enabled":true,"kind":"task","title":"补充证据","work_id":1,"notes":"待整理","waiting_for":"","at":1_788_888_888,"time_basis":"inferred","time_reason":"拟安排下次交流前准备","citations":citations}]}).to_string();
        db.conn().execute("INSERT INTO kol_drafts(expert_id,purpose,payload_json,evidence_json,created_at) VALUES(1,'organize',?1,?2,1)",params![raw,serde_json::to_string(&pack).unwrap()]).unwrap();
        (db.conn().last_insert_rowid(), raw)
    }
    #[test]
    fn knowledge_kol_original_and_draft_never_write_tasks_until_confirmation() {
        let db = db();
        let (id, raw) = draft(&db);
        assert_eq!(
            db.conn()
                .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
        review(&db, id, 1, "confirm", &raw).unwrap();
        assert_eq!(
            db.conn()
                .query_row(
                    "SELECT COUNT(*) FROM tasks WHERE work_id=1 AND scheduled_start=1788888888",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
            1
        );
        assert_eq!(
            db.conn()
                .query_row("SELECT COUNT(*) FROM calendar_events", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        review(&db, id, 1, "confirm", &raw).unwrap();
        assert_eq!(
            db.conn()
                .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            db.conn()
                .query_row("SELECT content FROM kol_notes", [], |r| r
                    .get::<_, String>(0))
                .unwrap(),
            "请提供长期随访证据"
        );
    }
    #[test]
    fn knowledge_kol_invalid_project_rolls_back_insights_and_all_actions() {
        let db = db();
        let (id, raw) = draft(&db);
        let mut value: Value = serde_json::from_str(&raw).unwrap();
        value["actions"][0]["work_id"] = json!(999);
        assert!(review(&db, id, 1, "confirm", &value.to_string()).is_err());
        assert_eq!(
            db.conn()
                .query_row("SELECT COUNT(*) FROM kol_insights", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    #[test]
    fn knowledge_kol_rejected_and_disabled_actions_keep_only_original() {
        let db = db();
        let (id, raw) = draft(&db);
        review(&db, id, 1, "reject", &raw).unwrap();
        let (id, raw) = draft(&db);
        let mut value: Value = serde_json::from_str(&raw).unwrap();
        value["actions"][0]["enabled"] = json!(false);
        review(&db, id, 1, "confirm", &value.to_string()).unwrap();
        assert_eq!(
            db.conn()
                .query_row("SELECT COUNT(*) FROM tasks", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            db.conn()
                .query_row("SELECT COUNT(*) FROM kol_notes", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            2
        );
    }
    #[test]
    fn knowledge_kol_followups_read_live_task_state() {
        let db = db();
        let (id, raw) = draft(&db);
        review(&db, id, 1, "confirm", &raw).unwrap();
        let items = followups(&db, Some(1)).unwrap();
        assert_eq!(items.len(), 1);
        super::super::task::TaskRepo::new(db.conn())
            .complete(items[0]["entity_id"].as_i64().unwrap())
            .unwrap();
        assert!(followups(&db, Some(1)).unwrap().is_empty());
    }
    #[test]
    fn knowledge_kol_secretary_uses_confirmed_sources_and_preparation_cannot_create_actions() {
        let db = db();
        db.conn()
            .execute("UPDATE kol_experts SET department='肾内科' WHERE id=1", [])
            .unwrap();
        let (id, raw) = draft(&db);
        assert!(secretary_context(&db, None).unwrap().is_empty());
        db.conn()
            .execute("UPDATE kol_drafts SET purpose='prepare' WHERE id=?1", [id])
            .unwrap();
        assert!(review(&db, id, 1, "confirm", &raw).is_err());
        db.conn()
            .execute("UPDATE kol_drafts SET purpose='organize' WHERE id=?1", [id])
            .unwrap();
        review(&db, id, 1, "confirm", &raw).unwrap();
        assert_eq!(secretary_context(&db, None).unwrap().len(), 1);
        assert_eq!(
            secretary_context(&db, None).unwrap()[0]["department"],
            "肾内科"
        );
    }
}
