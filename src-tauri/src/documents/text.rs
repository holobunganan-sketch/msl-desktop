use std::path::Path;

use super::ExtractStatus;

pub fn read(path: &Path) -> Result<String, (ExtractStatus, String)> {
    let bytes =
        std::fs::read(path).map_err(|error| (ExtractStatus::FailedParse, error.to_string()))?;
    if bytes.contains(&0) {
        return Err((ExtractStatus::Unsupported, "检测到二进制 NUL 字节".into()));
    }
    let (text, had_errors) = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        (String::from_utf8_lossy(&bytes[3..]).into_owned(), false)
    } else if bytes.starts_with(&[0xFF, 0xFE]) {
        let (decoded, _, errors) = encoding_rs::UTF_16LE.decode(&bytes[2..]);
        (decoded.into_owned(), errors)
    } else if bytes.starts_with(&[0xFE, 0xFF]) {
        let (decoded, _, errors) = encoding_rs::UTF_16BE.decode(&bytes[2..]);
        (decoded.into_owned(), errors)
    } else if let Ok(value) = std::str::from_utf8(&bytes) {
        (value.to_string(), false)
    } else {
        let (decoded, _, errors) = encoding_rs::GBK.decode(&bytes);
        (decoded.into_owned(), errors)
    };
    if had_errors
        || text.chars().filter(|value| *value == '\u{FFFD}').count() * 100
            > text.chars().count().max(1)
    {
        return Err((ExtractStatus::FailedEncoding, "文本编码无法可靠解码".into()));
    }
    Ok(text)
}
