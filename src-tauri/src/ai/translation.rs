//! Non-persistent, task-routed translation helper.

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum TranslationDirection {
    #[serde(rename = "zh-to-en")]
    ZhToEn,
    #[serde(rename = "en-to-zh")]
    EnToZh,
}

pub fn detect_direction(input: &str) -> TranslationDirection {
    let meaningful = input.chars().filter(|c| c.is_alphanumeric()).count().max(1);
    let han = input
        .chars()
        .filter(|c| ('\u{4e00}'..='\u{9fff}').contains(c))
        .count();
    if han * 5 >= meaningful {
        TranslationDirection::ZhToEn
    } else {
        TranslationDirection::EnToZh
    }
}

pub fn validate_style(style: &str) -> Result<(), String> {
    if ["written", "spoken"].contains(&style) {
        Ok(())
    } else {
        Err("翻译风格必须是 written 或 spoken".into())
    }
}

pub fn validate_output(
    input: &str,
    direction: TranslationDirection,
    output: &str,
) -> Result<String, String> {
    let output = output.trim();
    if output.is_empty() {
        return Err("翻译模型返回了空内容".into());
    }
    let lower = output.to_lowercase();
    let conversation_markers = [
        "我是msa",
        "我是msl",
        "我是ai",
        "作为ai",
        "how can i help",
        "i am an ai",
        "i'm an ai",
        "as an ai",
    ];
    if conversation_markers
        .iter()
        .any(|marker| lower.contains(marker))
    {
        return Err("模型返回了对话内容，未遵守翻译约束".into());
    }
    if output.eq_ignore_ascii_case(input.trim()) {
        return Err("模型没有翻译输入内容".into());
    }
    let han = output
        .chars()
        .filter(|c| ('\u{4e00}'..='\u{9fff}').contains(c))
        .count();
    if direction == TranslationDirection::EnToZh && han == 0 {
        return Err("英译中结果不包含中文".into());
    }
    Ok(output.to_string())
}

pub fn parse_output(
    input: &str,
    direction: TranslationDirection,
    raw: &str,
) -> Result<String, String> {
    let envelope = crate::ai::output::parse_translation(raw).map_err(|error| error.to_string())?;
    let languages_match = match direction {
        TranslationDirection::ZhToEn => {
            envelope.source_language.starts_with("zh") && envelope.target_language.starts_with("en")
        }
        TranslationDirection::EnToZh => {
            envelope.source_language.starts_with("en") && envelope.target_language.starts_with("zh")
        }
    };
    if !languages_match {
        return Err("翻译模型返回的语言方向与输入不一致".into());
    }
    validate_output(input, direction, &envelope.translated_text)
}

pub fn build_request(
    model_id: &str,
    input: &str,
    style: &str,
) -> Result<crate::ai::provider::AiTextRequest, String> {
    validate_style(style)?;
    if model_id.trim().is_empty() {
        return Err("翻译任务未绑定有效模型".into());
    }
    let direction = detect_direction(input);
    let direction_name = match direction {
        TranslationDirection::ZhToEn => "zh_to_en",
        TranslationDirection::EnToZh => "en_to_zh",
    };
    Ok(crate::ai::provider::AiTextRequest {
        model_id: model_id.to_string(),
        system: Some(crate::ai::prompts::build_system_prompt(
            crate::ai::prompts::SecretaryStage::Translation,
            crate::ai::prompts::PromptOptions {
                locale: "zh-CN",
                translation_style: Some(style),
                translation_direction: Some(direction_name),
            },
        )),
        messages: vec![crate::ai::provider::AiMessage {
            role: "user".into(),
            content: serde_json::json!({
                "direction": direction_name,
                "style": style,
                "text": input,
            })
            .to_string(),
        }],
        temperature: Some(0.2),
        max_output_tokens: Some(4000),
        output_format: crate::ai::output::OutputFormat::PromptJson,
        budget: Default::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn direction_and_style_are_deterministic() {
        assert_eq!(detect_direction("这是中文"), TranslationDirection::ZhToEn);
        assert_eq!(
            detect_direction("This is English"),
            TranslationDirection::EnToZh
        );
        assert_eq!(
            detect_direction("中文 English"),
            TranslationDirection::ZhToEn
        );
        assert!(validate_style("written").is_ok());
        assert!(validate_style("casual").is_err());
    }

    #[test]
    fn translation_request_uses_the_resolved_model_id() {
        let request = build_request("muse-spark-1.2-contributor", "你好", "written").unwrap();
        assert_eq!(request.model_id, "muse-spark-1.2-contributor");
        assert_eq!(request.messages.len(), 1);
        let payload: serde_json::Value =
            serde_json::from_str(&request.messages[0].content).unwrap();
        assert_eq!(payload["text"], "你好");
        assert_eq!(payload["direction"], "zh_to_en");
        let system = request.system.unwrap();
        assert!(system.contains("translation engine"));
        assert!(!system.contains("AI secretary"));
    }

    #[test]
    fn translation_output_rejects_conversation_and_accepts_the_target_language() {
        assert!(validate_output("你好", TranslationDirection::ZhToEn, "Hello").is_ok());
        assert!(
            validate_output("你好", TranslationDirection::ZhToEn, "你好，我是MSA助手").is_err()
        );
        assert!(validate_output("hello", TranslationDirection::EnToZh, "你好").is_ok());
        assert!(validate_output(
            "hello",
            TranslationDirection::EnToZh,
            "Hello! How can I help you?"
        )
        .is_err());
    }

    #[test]
    fn translation_envelope_is_parsed_before_behavior_validation() {
        let raw = r#"{"schema_version":"msl.translation.v1","translated_text":"Hello","source_language":"zh","target_language":"en"}"#;
        assert_eq!(
            parse_output("你好", TranslationDirection::ZhToEn, raw).unwrap(),
            "Hello"
        );
        assert!(parse_output("你好", TranslationDirection::ZhToEn, "Hello").is_err());
    }
}
