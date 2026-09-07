#[derive(Debug, Clone, serde::Serialize)]
pub struct DocumentChunk {
    pub index: usize,
    pub start: usize,
    pub end: usize,
    pub text: String,
    pub source_hash: String,
}

pub fn chunk(text: &str, source_hash: &str) -> Vec<DocumentChunk> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return Vec::new();
    }
    let mut start = 0;
    let mut index = 0;
    let mut chunks = Vec::new();
    while start < chars.len() {
        let end = (start + 6_000).min(chars.len());
        chunks.push(DocumentChunk {
            index,
            start,
            end,
            text: chars[start..end].iter().collect(),
            source_hash: source_hash.into(),
        });
        if end == chars.len() {
            break;
        }
        start = end.saturating_sub(300);
        index += 1;
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_have_fixed_size_and_overlap() {
        let text = "a".repeat(12_000);
        let values = chunk(&text, "hash");
        assert_eq!(values[0].text.len(), 6_000);
        assert_eq!(values[1].start, 5_700);
        assert_eq!(values[1].source_hash, "hash");
    }
}
