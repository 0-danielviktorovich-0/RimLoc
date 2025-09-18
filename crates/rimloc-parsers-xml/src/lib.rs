use rimloc_core::{Result, TransUnit, RimLocError};
use walkdir::WalkDir;
use std::path::{Path, PathBuf};
use quick_xml::Reader;
use quick_xml::events::Event;

/// Рекурсивно проходит по `root`, находит `.xml` и извлекает Keyed-строки + line.
pub fn scan_keyed_xml(root: &Path) -> Result<Vec<TransUnit>> {
    let mut out: Vec<TransUnit> = Vec::new();

    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().map(|e| e == "xml").unwrap_or(false) {
            if let Ok(txt) = std::fs::read_to_string(path) {
                match extract_with_lines(&txt, path) {
                    Ok(mut units) => out.append(&mut units),
                    Err(e) => eprintln!("[rimloc] WARN: {path:?}: {e}"),
                }
            }
        }
    }

    Ok(out)
}

fn extract_with_lines(xml: &str, path: &Path) -> Result<Vec<TransUnit>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut out = Vec::new();
    let mut tag_stack: Vec<String> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                tag_stack.push(
                    String::from_utf8_lossy(e.name().as_ref()).to_string()
                );
            }
            Ok(Event::End(_)) => {
                tag_stack.pop();
            }
            Ok(Event::Text(e)) => {
                if !tag_stack.is_empty() {
                    let key = tag_stack.join(".");
                    let value = e.unescape().unwrap_or_default().to_string();

                    if !value.trim().is_empty() {
                        out.push(TransUnit {
                            key,
                            source: Some(value.trim().to_string()),
                            path: PathBuf::from(path),
                            line: Some(reader.buffer_position() as u32), // позиция в байтах, грубо ≈ строка
                        });
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(RimLocError::Xml(format!("{e}"))),
            _ => {}
        }

        buf.clear();
    }

    Ok(out)
}
