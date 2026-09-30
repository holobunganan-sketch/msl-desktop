//! Routing a user's capture into expert knowledge. Generation never writes facts;
//! the normal editable proposal queue owns confirmation, audit, and undo.
use super::{analysis_snapshot::AnalysisSnapshot, schema::ProposalContract};
use crate::db::{knowledge, now_unix, Database, DbError, DbResult};
use rusqlite::{params, Connection};
use serde_json::{json, Value};

const FIELDS: &[&str] = &["observation", "implication", "uncertainty", "next_question"];

/// Content token, independent of machine-local primary keys. Citation text and
/// provenance remain protected; references are remapped separately by sync.
pub fn revision(row: &Value) -> String {
    let mut value = row.clone();
    if let Some(object) = value.as_object_mut() {
        for key in ["id", "draft_id", "expert_id", "name", "insight_revision"] {
            object.remove(key);
        }
        if let Some(raw) = object.get("citations_json").and_then(Value::as_str) {
            let mut citations: Value = serde_json::from_str(raw).unwrap_or(Value::Null);
            fn portable(v: &mut Value) {
                match v {
                    Value::Object(map) => {
                        for key in [
                            "source_id",
                            "entity_id",
                            "workspace_id",
                            "material_id",
                            "segment_id",
                            "expert_id",
                            "work_id",
                        ] {
                            map.remove(key);
                        }
                        for child in map.values_mut() {
                            portable(child);
                        }
                    }
                    Value::Array(items) => items.iter_mut().for_each(portable),
                    _ => (),
                }
            }
            portable(&mut citations);
            object.insert("citations_json".into(), citations);
        }
    }
    crate::cognition::digest(&value.to_string())
}

pub fn catalog(db: &Database) -> DbResult<Vec<Value>> {
    let mut experts = knowledge::rows(db.conn(), "SELECT id,name,institution,department,revision FROM kol_experts WHERE archived=0 ORDER BY name,id LIMIT 201", &[])?;
    let closed = knowledge::rows(db.conn(), "SELECT DISTINCT o.target_id FROM ai_proposal_outcomes o JOIN ai_proposals p ON p.id=o.proposal_id WHERE o.kind='kol_insight' AND p.status IN ('resolved','deleted')", &[])?;
    let insights = crate::db::kol::insights(db, None)?
        .into_iter()
        .filter(|i| {
            i["status"] != "dismissed"
                && !closed.iter().any(|c| c["target_id"] == i["id"])
                && crate::db::source_lifecycle::usable_insight(db.conn(), i)
        })
        .collect::<Vec<_>>();
    for expert in &mut experts {
        expert["insights"] = json!(insights.iter().filter(|i| i["expert_id"] == expert["id"] && i["status"] != "dismissed" && crate::db::source_lifecycle::usable_insight(db.conn(), i)).take(20).map(|i| json!({
            "id":i["id"],"title":i["title"],"observation":crate::cognition::bounded(i["observation"].as_str().unwrap_or(""),1000),
            "status":i["status"],"review_note":crate::cognition::bounded(i["review_note"].as_str().unwrap_or(""),500),"insight_revision":i["insight_revision"]
        })).collect::<Vec<_>>());
        expert["insights_omitted"] = json!(insights
            .iter()
            .filter(|i| i["expert_id"] == expert["id"] && i["status"] != "dismissed")
            .count()
            .saturating_sub(20));
    }
    Ok(experts)
}

pub fn validate_payload(payload: &Value) -> Result<(), String> {
    if !payload["expert_id"].as_i64().is_some_and(|id| id > 0) {
        return Err("请明确选择一位专家；姓名有歧义时先核对机构和科室".into());
    }
    for field in FIELDS {
        if let Some(value) = payload.get(*field).filter(|v| !v.is_null()) {
            if !value.as_str().is_some_and(|s| s.chars().count() <= 12000) {
                return Err("洞察字段应为不超过一万二千字的文本".into());
            }
        }
    }
    if payload.get("status").is_some() || payload.get("review_note").is_some() {
        return Err("秘书不能代替用户标记洞察审阅结论".into());
    }
    if let Some(categories) = payload.get("categories") {
        if !categories.as_array().is_some_and(|items| {
            !items.is_empty()
                && items.len() <= 12
                && items.iter().all(|v| {
                    v.as_str()
                        .is_some_and(|s| !s.trim().is_empty() && s.chars().count() <= 80)
                })
        }) {
            return Err("洞察类别应为非空的简短文本列表".into());
        }
    }
    Ok(())
}

