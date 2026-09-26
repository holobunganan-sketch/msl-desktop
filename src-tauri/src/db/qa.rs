use super::{
    knowledge::{self, EvidencePack},
    Database, DbError, DbResult,
};
use rusqlite::{params, OptionalExtension};
use serde_json::Value;

pub fn sessions(db: &Database) -> DbResult<Vec<Value>> {
    knowledge::rows(
        db.conn(),
        "SELECT * FROM qa_sessions ORDER BY updated_at DESC,id DESC",
        &[],
    )
}
pub fn create(db: &Database, title: &str, scope: &[i64]) -> DbResult<Value> {
    create_scoped(db, title, scope, None)
}
pub fn create_scoped(
    db: &Database,
    title: &str,
    scope: &[i64],
    expert: Option<i64>,
) -> DbResult<Value> {
    let (scope, label) = knowledge::expert_scope(db.conn(), scope, expert)?;
    let title = crate::cognition::bounded(title.trim(), 120);
    db.conn().execute(
        "INSERT INTO qa_sessions(title,scope_json,created_at,updated_at,expert_id,expert_scoped,expert_label) VALUES(?1,?2,?3,?3,?4,?5,?6)",
        params![
            if title.is_empty() {
                "新会话"
            } else {
                &title
            },
            serde_json::to_string(&scope).unwrap(),
            super::now_unix(),expert,expert.is_some(),label
        ],
    )?;
    Ok(knowledge::rows(
        db.conn(),
        "SELECT * FROM qa_sessions WHERE id=?1",
        &[&db.conn().last_insert_rowid()],
    )?
    .remove(0))
}
pub fn turns(db: &Database, session_id: i64) -> DbResult<Vec<Value>> {
    let mut rows = knowledge::rows(
        db.conn(),
        "SELECT * FROM qa_turns WHERE session_id=?1 ORDER BY id",
        &[&session_id],
    )?;
    for row in &mut rows {
        hydrate_evidence(db, row)?;
        if let Some(id) = row["id"].as_i64() {
            if let Some(stored) =
                super::ai_documents::get_document(db.conn(), "qa_turn", &id.to_string())?
            {
                row["document"] = serde_json::to_value(stored.document)
                    .map_err(|e| DbError::Migration(e.to_string()))?;
            }
        }
    }
    Ok(rows)
}
pub fn get(db: &Database, id: i64) -> DbResult<Value> {
    let mut row = knowledge::rows(db.conn(), "SELECT * FROM qa_turns WHERE id=?1", &[&id])?
        .into_iter()
        .next()
        .ok_or_else(|| DbError::NotFound("问答轮次".into()))?;
    hydrate_evidence(db, &mut row)?;
    Ok(row)
}
fn hydrate_evidence(db: &Database, row: &mut Value) -> DbResult<()> {
    if let Some(raw) = row["evidence_json"].as_str() {
        if let Ok(mut pack) = serde_json::from_str::<EvidencePack>(raw) {
            knowledge::attach_locations(db, &mut pack)?;
            row["evidence_json"] = Value::String(serde_json::to_string(&pack).unwrap());
        }
    }
    Ok(())
}
pub fn queue(db: &Database, session_id: i64, question: &str, scope: &[i64]) -> DbResult<Value> {
    queue_scoped(db, session_id, question, scope, None)
}
pub fn queue_scoped(
    db: &Database,
    session_id: i64,
    question: &str,
    scope: &[i64],
    expert: Option<i64>,
) -> DbResult<Value> {
    if question.trim().is_empty() || question.chars().count() > 6000 {
        return Err(DbError::Migration("请输入问题（最多 6000 字）".into()));
    }
    let tx = super::write_transaction(db.conn())?;
    let session = knowledge::rows(&tx, "SELECT * FROM qa_sessions WHERE id=?1", &[&session_id])?
        .into_iter()
        .next()
        .ok_or_else(|| DbError::NotFound("会话".into()))?;
    if session["expert_id"].as_i64() != expert
        || (session["expert_scoped"] == 1 && expert.is_none())
    {
        return Err(DbError::Migration(
            "专家范围与会话不一致或已失效，请选择正确会话".into(),
        ));
    }
    let (ids, label) = knowledge::expert_scope(&tx, scope, expert)?;
    let encoded = serde_json::to_string(&ids).unwrap();
    if tx.query_row("SELECT EXISTS(SELECT 1 FROM qa_turns WHERE session_id=?1 AND status IN ('pending','running'))",[session_id],|r|r.get::<_,bool>(0))? {return Err(DbError::Migration("当前会话还有一轮处理中，请等待或重试该轮".into()));}
    tx.execute(
        "INSERT INTO qa_turns(session_id,question,scope_json,created_at,expert_id,expert_scoped,expert_label) VALUES(?1,?2,?3,?4,?5,?6,?7)",
        params![session_id, question.trim(), encoded, super::now_unix(),expert,expert.is_some(),label],
    )?;
    let id = tx.last_insert_rowid();
    tx.execute("UPDATE qa_sessions SET scope_json=?1,updated_at=?2,title=CASE WHEN (SELECT COUNT(*) FROM qa_turns WHERE session_id=?3)=1 THEN ?4 ELSE title END WHERE id=?3",params![encoded,super::now_unix(),session_id,crate::cognition::bounded(question.trim(),60)])?;
    tx.commit()?;
    get(db, id)
}
pub fn claim(db: &Database, id: i64) -> DbResult<Value> {
    let turn = get(db, id)?;
    if turn["expert_scoped"] == 1 {
        knowledge::expert_scope(
            db.conn(),
            &[],
            Some(
                turn["expert_id"]
                    .as_i64()
                    .ok_or_else(|| DbError::NotFound("专家范围已删除，无法继续该轮".into()))?,
            ),
        )?;
    }
    let n=db.conn().execute("UPDATE qa_turns SET status='running',error=NULL WHERE id=?1 AND status IN ('pending','failed','interrupted')",[id])?;
    if n != 1 {
        return Err(DbError::Migration("该轮正在处理或已经完成".into()));
    }
    get(db, id)
}
pub fn history(db: &Database, turn: &Value) -> DbResult<Vec<Value>> {
    if turn["expert_scoped"] == 1 && turn["expert_id"].is_null() {
        return Ok(Vec::new());
    }
    let mut history=knowledge::rows(db.conn(),"SELECT question,answer_json,created_at FROM qa_turns WHERE NOT EXISTS(SELECT 1 FROM json_each(qa_turns.evidence_json,'$.sources') s WHERE json_extract(s.value,'$.trust')='deleted') AND session_id=?1 AND id<?2 AND scope_json=?3 AND expert_id IS ?4 AND expert_scoped=?5 AND status='completed' ORDER BY id DESC LIMIT 10",&[&turn["session_id"].as_i64(),&turn["id"].as_i64(),&turn["scope_json"].as_str(),&turn["expert_id"].as_i64(),&turn["expert_scoped"].as_i64().unwrap_or(0)])?;
    history.reverse();
    for row in &mut history {
        for key in ["question", "answer_json"] {
            if let Some(s) = row[key].as_str() {
                row[key] = Value::String(crate::cognition::bounded(s, 2400));
            }
        }
    }
    Ok(history)
}
pub fn save_evidence(db: &Database, id: i64, pack: &EvidencePack) -> DbResult<()> {
    let tx = super::write_transaction(db.conn())?;
    ensure_expert_live(&tx, id)?;
    super::source_lifecycle::ensure_live(&tx, pack)?;
    tx.execute(
        "UPDATE qa_turns SET evidence_json=?1 WHERE id=?2 AND status='running'",
        params![serde_json::to_string(pack).unwrap(), id],
    )?;
    tx.commit()?;
    Ok(())
}
pub fn finish(
    db: &Database,
    id: i64,
    result: Result<&crate::ai::knowledge_contract::Answer, &str>,
) -> DbResult<()> {
    let tx = super::write_transaction(db.conn())?;
    let (status, answer, error) = match result {
        Ok(value) => {
            ensure_expert_live(&tx, id)?;
            let question: Option<String> = tx
                .query_row(
                    "SELECT question FROM qa_turns WHERE id=?1 AND status='running'",
                    [id],
                    |row| row.get(0),
                )
                .optional()?;
            let Some(question) = question else {
                tx.commit()?;
                return Ok(());
            };
            let document = value.to_document(&question);
            crate::ai::output::parse_document(&serde_json::to_string(&document).unwrap())
                .map_err(|error| DbError::Migration(error.to_string()))?;
            crate::db::ai_documents::save_document(
                &tx,
                "qa_turn",
                &id.to_string(),
                &document,
                "",
                crate::ai::prompts::PROMPT_VERSION,
                "configured workbench model",
            )?;
            (
                "completed",
                Some(serde_json::to_string(value).unwrap()),
                None,
            )
        }
        Err(message) => (
            "failed",
            None,
            Some(crate::cognition::bounded(message, 1000)),
        ),
    };
    tx.execute("UPDATE qa_turns SET status=?1,answer_json=?2,error=?3,finished_at=?4 WHERE id=?5 AND status='running'",params![status,answer,error,super::now_unix(),id])?;
    tx.commit()?;
    Ok(())
}
fn ensure_expert_live(conn: &rusqlite::Connection, id: i64) -> DbResult<()> {
    if conn.query_row("SELECT EXISTS(SELECT 1 FROM qa_turns t WHERE t.id=?1 AND t.expert_scoped=1 AND (t.expert_id IS NULL OR NOT EXISTS(SELECT 1 FROM kol_experts e WHERE e.id=t.expert_id)))",[id],|r|r.get::<_,bool>(0))? {return Err(DbError::NotFound("专家范围已删除，不能继续该轮".into()));}
    Ok(())
}
pub fn remove(db: &Database, id: i64) -> DbResult<()> {
    let tx = super::write_transaction(db.conn())?;
    if tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM qa_turns WHERE session_id=?1 AND status='running')",
        [id],
        |r| r.get::<_, bool>(0),
    )? {
        return Err(DbError::Migration("会话仍在处理，完成后可删除".into()));
    }
    tx.execute(
        "DELETE FROM ai_readable_documents WHERE owner_kind='qa_turn' AND owner_id IN (SELECT CAST(id AS TEXT) FROM qa_turns WHERE session_id=?1)",
        [id],
    )?;
    tx.execute("DELETE FROM qa_sessions WHERE id=?1", [id])?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn deleted_expert_cannot_land_a_late_answer_before_evidence_save() {
        let db = Database::open_in_memory().unwrap();
        db.conn().execute("INSERT INTO kol_experts(name,institution,created_at,updated_at) VALUES('Expert','Clinic',1,1)",[]).unwrap();
        let s = create_scoped(&db, "Scope", &[], Some(1)).unwrap();
        let t = queue_scoped(&db, s["id"].as_i64().unwrap(), "Question", &[], Some(1)).unwrap();
        let id = t["id"].as_i64().unwrap();
        claim(&db, id).unwrap();
        db.conn()
            .execute("DELETE FROM kol_experts WHERE id=1", [])
            .unwrap();
        let answer = crate::ai::knowledge_contract::Answer {
            document: None,
            claims: vec![],
            gaps: vec!["No evidence".into()],
        };
        assert!(finish(&db, id, Ok(&answer)).is_err());
        assert_ne!(get(&db, id).unwrap()["status"], "completed");
    }
    #[test]
    fn expert_scope_unions_links_and_expert_only_excludes_unrelated_work() {
        let db = Database::open_in_memory().unwrap();
        db.conn().execute_batch("INSERT INTO kol_experts(id,name,institution,created_at,updated_at) VALUES(1,'Expert','Clinic',1,1); INSERT INTO works(id,title,created_at,updated_at) VALUES(1,'Linked',1,1),(2,'Unrelated',1,1);").unwrap();
        let pack = knowledge::collect_scoped(&db, &[], "", Some(1), true).unwrap();
        assert!(!pack.sources.iter().any(|s| s.kind == "work"));
        assert!(pack.sources.iter().any(|s| s.kind == "expert"));
        db.conn()
            .execute(
                "INSERT INTO kol_projects(expert_id,work_id) VALUES(1,1)",
                [],
            )
            .unwrap();
        let s = create_scoped(&db, "Linked", &[], Some(1)).unwrap();
        assert_eq!(s["scope_json"], "[1]");
        let pack = knowledge::collect_scoped(&db, &[], "", Some(1), true).unwrap();
        assert_eq!(pack.scope_ids, vec![1]);
        assert!(!pack
            .sources
            .iter()
            .any(|s| s.kind == "work" && s.entity_id == 2));
    }
    #[test]
    fn explicit_expert_scope_survives_empty_links_and_blocks_deleted_scope() {
        let db = Database::open_in_memory().unwrap();
        db.conn().execute("INSERT INTO kol_experts(name,institution,created_at,updated_at) VALUES('Expert','Clinic',1,1)",[]).unwrap();
        let s = create_scoped(&db, "Expert only", &[], Some(1)).unwrap();
        assert_eq!(s["expert_scoped"], 1);
        assert_eq!(s["expert_id"], 1);
        assert!(queue_scoped(&db, s["id"].as_i64().unwrap(), "Question", &[], None).is_err());
        db.conn()
            .execute("DELETE FROM kol_experts WHERE id=1", [])
            .unwrap();
        assert!(queue_scoped(&db, s["id"].as_i64().unwrap(), "Question", &[], None).is_err());
        assert_eq!(sessions(&db).unwrap()[0]["expert_scoped"], 1);
    }
    #[test]
    fn knowledge_qa_persists_scope_and_serializes_turns() {
        let db = Database::open_in_memory().unwrap();
        db.conn().execute_batch("INSERT INTO works(id,title,created_at,updated_at) VALUES(1,'A',1,1),(2,'B',1,1); INSERT INTO qa_sessions(id,title,created_at,updated_at) VALUES(1,'测试会话',1,1);").unwrap();
        let turn = queue(&db, 1, "当前推进到了哪里？", &[1]).unwrap();
        let id = turn["id"].as_i64().unwrap();
        assert_eq!(turn["scope_json"], "[1]");
        assert!(queue(&db, 1, "连续点击", &[1]).is_err());
        assert!(claim(&db, id).is_ok());
        assert!(claim(&db, id).is_err());
        db.conn()
            .execute(
                "UPDATE qa_turns SET status='completed',answer_json=?1 WHERE id=?2",
                params![json!({"claims":[],"gaps":["待补充"]}).to_string(), id],
            )
            .unwrap();
        let next = queue(&db, 1, "其中有哪些等待？", &[1]).unwrap();
        assert_eq!(history(&db, &next).unwrap().len(), 1);
        db.conn()
            .execute(
                "UPDATE qa_turns SET status='failed' WHERE id=?1",
                [next["id"].as_i64().unwrap()],
            )
            .unwrap();
        let other = queue(&db, 1, "换一个项目", &[2]).unwrap();
        assert!(history(&db, &other).unwrap().is_empty());
    }
    #[test]
    fn knowledge_qa_invalid_scope_never_saves_an_unscoped_turn() {
        let db = Database::open_in_memory().unwrap();
        db.conn()
            .execute(
                "INSERT INTO qa_sessions(id,title,created_at,updated_at) VALUES(1,'test',1,1)",
                [],
            )
            .unwrap();
        assert!(queue(&db, 1, "question", &[999]).is_err());
        let n: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM qa_turns", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0);
    }
}
