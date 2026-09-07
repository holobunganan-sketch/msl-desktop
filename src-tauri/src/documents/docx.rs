use std::io::Read;
use std::path::Path;

pub fn extract(path: &Path) -> Result<String, String> {
    let file = std::fs::File::open(path).map_err(|error| error.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|error| error.to_string())?;
    let mut output = String::new();
    for name in [
        "word/document.xml",
        "word/header1.xml",
        "word/footer1.xml",
        "word/footnotes.xml",
        "word/endnotes.xml",
    ] {
        let Ok(mut entry) = archive.by_name(name) else {
            continue;
        };
        let mut xml = String::new();
        entry
            .read_to_string(&mut xml)
            .map_err(|error| error.to_string())?;
        let mut reader = quick_xml::Reader::from_str(&xml);
        reader.config_mut().trim_text(true);
        let mut buffer = Vec::new();
        loop {
            match reader.read_event_into(&mut buffer) {
                Ok(quick_xml::events::Event::Start(event))
                    if event.name().as_ref().ends_with(b"p") =>
                {
                    output.push('\n')
                }
                Ok(quick_xml::events::Event::Text(event)) => {
                    output.push_str(&event.unescape().map_err(|error| error.to_string())?)
                }
                Ok(quick_xml::events::Event::Empty(event))
                    if event.name().as_ref().ends_with(b"tab") =>
                {
                    output.push('\t')
                }
                Ok(quick_xml::events::Event::Eof) => break,
                Err(error) => return Err(error.to_string()),
                _ => {}
            }
            buffer.clear();
        }
    }
    Ok(output)
}
