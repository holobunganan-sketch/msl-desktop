use std::path::Path;

pub fn extract(path: &Path) -> Result<String, String> {
    guard_parser(|| pdf_extract::extract_text(path).map_err(|error| error.to_string()))
}

fn guard_parser(parser: impl FnOnce() -> Result<String, String>) -> Result<String, String> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(parser)) {
        Ok(result) => result,
        Err(_) => Err("PDF 解析器无法处理该文件".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_panic_becomes_a_parse_error() {
        let result =
            guard_parser(|| -> Result<String, String> { panic!("synthetic malformed PDF panic") });

        assert_eq!(result.unwrap_err(), "PDF 解析器无法处理该文件");
    }
}
