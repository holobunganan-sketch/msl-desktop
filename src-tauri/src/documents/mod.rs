//! 受支持文档的本地正文提取；正文只返回给调用方，不写日志。

use std::path::Path;

use serde::Serialize;
use sha2::{Digest, Sha256};

pub mod chunk;
pub mod docx;
pub mod indexer;
pub mod pdf;
pub mod text;

pub const MAX_FILE_BYTES: u64 = 50 * 1024 * 1024;
pub const MAX_TEXT_CHARS: usize = 2_000_000;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExtractStatus {
    Ready,
    Unsupported,
    TooLarge,
    NeedsOcr,
    FailedParse,
    FailedEncoding,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExtractedDocument {
    pub text: String,
    pub char_count: usize,
    pub truncated: bool,
    pub content_hash: String,
    pub language_hint: Option<String>,
    pub warnings: Vec<String>,
    pub status: ExtractStatus,
}

fn hash_text(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn finish(text: String, mut warnings: Vec<String>) -> ExtractedDocument {
    let original_len = text.chars().count();
    let (text, truncated) = if original_len > MAX_TEXT_CHARS {
        let head: String = text.chars().take(1_500_000).collect();
        let tail: String = text.chars().skip(original_len - 500_000).collect();
        warnings.push("正文超过 2,000,000 字符，已按固定规则截断".into());
        (format!("{head}\n...[truncated]...\n{tail}"), true)
    } else {
        (text, false)
    };
    let char_count = text.chars().count();
    ExtractedDocument {
        content_hash: hash_text(&text),
        text,
        char_count,
        truncated,
        language_hint: None,
        warnings,
        status: ExtractStatus::Ready,
    }
}

pub fn extract_path(path: &Path) -> ExtractedDocument {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(error) => {
            return ExtractedDocument {
                text: String::new(),
                char_count: 0,
                truncated: false,
                content_hash: String::new(),
                language_hint: None,
                warnings: vec![error.to_string()],
                status: ExtractStatus::FailedParse,
            }
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return ExtractedDocument {
            text: String::new(),
            char_count: 0,
            truncated: false,
            content_hash: String::new(),
            language_hint: None,
            warnings: vec!["不是普通文件".into()],
            status: ExtractStatus::Unsupported,
        };
    }
    if metadata.len() > MAX_FILE_BYTES {
        return ExtractedDocument {
            text: String::new(),
            char_count: 0,
            truncated: false,
            content_hash: String::new(),
            language_hint: None,
            warnings: vec!["文件超过 50 MiB 上限".into()],
            status: ExtractStatus::TooLarge,
        };
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match extension.as_str() {
        "docx" => match docx::extract(path) {
            Ok(text) => finish(text, Vec::new()),
            Err(error) => ExtractedDocument {
                text: String::new(),
                char_count: 0,
                truncated: false,
                content_hash: String::new(),
                language_hint: None,
                warnings: vec![error],
                status: ExtractStatus::FailedParse,
            },
        },
        "pdf" => match pdf::extract(path) {
            Ok(text) if !text.trim().is_empty() => finish(text, Vec::new()),
            Ok(_) => ExtractedDocument {
                text: String::new(),
                char_count: 0,
                truncated: false,
                content_hash: String::new(),
                language_hint: None,
                warnings: vec!["PDF 没有文字层，需 OCR".into()],
                status: ExtractStatus::NeedsOcr,
            },
            Err(error) => ExtractedDocument {
                text: String::new(),
                char_count: 0,
                truncated: false,
                content_hash: String::new(),
                language_hint: None,
                warnings: vec![error],
                status: ExtractStatus::FailedParse,
            },
        },
        "txt" | "md" | "markdown" | "csv" | "json" | "jsonl" | "log" | "yaml" | "yml" | "toml"
        | "xml" | "html" | "htm" | "sql" | "py" | "rs" | "ts" | "js" | "svelte" => {
            match text::read(path) {
                Ok(value) => finish(value, Vec::new()),
                Err((status, error)) => ExtractedDocument {
                    text: String::new(),
                    char_count: 0,
                    truncated: false,
                    content_hash: String::new(),
                    language_hint: None,
                    warnings: vec![error],
                    status,
                },
            }
        }
        "" if metadata.len() < 1_048_576 => match text::read(path) {
            Ok(value) => finish(value, Vec::new()),
            Err((status, error)) => ExtractedDocument {
                text: String::new(),
                char_count: 0,
                truncated: false,
                content_hash: String::new(),
                language_hint: None,
                warnings: vec![error],
                status,
            },
        },
        _ => ExtractedDocument {
            text: String::new(),
            char_count: 0,
            truncated: false,
            content_hash: String::new(),
            language_hint: None,
            warnings: vec!["文件类型未在正文白名单中".into()],
            status: ExtractStatus::Unsupported,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_and_unsupported_statuses_are_stable() {
        let root = std::env::temp_dir().join(format!("msl-docs-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.txt"), "alpha\n中文").unwrap();
        std::fs::write(root.join("a.bin"), [0_u8, 1, 2, 0]).unwrap();
        assert_eq!(
            extract_path(&root.join("a.txt")).status,
            ExtractStatus::Ready
        );
        assert_eq!(
            extract_path(&root.join("a.bin")).status,
            ExtractStatus::Unsupported
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn long_text_uses_fixed_truncation_and_hash() {
        let root = std::env::temp_dir().join(format!("msl-docs-long-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("long.txt"), "x".repeat(MAX_TEXT_CHARS + 100)).unwrap();
        let result = extract_path(&root.join("long.txt"));
        assert!(result.truncated);
        assert!(result.text.contains("truncated"));
        assert_eq!(result.content_hash.len(), 64);
        let _ = std::fs::remove_dir_all(root);
    }
}
