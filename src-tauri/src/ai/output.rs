use serde::{Deserialize, Serialize};

const MAX_OUTPUT_BYTES: usize = 2 * 1024 * 1024;
const MAX_NESTING_DEPTH: usize = 64;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Basis {
    Fact,
    Inference,
    Suggestion,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Citation {
    pub source_id: String,
    pub quote: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Statement {
    pub text: String,
    pub basis: Basis,
    pub citations: Vec<Citation>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Block {
    Paragraph {
        content: Statement,
    },
    Bullets {
        items: Vec<Statement>,
    },
    Numbered {
        items: Vec<Statement>,
    },
    Table {
        columns: Vec<String>,
        rows: Vec<Vec<Statement>>,
    },
}

impl Block {
    pub fn statements(&self) -> Vec<&Statement> {
        match self {
            Self::Paragraph { content } => vec![content],
            Self::Bullets { items } | Self::Numbered { items } => items.iter().collect(),
            Self::Table { rows, .. } => rows.iter().flatten().collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Section {
    pub title: String,
    pub blocks: Vec<Block>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AiDocument {
    pub schema_version: String,
    pub title: String,
    pub sections: Vec<Section>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TranslationOutput {
    pub schema_version: String,
    pub translated_text: String,
    pub source_language: String,
    pub target_language: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputError {
    pub code: &'static str,
    pub message: String,
}

impl std::fmt::Display for OutputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for OutputError {}

fn error(code: &'static str, message: impl Into<String>) -> OutputError {
    OutputError {
        code,
        message: message.into(),
    }
}

fn json_depth(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Array(values) => 1 + values.iter().map(json_depth).max().unwrap_or(0),
        serde_json::Value::Object(values) => 1 + values.values().map(json_depth).max().unwrap_or(0),
        _ => 1,
    }
}

fn parse_bounded<T: for<'de> Deserialize<'de>>(raw: &str) -> Result<T, OutputError> {
    if raw.len() > MAX_OUTPUT_BYTES {
        return Err(error(
            "AI_OUTPUT_TOO_LARGE",
            "模型输出超过 2 MiB，已保留任务供分段继续处理",
        ));
    }
    let cleaned = crate::ai::schema::strip_single_code_fence(raw);
    let value: serde_json::Value = serde_json::from_str(&cleaned)
        .map_err(|_| error("AI_OUTPUT_INVALID_JSON", "模型没有返回可验证的 JSON"))?;
    if json_depth(&value) > MAX_NESTING_DEPTH {
        return Err(error(
            "AI_OUTPUT_TOO_DEEP",
            "模型输出结构过深，无法安全读取",
        ));
    }
    serde_json::from_value(value).map_err(|_| {
        error(
            "AI_OUTPUT_SCHEMA_MISMATCH",
            "模型输出字段与当前阅读格式不一致",
        )
    })
}

pub fn parse_document(raw: &str) -> Result<AiDocument, OutputError> {
    let document: AiDocument = parse_bounded(raw)?;
    if document.schema_version != "msl.readable.v1" {
        return Err(error(
            "AI_OUTPUT_VERSION_UNSUPPORTED",
            "模型输出版本无法读取",
        ));
    }
    if document.title.trim().is_empty() || document.sections.is_empty() {
        return Err(error("AI_OUTPUT_EMPTY", "模型没有返回可供阅读的内容"));
    }
    for section in &document.sections {
        if section.title.trim().is_empty() || section.blocks.is_empty() {
            return Err(error("AI_OUTPUT_EMPTY_SECTION", "模型返回了空白内容分组"));
        }
        for block in &section.blocks {
            if let Block::Table { columns, rows } = block {
                if columns.is_empty()
                    || rows.is_empty()
                    || rows.iter().any(|row| row.len() != columns.len())
                {
                    return Err(error("AI_OUTPUT_TABLE_INVALID", "模型返回的表格列数不一致"));
                }
            }
            let statements = block.statements();
            if statements.is_empty() {
                return Err(error("AI_OUTPUT_EMPTY_BLOCK", "模型返回了空白内容块"));
            }
            for statement in statements {
                if statement.text.trim().is_empty() {
                    return Err(error("AI_OUTPUT_EMPTY_TEXT", "模型返回了空白文字"));
                }
                if matches!(statement.basis, Basis::Fact | Basis::Inference)
                    && statement.citations.is_empty()
                {
                    return Err(error(
                        "AI_OUTPUT_SOURCE_REQUIRED",
                        "事实和推断需要可核对的来源",
                    ));
                }
                for citation in &statement.citations {
                    if citation.source_id.trim().is_empty() || citation.quote.trim().is_empty() {
                        return Err(error(
                            "AI_OUTPUT_SOURCE_INVALID",
                            "模型返回的来源标识或摘录为空",
                        ));
                    }
                }
            }
        }
    }
    Ok(document)
}

pub fn parse_translation(raw: &str) -> Result<TranslationOutput, OutputError> {
    let translation: TranslationOutput = parse_bounded(raw)?;
    if translation.schema_version != "msl.translation.v1"
        || translation.translated_text.trim().is_empty()
        || translation.source_language.trim().is_empty()
        || translation.target_language.trim().is_empty()
        || translation.source_language == translation.target_language
    {
        return Err(error(
            "AI_TRANSLATION_SCHEMA_INVALID",
            "翻译输出缺少必要信息",
        ));
    }
    Ok(translation)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttemptKind {
    Initial,
    TransportRetry,
    FormatFallback,
    FormatRepair,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputFormat {
    #[default]
    Text,
    PromptJson,
    JsonObject,
    JsonSchema {
        name: String,
        schema: serde_json::Value,
    },
}

pub fn apply_output_format(
    protocol: &str,
    body: &mut serde_json::Value,
    format: &OutputFormat,
) -> Result<(), OutputError> {
    match format {
        OutputFormat::Text | OutputFormat::PromptJson => Ok(()),
        OutputFormat::JsonObject => match protocol {
            "chat_completions" => {
                body["response_format"] = serde_json::json!({"type":"json_object"});
                Ok(())
            }
            _ => Err(error(
                "AI_OUTPUT_FORMAT_UNSUPPORTED",
                "当前协议没有启用 JSON Object 参数",
            )),
        },
        OutputFormat::JsonSchema { name, schema } => {
            if name.trim().is_empty() || !schema.is_object() {
                return Err(error(
                    "AI_OUTPUT_FORMAT_INVALID",
                    "结构化输出名称或 Schema 无效",
                ));
            }
            match protocol {
                "chat_completions" => {
                    body["response_format"] = serde_json::json!({
                        "type":"json_schema",
                        "json_schema":{"name":name,"strict":true,"schema":schema}
                    });
                }
                "responses" => {
                    body["text"]["format"] = serde_json::json!({
                        "type":"json_schema","name":name,"strict":true,"schema":schema
                    });
                }
                "anthropic_messages" => {
                    body["output_config"]["format"] = serde_json::json!({
                        "type":"json_schema","schema":schema
                    });
                }
                _ => {
                    return Err(error(
                        "AI_OUTPUT_FORMAT_UNSUPPORTED",
                        "当前模型协议不支持结构化输出参数",
                    ))
                }
            }
            Ok(())
        }
    }
}

#[derive(Clone, Debug)]
pub struct RequestBudget {
    max_http: u8,
    max_repairs: u8,
    used: u8,
    repairs: u8,
}
impl Default for RequestBudget {
    fn default() -> Self {
        Self::new(3, 1)
    }
}

impl RequestBudget {
    pub fn new(max_http: u8, max_repairs: u8) -> Self {
        Self {
            max_http,
            max_repairs,
            used: 0,
            repairs: 0,
        }
    }

    pub fn claim(&mut self, kind: AttemptKind) -> Result<(), OutputError> {
        if self.used >= self.max_http {
            return Err(error(
                "AI_REQUEST_BUDGET_EXHAUSTED",
                "本次任务已达到请求次数上限",
            ));
        }
        if kind == AttemptKind::FormatRepair {
            if self.repairs >= self.max_repairs {
                return Err(error(
                    "AI_FORMAT_REPAIR_EXHAUSTED",
                    "本次任务已经尝试过格式修复",
                ));
            }
            self.repairs += 1;
        }
        self.used += 1;
        Ok(())
    }

    pub fn used(&self) -> u8 {
        self.used
    }
}

pub fn to_plain_text(document: &AiDocument) -> String {
    let mut output = String::new();
    output.push_str(&document.title);
    for section in &document.sections {
        output.push_str("\n\n");
        output.push_str(&section.title);
        for block in &section.blocks {
            match block {
                Block::Paragraph { content } => {
                    output.push('\n');
                    output.push_str(&content.text);
                }
                Block::Bullets { items } => {
                    for item in items {
                        output.push_str("\n- ");
                        output.push_str(&item.text);
                    }
                }
                Block::Numbered { items } => {
                    for (index, item) in items.iter().enumerate() {
                        output.push_str(&format!("\n{}. {}", index + 1, item.text));
                    }
                }
                Block::Table { columns, rows } => {
                    output.push('\n');
                    output.push_str(&columns.join(" | "));
                    for row in rows {
                        output.push('\n');
                        output.push_str(
                            &row.iter()
                                .map(|cell| cell.text.as_str())
                                .collect::<Vec<_>>()
                                .join(" | "),
                        );
                    }
                }
            }
        }
    }
    output
}
