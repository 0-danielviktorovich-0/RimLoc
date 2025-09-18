use rimloc_core::{Result, TransUnit, RimLocError};
use walkdir::WalkDir;
use std::path::{Path, PathBuf};
use quick_xml::Reader;
use quick_xml::events::Event;

/// Сканирует `root`, находит `.xml` и извлекает строки + реальный номер строки.
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

/// Построить список стартовых позиций строк (байтовые оффсеты).
fn line_starts_of(text: &str) -> Vec<usize> {
    let mut starts = Vec::with_capacity(256);
    starts.push(0);
    for (i, b) in text.as_bytes().iter().enumerate() {
        if *b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

/// Перевести байтовую позицию в номер строки (1-based).
fn byte_pos_to_line(pos: usize, starts: &[usize]) -> u32 {
    // количество стартов строк, которые <= pos
    let idx = starts.partition_point(|&s| s <= pos);
    (idx as u32).max(1)
}

fn extract_with_lines(xml: &str, path: &Path) -> Result<Vec<TransUnit>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let line_starts = line_starts_of(xml);

    let mut buf = Vec::new();
    let mut out = Vec::new();
    let mut tag_stack: Vec<String> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                tag_stack.push(String::from_utf8_lossy(e.name().as_ref()).to_string());
            }
            Ok(Event::End(_)) => {
                tag_stack.pop();
            }
            Ok(Event::Text(e)) => {
                if !tag_stack.is_empty() {
                    let mut key = tag_stack.join(".");

                    // Для классических Keyed-файлов красивее без префикса LanguageData.
                    if let Some(stripped) = key.strip_prefix("LanguageData.") {
                        key = stripped.to_string();
                    }

                    let value = e.unescape().unwrap_or_default().to_string();
                    let value = value.trim();
                    if !value.is_empty() {
                        let byte_pos = reader.buffer_position() as usize;
                        let line_no = byte_pos_to_line(byte_pos, &line_starts);

                        out.push(TransUnit {
                            key,
                            source: Some(value.to_string()),
                            path: PathBuf::from(path),
                            line: Some(line_no),
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
