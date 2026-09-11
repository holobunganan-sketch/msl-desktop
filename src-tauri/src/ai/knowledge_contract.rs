use crate::db::knowledge::EvidencePack;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Citation {
    pub source_id: String,
    pub quote: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub text: String,
    pub basis: String,
    pub citations: Vec<Citation>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Answer {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<crate::ai::output::AiDocument>,
    pub claims: Vec<Claim>,
    pub gaps: Vec<String>,
}

impl Answer {
    pub fn to_document(&self, title: &str) -> crate::ai::output::AiDocument {
        if let Some(document) = &self.document {
            return document.clone();
        }
        use crate::ai::output::{
            AiDocument, Basis, Block, Citation as DocCitation, Section, Statement,
        };
        let mut sections = Vec::new();
        if !self.claims.is_empty() {
            sections.push(Section {
                title: "回答".into(),
                blocks: self
                    .claims
                    .iter()
                    .map(|claim| Block::Paragraph {
                        content: Statement {
                            text: claim.text.clone(),
                            basis: match claim.basis.as_str() {
                                "fact" => Basis::Fact,
                                "inference" => Basis::Inference,
                                "suggestion" => Basis::Suggestion,
                                _ => Basis::Unknown,
                            },
                            citations: claim
                                .citations
                                .iter()
                                .map(|citation| DocCitation {
                                    source_id: citation.source_id.clone(),
                                    quote: citation.quote.clone(),
                                })
                                .collect(),
                        },
                    })
                    .collect(),
            });
        }
        if !self.gaps.is_empty() {
            sections.push(Section {
                title: "尚待补充".into(),
                blocks: vec![Block::Bullets {
                    items: self
                        .gaps
                        .iter()
                        .map(|gap| Statement {
                            text: gap.clone(),
                            basis: Basis::Unknown,
                            citations: Vec::new(),
                        })
                        .collect(),
                }],
            });
        }
        AiDocument {
            schema_version: "msl.readable.v1".into(),
            title: if title.trim().is_empty() {
                "工作台回答".into()
            } else {
                title.trim().into()
            },
            sections,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Insight {
    pub title: String,
    pub categories: Vec<String>,
    pub observation: String,
    pub implication: String,
    pub uncertainty: String,
    pub next_question: String,
    pub citations: Vec<Citation>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub enabled: bool,
    pub kind: String,
    pub title: String,
    pub work_id: Option<i64>,
    pub notes: String,
    pub waiting_for: String,
    pub at: Option<i64>,
    pub time_basis: String,
    pub time_reason: String,
    pub citations: Vec<Citation>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KolOutput {
    pub summary: String,
    pub insights: Vec<Insight>,
    pub actions: Vec<Action>,
    pub citations: Vec<Citation>,
}

fn text(value: &str, max: usize, required: bool) -> Result<(), String> {
    if (required && value.trim().is_empty()) || value.chars().count() > max {
        Err("内容为空或超过长度限制".into())
    } else {
        Ok(())
    }
}
pub fn citations(values: &[Citation], pack: &EvidencePack) -> Result<(), String> {
    if values.is_empty() || values.len() > 6 {
        return Err("每项结论需要 1–6 个可核对来源".into());
    }
    for c in values {
        if c.quote.trim().chars().count() < 2 || c.quote.chars().count() > 400 {
            return Err("来源引文长度无效".into());
        }
        let source = pack
            .sources
            .iter()
            .find(|s| s.id == c.source_id)
            .ok_or("引用了未提供的来源")?;
        // JSON escaping must not invalidate a genuine continuous quote from a field.
        fn contains(value: &serde_json::Value, quote: &str) -> bool {
            match value {
                serde_json::Value::String(s) => s.contains(quote),
                serde_json::Value::Array(a) => a.iter().any(|v| contains(v, quote)),
                serde_json::Value::Object(o) => o.values().any(|v| contains(v, quote)),
                _ => false,
            }
        }
        if !source.text.contains(&c.quote)
            && !serde_json::from_str::<serde_json::Value>(&source.text)
                .is_ok_and(|v| contains(&v, &c.quote))
        {
            return Err("引文与原始记录不匹配".into());
        }
    }
    Ok(())
}
pub fn parse_answer(raw: &str, pack: &EvidencePack) -> Result<Answer, String> {
    if raw.len() > 2 * 1024 * 1024 {
        return Err("回答超过安全读取限制，请拆分问题后重试".into());
    }
    let answer: Answer = serde_json::from_str(&super::schema::strip_single_code_fence(raw))
        .map_err(|_| "问答输出格式无效，未保存为成功回答")?;
    if answer.claims.is_empty() && answer.gaps.is_empty() && answer.document.is_none() {
        return Err("回答条目数量无效".into());
    }
    for claim in &answer.claims {
        text(&claim.text, 2000, true)?;
        if !["fact", "inference", "suggestion", "unknown"].contains(&claim.basis.as_str()) {
            return Err("回答必须区分事实、推断、建议和待核实内容".into());
        }
        if !claim.citations.is_empty() || matches!(claim.basis.as_str(), "fact" | "inference") {
            citations(&claim.citations, pack)?;
        }
        if claim.basis == "fact"
            && claim.citations.iter().any(|c| {
                pack.sources
                    .iter()
                    .any(|s| s.id == c.source_id && s.trust == "model_reading")
            })
        {
            return Err("模型解读尚未核验，必须标记为推断".into());
        }
    }
    for gap in &answer.gaps {
        text(gap, 500, true)?;
    }
    if let Some(document) = &answer.document {
        crate::ai::output::parse_document(&serde_json::to_string(document).unwrap())
            .map_err(|e| e.to_string())?;
        for statement in document
            .sections
            .iter()
            .flat_map(|s| &s.blocks)
            .flat_map(|b| b.statements())
        {
            let refs: Vec<_> = statement
                .citations
                .iter()
                .map(|c| Citation {
                    source_id: c.source_id.clone(),
                    quote: c.quote.clone(),
                })
                .collect();
            if !refs.is_empty() {
                citations(&refs, pack)?;
            }
            if statement.basis == crate::ai::output::Basis::Fact
                && refs.iter().any(|c| {
                    pack.sources
                        .iter()
                        .any(|s| s.id == c.source_id && s.trust == "model_reading")
                })
            {
                return Err("模型解读须标为推断，不能作为原始事实".into());
            }
        }
    }
    Ok(answer)
}
pub fn parse_kol(raw: &str, pack: &EvidencePack) -> Result<KolOutput, String> {
    if raw.len() > 2 * 1024 * 1024 {
        return Err("专家整理超过安全读取限制，请分批整理".into());
    }
    let output: KolOutput = serde_json::from_str(&super::schema::strip_single_code_fence(raw))
        .map_err(|_| "专家整理输出格式无效")?;
    text(&output.summary, 2500, true)?;
    citations(&output.citations, pack)?;
    for insight in &output.insights {
        text(&insight.title, 200, true)?;
        for value in [
            &insight.observation,
            &insight.implication,
            &insight.uncertainty,
            &insight.next_question,
        ] {
            text(value, 2000, false)?;
        }
        if insight.observation.trim().is_empty()
            || insight.categories.is_empty()
            || insight
                .categories
                .iter()
                .any(|s| s.trim().is_empty() || s.len() > 256)
        {
            return Err("洞察需要原始观察及有效分类".into());
        }
        citations(&insight.citations, pack)?;
        if insight.citations.iter().any(|c| {
            pack.sources
                .iter()
                .any(|s| s.id == c.source_id && s.trust == "model_reading")
        }) && insight.uncertainty.trim().is_empty()
        {
            return Err("资料模型解读需要明确说明待核实之处".into());
        }
    }
    for action in &output.actions {
        text(&action.title, 200, true)?;
        text(&action.notes, 2000, false)?;
        text(&action.waiting_for, 300, false)?;
        if !["task", "waiting", "calendar", "inbox"].contains(&action.kind.as_str()) {
            return Err("后续动作类型无效".into());
        }
        if action.enabled && action.kind == "calendar" && action.at.is_none() {
            return Err("加入日历需要明确时间；时间未知可改为任务".into());
        }
        if !["explicit", "inferred", "unknown"].contains(&action.time_basis.as_str()) {
            return Err("时间依据无效".into());
        }
        if let Some(at) = action.at {
            if !(946684800..4102444800).contains(&at) || action.time_basis == "unknown" {
                return Err("请检查动作时间和依据".into());
            }
            if action.time_basis == "inferred" && action.time_reason.trim().is_empty() {
                return Err("推算时间需要说明依据".into());
            }
        } else if action.time_basis != "unknown" {
            return Err("缺少时间时应标记为未安排".into());
        }
        text(&action.time_reason, 500, false)?;
        citations(&action.citations, pack)?;
        if !pack.scope_ids.is_empty()
            && action.work_id.is_some_and(|w| !pack.scope_ids.contains(&w))
        {
            return Err("动作超出当前项目范围".into());
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn pack() -> EvidencePack {
        let mut pack = EvidencePack::default();
        pack.sources.push(crate::db::knowledge::evidence(
            "kol_note",
            &json!({"id":1,"content":"希望了解长期随访证据","expert_id":1}),
            "user_record",
            "",
        ));
        pack
    }
    #[test]
    fn knowledge_answer_requires_genuine_quotes_and_discloses_inference() {
        let pack = pack();
        let good = json!({"claims":[{"text":"记录显示专家希望了解长期随访证据。","basis":"fact","citations":[{"source_id":"kol_note:1","quote":"希望了解长期随访证据"}]}],"gaps":[]});
        assert!(parse_answer(&good.to_string(), &pack).is_ok());
        let mut bad = good.clone();
        bad["claims"][0]["citations"][0]["source_id"] = json!("kol_note:999");
        assert!(parse_answer(&bad.to_string(), &pack).is_err());
        bad = good.clone();
        bad["claims"][0]["citations"][0]["quote"] = json!("已经证明疗效更好");
        assert!(parse_answer(&bad.to_string(), &pack).is_err());
        bad = good.clone();
        bad["claims"][0]["basis"] = json!("certain");
        assert!(parse_answer(&bad.to_string(), &pack).is_err());
        assert!(parse_answer(r#"{"claims":[],"gaps":["缺少记录"]}"#, &pack).is_ok());
        assert!(parse_answer(r#"{"claims":[],"gaps":[]}"#, &pack).is_err());
    }
    #[test]
    fn open_answer_preserves_forty_findings_and_unverified_suggestions() {
        let pack = pack();
        let claims:Vec<_>=(0..40).map(|i|json!({"text":format!("开放事项 {i}"),"basis":"fact","citations":[{"source_id":"kol_note:1","quote":"希望了解长期随访证据"}]})).collect();
        let mut value = json!({"claims":claims,"gaps":[]});
        assert_eq!(
            parse_answer(&value.to_string(), &pack)
                .unwrap()
                .claims
                .len(),
            40
        );
        value["claims"] =
            json!([{"text":"建议下次讨论信息交接方式","basis":"suggestion","citations":[]}]);
        assert!(parse_answer(&value.to_string(), &pack).is_ok());
    }
    #[test]
    fn qa_document_tables_are_evidence_checked_and_preserved() {
        let document = json!({"schema_version":"msl.readable.v1","title":"问题梳理","sections":[{"title":"资料比较","blocks":[{"type":"table","columns":["观察"],"rows":[[{"text":"希望了解长期随访证据","basis":"fact","citations":[{"source_id":"kol_note:1","quote":"希望了解长期随访证据"}]}]]}]}]});
        let input = json!({"claims":[],"gaps":[],"document":document});
        let answer = parse_answer(&input.to_string(), &pack()).unwrap();
        assert!(serde_json::to_string(&answer.to_document("test"))
            .unwrap()
            .contains("table"));
        let bad = input.to_string().replace("kol_note:1", "kol_note:999");
        assert!(parse_answer(&bad, &pack()).is_err());
    }
    #[test]
    fn expert_topics_are_open_and_not_capped_at_eight() {
        let citation = json!([{"source_id":"kol_note:1","quote":"希望了解长期随访证据"}]);
        let insights:Vec<_>=(0..12).map(|i|json!({"title":format!("跨团队事项 {i}"),"categories":["知识交接"],"observation":"希望了解长期随访证据","implication":"需要确认","uncertainty":"尚待核实","next_question":"如何交接？","citations":citation})).collect();
        let output =
            json!({"summary":"交流事项","citations":citation,"insights":insights,"actions":[]});
        assert_eq!(
            parse_kol(&output.to_string(), &pack())
                .unwrap()
                .insights
                .len(),
            12
        );
    }
    #[test]
    fn knowledge_kol_preserves_three_categories_and_rejects_invented_times() {
        let pack = pack();
        let citation = json!([{"source_id":"kol_note:1","quote":"希望了解长期随访证据"}]);
        let good = json!({"summary":"准备补充长期证据。","citations":citation,"insights":[{"title":"长期证据需求","categories":["practice_barrier","evidence_need","research_opportunity"],"observation":"希望了解长期随访证据","implication":"可能需要补充材料","uncertainty":"具体问题未明","next_question":"您最关心哪类结局？","citations":citation}],"actions":[]});
        assert!(parse_kol(&good.to_string(), &pack).is_ok());
        let mut bad = good.clone();
        bad["insights"][0]["categories"] = json!([""]);
        assert!(parse_kol(&bad.to_string(), &pack).is_err());
        let mut bad = good;
        bad["actions"] = json!([{"enabled":true,"kind":"calendar","title":"交流","work_id":null,"notes":"","waiting_for":"","at":null,"time_basis":"unknown","time_reason":"","citations":citation}]);
        assert!(parse_kol(&bad.to_string(), &pack).is_err());
    }
}