pub fn validate_generated(
    p: &mut ProposalContract,
    snapshot: &AnalysisSnapshot,
) -> Result<(), String> {
    let expert = snapshot
        .expert_catalog
        .iter()
        .find(|e| e["id"] == p.payload["expert_id"])
        .ok_or("专家未出现在本次资料中，请先澄清身份")?;
    let input = serde_json::to_value(snapshot).map_err(|_| "无法核验专家依据")?;
    let evidence = &p.payload["field_evidence"]["expert_id"];
    let path = evidence["snapshot_path"]
        .as_str()
        .ok_or("专家归属缺少原话依据")?;
    let quote = evidence["quote"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or("专家归属缺少原话")?;
    let human_path = path == "/focused_inbox/content"
        || (path.starts_with("/brief/inbox/") && path.ends_with("/content"))
        || (path.starts_with("/expert_context/") && path.ends_with("/content"));
    if !human_path
        || evidence["basis"] != "explicit"
        || !input
            .pointer(path)
            .and_then(Value::as_str)
            .is_some_and(|text| text.contains(quote))
    {
        return Err("专家归属必须引用本次提供的原话或交流记录".into());
    }
    let parent = input
        .pointer(path.rsplit_once('/').unwrap().0)
        .unwrap_or(&Value::Null);
    let direct_note = path.starts_with("/expert_context/")
        && parent["expert_id"] == expert["id"]
        && parent["source_type"] != "kol_insight";
    if !direct_note {
        let named: Vec<_> = snapshot
            .expert_catalog
            .iter()
            .filter(|e| {
                let name = e["name"].as_str().unwrap_or("");
                !name.is_empty() && quote.contains(name)
            })
            .collect();
        let candidates: Vec<_> = named
            .iter()
            .filter(|e| {
                named.len() == 1
                    || ["institution", "department"].iter().any(|key| {
                        e[*key]
                            .as_str()
                            .is_some_and(|s| !s.is_empty() && quote.contains(s))
                            && named.iter().filter(|other| other[*key] == e[*key]).count() == 1
                    })
            })
            .collect();
        if candidates.len() != 1 || candidates[0]["id"] != expert["id"] {
            return Err("专家姓名存在歧义，请保留在收件箱并询问机构或科室，不可猜测归属".into());
        }
    }
    let source_kind = if path.starts_with("/expert_context/") {
        "kol_note"
    } else {
        "inbox"
    };
    let source_id = parent["id"].as_i64().or(parent["entity_id"].as_i64());
    if !p
        .source_refs
        .iter()
        .any(|r| r["source_type"] == source_kind && r["entity_id"].as_i64() == source_id)
    {
        return Err("专家洞察建议缺少对应原话来源".into());
    }
    p.payload["expert_revision"] = expert["revision"].clone();
    if p.operation == "update" {
        let items = expert["insights"].as_array().ok_or("缺少已有洞察目录")?;
        let current = items
            .iter()
            .find(|i| i["id"].as_i64() == p.target_id)
            .ok_or("洞察不存在、已移除或属于其他专家")?;
        let user_text = parent["content"].as_str().unwrap_or("");
        if items.len() > 1
            && !current["title"]
                .as_str()
                .is_some_and(|s| !s.is_empty() && user_text.contains(s))
        {
            return Err("这位专家有多条洞察，请明确要调整哪一条".into());
        }
        p.payload["insight_revision"] = current["insight_revision"].clone();
        p.title = current["title"].as_str().unwrap_or(&p.title).to_string();
    } else if !p.payload["observation"]
        .as_str()
        .is_some_and(|s| !s.trim().is_empty())
        || p.payload.get("categories").is_none()
    {
        return Err("新增专家洞察需要有依据的观察内容和类别".into());
    }
    Ok(())
}

pub fn confirm(conn: &Connection, p: &crate::db::ai::AiProposal, payload: &Value) -> DbResult<i64> {
    validate_payload(payload).map_err(DbError::Migration)?;
    let expert = payload["expert_id"].as_i64().unwrap();
    if !conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM kol_experts WHERE id=?1 AND archived=0 AND revision=?2)",
        params![expert, payload["expert_revision"].as_i64()],
        |r| r.get::<_, bool>(0),
    )? {
        return Err(DbError::Migration(
            "专家资料已变更或已删除，请刷新后重新选择专家".into(),
        ));
    }
    crate::db::work::validate_project_scope(conn, "task", p.work_id)?;
    let old = if p.operation == "update" {
        let item = knowledge::rows(
            conn,
            "SELECT * FROM kol_insights WHERE id=?1 AND expert_id=?2 AND status!='dismissed'",
            &[&p.target_id, &expert],
        )?
        .into_iter()
        .next()
        .ok_or_else(|| DbError::Migration("洞察已删除、搁置或属于其他专家，请刷新".into()))?;
        if payload["insight_revision"].as_str() != Some(revision(&item).as_str()) {
            return Err(DbError::Migration(
                "洞察已被修改，请刷新后重新审阅，未覆盖现有内容".into(),
            ));
        }
        Some(item)
    } else {
        None
    };
    let mut citations: Vec<Value> = old
        .as_ref()
        .and_then(|i| i["citations_json"].as_str())
        .and_then(|raw| serde_json::from_str(raw).ok())
        .unwrap_or_default();
    let mut pack = knowledge::EvidencePack {
        as_of: now_unix(),
        ..Default::default()
    };
    let refs: Vec<Value> = serde_json::from_str(&p.source_refs_json)
        .map_err(|_| DbError::Migration("原话来源记录无效".into()))?;
    for source in refs {
        let kind = source["source_type"].as_str().unwrap_or("");
        let Some(id) = source["entity_id"].as_i64() else {
            continue;
        };
        let table = match kind {
            "inbox" => "inbox_items",
            "kol_note" => "kol_notes",
            _ => continue,
        };
        if crate::db::source_lifecycle::is_retired(conn, &format!("{kind}:{id}"))? {
            return Err(DbError::Migration("引用的原话已删除，请重新整理".into()));
        }
        let row = knowledge::rows(conn, &format!("SELECT * FROM {table} WHERE id=?1"), &[&id])?
            .into_iter()
            .next()
            .ok_or_else(|| DbError::Migration("引用的原话已删除，请重新整理".into()))?;
        if kind == "kol_note" && row["expert_id"] != expert {
            return Err(DbError::Migration(
                "引用的交流记录属于其他专家，请先核对".into(),
            ));
        }
        let ev = knowledge::evidence(kind, &row, "user_record", "");
        let quote = crate::cognition::bounded(row["content"].as_str().unwrap_or(""), 1200);
        let citation = json!({"source_id":ev.id,"quote":quote,"basis":"fact","location":knowledge::source_location(conn,kind,id)?});
        if !citations.contains(&citation) {
            citations.push(citation);
        }
        pack.sources.push(ev);
    }
    if pack.sources.is_empty() {
        return Err(DbError::Migration("专家洞察需要保留有效原话来源".into()));
    }
    // A partial update retains previous citations and their original evidence.
    // Require a new, live source above before merging historical evidence.
    if let Some(draft) = old.as_ref().and_then(|i| i["draft_id"].as_i64()) {
        let evidence: String = conn.query_row(
            "SELECT evidence_json FROM kol_drafts WHERE id=?1",
            [draft],
            |r| r.get(0),
        )?;
        let previous: knowledge::EvidencePack = serde_json::from_str(&evidence)
            .map_err(|_| DbError::Migration("原洞察的依据无法读取，未修改现有内容".into()))?;
        for source in previous.sources {
            if !pack.sources.iter().any(|s| s.id == source.id) {
                pack.sources.push(source);
            }
        }
        pack.notes = previous.notes;
        pack.omitted = previous.omitted;
    }
    pack.scope_ids = p.work_id.into_iter().collect();
    for source in &pack.sources {
        *pack.counts.entry(source.kind.clone()).or_default() += 1;
    }
    let field = |key: &str| {
        payload[key]
            .as_str()
            .or_else(|| old.as_ref().and_then(|v| v[key].as_str()))
            .unwrap_or("")
            .to_string()
    };
    let observation = field("observation");
    if observation.trim().is_empty() {
        return Err(DbError::Migration("请保留洞察的观察内容".into()));
    }
    let categories = payload
        .get("categories")
        .cloned()
        .or_else(|| {
            old.as_ref()
                .and_then(|i| i["categories_json"].as_str())
                .and_then(|s| serde_json::from_str(s).ok())
        })
        .ok_or_else(|| DbError::Migration("请选择洞察类别".into()))?;
    let now = now_unix();
    conn.execute("INSERT INTO kol_drafts(expert_id,purpose,status,payload_json,evidence_json,created_at,decided_at) VALUES(?1,'organize','confirmed',?2,?3,?4,?4)",params![expert,json!({"summary":"根据用户原话更新专家洞察","citations":citations,"insights":[],"actions":[]}).to_string(),serde_json::to_string(&pack).unwrap(),now])?;
    let draft = conn.last_insert_rowid();
    let id = if let Some(old) = &old {
        let id = old["id"].as_i64().unwrap();
        conn.execute("UPDATE kol_insights SET draft_id=?1,title=?2,categories_json=?3,observation=?4,implication=?5,uncertainty=?6,next_question=?7,citations_json=?8,updated_at=MAX(updated_at+1,?9) WHERE id=?10",params![draft,p.title,categories.to_string(),observation,field("implication"),field("uncertainty"),field("next_question"),json!(citations).to_string(),now,id])?;
        id
    } else {
        conn.execute("INSERT INTO kol_insights(draft_id,expert_id,title,categories_json,observation,implication,uncertainty,next_question,citations_json,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?10)",params![draft,expert,p.title,categories.to_string(),observation,field("implication"),field("uncertainty"),field("next_question"),json!(citations).to_string(),now])?;
        conn.last_insert_rowid()
    };
    if let Some(work) = p.work_id {
        conn.execute(
            "INSERT OR IGNORE INTO kol_projects(expert_id,work_id) VALUES(?1,?2)",
            params![expert, work],
        )?;
    }
    Ok(id)
}
