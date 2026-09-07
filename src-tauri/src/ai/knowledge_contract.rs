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
    pub claims: Vec<Claim>,
    pub gaps: Vec<String>,
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
    let answer: Answer = serde_json::from_str(&super::schema::strip_single_code_fence(raw))
        .map_err(|_| "问答输出格式无效，未保存为成功回答")?;
    if answer.claims.len() > 16
        || answer.gaps.len() > 8
        || (answer.claims.is_empty() && answer.gaps.is_empty())
    {
        return Err("回答条目数量无效".into());
    }
    for claim in &answer.claims {
        text(&claim.text, 2000, true)?;
        if !["fact", "inference"].contains(&claim.basis.as_str()) {
            return Err("回答必须区分事实与推断".into());
        }
        citations(&claim.citations, pack)?;
    }
    for gap in &answer.gaps {
        text(gap, 500, true)?;
    }
    Ok(answer)
}
pub fn parse_kol(raw: &str, pack: &EvidencePack) -> Result<KolOutput, String> {
    let output: KolOutput = serde_json::from_str(&super::schema::strip_single_code_fence(raw))
        .map_err(|_| "专家整理输出格式无效")?;
    text(&output.summary, 2500, true)?;
    citations(&output.citations, pack)?;
    if output.insights.len() > 8 || output.actions.len() > 8 {
        return Err("请优先保留少量有用的洞察和动作".into());
    }
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
            || insight.categories.len() > 3
            || insight.categories.iter().any(|s| {
                !["practice_barrier", "evidence_need", "research_opportunity"].contains(&s.as_str())
            })
        {
            return Err("洞察需要原始观察及有效分类".into());
        }
        citations(&insight.citations, pack)?;
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
    fn knowledge_kol_preserves_three_categories_and_rejects_invented_times() {
        let pack = pack();
        let citation = json!([{"source_id":"kol_note:1","quote":"希望了解长期随访证据"}]);
        let good = json!({"summary":"准备补充长期证据。","citations":citation,"insights":[{"title":"长期证据需求","categories":["practice_barrier","evidence_need","research_opportunity"],"observation":"希望了解长期随访证据","implication":"可能需要补充材料","uncertainty":"具体问题未明","next_question":"您最关心哪类结局？","citations":citation}],"actions":[]});
        assert!(parse_kol(&good.to_string(), &pack).is_ok());
        let mut bad = good.clone();
        bad["insights"][0]["categories"] = json!(["prescribing_potential"]);
        assert!(parse_kol(&bad.to_string(), &pack).is_err());
        let mut bad = good;
        bad["actions"] = json!([{"enabled":true,"kind":"calendar","title":"交流","work_id":null,"notes":"","waiting_for":"","at":null,"time_basis":"unknown","time_reason":"","citations":citation}]);
        assert!(parse_kol(&bad.to_string(), &pack).is_err());
    }
}
