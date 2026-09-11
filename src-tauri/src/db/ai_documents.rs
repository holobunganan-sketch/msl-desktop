use crate::ai::output::{AiDocument, Basis, Block, Section, Statement};
use crate::db::{now_unix, DbError, DbResult};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const READABLE_SCHEMA_HASH: &str = "msl-readable-v1-20260910";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredAiDocument {
    pub owner_kind: String,
    pub owner_id: String,
    pub schema_version: String,
    pub document: AiDocument,
    pub revision: i64,
    pub updated_at: i64,
}

pub fn save_document(
    conn: &Connection,
    owner_kind: &str,
    owner_id: &str,
    document: &AiDocument,
    input_fingerprint: &str,
    prompt_version: &str,
    model_description: &str,
) -> DbResult<()> {
    if owner_kind.trim().is_empty() || owner_id.trim().is_empty() {
        return Err(DbError::Migration("AI 文档缺少归属".into()));
    }
    let raw = serde_json::to_string(document)
        .map_err(|_| DbError::Migration("AI 文档无法保存".into()))?;
    let now = now_unix();
    conn.execute(
        "INSERT INTO ai_readable_documents(owner_kind,owner_id,schema_version,schema_hash,document_json,input_fingerprint,prompt_version,model_description,created_at,updated_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?9) ON CONFLICT(owner_kind,owner_id) DO UPDATE SET schema_version=excluded.schema_version,schema_hash=excluded.schema_hash,document_json=excluded.document_json,input_fingerprint=excluded.input_fingerprint,prompt_version=excluded.prompt_version,model_description=excluded.model_description,revision=ai_readable_documents.revision+1,updated_at=excluded.updated_at",
        params![owner_kind.trim(),owner_id.trim(),document.schema_version,READABLE_SCHEMA_HASH,raw,input_fingerprint,prompt_version,model_description,now],
    )?;
    Ok(())
}

pub fn get_document(
    conn: &Connection,
    owner_kind: &str,
    owner_id: &str,
) -> DbResult<Option<StoredAiDocument>> {
    conn.query_row(
        "SELECT owner_kind,owner_id,schema_version,document_json,revision,updated_at FROM ai_readable_documents WHERE owner_kind=?1 AND owner_id=?2",
        params![owner_kind, owner_id],
        |row| {
            let raw: String = row.get(3)?;
            let document = serde_json::from_str(&raw).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    3,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
            Ok(StoredAiDocument {
                owner_kind: row.get(0)?,
                owner_id: row.get(1)?,
                schema_version: row.get(2)?,
                document,
                revision: row.get(4)?,
                updated_at: row.get(5)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

pub fn input_fingerprint(input: &[u8]) -> String {
    format!("{:x}", Sha256::digest(input))
}

pub fn read_legacy(title: &str, raw: &str) -> AiDocument {
    AiDocument {
        schema_version: "msl.readable.v1".into(),
        title: if title.trim().is_empty() {
            "历史内容".into()
        } else {
            title.trim().into()
        },
        sections: vec![Section {
            title: "原始记录".into(),
            blocks: vec![Block::Paragraph {
                content: Statement {
                    text: raw.trim().to_string(),
                    basis: Basis::Unknown,
                    citations: Vec::new(),
                },
            }],
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_document_roundtrips_without_losing_open_sections() {
        let db = crate::db::Database::open_in_memory().unwrap();
        let mut document = read_legacy("合成报告", "第一段");
        document.sections.push(Section {
            title: "自由主题".into(),
            blocks: vec![Block::Bullets {
                items: vec![Statement {
                    text: "跨团队知识整理".into(),
                    basis: Basis::Suggestion,
                    citations: Vec::new(),
                }],
            }],
        });
        save_document(
            db.conn(),
            "report",
            "1",
            &document,
            "input",
            "prompt",
            "synthetic",
        )
        .unwrap();
        let loaded = get_document(db.conn(), "report", "1").unwrap().unwrap();
        assert_eq!(loaded.document, document);
        assert_eq!(loaded.revision, 1);
        assert!(crate::ai::output::to_plain_text(&loaded.document).contains("跨团队知识整理"));
    }
}
