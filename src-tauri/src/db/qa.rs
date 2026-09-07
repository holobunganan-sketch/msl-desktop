use super::{
    knowledge::{self, EvidencePack},
    Database, DbError, DbResult,
};
use rusqlite::params;
use serde_json::Value;

pub fn sessions(db: &Database) -> DbResult<Vec<Value>> {
    knowledge::rows(
        db.conn(),
        "SELECT * FROM qa_sessions ORDER BY updated_at DESC,id DESC",
        &[],
    )
}
pub fn create(db: &Database, title: &str, scope: &[i64]) -> DbResult<Value> {
    let scope = knowledge::validate_scope(db.conn(), scope)?;
    let title = crate::cognition::bounded(title.trim(), 120);
    db.conn().execute(
        "INSERT INTO qa_sessions(title,scope_json,created_at,updated_at) VALUES(?1,?2,?3,?3)",
        params![
            if title.is_empty() {
                "新会话"
            } else {
                &title
            },
            serde_json::to_string(&scope).unwrap(),
            super::now_unix()
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
    knowledge::rows(
        db.conn(),
        "SELECT * FROM qa_turns WHERE session_id=?1 ORDER BY id",
        &[&session_id],
    )
}
pub fn get(db: &Database, id: i64) -> DbResult<Value> {
    knowledge::rows(db.conn(), "SELECT * FROM qa_turns WHERE id=?1", &[&id])?
        .into_iter()
        .next()
        .ok_or_else(|| DbError::NotFound("问答轮次".into()))
}
pub fn queue(db: &Database, session_id: i64, question: &str, scope: &[i64]) -> DbResult<Value> {
    if question.trim().is_empty() || question.chars().count() > 6000 {
        return Err(DbError::Migration("请输入问题（最多 6000 字）".into()));
    }
    let tx = super::write_transaction(db.conn())?;
    let ids = knowledge::validate_scope(&tx, scope)?;
    let encoded = serde_json::to_string(&ids).unwrap();
    if tx.query_row("SELECT EXISTS(SELECT 1 FROM qa_turns WHERE session_id=?1 AND status IN ('pending','running'))",[session_id],|r|r.get::<_,bool>(0))? {return Err(DbError::Migration("当前会话还有一轮处理中，请等待或重试该轮".into()));}
    tx.execute(
        "INSERT INTO qa_turns(session_id,question,scope_json,created_at) VALUES(?1,?2,?3,?4)",
        params![session_id, question.trim(), encoded, super::now_unix()],
    )?;
    let id = tx.last_insert_rowid();
    tx.execute("UPDATE qa_sessions SET scope_json=?1,updated_at=?2,title=CASE WHEN (SELECT COUNT(*) FROM qa_turns WHERE session_id=?3)=1 THEN ?4 ELSE title END WHERE id=?3",params![encoded,super::now_unix(),session_id,crate::cognition::bounded(question.trim(),60)])?;
    tx.commit()?;
    get(db, id)
}
pub fn claim(db: &Database, id: i64) -> DbResult<Value> {
    let n=db.conn().execute("UPDATE qa_turns SET status='running',error=NULL WHERE id=?1 AND status IN ('pending','failed','interrupted')",[id])?;
    if n != 1 {
        return Err(DbError::Migration("该轮正在处理或已经完成".into()));
    }
    get(db, id)
}
pub fn history(db: &Database, turn: &Value) -> DbResult<Vec<Value>> {
    let mut history=knowledge::rows(db.conn(),"SELECT question,answer_json,created_at FROM qa_turns WHERE session_id=?1 AND id<?2 AND scope_json=?3 AND status='completed' ORDER BY id DESC LIMIT 10",&[&turn["session_id"].as_i64(),&turn["id"].as_i64(),&turn["scope_json"].as_str()])?;
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
    db.conn().execute(
        "UPDATE qa_turns SET evidence_json=?1 WHERE id=?2 AND status='running'",
        params![serde_json::to_string(pack).unwrap(), id],
    )?;
    Ok(())
}
pub fn finish(
    db: &Database,
    id: i64,
    result: Result<&crate::ai::knowledge_contract::Answer, &str>,
) -> DbResult<()> {
    let (status, answer, error) = match result {
        Ok(a) => ("completed", Some(serde_json::to_string(a).unwrap()), None),
        Err(e) => ("failed", None, Some(crate::cognition::bounded(e, 1000))),
    };
    db.conn().execute("UPDATE qa_turns SET status=?1,answer_json=?2,error=?3,finished_at=?4 WHERE id=?5 AND status='running'",params![status,answer,error,super::now_unix(),id])?;
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
    tx.execute("DELETE FROM qa_sessions WHERE id=?1", [id])?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
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
